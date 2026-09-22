#!/usr/bin/env python3
"""Speak arbitrary text in a voice pack's voice.

The voice packs are a few short recordings, so nothing in them can say a sentence that was
not recorded. F5-TTS clones a voice from a single reference clip without any training, which
is enough to read out answers and command results in the assistant's own voice.

Two modes:
  --phrases FILE --out DIR   generate a phrase bank ahead of time (no runtime dependency)
  --serve                    keep the model loaded and synthesize on demand, one JSON
                             request per line on stdin: {"text": "...", "out": "/tmp/x.wav"}

A phrase file has one phrase per line. Where what the assistant writes and what should be
said differ - it writes "Заряд 82 процентов", speech needs "восемьдесят два процента" -
the line is `written | said out loud`. --phrases can be given more than once.
"""

import argparse
import json
import os
import shutil
import sys
import time
import wave

MODEL_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "model")


def log(*args):
    print(*args, file=sys.stderr, flush=True)


def load(device, nfe_step):
    from f5_tts.api import F5TTS

    started = time.time()
    tts = F5TTS(
        model="F5TTS_v1_Base",
        ckpt_file=os.path.join(MODEL_DIR, "model.safetensors"),
        vocab_file=os.path.join(MODEL_DIR, "vocab.txt"),
        device=device,
    )
    log(f"model loaded in {time.time() - started:.1f}s (device={device}, nfe_step={nfe_step})")
    return tts


# Silence at the ends is dead air in a whole phrase and an audible gap between two
# fragments played back to back, so it is cut off.
#
# The threshold is deliberately low and measured over 10 ms windows: the last syllable of a
# Russian word is unstressed and quiet ("не отвеча-ет"), and cutting it makes the phrase
# say something else.
def trim_silence(wav, sample_rate, threshold=0.005, keep=0.05):
    import numpy as np

    window = max(1, int(0.01 * sample_rate))
    windows = len(wav) // window
    if windows == 0:
        return wav

    energy = np.abs(wav[:windows * window]).reshape(windows, window).max(axis=1)
    loud = energy > threshold * energy.max()
    if not loud.any():
        return wav

    padding = int(keep * sample_rate)
    start = max(0, int(np.argmax(loud)) * window - padding)
    end = min(len(wav), (windows - int(np.argmax(loud[::-1]))) * window + padding)
    return wav[start:end]


def audio_length(path):
    import soundfile as sf

    info = sf.info(path)
    return info.frames / info.samplerate


# How long the model gives itself to say something: the reference's own speaking rate,
# applied to this text. Left to itself it budgets exactly this, and since it also starts
# with a moment of silence, the budget runs out before the last word ("Не понял вопрос"
# comes out as "Не понял"). Hence the headroom - the extra is silence, which is trimmed.
def budget(reference_length, reference_text, text, headroom):
    rate = reference_length / len((reference_text + ". ").encode())
    return reference_length + rate * len(text.encode()) + headroom


def synthesize(tts, reference, reference_text, text, out_path, nfe_step, duration):
    import soundfile as sf

    started = time.time()
    wav, sample_rate, _ = tts.infer(
        ref_file=reference,
        ref_text=reference_text,
        gen_text=text,
        nfe_step=nfe_step,
        fix_duration=duration,
        remove_silence=False,
    )
    wav = trim_silence(wav, sample_rate)
    sf.write(out_path, wav, sample_rate)
    return len(wav) / sample_rate, time.time() - started


# Generating is not deterministic and a phrase now and then still comes out as something
# else, so every clip is listened to with the recognizer the assistant itself uses and
# generated again when it is not its own text.
def listener(model_path):
    import vosk

    vosk.SetLogLevel(-1)
    model = vosk.Model(model_path)

    def heard(path):
        with wave.open(path, "rb") as audio:
            rate = audio.getframerate()
            frame = audio.getsampwidth() * audio.getnchannels()
            speech = audio.readframes(audio.getnframes())

        # a fragment can be half a second long, and the recognizer needs some silence
        # around an utterance to find where it begins and ends
        silence = bytes(int(0.3 * rate) * frame)

        recognizer = vosk.KaldiRecognizer(model, rate)
        recognizer.AcceptWaveform(silence + speech + silence)
        return json.loads(recognizer.FinalResult()).get("text", "")

    return heard


def words(text):
    return text.lower().replace("ё", "е").replace(",", "").replace("?", "").split()


# The recognizer has trouble telling Russian endings apart on a clip this short ("третье"
# comes back as "третья"), which says nothing about the clip. A word counts as heard when
# all but its ending is there.
def sounds_like(expected, candidate):
    if expected == candidate:
        return True
    if len(expected) < 5:
        return False
    common = os.path.commonprefix([expected, candidate])
    return len(common) >= len(expected) - 2


def missing_words(expected, heard):
    # the recognizer also splits words differently than we write them ("вайфай" comes back
    # as "вай фай"), which is not the clip's fault either
    joined = "".join(words(heard))

    return [word for word in words(expected)
            if word not in joined
            and not any(sounds_like(word, candidate) for candidate in words(heard))]


# Headroom to try, in seconds. Generating is not deterministic, so a second attempt is
# worth making on its own; more room helps when a word went missing anyway.
HEADROOM = (0.7, 1.1, 0.5, 1.6, 0.9, 2.4)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--reference", required=True, help="reference recording (wav/mp3) of the voice")
    parser.add_argument("--reference-text", required=True, help="what is said in the reference recording")
    parser.add_argument("--phrases", action="append", metavar="FILE",
                        help="file with one phrase per line; may be given more than once")
    parser.add_argument("--out", help="output directory for --phrases")
    parser.add_argument("--serve", action="store_true", help="read requests from stdin")
    parser.add_argument("--device", default="mps" if sys.platform == "darwin" else "cpu")
    parser.add_argument("--nfe-step", type=int, default=16,
                        help="diffusion steps: 32 best quality, 16 balanced, 8 fastest")
    parser.add_argument("--verify", metavar="VOSK_MODEL",
                        help="transcribe every phrase back with this model and generate it "
                             "again when it does not come out as its own text")
    args = parser.parse_args()

    reference_length = audio_length(args.reference)
    tts = load(args.device, args.nfe_step)

    if args.serve:
        # one request per line; the model stays loaded between them
        print(json.dumps({"ready": True}), flush=True)
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            try:
                request = json.loads(line)
                duration, took = synthesize(
                    tts, args.reference, args.reference_text,
                    request["text"], request["out"], args.nfe_step,
                    budget(reference_length, args.reference_text, request["text"], HEADROOM[0]),
                )
                print(json.dumps({"out": request["out"], "duration": duration, "took": took}), flush=True)
            except Exception as error:  # a bad request must not kill the server
                print(json.dumps({"error": str(error)}), flush=True)
        return

    if not args.phrases or not args.out:
        parser.error("either --serve or --phrases with --out is required")

    os.makedirs(args.out, exist_ok=True)
    manifest = {}

    phrases = []
    for path in args.phrases:
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                written, _, spoken = line.partition("|")
                written = written.strip()
                phrases.append((written, spoken.strip() or written))

    heard = listener(args.verify) if args.verify else None
    unverified = []

    for index, (written, spoken) in enumerate(phrases, start=1):
        name = f"phrase{index:03d}.wav"
        path = os.path.join(args.out, name)

        closest = None  # the attempt that came out best, when none came out right

        for attempt, headroom in enumerate(HEADROOM, start=1):
            duration, took = synthesize(
                tts, args.reference, args.reference_text, spoken, path, args.nfe_step,
                budget(reference_length, args.reference_text, spoken, headroom),
            )
            if heard is None:
                break

            transcript = heard(path)
            missing = missing_words(spoken, transcript)
            if not missing:
                closest = None
                break

            if closest is None or len(missing) < len(closest[0]):
                shutil.copyfile(path, path + ".best")
                closest = (missing, transcript)

            log(f"    attempt {attempt} with {headroom}s of headroom: "
                f"heard {transcript!r}, missing {missing}")

        if closest is not None:
            shutil.move(path + ".best", path)
            unverified.append((name, spoken, closest[1]))
        elif os.path.exists(path + ".best"):
            os.remove(path + ".best")

        manifest[written] = name
        log(f"[{index}/{len(phrases)}] {spoken[:60]!r} -> {name} ({duration:.1f}s audio in {took:.1f}s)")

    # the assistant looks phrases up by their text
    with open(os.path.join(args.out, "phrases.toml"), "w", encoding="utf-8") as handle:
        handle.write("# Generated by scripts/voice-clone/synth.py - phrases spoken in this pack's voice.\n")
        handle.write("# The assistant plays these instead of synthesizing, so they cost nothing at runtime.\n\n")
        for phrase, name in manifest.items():
            escaped = phrase.replace('"', '\\"')
            handle.write(f'"{escaped}" = "{name}"\n')

    log(f"done: {len(manifest)} phrases in {args.out}")

    for name, spoken, transcript in unverified:
        log(f"NOT UNDERSTOOD  {name}  want {spoken!r}  heard {transcript!r}")
    if unverified:
        log(f"{len(unverified)} phrase(s) never came out right - fix them by hand or reword them")


if __name__ == "__main__":
    main()

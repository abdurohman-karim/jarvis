#!/usr/bin/env python3
"""Check that a generated phrase bank is intelligible.

Cloning short fragments is the part that can quietly go wrong: the model rushes a phrase
of two words, swallows an ending, or produces something that is not the text at all. Every
clip is therefore transcribed back with the speech recognizer the assistant itself uses,
and anything that does not come back as its own text is printed.

    venv/bin/python check.py --dir ../../resources/sound/voices/jarvis-remaster/ru/phrases \\
        --phrases phrases-ru.txt --phrases fragments-ru.txt \\
        --model ../../resources/vosk/vosk-model-small-ru-0.22
"""

import argparse
import json
import os
import sys
import tomllib
import wave


def spoken_texts(paths):
    # written text -> how it should be said (see synth.py)
    texts = {}
    for path in paths:
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                written, _, spoken = line.partition("|")
                written = written.strip()
                texts[written] = spoken.strip() or written
    return texts


def transcribe(recognizer_factory, path):
    with wave.open(path, "rb") as audio:
        rate = audio.getframerate()
        frame = audio.getsampwidth() * audio.getnchannels()
        speech = audio.readframes(audio.getnframes())
        duration = audio.getnframes() / rate

    # a fragment can be half a second long, and the recognizer needs some silence around
    # an utterance to find where it begins and ends
    silence = bytes(int(0.3 * rate) * frame)

    recognizer = recognizer_factory(rate)
    recognizer.AcceptWaveform(silence + speech + silence)
    return json.loads(recognizer.FinalResult()).get("text", ""), duration


def words(text):
    return text.lower().replace("ё", "е").replace(",", "").replace("?", "").split()


# The recognizer has trouble telling Russian endings apart on a clip this short ("третье"
# comes back as "третья") and splits words differently than we write them ("вай фай"),
# neither of which says anything about the clip.
def sounds_like(expected, candidate):
    if expected == candidate:
        return True
    if len(expected) < 5:
        return False
    return len(os.path.commonprefix([expected, candidate])) >= len(expected) - 2


def missing_words(expected, heard):
    joined = "".join(words(heard))

    return [word for word in words(expected)
            if word not in joined
            and not any(sounds_like(word, candidate) for candidate in words(heard))]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dir", required=True, help="directory with the generated bank")
    parser.add_argument("--phrases", action="append", required=True, help="the phrase files it was generated from")
    parser.add_argument("--model", required=True, help="Vosk model to transcribe with")
    args = parser.parse_args()

    import vosk
    vosk.SetLogLevel(-1)
    model = vosk.Model(args.model)

    with open(os.path.join(args.dir, "phrases.toml"), "rb") as handle:
        manifest = tomllib.load(handle)

    texts = spoken_texts(args.phrases)
    suspect = []

    for written, name in manifest.items():
        expected = texts.get(written, written)
        heard, duration = transcribe(lambda rate: vosk.KaldiRecognizer(model, rate),
                                     os.path.join(args.dir, name))

        missing = missing_words(expected, heard)
        if missing:
            suspect.append((name, expected, heard, duration, len(missing) / len(words(expected))))

    for name, expected, heard, duration, share in sorted(suspect, key=lambda s: -s[4]):
        print(f"{name}  {duration:.2f}s  want {expected!r}  heard {heard!r}")

    print(f"\n{len(manifest) - len(suspect)}/{len(manifest)} clips transcribe back to their own text",
          file=sys.stderr)
    return 1 if suspect else 0


if __name__ == "__main__":
    sys.exit(main())

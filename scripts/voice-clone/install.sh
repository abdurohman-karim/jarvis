#!/usr/bin/env bash
# Sets up the local voice cloning component.
#
# This is optional and deliberately not part of the app bundle: it pulls in PyTorch and a
# 1.3 GB speech model (about 4 GB in total). Without it the assistant speaks with the
# operating system's voice.
set -euo pipefail

cd "$(dirname "$0")"
VENV="venv"
MODEL_DIR="model"
MODEL_URL="https://huggingface.co/Misha24-10/F5-TTS_RUSSIAN/resolve/main/F5TTS_v1_Base_v2/model_last_inference.safetensors"
VOCAB_URL="https://huggingface.co/Misha24-10/F5-TTS_RUSSIAN/resolve/main/F5TTS_v1_Base/vocab.txt"

python_bin=$(command -v python3.13 || command -v python3.12 || command -v python3.11 || true)
if [ -z "$python_bin" ]; then
    echo "Python 3.11+ is required (brew install python@3.13)." >&2
    exit 1
fi

echo "==> virtualenv ($python_bin)"
[ -d "$VENV" ] || "$python_bin" -m venv "$VENV"

echo "==> dependencies (a few GB, this takes a while)"
"$VENV/bin/pip" install --quiet --upgrade pip
"$VENV/bin/pip" install --quiet torch torchaudio numpy soundfile f5-tts

if ! command -v ffmpeg >/dev/null; then
    echo "==> ffmpeg is required by torchcodec; install it with: brew install ffmpeg" >&2
fi

echo "==> speech model"
mkdir -p "$MODEL_DIR"
[ -f "$MODEL_DIR/model.safetensors" ] || curl -L --progress-bar -o "$MODEL_DIR/model.safetensors" "$MODEL_URL"
[ -f "$MODEL_DIR/vocab.txt" ] || curl -L --progress-bar -o "$MODEL_DIR/vocab.txt" "$VOCAB_URL"

echo
echo "Ready. Total size: $(du -sh . | cut -f1)"
echo "Select \"Jarvis voice\" as the speech engine in the settings."

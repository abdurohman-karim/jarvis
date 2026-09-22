# ### APP INFO
app-name = JARVIS
app-description = Voice Assistant

# ### TRAY MENU
tray-restart = Restart
tray-settings = Settings
tray-exit = Exit
tray-tooltip = JARVIS - Voice Assistant
tray-language = Language
tray-voice = Voice
tray-wake-word = Wake Word Engine
tray-noise-suppression = Noise Suppression
tray-vad = Voice Activity Detection
tray-gain-normalizer = Gain Normalizer

# ### HEADER

# ### SEARCH
search-placeholder = Enter a command manually or say «Jarvis» ...

# ### MAIN PAGE
assistant-not-running = ASSISTANT NOT RUNNING
assistant-offline-hint = You can configure it without starting.
btn-start = START
btn-starting = STARTING...

# ### STATUS
status-disconnected = Disconnected
status-standby = Standby
status-listening = Listening...
status-processing = Processing...

# ### STATS
stats-microphone = MICROPHONE
stats-neural-networks = NEURAL NETWORKS
stats-resources = RESOURCES
stats-system-default = System Default
stats-not-selected = Not selected
stats-loading = Loading...


# ### SETTINGS
settings-title = Settings
settings-general = General
settings-devices = Devices
settings-neural-networks = Neural Networks
settings-audio = Audio
settings-recognition = Recognition
settings-about = About
settings-language = Language
settings-microphone = Microphone
settings-microphone-desc = The assistant will listen to this microphone.
settings-mic-default = Default (System)
settings-voice = Assistant voice
settings-voice-desc =
    Not all commands work with all sound packs.
    Click to listen the preview of sound.
settings-wake-word-engine = Wake word engine
settings-wake-word-desc = Choose the engine for wake word recognition.
settings-stt-engine = Speech recognition
settings-intent-engine = Intent recognition
settings-intent-engine-desc = Select neural network for command recognition.
settings-noise-suppression = Noise suppression
settings-noise-suppression-desc = Reduces background noise. May negatively affect recognition.
settings-vad = Voice detection (VAD)
settings-vad-desc = Skips silence, saves CPU resources.
settings-gain-normalizer = Gain normalizer
settings-gain-normalizer-desc = Automatically adjusts volume level.
settings-api-keys = API Keys
settings-save = Save
settings-cancel = Cancel
settings-back = Back
settings-enabled = Enabled
settings-disabled = Disabled


# settings - picovoice
settings-picovoice-warning = This neural network doesn't work for everyone!
settings-picovoice-key-desc = Enter your Picovoice key here. It is issued for free upon registration at
settings-picovoice-key = Picovoice Key

# settings - vosk
settings-auto-detect = Auto-detect
settings-vosk-model = Speech recognition model (Vosk)
settings-vosk-model-desc =
    Select Vosk model for speech recognition.
    You can download models here: https://alphacephei.com/vosk/models
settings-models-not-found = Models not found

# settings - openai
settings-openai-key = OpenAI Key
settings-openai-not-supported = ChatGPT is not currently supported. It will be added in future updates.

# ### COMMANDS PAGE
commands-title = Commands
commands-search = Search commands...
commands-count = { $count } commands

# ### ERRORS
error-generic = An error occurred
error-connection = Connection error
error-not-found = Not found

# ### NOTIFICATIONS
notification-saved = Settings saved!
notification-error = Error
notification-assistant-started = Assistant started
notification-assistant-stopped = Assistant stopped

# SLOTS EXTRACTION
settings-slot-engine = Slot extraction
settings-slot-engine-desc = Extract parameters from voice commands (e.g. city name, number).
settings-gliner-model = GLiNER ONNX model
settings-gliner-model-desc =
    Select model variant.
    Smaller quantized models (int8, uint8) are faster but less accurate.
settings-gliner-models-hint = No GLiNER models found.

# ETC
search-error-not-running = Assistant is not running
search-error-failed = Failed to execute command
settings-no-voices = No voices found

# ### UI (redesign)
nav-assistant = Assistant
nav-commands = Commands
nav-settings = Settings
assistant-subtitle = Voice control for your computer
assistant-ready = Ready
assistant-ready-hint = Say “Jarvis” or type a command below.
assistant-last-heard = Last recognized phrase
status-offline = Not running
status-connecting = Connecting…
btn-stop = Stop
btn-stopping = Stopping…
settings-subtitle = Voice, devices and neural networks
settings-diagnostics = Diagnostics
settings-logs = Logs
settings-open-logs = Open logs folder
commands-empty-title = No commands found
commands-empty-desc = Put command packs into the resources/commands folder.
commands-no-results = Nothing found
status-muted = Microphone muted
btn-mute = Mute
btn-unmute = Unmute
btn-restart = Restart
btn-restarting = Restarting…
commands-reload = Reload
settings-vosk-catalog = Available models
settings-vosk-catalog-desc = Models are downloaded from alphacephei.com into the app data folder.
settings-models-hint = Download a model from the list below - the assistant cannot start without one.
models-download = Download
models-downloading = Downloading…
models-extracting = Extracting…
models-delete = Delete
models-installed = Installed
models-bundled = Bundled
models-error = Error
assistant-no-model = No speech recognition model
assistant-no-model-hint = Download a Vosk model for your language in Settings.
btn-get-model = Get a model
settings-applying = Applying settings…
settings-applied = Settings applied
settings-restart-hint = The assistant did not answer - changes take effect after a restart

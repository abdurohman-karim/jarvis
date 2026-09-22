<script lang="ts">
    import { onMount } from "svelte"
    import { invoke } from "@tauri-apps/api/core"

    import { showInExplorer } from "@/functions"
    import { appInfo, assistantVoice, translations, translate, isJarvisRunning, restartAssistant, assistantBusy } from "@/stores"

    import Card from "@/components/ui/Card.svelte"
    import Field from "@/components/ui/Field.svelte"
    import Select from "@/components/ui/Select.svelte"
    import Toggle from "@/components/ui/Toggle.svelte"
    import Input from "@/components/ui/Input.svelte"
    import Button from "@/components/ui/Button.svelte"
    import Icon from "@/components/ui/Icon.svelte"

    $: t = (key: string) => translate($translations, key)

    // ### TYPES
    interface VoiceMeta {
        id: string
        name: string
        languages: string[]
    }

    interface Option {
        label: string
        value: string
    }

    // ### STATE
    interface BackendOption {
        id: string
        name: string
        model_id: string | null
    }

    let availableVoices: VoiceMeta[] = []
    let availableMicrophones: Option[] = []
    let availableVoskModels: Option[] = []
    let availableGlinerModels: Option[] = []

    // selectable backends per task, ids come straight from the backend and are what the
    // matching setting key accepts (see jarvis_core::models::catalog)
    let intentOptions: Option[] = []
    let slotsOptions: Option[] = []
    let vadOptions: Option[] = []

    let saving = false
    let saved = false
    // settings were saved while the assistant was running: it only reads them at startup
    let restartNeeded = false
    let savedTimer: ReturnType<typeof setTimeout> | null = null

    let voiceVal = ""
    let selectedMicrophone = "-1"
    let selectedWakeWordEngine = "Rustpotter"
    let selectedIntentBackend = "intent-classifier"
    let selectedSlotsBackend = "none"
    let selectedGlinerModel = ""
    let selectedVoskModel = ""
    let selectedNoiseSuppression = "None"
    let selectedVadBackend = "energy"
    let gainNormalizerEnabled = false
    let apiKeyPicovoice = ""
    let apiKeyOpenai = ""

    let logFilePath = ""
    appInfo.subscribe(info => {
        logFilePath = info.logFilePath
    })

    assistantVoice.subscribe(value => {
        voiceVal = value
    })

    // ### ACTIONS
    async function selectVoice(voiceId: string) {
        voiceVal = voiceId
        try {
            await invoke("preview_voice", { voiceId })
        } catch (err) {
            console.error("Failed to preview voice:", err)
        }
    }

    async function saveSettings() {
        saving = true
        saved = false

        try {
            await Promise.all([
                invoke("db_write", { key: "assistant_voice", val: voiceVal }),
                invoke("db_write", { key: "selected_microphone", val: selectedMicrophone }),
                invoke("db_write", { key: "selected_wake_word_engine", val: selectedWakeWordEngine }),
                invoke("db_write", { key: "intent_backend", val: selectedIntentBackend }),
                invoke("db_write", { key: "slots_backend", val: selectedSlotsBackend }),
                invoke("db_write", { key: "selected_gliner_model", val: selectedGlinerModel }),
                invoke("db_write", { key: "selected_vosk_model", val: selectedVoskModel }),
                invoke("db_write", { key: "noise_suppression", val: selectedNoiseSuppression }),
                invoke("db_write", { key: "vad_backend", val: selectedVadBackend }),
                invoke("db_write", { key: "gain_normalizer", val: gainNormalizerEnabled.toString() }),
                invoke("db_write", { key: "api_key__picovoice", val: apiKeyPicovoice }),
                invoke("db_write", { key: "api_key__openai", val: apiKeyOpenai })
            ])

            assistantVoice.set(voiceVal)
            saved = true
            restartNeeded = $isJarvisRunning
            if (savedTimer) clearTimeout(savedTimer)
            savedTimer = setTimeout(() => saved = false, 4000)
        } catch (err) {
            console.error("failed to save settings:", err)
        } finally {
            saving = false
        }
    }

    // ### INIT
    onMount(async () => {
        try {
            const voices = await invoke<{ voice: VoiceMeta }[]>("list_voices")
            availableVoices = voices.map(v => v.voice)
        } catch (err) {
            console.error("Failed to load voices:", err)
        }

        try {
            const mics = await invoke<string[]>("pv_get_audio_devices")
            availableMicrophones = [
                { label: t("settings-mic-default"), value: "-1" },
                ...mics.map((name, idx) => ({ label: name, value: String(idx) }))
            ]

            const languageNames: Record<string, string> = {
                us: "English", ru: "Русский", uk: "Українська", de: "Deutsch", fr: "Français", es: "Español",
            }
            const voskModels = await invoke<{ name: string; language: string; size: string }[]>("list_vosk_models")
            availableVoskModels = voskModels.map(m => ({
                label: `${m.name} (${languageNames[m.language] ?? m.language}, ${m.size})`,
                value: m.name
            }))

            const glinerModels = await invoke<{ display_name: string; value: string }[]>("list_gliner_models")
            availableGlinerModels = glinerModels.map(m => ({ label: m.display_name, value: m.value }))

            const toOptions = (opts: BackendOption[]): Option[] =>
                opts.map(o => ({ label: o.id === "none" ? t("settings-disabled") : o.name, value: o.id }))
            const [intentOpts, slotsOpts, vadOpts] = await Promise.all([
                invoke<BackendOption[]>("get_backend_options", { task: "intent" }),
                invoke<BackendOption[]>("get_backend_options", { task: "slots" }),
                invoke<BackendOption[]>("get_backend_options", { task: "vad" }),
            ])
            intentOptions = toOptions(intentOpts)
            slotsOptions = toOptions(slotsOpts)
            vadOptions = toOptions(vadOpts)

            const [mic, wakeWord, intentReco, slotEngine, glinerModel, voskModel,
                   noiseSuppression, vad, gainNormalizer, pico, openai] = await Promise.all([
                invoke<string>("db_read", { key: "selected_microphone" }),
                invoke<string>("db_read", { key: "selected_wake_word_engine" }),
                invoke<string>("db_read", { key: "intent_backend" }),
                invoke<string>("db_read", { key: "slots_backend" }),
                invoke<string>("db_read", { key: "selected_gliner_model" }),
                invoke<string>("db_read", { key: "selected_vosk_model" }),
                invoke<string>("db_read", { key: "noise_suppression" }),
                invoke<string>("db_read", { key: "vad_backend" }),
                invoke<string>("db_read", { key: "gain_normalizer" }),
                invoke<string>("db_read", { key: "api_key__picovoice" }),
                invoke<string>("db_read", { key: "api_key__openai" })
            ])

            selectedMicrophone = mic || "-1"
            selectedWakeWordEngine = wakeWord || "Rustpotter"
            selectedIntentBackend = intentReco || "intent-classifier"
            selectedSlotsBackend = slotEngine || "none"
            selectedVoskModel = voskModel || ""
            selectedGlinerModel = glinerModel || ""
            selectedNoiseSuppression = noiseSuppression || "None"
            selectedVadBackend = vad || "energy"
            gainNormalizerEnabled = gainNormalizer === "true"
            apiKeyPicovoice = pico || ""
            apiKeyOpenai = openai || ""
        } catch (err) {
            console.error("failed to load settings:", err)
        }
    })
</script>

<div class="page settings">
    <header class="page-header">
        <div>
            <h1>{t("settings-title")}</h1>
            <p class="page-subtitle">{t("settings-subtitle")}</p>
        </div>
    </header>

    <!-- voice -->
    <Card title={t("settings-voice")} description={t("settings-voice-desc")}>
        {#if availableVoices.length === 0}
            <p class="muted">{t("settings-no-voices")}</p>
        {:else}
            <div class="voices">
                {#each availableVoices as voice (voice.id)}
                    <button
                        type="button"
                        class="voice"
                        class:selected={voiceVal === voice.id}
                        on:click={() => selectVoice(voice.id)}
                    >
                        <span class="voice-icon"><Icon name="volume" size={15} /></span>
                        <span class="voice-name">{voice.name}</span>
                        <span class="voice-langs">
                            {#each voice.languages as lang}
                                <img src="/media/flags/{lang.toUpperCase()}.png" alt={lang} title={lang} width="16" />
                            {/each}
                        </span>
                        {#if voiceVal === voice.id}
                            <span class="voice-check"><Icon name="check" size={15} /></span>
                        {/if}
                    </button>
                {/each}
            </div>
        {/if}
    </Card>

    <!-- devices -->
    <Card title={t("settings-devices")}>
        <Field label={t("settings-microphone")} description={t("settings-microphone-desc")}>
            <Select bind:value={selectedMicrophone} options={availableMicrophones} />
        </Field>
    </Card>

    <!-- recognition -->
    <Card title={t("settings-neural-networks")}>
        <Field label={t("settings-wake-word-engine")} description={t("settings-wake-word-desc")}>
            <Select
                bind:value={selectedWakeWordEngine}
                options={[
                    { label: "Rustpotter", value: "Rustpotter" },
                    { label: "Vosk", value: "Vosk" },
                    { label: "Picovoice Porcupine", value: "Porcupine" }
                ]}
            />
        </Field>

        {#if selectedWakeWordEngine === "Porcupine"}
            <div class="notice warning">
                <Icon name="alert" size={16} />
                <div>
                    <p class="notice-title">{t("settings-picovoice-warning")}</p>
                    <p class="notice-text">{t("settings-picovoice-key-desc")} <a href="https://console.picovoice.ai/" target="_blank">Picovoice Console</a>.</p>
                </div>
            </div>
            <Field label={t("settings-picovoice-key")}>
                <Input bind:value={apiKeyPicovoice} icon="key" placeholder={t("settings-picovoice-key")} type="password" mono />
            </Field>
        {/if}

        {#key availableVoskModels}
            <Field label={t("settings-vosk-model")} description={t("settings-vosk-model-desc")}>
                <Select
                    bind:value={selectedVoskModel}
                    options={[{ label: t("settings-auto-detect"), value: "" }, ...availableVoskModels]}
                />
            </Field>
        {/key}

        {#if availableVoskModels.length === 0}
            <div class="notice warning">
                <Icon name="alert" size={16} />
                <div>
                    <p class="notice-title">{t("settings-models-not-found")}</p>
                    <p class="notice-text">{t("settings-models-hint")}</p>
                </div>
            </div>
        {/if}

        {#key intentOptions}
            <Field label={t("settings-intent-engine")} description={t("settings-intent-engine-desc")}>
                <Select bind:value={selectedIntentBackend} options={intentOptions} />
            </Field>
        {/key}

        {#key slotsOptions}
            <Field label={t("settings-slot-engine")} description={t("settings-slot-engine-desc")}>
                <Select bind:value={selectedSlotsBackend} options={slotsOptions} />
            </Field>
        {/key}

        {#if selectedSlotsBackend !== "none"}
            {#key availableGlinerModels}
                <Field label={t("settings-gliner-model")} description={t("settings-gliner-model-desc")}>
                    <Select
                        bind:value={selectedGlinerModel}
                        options={[{ label: t("settings-auto-detect"), value: "" }, ...availableGlinerModels]}
                    />
                </Field>
            {/key}

            {#if availableGlinerModels.length === 0}
                <div class="notice warning">
                    <Icon name="alert" size={16} />
                    <div>
                        <p class="notice-title">{t("settings-models-not-found")}</p>
                        <p class="notice-text">{t("settings-gliner-models-hint")}</p>
                    </div>
                </div>
            {/if}
        {/if}
    </Card>

    <!-- audio processing -->
    <Card title={t("settings-audio")}>
        <Field label={t("settings-noise-suppression")} description={t("settings-noise-suppression-desc")}>
            <Select
                bind:value={selectedNoiseSuppression}
                options={[
                    { label: t("settings-disabled"), value: "None" },
                    { label: "Nnnoiseless", value: "Nnnoiseless" }
                ]}
            />
        </Field>

        {#key vadOptions}
            <Field label={t("settings-vad")} description={t("settings-vad-desc")}>
                <Select bind:value={selectedVadBackend} options={vadOptions} />
            </Field>
        {/key}

        <Field label={t("settings-gain-normalizer")} description={t("settings-gain-normalizer-desc")} inline>
            <Toggle bind:checked={gainNormalizerEnabled} />
        </Field>
    </Card>

    <!-- api keys -->
    <Card title={t("settings-api-keys")}>
        <Field label={t("settings-openai-key")} description={t("settings-openai-not-supported")}>
            <Input bind:value={apiKeyOpenai} icon="key" placeholder="sk-..." type="password" mono disabled />
        </Field>
    </Card>

    <!-- diagnostics -->
    <Card title={t("settings-diagnostics")}>
        <Field label={t("settings-logs")} description={logFilePath} inline>
            <Button size="sm" on:click={() => showInExplorer(logFilePath)}>
                <Icon name="folder" size={14} />
                {t("settings-open-logs")}
            </Button>
        </Field>
    </Card>

    <div class="save-bar">
        {#if restartNeeded && $isJarvisRunning}
            <span class="restart-hint">{t("settings-restart-hint")}</span>
            <Button size="sm" on:click={async () => { await restartAssistant(); restartNeeded = false }} disabled={$assistantBusy}>
                <Icon name="refresh" size={14} />
                {$assistantBusy ? t("btn-restarting") : t("btn-restart")}
            </Button>
        {:else if saved}
            <span class="saved"><Icon name="check" size={14} /> {t("notification-saved")}</span>
        {/if}
        <Button variant="primary" on:click={saveSettings} disabled={saving}>
            {t("settings-save")}
        </Button>
    </div>
</div>

<style lang="scss">
    .settings {
        padding-bottom: 64px;
    }

    // ### voices
    .voices {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .voice {
        display: flex;
        align-items: center;
        gap: 10px;
        width: 100%;
        padding: 10px 12px;
        border-radius: var(--radius-sm);
        background: var(--bg-elevated);
        border: 1px solid var(--border);
        text-align: left;
        transition: border-color var(--duration) var(--ease), background var(--duration) var(--ease);

        &:hover {
            border-color: var(--border-strong);
            background: var(--surface-hover);
        }

        &.selected {
            border-color: var(--accent);
            background: var(--accent-soft);

            .voice-icon { color: var(--accent-hover); }
        }
    }

    .voice-icon {
        display: flex;
        color: var(--text-muted);
    }

    .voice-name {
        flex: 1;
        font-size: 13px;
        font-weight: 500;
    }

    .voice-langs {
        display: flex;
        gap: 4px;

        img { border-radius: 2px; opacity: 0.85; }
    }

    .voice-check {
        display: flex;
        color: var(--accent-hover);
    }

    // ### notices
    .notice {
        display: flex;
        gap: 10px;
        padding: 10px 12px;
        border-radius: var(--radius-sm);
        font-size: 12.5px;
        line-height: 1.45;

        &.warning {
            background: var(--warning-soft);
            color: var(--warning);
        }

        .notice-title {
            font-weight: 600;
        }

        .notice-text {
            color: var(--text-secondary);
            white-space: pre-line;
        }
    }

    // ### save bar
    .save-bar {
        position: fixed;
        left: var(--sidebar-width);
        right: 0;
        bottom: 0;
        display: flex;
        align-items: center;
        justify-content: flex-end;
        gap: 14px;
        padding: 12px var(--content-padding);
        background: rgba(11, 13, 18, 0.85);
        backdrop-filter: blur(12px);
        -webkit-backdrop-filter: blur(12px);
        border-top: 1px solid var(--border);
        z-index: 10;
    }

    .restart-hint {
        flex: 1;
        font-size: 12.5px;
        color: var(--warning);
    }

    .saved {
        display: inline-flex;
        align-items: center;
        gap: 6px;
        font-size: 12.5px;
        color: var(--success);
    }
</style>

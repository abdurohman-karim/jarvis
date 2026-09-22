<script lang="ts">
    import { invoke } from "@tauri-apps/api/core"
    import { onMount } from "svelte"
    import Icon from "./ui/Icon.svelte"
    import {
        isJarvisRunning,
        jarvisRamUsage,
        ipcConnected,
        translations,
        translate
    } from "@/stores"

    $: t = (key: string) => translate($translations, key)

    let microphoneName = ""
    let wakeWordEngine = ""
    let sttEngine = ""
    let vad = ""

    onMount(async () => {
        microphoneName = t("stats-loading")

        try {
            const micIndex = await invoke<string>("db_read", { key: "selected_microphone" })
            if (micIndex && micIndex !== "-1") {
                const devices = await invoke<string[]>("pv_get_audio_devices")
                microphoneName = devices[parseInt(micIndex)] ?? t("stats-not-selected")
            } else {
                microphoneName = t("stats-system-default")
            }

            wakeWordEngine = await invoke<string>("db_read", { key: "selected_wake_word_engine" }) || "Rustpotter"
            sttEngine = await invoke<string>("db_read", { key: "speech_to_text_engine" }) || "Vosk"
            vad = await invoke<string>("db_read", { key: "vad_backend" }) || ""
        } catch (err) {
            console.error("Failed to load stats:", err)
            microphoneName = t("stats-not-selected")
        }
    })
</script>

<div class="stats">
    <div class="stat">
        <span class="stat-icon" class:on={$isJarvisRunning}><Icon name="mic" size={15} /></span>
        <span class="stat-label">{t("stats-microphone")}</span>
        <span class="stat-value" title={microphoneName}>{microphoneName}</span>
    </div>

    <div class="stat">
        <span class="stat-icon" class:on={$ipcConnected}><Icon name="cpu" size={15} /></span>
        <span class="stat-label">{t("stats-neural-networks")}</span>
        <span class="stat-value">
            {wakeWordEngine} · {sttEngine}
            {#if vad && vad !== "none"}<span class="stat-sub">· VAD {vad}</span>{/if}
        </span>
    </div>

    <div class="stat">
        <span class="stat-icon" class:on={$isJarvisRunning}><Icon name="activity" size={15} /></span>
        <span class="stat-label">{t("stats-resources")}</span>
        <span class="stat-value">{#if $isJarvisRunning && $jarvisRamUsage}RAM {$jarvisRamUsage} MB{:else}—{/if}</span>
    </div>
</div>

<style lang="scss">
    .stats {
        display: flex;
        flex-direction: column;
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        overflow: hidden;
    }

    .stat {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 10px 14px;
        min-width: 0;

        & + .stat {
            border-top: 1px solid var(--border);
        }
    }

    .stat-icon {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        border-radius: 7px;
        background: var(--surface-active);
        color: var(--text-muted);
        flex-shrink: 0;
        transition: background var(--duration) var(--ease), color var(--duration) var(--ease);

        &.on {
            background: var(--accent-soft);
            color: var(--accent-hover);
        }
    }

    .stat-label {
        flex-shrink: 0;
        width: 110px;
        font-size: 11px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.06em;
        color: var(--text-muted);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .stat-value {
        flex: 1;
        min-width: 0;
        text-align: right;
        font-size: 13px;
        font-weight: 500;
        color: var(--text);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .stat-sub {
        color: var(--text-muted);
        font-weight: 400;
    }
</style>

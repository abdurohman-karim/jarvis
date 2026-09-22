<script lang="ts">
    import { onMount, onDestroy } from "svelte"
    import { invoke } from "@tauri-apps/api/core"

    import StatusOrb from "@/components/StatusOrb.svelte"
    import CommandInput from "@/components/CommandInput.svelte"
    import StatsGrid from "@/components/StatsGrid.svelte"
    import Button from "@/components/ui/Button.svelte"
    import Icon from "@/components/ui/Icon.svelte"

    import {
        isJarvisRunning,
        jarvisState,
        lastRecognizedText,
        updateJarvisStats,
        enableIpc,
        disableIpc,
        stopJarvisApp,
        translate,
        translations
    } from "@/stores"

    $: t = (key: string) => translate($translations, key)

    let launching = false
    let stopping = false
    let wasRunning = false

    isJarvisRunning.subscribe((value) => {
        if (value) {
            enableIpc()
            wasRunning = true
        } else if (wasRunning) {
            disableIpc()
            wasRunning = false
        }
    })

    onMount(() => {
        updateJarvisStats()
    })

    onDestroy(() => {
        disableIpc()
    })

    async function start() {
        launching = true
        try {
            await invoke("run_jarvis_app")
            setTimeout(async () => {
                await updateJarvisStats()
                launching = false
            }, 2500)
        } catch (err) {
            console.error("Failed to run jarvis-app:", err)
            launching = false
        }
    }

    async function stop() {
        stopping = true
        try {
            stopJarvisApp()
            // the process plays a goodbye sound before exiting, so poll for a while
            for (let i = 0; i < 15; i++) {
                await new Promise(r => setTimeout(r, 1000))
                await updateJarvisStats()
                if (!$isJarvisRunning) break
            }
        } catch (err) {
            console.error("Failed to stop jarvis-app:", err)
        } finally {
            stopping = false
        }
    }

    $: statusKey = !$isJarvisRunning
        ? "status-offline"
        : { disconnected: "status-connecting", idle: "status-standby", listening: "status-listening", processing: "status-processing" }[$jarvisState]

    $: statusTone = !$isJarvisRunning
        ? ""
        : $jarvisState === "disconnected" ? "warning"
        : $jarvisState === "idle" ? "success"
        : "accent"
</script>

<div class="page">
    <header class="page-header">
        <div>
            <h1>{t("nav-assistant")}</h1>
            <p class="page-subtitle">{t("assistant-subtitle")}</p>
        </div>
        <span class="pill {statusTone}">
            <span class="dot"></span>
            {t(statusKey)}
        </span>
    </header>

    <section class="hero">
        <StatusOrb />

        <div class="hero-text">
            {#if !$isJarvisRunning}
                <p class="hero-title">{t("assistant-not-running")}</p>
                <p class="hero-hint">{t("assistant-offline-hint")}</p>
            {:else if $lastRecognizedText}
                <p class="hero-title">«{$lastRecognizedText}»</p>
                <p class="hero-hint">{t("assistant-last-heard")}</p>
            {:else}
                <p class="hero-title">{t("assistant-ready")}</p>
                <p class="hero-hint">{t("assistant-ready-hint")}</p>
            {/if}
        </div>

        <div class="hero-actions">
            {#if !$isJarvisRunning}
                <Button variant="primary" size="lg" on:click={start} disabled={launching}>
                    <Icon name="play" size={15} />
                    {launching ? t("btn-starting") : t("btn-start")}
                </Button>
            {:else}
                <Button variant="ghost" size="sm" on:click={stop} disabled={stopping}>
                    <Icon name="square" size={13} />
                    {stopping ? t("btn-stopping") : t("btn-stop")}
                </Button>
            {/if}
        </div>
    </section>

    <CommandInput />

    <StatsGrid />
</div>

<style lang="scss">
    .hero {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 14px;
        padding: 10px 0 4px;
    }

    .hero-text {
        text-align: center;
        max-width: 360px;
    }

    .hero-title {
        font-size: 15px;
        font-weight: 600;
        color: var(--text);
    }

    .hero-hint {
        margin-top: 2px;
        font-size: 12.5px;
        color: var(--text-muted);
    }

    .hero-actions {
        min-height: 36px;
        display: flex;
        align-items: center;
    }
</style>

<script lang="ts">
    import { onMount } from "svelte"
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
        startAssistant,
        stopAssistant,
        assistantBusy,
        isMuted,
        setMuted,
        translate,
        translations
    } from "@/stores"

    $: t = (key: string) => translate($translations, key)

    onMount(() => {
        updateJarvisStats()
    })

    function toggleMute() {
        setMuted(!$isMuted)
    }

    $: statusKey = !$isJarvisRunning
        ? "status-offline"
        : $isMuted ? "status-muted"
        : { disconnected: "status-connecting", idle: "status-standby", listening: "status-listening", processing: "status-processing" }[$jarvisState]

    $: statusTone = !$isJarvisRunning
        ? ""
        : $isMuted ? "warning"
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
                <Button variant="primary" size="lg" on:click={startAssistant} disabled={$assistantBusy}>
                    <Icon name="play" size={15} />
                    {$assistantBusy ? t("btn-starting") : t("btn-start")}
                </Button>
            {:else}
                <Button variant={$isMuted ? "secondary" : "ghost"} size="sm" on:click={toggleMute}>
                    <Icon name={$isMuted ? "mic-off" : "mic"} size={13} />
                    {$isMuted ? t("btn-unmute") : t("btn-mute")}
                </Button>
                <Button variant="ghost" size="sm" on:click={stopAssistant} disabled={$assistantBusy}>
                    <Icon name="square" size={13} />
                    {$assistantBusy ? t("btn-stopping") : t("btn-stop")}
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
        gap: 8px;
    }
</style>

<script lang="ts">
    // Speech recognition models: what is installed, what can be downloaded, with progress.
    import { onMount, onDestroy, createEventDispatcher } from "svelte"
    import { invoke } from "@tauri-apps/api/core"
    import { listen, type UnlistenFn } from "@tauri-apps/api/event"

    import Button from "./ui/Button.svelte"
    import Icon from "./ui/Icon.svelte"
    import { translations, translate } from "@/stores"

    $: t = (key: string) => translate($translations, key)

    interface CatalogModel {
        name: string
        language: string
        description: string
        size_mb: number
        installed: boolean
        bundled: boolean
        recommended: boolean
    }

    interface Progress {
        downloaded: number
        total: number | null
        stage: "downloading" | "extracting" | "done" | "error"
        message?: string
    }

    const dispatch = createEventDispatcher<{ change: void }>()

    let catalog: CatalogModel[] = []
    let progress: Record<string, Progress> = {}
    let busy: Record<string, boolean> = {}
    let unlisten: UnlistenFn | null = null

    const languageNames: Record<string, string> = { ru: "Русский", en: "English", ua: "Українська" }

    async function refresh() {
        try {
            catalog = await invoke<CatalogModel[]>("vosk_model_catalog")
        } catch (err) {
            console.error("Failed to load model catalog:", err)
        }
    }

    async function download(name: string) {
        busy = { ...busy, [name]: true }
        progress = { ...progress, [name]: { downloaded: 0, total: null, stage: "downloading" } }
        try {
            await invoke("download_vosk_model", { name })
        } catch (err) {
            progress = { ...progress, [name]: { downloaded: 0, total: null, stage: "error", message: String(err) } }
        } finally {
            busy = { ...busy, [name]: false }
            await refresh()
            dispatch("change")
        }
    }

    async function remove(name: string) {
        busy = { ...busy, [name]: true }
        try {
            await invoke("delete_vosk_model", { name })
        } catch (err) {
            progress = { ...progress, [name]: { downloaded: 0, total: null, stage: "error", message: String(err) } }
        } finally {
            busy = { ...busy, [name]: false }
            await refresh()
            dispatch("change")
        }
    }

    function formatMb(bytes: number): string {
        return (bytes / 1024 / 1024).toFixed(0)
    }

    function percent(p: Progress): number {
        return p.total ? Math.min(100, Math.round((p.downloaded / p.total) * 100)) : 0
    }

    onMount(async () => {
        unlisten = await listen<{ name: string } & Progress>("vosk-download-progress", (event) => {
            const { name, ...p } = event.payload
            progress = { ...progress, [name]: p }
            if (p.stage === "done") {
                // keep the row tidy once installed
                setTimeout(() => { const { [name]: _, ...rest } = progress; progress = rest }, 1500)
            }
        })
        await refresh()
    })

    onDestroy(() => {
        unlisten?.()
    })
</script>

<div class="models">
    {#each catalog as m (m.name)}
        {@const p = progress[m.name]}
        <div class="model" class:installed={m.installed} class:suggested={m.recommended && !m.installed}>
            <div class="model-main">
                <div class="model-title">
                    <span class="model-name">{m.name}</span>
                    <span class="model-lang">{languageNames[m.language] ?? m.language}</span>
                    {#if m.recommended}<span class="badge">{t("models-recommended")}</span>{/if}
                </div>
                <span class="model-desc">{m.description} · {m.size_mb} MB</span>

                {#if p && (p.stage === "downloading" || p.stage === "extracting")}
                    <div class="progress">
                        <div class="bar" style="width: {p.stage === 'extracting' ? 100 : percent(p)}%"></div>
                    </div>
                    <span class="progress-text">
                        {#if p.stage === "extracting"}
                            {t("models-extracting")}
                        {:else if p.total}
                            {formatMb(p.downloaded)} / {formatMb(p.total)} MB
                        {:else}
                            {formatMb(p.downloaded)} MB
                        {/if}
                    </span>
                {:else if p && p.stage === "error"}
                    <span class="model-error" title={p.message}>{t("models-error")}: {p.message}</span>
                {/if}
            </div>

            <div class="model-actions">
                {#if m.installed}
                    <span class="pill success"><span class="dot"></span>{m.bundled ? t("models-bundled") : t("models-installed")}</span>
                    {#if !m.bundled}
                        <Button size="sm" variant="ghost" on:click={() => remove(m.name)} disabled={busy[m.name]}>
                            <Icon name="x" size={13} />
                            {t("models-delete")}
                        </Button>
                    {/if}
                {:else}
                    <Button size="sm" on:click={() => download(m.name)} disabled={busy[m.name]}>
                        <Icon name="download" size={13} />
                        {busy[m.name] ? t("models-downloading") : t("models-download")}
                    </Button>
                {/if}
            </div>
        </div>
    {/each}

    {#if catalog.length === 0}
        <p class="muted">{t("stats-loading")}</p>
    {/if}
</div>

<style lang="scss">
    .models {
        display: flex;
        flex-direction: column;
        gap: 6px;
    }

    .model.suggested {
        border-color: rgba(99, 102, 241, 0.3);
    }

    .model {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 10px 12px;
        border-radius: var(--radius-sm);
        background: var(--bg-elevated);
        border: 1px solid var(--border);

        &.installed {
            border-color: rgba(34, 197, 94, 0.25);
        }
    }

    .model-main {
        flex: 1;
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 2px;
    }

    .model-title {
        display: flex;
        align-items: baseline;
        gap: 8px;
        min-width: 0;
    }

    .model-name {
        font-size: 12.5px;
        font-weight: 600;
        font-family: var(--font-mono);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .model-lang {
        font-size: 11px;
        color: var(--text-muted);
        flex-shrink: 0;
    }

    .model-desc {
        font-size: 12px;
        color: var(--text-muted);
    }

    .model-actions {
        display: flex;
        align-items: center;
        gap: 6px;
        flex-shrink: 0;
    }

    .progress {
        margin-top: 6px;
        height: 4px;
        border-radius: var(--radius-full);
        background: var(--surface-active);
        overflow: hidden;

        .bar {
            height: 100%;
            background: var(--accent);
            transition: width 200ms var(--ease);
        }
    }

    .progress-text {
        font-size: 11px;
        color: var(--text-muted);
        font-family: var(--font-mono);
    }

    .model-error {
        font-size: 11.5px;
        color: var(--danger);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
</style>

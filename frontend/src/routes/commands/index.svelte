<script lang="ts">
    import { onMount } from "svelte"
    import { invoke } from "@tauri-apps/api/core"

    import Input from "@/components/ui/Input.svelte"
    import Icon from "@/components/ui/Icon.svelte"
    import EmptyState from "@/components/ui/EmptyState.svelte"
    import { currentLanguage, translations, translate } from "@/stores"

    $: t = (key: string) => translate($translations, key)

    interface Command {
        id: string
        type: string
        description: string
        phrases: Record<string, string[]>
    }

    let commands: Command[] = []
    let loading = true
    let query = ""

    onMount(async () => {
        try {
            commands = await invoke<Command[]>("get_commands_list")
        } catch (err) {
            console.error("Failed to load commands:", err)
            commands = []
        } finally {
            loading = false
        }
    })

    function phrasesFor(cmd: Command): string[] {
        return cmd.phrases[$currentLanguage] ?? cmd.phrases["en"] ?? Object.values(cmd.phrases)[0] ?? []
    }

    $: normalized = query.trim().toLowerCase()
    $: filtered = normalized
        ? commands.filter(c =>
            c.id.toLowerCase().includes(normalized) ||
            c.description.toLowerCase().includes(normalized) ||
            phrasesFor(c).some(p => p.toLowerCase().includes(normalized)))
        : commands

    const typeIcon: Record<string, string> = {
        lua: "code",
        ahk: "zap",
        cli: "terminal",
        voice: "volume",
        terminate: "square",
    }
</script>

<div class="page">
    <header class="page-header">
        <div>
            <h1>{t("commands-title")}</h1>
            <p class="page-subtitle">{t("commands-count").replace(/\{\s*\$count\s*\}/, String(commands.length))}</p>
        </div>
    </header>

    <Input bind:value={query} placeholder={t("commands-search")} icon="search" />

    {#if loading}
        <p class="muted">{t("stats-loading")}</p>
    {:else if commands.length === 0}
        <EmptyState icon="terminal" title={t("commands-empty-title")} description={t("commands-empty-desc")} />
    {:else if filtered.length === 0}
        <EmptyState icon="search" title={t("commands-no-results")} />
    {:else}
        <div class="list">
            {#each filtered as cmd (cmd.id)}
                {@const phrases = phrasesFor(cmd)}
                <article class="command">
                    <div class="command-head">
                        <span class="command-icon"><Icon name={typeIcon[cmd.type] ?? "list"} size={15} /></span>
                        <div class="command-title">
                            <span class="command-id">{cmd.id}</span>
                            {#if cmd.description}<span class="command-desc">{cmd.description}</span>{/if}
                        </div>
                        <span class="badge">{cmd.type}</span>
                    </div>
                    {#if phrases.length}
                        <div class="phrases">
                            {#each phrases.slice(0, 6) as phrase}
                                <span class="phrase">{phrase}</span>
                            {/each}
                            {#if phrases.length > 6}
                                <span class="phrase more">+{phrases.length - 6}</span>
                            {/if}
                        </div>
                    {/if}
                </article>
            {/each}
        </div>
    {/if}
</div>

<style lang="scss">
    .list {
        display: flex;
        flex-direction: column;
        gap: 8px;
    }

    .command {
        display: flex;
        flex-direction: column;
        gap: 10px;
        padding: 12px 14px;
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        transition: border-color var(--duration) var(--ease);

        &:hover { border-color: var(--border-strong); }
    }

    .command-head {
        display: flex;
        align-items: center;
        gap: 10px;
    }

    .command-icon {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 28px;
        height: 28px;
        border-radius: 7px;
        background: var(--accent-soft);
        color: var(--accent-hover);
        flex-shrink: 0;
    }

    .command-title {
        display: flex;
        flex-direction: column;
        min-width: 0;
        flex: 1;
    }

    .command-id {
        font-size: 13px;
        font-weight: 600;
        font-family: var(--font-mono);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .command-desc {
        font-size: 12px;
        color: var(--text-muted);
    }

    .phrases {
        display: flex;
        flex-wrap: wrap;
        gap: 6px;
    }

    .phrase {
        padding: 3px 8px;
        border-radius: 6px;
        background: var(--bg-elevated);
        border: 1px solid var(--border);
        font-size: 12px;
        color: var(--text-secondary);

        &.more { color: var(--text-muted); }
    }
</style>

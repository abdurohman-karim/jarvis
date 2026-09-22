<script lang="ts">
    import { onMount } from "svelte"
    import { invoke } from "@tauri-apps/api/core"

    import Input from "@/components/ui/Input.svelte"
    import Icon from "@/components/ui/Icon.svelte"
    import Button from "@/components/ui/Button.svelte"
    import EmptyState from "@/components/ui/EmptyState.svelte"
    import { currentLanguage, translations, translate, isJarvisRunning, ipcConnected, reloadCommands, commandsVersion } from "@/stores"

    $: t = (key: string) => translate($translations, key)

    interface Command {
        id: string
        type: string
        description: string
        platforms: string[]
        phrases: Record<string, string[]>
        pack: string
    }

    let commands: Command[] = []
    let loading = true
    let reloading = false
    let query = ""

    async function load() {
        try {
            commands = await invoke<Command[]>("get_commands_list")
        } catch (err) {
            console.error("Failed to load commands:", err)
            commands = []
        } finally {
            loading = false
        }
    }

    // re-read packs from disk; if the assistant is running, ask it to reload too
    // (it answers with `commands_reloaded`, which bumps commandsVersion)
    async function reload() {
        reloading = true
        if ($isJarvisRunning && $ipcConnected) {
            reloadCommands()
        }
        await load()
        setTimeout(() => reloading = false, 600)
    }

    onMount(load)

    // assistant finished a reload -> refresh the list
    $: if ($commandsVersion) load()

    function phrasesFor(cmd: Command): string[] {
        return cmd.phrases[$currentLanguage] ?? cmd.phrases["en"] ?? Object.values(cmd.phrases)[0] ?? []
    }

    $: normalized = query.trim().toLowerCase()
    $: filtered = normalized
        ? commands.filter(c =>
            c.id.toLowerCase().includes(normalized) ||
            c.pack.toLowerCase().includes(normalized) ||
            c.description.toLowerCase().includes(normalized) ||
            phrasesFor(c).some(p => p.toLowerCase().includes(normalized)))
        : commands

    // commands come from packs (a folder each); showing them grouped makes it obvious
    // where a command lives and which pack to edit
    $: groups = filtered.reduce<Record<string, Command[]>>((acc, cmd) => {
        (acc[cmd.pack] ??= []).push(cmd)
        return acc
    }, {})
    $: groupNames = Object.keys(groups).sort()

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
        <Button size="sm" on:click={reload} disabled={reloading}>
            <Icon name="refresh" size={14} />
            {t("commands-reload")}
        </Button>
    </header>

    <Input bind:value={query} placeholder={t("commands-search")} icon="search" />

    {#if loading}
        <p class="muted">{t("stats-loading")}</p>
    {:else if commands.length === 0}
        <EmptyState icon="terminal" title={t("commands-empty-title")} description={t("commands-empty-desc")} />
    {:else if filtered.length === 0}
        <EmptyState icon="search" title={t("commands-no-results")} />
    {:else}
        <div class="packs">
            {#each groupNames as pack (pack)}
                <section class="pack">
                    <header class="pack-head">
                        <Icon name="folder" size={14} />
                        <span class="pack-name">{pack}</span>
                        <span class="pack-count">{groups[pack].length}</span>
                    </header>

                    <div class="list">
                        {#each groups[pack] as cmd (cmd.pack + cmd.id)}
                            {@const phrases = phrasesFor(cmd)}
                            <article class="command">
                                <div class="command-head">
                                    <span class="command-icon"><Icon name={typeIcon[cmd.type] ?? "list"} size={15} /></span>
                                    <div class="command-title">
                                        <span class="command-id">{cmd.id}</span>
                                        {#if cmd.description}<span class="command-desc">{cmd.description}</span>{/if}
                                    </div>
                                    {#if cmd.platforms?.length}
                                        <span class="platforms" title={cmd.platforms.join(", ")}>{cmd.platforms.join(" · ")}</span>
                                    {/if}
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
                </section>
            {/each}
        </div>

    {/if}
</div>

<style lang="scss">
    .packs {
        display: flex;
        flex-direction: column;
        gap: 18px;
    }

    .pack-head {
        display: flex;
        align-items: center;
        gap: 8px;
        margin-bottom: 8px;
        color: var(--text-muted);
    }

    .pack-name {
        font-size: 11px;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.06em;
    }

    .pack-count {
        font-size: 11px;
        padding: 1px 6px;
        border-radius: var(--radius-full);
        background: var(--surface);
        border: 1px solid var(--border);
    }

    .platforms {
        font-size: 11px;
        color: var(--text-muted);
        white-space: nowrap;
    }

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

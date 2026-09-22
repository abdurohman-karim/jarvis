<script lang="ts">
    import Icon from "./ui/Icon.svelte"
    import { translations, translate, isJarvisRunning, ipcConnected, sendTextCommand } from "@/stores"

    $: t = (key: string) => translate($translations, key)

    let query = ""
    let busy = false
    let error = ""
    let errorTimer: ReturnType<typeof setTimeout> | null = null

    function flash(msg: string) {
        error = msg
        if (errorTimer) clearTimeout(errorTimer)
        errorTimer = setTimeout(() => error = "", 3000)
    }

    async function submit(e: Event) {
        e.preventDefault()
        const command = query.trim()
        if (!command || busy) return

        if (!$isJarvisRunning || !$ipcConnected) {
            flash(t("search-error-not-running"))
            return
        }

        busy = true
        try {
            await sendTextCommand(command)
            query = ""
        } catch (err) {
            console.error("Failed to send command:", err)
            flash(t("search-error-failed"))
        } finally {
            busy = false
        }
    }

    function onKeydown(e: KeyboardEvent) {
        if (e.key === "Escape") query = ""
    }
</script>

<form class="command" class:busy class:has-error={error} on:submit={submit}>
    <span class="lead"><Icon name="terminal" size={17} /></span>
    <input
        bind:value={query}
        on:keydown={onKeydown}
        type="text"
        placeholder={t("search-placeholder")}
        autocomplete="off"
        spellcheck="false"
        maxlength="200"
        disabled={busy}
    />
    <button type="submit" class="send" disabled={busy || !query.trim()} title="Enter">
        <Icon name="send" size={15} />
    </button>
    {#if error}
        <span class="error">{error}</span>
    {/if}
</form>

<style lang="scss">
    .command {
        position: relative;
        display: flex;
        align-items: center;
        gap: 8px;
        height: 44px;
        padding: 0 8px 0 14px;
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        transition: border-color var(--duration) var(--ease), box-shadow var(--duration) var(--ease);

        &:focus-within {
            border-color: var(--accent);
            box-shadow: 0 0 0 3px var(--accent-soft);
        }

        &.has-error {
            border-color: var(--danger);
        }

        &.busy { opacity: 0.7; }
    }

    .lead {
        display: flex;
        color: var(--text-muted);
    }

    input {
        flex: 1;
        min-width: 0;
        height: 100%;
        background: transparent;
        border: none;
        outline: none;
        font-size: 13.5px;
        color: var(--text);
        user-select: text;
        -webkit-user-select: text;

        &::placeholder { color: var(--text-faint); }
    }

    .send {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 30px;
        height: 30px;
        border-radius: var(--radius-sm);
        background: var(--accent);
        color: var(--on-accent);
        transition: background var(--duration) var(--ease), opacity var(--duration) var(--ease);

        &:hover:not(:disabled) { background: var(--accent-hover); }
        &:disabled { background: var(--surface-active); color: var(--text-faint); cursor: default; }
    }

    .error {
        position: absolute;
        left: 14px;
        top: calc(100% + 6px);
        font-size: 12px;
        color: var(--danger);
    }
</style>

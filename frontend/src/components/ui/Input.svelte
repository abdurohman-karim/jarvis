<script lang="ts">
    import Icon from "./Icon.svelte"

    export let value: string = ""
    export let placeholder: string = ""
    export let icon: string = ""
    export let type: "text" | "password" = "text"
    export let disabled = false
    export let mono = false

    // svelte doesn't allow dynamic `type` with bind:value, so handle input manually
    function onInput(e: Event) {
        value = (e.target as HTMLInputElement).value
    }
</script>

<div class="input" class:with-icon={icon} class:disabled>
    {#if icon}<span class="input-icon"><Icon name={icon} size={16} /></span>{/if}
    <input
        {type}
        {value}
        {placeholder}
        {disabled}
        class:mono
        autocomplete="off"
        spellcheck="false"
        on:input={onInput}
    />
</div>

<style lang="scss">
    .input {
        position: relative;
        width: 100%;

        input {
            width: 100%;
            height: 36px;
            padding: 0 12px;
            border-radius: var(--radius-sm);
            background: var(--bg-elevated);
            border: 1px solid var(--border);
            color: var(--text);
            font-size: 13px;
            transition: border-color var(--duration) var(--ease);
            user-select: text;
            -webkit-user-select: text;

            &::placeholder { color: var(--text-faint); }

            &:hover { border-color: var(--border-strong); }

            &:focus {
                outline: none;
                border-color: var(--accent);
                box-shadow: 0 0 0 3px var(--accent-soft);
            }

            &.mono { font-family: var(--font-mono); font-size: 12px; }
        }

        &.with-icon input { padding-left: 36px; }

        .input-icon {
            position: absolute;
            left: 12px;
            top: 50%;
            transform: translateY(-50%);
            color: var(--text-muted);
            display: flex;
            pointer-events: none;
        }

        &.disabled {
            opacity: 0.5;
            input { cursor: not-allowed; }
        }
    }
</style>

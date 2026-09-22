<script lang="ts">
    export let variant: "primary" | "secondary" | "ghost" | "danger" = "secondary"
    export let size: "sm" | "md" | "lg" = "md"
    export let disabled = false
    export let block = false
    export let type: "button" | "submit" = "button"
</script>

<button
    {type}
    class="btn {variant} {size}"
    class:block
    {disabled}
    on:click
>
    <slot />
</button>

<style lang="scss">
    .btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: 8px;
        height: 36px;
        padding: 0 16px;
        border-radius: var(--radius-sm);
        font-size: 13px;
        font-weight: 500;
        white-space: nowrap;
        border: 1px solid transparent;
        transition: background var(--duration) var(--ease),
                    border-color var(--duration) var(--ease),
                    color var(--duration) var(--ease),
                    transform var(--duration) var(--ease),
                    opacity var(--duration) var(--ease);

        &:active:not(:disabled) {
            transform: translateY(1px);
        }

        &:disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }

        &.block {
            width: 100%;
        }

        &.sm { height: 30px; padding: 0 12px; font-size: 12px; }
        &.lg { height: 42px; padding: 0 22px; font-size: 14px; }

        &.primary {
            background: var(--accent);
            color: var(--on-accent);

            &:hover:not(:disabled) { background: var(--accent-hover); }
            &:active:not(:disabled) { background: var(--accent-active); }
        }

        &.secondary {
            background: var(--surface);
            border-color: var(--border);
            color: var(--text);

            &:hover:not(:disabled) { background: var(--surface-hover); border-color: var(--border-strong); }
        }

        &.ghost {
            background: transparent;
            color: var(--text-secondary);

            &:hover:not(:disabled) { background: var(--surface); color: var(--text); }
        }

        &.danger {
            background: var(--danger-soft);
            color: var(--danger);

            &:hover:not(:disabled) { background: rgba(239, 68, 68, 0.22); }
        }
    }
</style>

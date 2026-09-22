<script lang="ts">
    import Icon from "./Icon.svelte"

    export let icon: string = "info"
    export let title: string
    export let description: string = ""
</script>

<div class="empty">
    <div class="empty-icon"><Icon name={icon} size={22} /></div>
    <p class="empty-title">{title}</p>
    {#if description}<p class="empty-desc">{description}</p>{/if}
    <slot />
</div>

<style lang="scss">
    .empty {
        display: flex;
        flex-direction: column;
        align-items: center;
        text-align: center;
        gap: 6px;
        padding: 40px 20px;
    }

    .empty-icon {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 48px;
        height: 48px;
        border-radius: var(--radius);
        background: var(--surface);
        border: 1px solid var(--border);
        color: var(--text-muted);
        margin-bottom: 8px;
    }

    .empty-title {
        font-size: 14px;
        font-weight: 600;
    }

    .empty-desc {
        font-size: 13px;
        color: var(--text-muted);
        max-width: 320px;
    }
</style>

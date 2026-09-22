<script lang="ts">
    export let title: string = ""
    export let description: string = ""
    export let padded = true
</script>

<section class="card" class:padded>
    {#if title || $$slots.header}
        <header class="card-header">
            <div>
                {#if title}<h2 class="card-title">{title}</h2>{/if}
                {#if description}<p class="card-desc">{description}</p>{/if}
            </div>
            <slot name="header" />
        </header>
    {/if}
    <div class="card-body">
        <slot />
    </div>
</section>

<style lang="scss">
    .card {
        background: var(--surface);
        border: 1px solid var(--border);
        border-radius: var(--radius);
        overflow: hidden;

        &.padded .card-body {
            padding: 16px;
        }
    }

    .card-header {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 12px;
        padding: 14px 16px 12px;
        border-bottom: 1px solid var(--border);
    }

    .card-title {
        font-size: 14px;
        font-weight: 600;
    }

    .card-desc {
        margin-top: 2px;
        font-size: 12px;
        color: var(--text-muted);
        white-space: pre-line;
    }

    .card-body {
        display: flex;
        flex-direction: column;
        gap: 16px;
    }
</style>

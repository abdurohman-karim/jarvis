<script lang="ts">
    export let label: string
    export let description: string = ""
    export let inline = false
</script>

<div class="field" class:inline>
    <div class="field-text">
        <span class="field-label">{label}</span>
        {#if description}<span class="field-desc">{description}</span>{/if}
    </div>
    <div class="field-control">
        <slot />
    </div>
</div>

<style lang="scss">
    .field {
        display: flex;
        flex-direction: column;
        gap: 8px;

        &.inline {
            flex-direction: row;
            align-items: center;
            justify-content: space-between;
            gap: 16px;

            .field-control {
                flex-shrink: 0;
            }
        }
    }

    .field-text {
        display: flex;
        flex-direction: column;
        gap: 2px;
        min-width: 0;
    }

    .field-label {
        font-size: 13px;
        font-weight: 500;
        color: var(--text);
    }

    .field-desc {
        font-size: 12px;
        line-height: 1.45;
        color: var(--text-muted);
        white-space: pre-line;
    }

    .field-control {
        min-width: 0;
    }
</style>

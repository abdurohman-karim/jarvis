<script lang="ts">
    import Icon from "./Icon.svelte"

    export let value: string = ""
    export let options: { label: string; value: string }[] = []
    export let disabled = false
</script>

<div class="select" class:disabled>
    <select bind:value {disabled} on:change>
        {#each options as opt}
            <option value={opt.value}>{opt.label}</option>
        {/each}
    </select>
    <span class="chevron"><Icon name="chevron" size={16} /></span>
</div>

<style lang="scss">
    .select {
        position: relative;
        width: 100%;

        select {
            width: 100%;
            height: 36px;
            padding: 0 36px 0 12px;
            border-radius: var(--radius-sm);
            background: var(--bg-elevated);
            border: 1px solid var(--border);
            color: var(--text);
            font-size: 13px;
            appearance: none;
            -webkit-appearance: none;
            cursor: pointer;
            transition: border-color var(--duration) var(--ease), background var(--duration) var(--ease);
            text-overflow: ellipsis;

            &:hover {
                border-color: var(--border-strong);
            }

            &:focus {
                outline: none;
                border-color: var(--accent);
                box-shadow: 0 0 0 3px var(--accent-soft);
            }

            option {
                background: var(--surface);
                color: var(--text);
            }
        }

        .chevron {
            position: absolute;
            right: 10px;
            top: 50%;
            transform: translateY(-50%);
            color: var(--text-muted);
            pointer-events: none;
            display: flex;
        }

        &.disabled {
            opacity: 0.5;

            select { cursor: not-allowed; }
        }
    }
</style>

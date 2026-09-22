<script lang="ts">
    export let checked = false
    export let disabled = false
    export let label: string = ""
</script>

<label class="toggle" class:disabled>
    <input type="checkbox" bind:checked {disabled} on:change />
    <span class="track"><span class="thumb"></span></span>
    {#if label}<span class="toggle-label">{label}</span>{/if}
</label>

<style lang="scss">
    .toggle {
        display: inline-flex;
        align-items: center;
        gap: 10px;
        cursor: pointer;

        &.disabled { opacity: 0.5; cursor: not-allowed; }

        input {
            position: absolute;
            opacity: 0;
            width: 0;
            height: 0;
        }

        .track {
            position: relative;
            width: 40px;
            height: 22px;
            border-radius: var(--radius-full);
            background: var(--surface-active);
            border: 1px solid var(--border-strong);
            transition: background var(--duration) var(--ease), border-color var(--duration) var(--ease);
        }

        .thumb {
            position: absolute;
            top: 2px;
            left: 2px;
            width: 16px;
            height: 16px;
            border-radius: 50%;
            background: var(--text-secondary);
            transition: transform var(--duration) var(--ease), background var(--duration) var(--ease);
        }

        input:checked + .track {
            background: var(--accent);
            border-color: var(--accent);

            .thumb {
                background: #fff;
                transform: translateX(18px);
            }
        }

        input:focus-visible + .track {
            box-shadow: 0 0 0 3px var(--accent-soft);
        }

        .toggle-label {
            font-size: 13px;
            color: var(--text-secondary);
        }
    }
</style>

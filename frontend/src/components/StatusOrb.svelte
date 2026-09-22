<script lang="ts">
    import { jarvisState, isJarvisRunning } from "@/stores"
    import Icon from "./ui/Icon.svelte"

    // offline | idle | listening | processing
    $: state = !$isJarvisRunning ? "offline" : ($jarvisState === "disconnected" ? "idle" : $jarvisState)
</script>

<div class="orb {state}" aria-label={state}>
    <span class="ring ring-3"></span>
    <span class="ring ring-2"></span>
    <span class="ring ring-1"></span>
    <span class="arc"></span>
    <span class="core">
        {#if state === "listening"}
            <Icon name="mic" size={30} strokeWidth={1.6} />
        {:else if state === "processing"}
            <Icon name="zap" size={30} strokeWidth={1.6} />
        {:else if state === "offline"}
            <Icon name="square" size={26} strokeWidth={1.6} />
        {:else}
            <Icon name="waveform" size={30} strokeWidth={1.6} />
        {/if}
    </span>
</div>

<style lang="scss">
    .orb {
        --orb-color: var(--text-faint);
        --orb-glow: transparent;
        position: relative;
        width: 200px;
        height: 200px;
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .ring,
    .core,
    .arc {
        position: absolute;
        border-radius: 50%;
        left: 50%;
        top: 50%;
        transform: translate(-50%, -50%);
    }

    .ring {
        border: 1px solid var(--orb-color);
        opacity: 0.35;
        transition: opacity 400ms var(--ease), border-color 400ms var(--ease);
    }

    .ring-1 { width: 132px; height: 132px; }
    .ring-2 { width: 166px; height: 166px; opacity: 0.2; }
    .ring-3 { width: 200px; height: 200px; opacity: 0.1; }

    .arc {
        width: 132px;
        height: 132px;
        border: 2px solid transparent;
        border-top-color: var(--orb-color);
        opacity: 0;
        transition: opacity 300ms var(--ease);
    }

    .core {
        width: 100px;
        height: 100px;
        display: flex;
        align-items: center;
        justify-content: center;
        background: var(--surface);
        border: 1px solid var(--border-strong);
        color: var(--orb-color);
        box-shadow: 0 0 0 0 var(--orb-glow);
        transition: color 400ms var(--ease), box-shadow 400ms var(--ease), border-color 400ms var(--ease), background 400ms var(--ease);
    }

    // ### states

    .idle {
        --orb-color: var(--accent);
        --orb-glow: var(--accent-glow);

        .core {
            border-color: rgba(99, 102, 241, 0.4);
            box-shadow: 0 0 40px -8px var(--orb-glow);
            animation: breathe 3.2s ease-in-out infinite;
        }
    }

    .listening {
        --orb-color: var(--accent);
        --orb-glow: var(--accent-glow);

        .core {
            background: var(--accent-soft);
            border-color: var(--accent);
            box-shadow: 0 0 60px -6px var(--orb-glow);
        }

        .ring-1 { animation: pulse 1.6s ease-out infinite; }
        .ring-2 { animation: pulse 1.6s ease-out 0.35s infinite; }
        .ring-3 { animation: pulse 1.6s ease-out 0.7s infinite; }
    }

    .processing {
        --orb-color: var(--accent);
        --orb-glow: var(--accent-glow);

        .core {
            border-color: rgba(99, 102, 241, 0.5);
            box-shadow: 0 0 40px -8px var(--orb-glow);
        }

        .arc {
            opacity: 1;
            animation: spin 0.9s linear infinite;
        }
    }

    .offline {
        .ring { opacity: 0.12; }
        .ring-2, .ring-3 { opacity: 0.06; }
    }

    @keyframes breathe {
        0%, 100% { box-shadow: 0 0 36px -10px var(--orb-glow); }
        50% { box-shadow: 0 0 56px -6px var(--orb-glow); }
    }

    @keyframes pulse {
        0% { transform: translate(-50%, -50%) scale(0.72); opacity: 0.6; }
        100% { transform: translate(-50%, -50%) scale(1.06); opacity: 0; }
    }

    @keyframes spin {
        to { transform: translate(-50%, -50%) rotate(360deg); }
    }
</style>

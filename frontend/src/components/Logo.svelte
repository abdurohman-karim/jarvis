<script lang="ts">
    // Neutral wordless mark: rounded tile with a sound-wave glyph
    export let size: number = 36
</script>

<svg width={size} height={size} viewBox="0 0 64 64" fill="none" aria-label="Jarvis" class="logo">
    <defs>
        <linearGradient id="logo-bg" x1="0" y1="0" x2="64" y2="64" gradientUnits="userSpaceOnUse">
            <stop offset="0" stop-color="#6d70f5" />
            <stop offset="1" stop-color="#4f52d9" />
        </linearGradient>
    </defs>
    <rect width="64" height="64" rx="16" fill="url(#logo-bg)" />
    <g stroke="#ffffff" stroke-width="5" stroke-linecap="round">
        <path d="M16 28v8" />
        <path d="M24 21v22" />
        <path d="M32 15v34" />
        <path d="M40 21v22" />
        <path d="M48 28v8" />
    </g>
</svg>

<style>
    .logo {
        display: block;
        flex-shrink: 0;
    }
</style>

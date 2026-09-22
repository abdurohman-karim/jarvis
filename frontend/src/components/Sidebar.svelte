<script lang="ts">
    import { goto, isActive } from "@roxi/routify"
    import { invoke } from "@tauri-apps/api/core"
    import { onMount } from "svelte"

    import Logo from "./Logo.svelte"
    import Icon from "./ui/Icon.svelte"
    import { currentLanguage, setLanguage, translations, translate, isJarvisRunning } from "@/stores"

    $: t = (key: string) => translate($translations, key)

    let appVersion = ""
    let langOpen = false

    const languages = [
        { code: "ru", label: "RU", name: "Русский" },
        { code: "en", label: "EN", name: "English" },
        { code: "ua", label: "UA", name: "Українська" },
    ]

    $: navItems = [
        { path: "/", icon: "home", label: t("nav-assistant") },
        { path: "/commands", icon: "terminal", label: t("nav-commands") },
        { path: "/settings", icon: "settings", label: t("nav-settings") },
    ]

    $: currentLang = languages.find(l => l.code === $currentLanguage) || languages[0]

    onMount(async () => {
        try {
            appVersion = await invoke<string>("get_app_version")
        } catch {
            appVersion = ""
        }
    })

    async function pickLanguage(code: string) {
        langOpen = false
        await setLanguage(code)
    }

    function closeLang(e: MouseEvent) {
        if (!(e.target as HTMLElement).closest(".lang")) langOpen = false
    }
</script>

<svelte:window on:click={closeLang} />

<aside class="sidebar">
    <div class="brand" title="Jarvis {appVersion}">
        <Logo size={38} />
        <span class="status-dot" class:online={$isJarvisRunning}></span>
    </div>

    <nav class="nav">
        {#each navItems as item}
            <button
                class="nav-item"
                class:active={$isActive(item.path)}
                on:click={() => $goto(item.path)}
            >
                <Icon name={item.icon} size={20} />
                <span>{item.label}</span>
            </button>
        {/each}
    </nav>

    <div class="bottom">
        <div class="lang">
            <button class="lang-btn" class:open={langOpen} on:click|stopPropagation={() => langOpen = !langOpen} title={currentLang.name}>
                <img src="/media/flags/{currentLang.label}.png" alt={currentLang.label} width="20" />
                <span>{currentLang.label}</span>
            </button>

            {#if langOpen}
                <div class="lang-menu">
                    {#each languages as lang}
                        <button
                            class="lang-option"
                            class:active={lang.code === $currentLanguage}
                            on:click|stopPropagation={() => pickLanguage(lang.code)}
                        >
                            <img src="/media/flags/{lang.label}.png" alt={lang.label} width="18" />
                            <span>{lang.name}</span>
                            {#if lang.code === $currentLanguage}<Icon name="check" size={14} />{/if}
                        </button>
                    {/each}
                </div>
            {/if}
        </div>

        {#if appVersion}
            <span class="version">v{appVersion}</span>
        {/if}
    </div>
</aside>

<style lang="scss">
    .sidebar {
        display: flex;
        flex-direction: column;
        align-items: center;
        width: var(--sidebar-width);
        height: 100%;
        padding: 18px 8px 14px;
        background: var(--bg-elevated);
        border-right: 1px solid var(--border);
        flex-shrink: 0;
    }

    .brand {
        position: relative;
        margin-bottom: 22px;

        .status-dot {
            position: absolute;
            right: -3px;
            bottom: -3px;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            background: var(--text-faint);
            border: 2px solid var(--bg-elevated);
            transition: background var(--duration) var(--ease), box-shadow var(--duration) var(--ease);

            &.online {
                background: var(--success);
                box-shadow: 0 0 8px rgba(34, 197, 94, 0.6);
            }
        }
    }

    .nav {
        display: flex;
        flex-direction: column;
        gap: 4px;
        width: 100%;
    }

    .nav-item {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 4px;
        width: 100%;
        padding: 10px 4px 8px;
        border-radius: var(--radius-sm);
        color: var(--text-muted);
        font-size: 10px;
        font-weight: 500;
        letter-spacing: 0.01em;
        transition: background var(--duration) var(--ease), color var(--duration) var(--ease);

        span {
            max-width: 100%;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
        }

        &:hover {
            background: var(--surface);
            color: var(--text-secondary);
        }

        &.active {
            background: var(--accent-soft);
            color: var(--accent-hover);
        }
    }

    .bottom {
        margin-top: auto;
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 8px;
        width: 100%;
    }

    .lang {
        position: relative;
        width: 100%;
    }

    .lang-btn {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 3px;
        width: 100%;
        padding: 8px 4px;
        border-radius: var(--radius-sm);
        color: var(--text-muted);
        font-size: 10.5px;
        font-weight: 600;
        transition: background var(--duration) var(--ease), color var(--duration) var(--ease);

        img { border-radius: 3px; opacity: 0.85; }

        &:hover, &.open {
            background: var(--surface);
            color: var(--text-secondary);
        }
    }

    .lang-menu {
        position: absolute;
        left: calc(100% + 6px);
        bottom: 0;
        min-width: 160px;
        padding: 4px;
        background: var(--surface);
        border: 1px solid var(--border-strong);
        border-radius: var(--radius-sm);
        box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
        z-index: 50;
    }

    .lang-option {
        display: flex;
        align-items: center;
        gap: 10px;
        width: 100%;
        padding: 8px 10px;
        border-radius: 6px;
        font-size: 13px;
        color: var(--text-secondary);
        text-align: left;
        transition: background var(--duration) var(--ease);

        img { border-radius: 2px; }
        span { flex: 1; }

        &:hover { background: var(--surface-hover); color: var(--text); }
        &.active { color: var(--accent-hover); }
    }

    .version {
        font-size: 10px;
        color: var(--text-faint);
        font-family: var(--font-mono);
    }
</style>

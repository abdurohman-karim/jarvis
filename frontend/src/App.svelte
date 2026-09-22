<script lang="ts">
    import { onMount, onDestroy } from "svelte"
    import { Router } from "@roxi/routify"
    import routes from "../.routify/routes.default.js"
    import Events from "./Events.svelte"

    import {
        loadVoiceSetting,
        loadAppInfo,
        startStatsPolling,
        stopStatsPolling,
        enableIpc,
        disableIpc,
        isJarvisRunning,
        loadTranslations
    } from "@/stores"

    let unsubscribe: (() => void) | null = null

    onMount(() => {
        loadVoiceSetting()
        loadAppInfo()
        startStatsPolling(5000)
        loadTranslations()

        // the IPC connection follows the assistant process, independent of which page is open
        unsubscribe = isJarvisRunning.subscribe(running => {
            if (running) enableIpc()
            else disableIpc()
        })
    })

    onDestroy(() => {
        unsubscribe?.()
        stopStatsPolling()
        disableIpc()
    })
</script>

<Router {routes} />
<Events />

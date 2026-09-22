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
        connectIpc,
        disconnectIpc,
        loadTranslations
    } from "@/stores"

    onMount(() => {
        loadVoiceSetting()
        loadAppInfo()
        startStatsPolling(5000)
        connectIpc()
        loadTranslations()
    })

    onDestroy(() => {
        stopStatsPolling()
        disconnectIpc()
    })
</script>

<Router {routes} />
<Events />

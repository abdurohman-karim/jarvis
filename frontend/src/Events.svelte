<script lang="ts">
    import { onMount } from "svelte"
    import { listen } from "@tauri-apps/api/event"
    import { invoke } from "@tauri-apps/api/core"
    import { assistantVoice } from "@/stores"

    let voiceVal = ""
    assistantVoice.subscribe(value => {
        voiceVal = value
    })

    onMount(async () => {
        // audio playback requested by the backend
        await listen<{ data: string }>("audio-play", async (event) => {
            const filename = `sound/${voiceVal}/${event.payload.data}.wav`

            try {
                await invoke("play_sound", { filename, sleep: true })
            } catch (err) {
                console.error("failed to play sound:", err)
            }
        })
    })
</script>

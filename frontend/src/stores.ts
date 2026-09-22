import { writable } from "svelte/store"
import { invoke } from "@tauri-apps/api/core"

// ### RE-EXPORT IPC STORES
export {
    jarvisState,
    ipcConnected,
    lastRecognizedText,
    lastExecutedCommand,
    lastError,
    connectIpc,
    enableIpc,
    disableIpc,
    disconnectIpc,
    sendIpcMessage,
    sendTextCommand,
    stopJarvisApp,
    reloadCommands,
    setMuted,
    applySettings,
    isMuted,
    commandsVersion,
    lastAppliedSettings,
    lastAiAnswer
} from "./lib/ipc"

import { get } from "svelte/store"
import { stopJarvisApp as ipcStop } from "./lib/ipc"

// re-export i18n
export {
    translations,
    currentLanguage,
    translate,
    loadTranslations,
    setLanguage,
    loadLanguage,
    getSupportedLanguages
} from "./lib/i18n"

// ### RUNNING STATE
export const isJarvisRunning = writable(false)
export const jarvisRamUsage = writable(0)
export const jarvisCpuUsage = writable(0)

// ### ASSISTANT VOICE
export const assistantVoice = writable("")

// ### APP INFO
export const appInfo = writable({
    logFilePath: ""
})

// ### INIT FUNCTIONS (call these from a component)
export async function loadVoiceSetting() {
    try {
        const voice = await invoke<string>("db_read", { key: "assistant_voice" })
        assistantVoice.set(voice)
    } catch (err) {
        console.error("failed to load voice setting:", err)
    }
}

export async function loadAppInfo() {
    try {
        const logPath = await invoke<string>("get_log_file_path")
        appInfo.set({ logFilePath: logPath })
    } catch (err) {
        console.error("failed to load app info:", err)
    }
}

export async function updateJarvisStats() {
    try {
        const stats = await invoke<{running: boolean, ram_mb: number, cpu_usage: number}>("get_jarvis_app_stats")
        isJarvisRunning.set(stats.running)
        jarvisRamUsage.set(stats.ram_mb)
        jarvisCpuUsage.set(stats.cpu_usage)
    } catch (err) {
        console.error("failed to get jarvis stats:", err)
    }
}

// ### ASSISTANT PROCESS CONTROL
export const assistantBusy = writable(false)

export async function startAssistant() {
    assistantBusy.set(true)
    try {
        await invoke("run_jarvis_app")
        // give it a moment to come up, then poll until the process is visible
        for (let i = 0; i < 10; i++) {
            await new Promise(r => setTimeout(r, 500))
            await updateJarvisStats()
            if (get(isJarvisRunning)) break
        }
    } catch (err) {
        console.error("Failed to run jarvis-app:", err)
    } finally {
        assistantBusy.set(false)
    }
}

export async function stopAssistant() {
    assistantBusy.set(true)
    try {
        ipcStop()
        // the process plays a goodbye sound before exiting, so poll for a while
        for (let i = 0; i < 15; i++) {
            await new Promise(r => setTimeout(r, 1000))
            await updateJarvisStats()
            if (!get(isJarvisRunning)) break
        }
    } catch (err) {
        console.error("Failed to stop jarvis-app:", err)
    } finally {
        assistantBusy.set(false)
    }
}

export async function restartAssistant() {
    await stopAssistant()
    await startAssistant()
}

// polling manager
let statsInterval: ReturnType<typeof setInterval> | null = null

export function startStatsPolling(intervalMs = 5000) {
    if (statsInterval) return // already running
    
    updateJarvisStats()
    statsInterval = setInterval(updateJarvisStats, intervalMs)
}

export function stopStatsPolling() {
    if (statsInterval) {
        clearInterval(statsInterval)
        statsInterval = null
    }
}
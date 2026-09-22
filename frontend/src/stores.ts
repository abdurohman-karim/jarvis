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
    sendAction,
    sendIpcMessage,
    sendTextCommand,
    stopJarvisApp,
    reloadCommands
} from "./lib/ipc"

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
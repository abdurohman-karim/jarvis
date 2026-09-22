import { writable, get } from "svelte/store"
import { invoke } from "@tauri-apps/api/core"
import { getCurrentWindow } from "@tauri-apps/api/window"

// generated from the Rust definitions by `cargo test -p jarvis-core` - do not hand-edit
import type { IpcEvent, IpcAction } from "./ipc-types"

// bump together with PROTOCOL_VERSION in crates/jarvis-core/src/ipc/events.rs
const PROTOCOL_VERSION = 1

// ### IPC STORES ###

export type JarvisState = "disconnected" | "idle" | "listening" | "processing"

export const jarvisState = writable<JarvisState>("disconnected")
export const ipcConnected = writable(false)
export const lastRecognizedText = writable("")
export const lastExecutedCommand = writable("")
export const lastError = writable("")
export const isMuted = writable(false)
// bumps every time the assistant reloaded its command packs
export const commandsVersion = writable(0)
// what the assistant re-applied after the last settings save
export const lastAppliedSettings = writable<string[] | null>(null)
// set to the assistant's protocol version when it differs from this UI's
export const protocolMismatch = writable<number | null>(null)

// ### CONNECTION ###

const IPC_URL = "ws://127.0.0.1:9712"
const RECONNECT_DELAY = 5000

let ws: WebSocket | null = null
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let manualDisconnect = false
let enabled = false  // only connect when enabled

export function enableIpc() {
    enabled = true
    connectIpc()
}

export function disableIpc() {
    enabled = false
    disconnectIpc()
}

export async function connectIpc(port: number = 9712) {
    if (ws?.readyState === WebSocket.OPEN || ws?.readyState === WebSocket.CONNECTING) return
    manualDisconnect = false

    // the assistant writes a session token on startup; no token means it is not running yet
    let token: string
    try {
        token = await invoke<string>("get_ipc_token")
    } catch {
        scheduleReconnect()
        return
    }

    ws = new WebSocket(`ws://127.0.0.1:${port}`)

    ws.onopen = () => {
        // the server answers a valid token with `started`; that marks us connected
        send({ action: "auth", token })
    }

    ws.onclose = () => {
        ipcConnected.set(false)
        jarvisState.set("disconnected")
        console.log("[IPC] disconnected")
        scheduleReconnect()
    }

    ws.onerror = (err) => {
        console.error("[IPC] error:", err)
    }

    ws.onmessage = (event) => {
        try {
            const msg = JSON.parse(event.data)
            handleEvent(msg)
        } catch (e) {
            console.error("[IPC] failed to parse message:", e)
        }
    }
}

function scheduleReconnect() {
    if (reconnectTimer || manualDisconnect || !enabled) return

    console.log(`IPC: Will retry in ${RECONNECT_DELAY / 1000}s...`)
    reconnectTimer = setTimeout(() => {
        reconnectTimer = null
        connectIpc()
    }, RECONNECT_DELAY)
}

export function disconnectIpc() {
    manualDisconnect = true

    if (reconnectTimer) {
        clearTimeout(reconnectTimer)
        reconnectTimer = null
    }

    if (ws) {
        ws.close()
        ws = null
    }

    ipcConnected.set(false)
    jarvisState.set("disconnected")
}

// ### EVENT HANDLING ###

function handleEvent(data: IpcEvent) {
    console.log("IPC: Event", data.event, data)

    switch (data.event) {
        case "wake_word_detected":
        case "listening":
            jarvisState.set("listening")
            break

        case "speech_recognized":
            lastRecognizedText.set(data.text)
            jarvisState.set("processing")
            break

        case "command_executed":
            lastExecutedCommand.set(data.id)
            break

        case "idle":
            jarvisState.set("idle")
            break

        case "error":
            lastError.set(data.message)
            break

        case "started":
            if (data.protocol !== PROTOCOL_VERSION) {
                console.warn(`[IPC] protocol mismatch: assistant speaks v${data.protocol}, this UI v${PROTOCOL_VERSION}`)
                protocolMismatch.set(data.protocol)
            }
            if (!get(ipcConnected)) {
                console.log("[IPC] connected")
                ipcConnected.set(true)
                isMuted.set(false)
            }
            jarvisState.set("idle")
            break

        case "stopping":
            jarvisState.set("disconnected")
            break

        case "pong":
            // connection verified
            break

        case "reveal_window":
            // bring window to foreground
            revealWindow()
            break

        case "muted":
            isMuted.set(data.muted)
            break

        case "commands_reloaded":
            commandsVersion.update(v => v + 1)
            break

        case "settings_applied":
            lastAppliedSettings.set(data.changed)
            break
    }
}

// ### ACTIONS ###

// only actions the assistant actually understands can be sent
function send(action: IpcAction): boolean {
    if (ws?.readyState !== WebSocket.OPEN) {
        return false
    }

    ws.send(JSON.stringify(action))
    return true
}

export function stopJarvisApp() {
    return send({ action: "stop" })
}

export function reloadCommands() {
    return send({ action: "reload_commands" })
}

export function setMuted(muted: boolean) {
    return send({ action: "set_muted", muted })
}

// ask the running assistant to re-read the settings file and apply what changed
export function applySettings() {
    return send({ action: "apply_settings" })
}

export function sendIpcMessage(message: object): Promise<void> {
    return new Promise((resolve, reject) => {
        if (!ws || ws.readyState !== WebSocket.OPEN) {
            reject(new Error("IPC not connected"))
            return
        }

        try {
            ws.send(JSON.stringify(message))
            resolve()
        } catch (err) {
            reject(err)
        }
    })
}

export function sendTextCommand(text: string): boolean {
    return send({ action: "text_command", text })
}

async function revealWindow() {
    try {
        const window = getCurrentWindow()
        await window.show()
        await window.unminimize()
        await window.setFocus()
    } catch (e) {
        console.error("[IPC] Failed to reveal window:", e)
    }
}

use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use once_cell::sync::OnceCell;
use parking_lot::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_hdr_async, tungstenite::Message};
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response, ErrorResponse};

use super::events::{IpcAction, IpcEvent, PROTOCOL_VERSION};
use crate::APP_CONFIG_DIR;

pub const IPC_PORT: u16 = 9712;
pub const IPC_ADDR: &str = "127.0.0.1";

// The IPC socket is reachable by every local process (and every web page, via
// WebSocket), and it can execute commands. Clients must therefore present the token
// from config::IPC_TOKEN_FILE, which only the current user can read.
pub use crate::config::IPC_TOKEN_FILE;

static TOKEN: OnceCell<String> = OnceCell::new();

static BROADCAST_TX: OnceCell<broadcast::Sender<IpcEvent>> = OnceCell::new();
static ACTION_HANDLER: OnceCell<Arc<RwLock<Option<Box<dyn Fn(IpcAction) + Send + Sync>>>>> = OnceCell::new();

// Initialize the IPC broadcast channel
pub fn init() -> broadcast::Sender<IpcEvent> {
    if let Some(tx) = BROADCAST_TX.get() {
        return tx.clone();
    }

    let (tx, _) = broadcast::channel::<IpcEvent>(32);
    BROADCAST_TX.set(tx.clone()).ok();
    ACTION_HANDLER.set(Arc::new(RwLock::new(None))).ok();
    
    info!("IPC: Broadcast channel initialized");
    tx
}

// Generate a fresh session token and write it to the config dir (user-only permissions)
pub fn init_token() -> std::io::Result<String> {
    use rand::RngCore;

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();

    let path = token_path()?;
    std::fs::write(&path, &token)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    let _ = TOKEN.set(token.clone());
    info!("IPC: token written to {}", path.display());
    Ok(token)
}

// Read the current token (used by clients running as the same user, e.g. the GUI)
pub fn read_token() -> std::io::Result<String> {
    Ok(std::fs::read_to_string(token_path()?)?.trim().to_string())
}

fn token_path() -> std::io::Result<std::path::PathBuf> {
    APP_CONFIG_DIR.get()
        .map(|dir| dir.join(IPC_TOKEN_FILE))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "config dir not initialized"))
}

fn token_matches(candidate: &str) -> bool {
    // constant-time compare, tokens are hex strings of equal length
    match TOKEN.get() {
        Some(token) if token.len() == candidate.len() => {
            token.bytes().zip(candidate.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
        }
        _ => false,
    }
}

// Reject WebSocket handshakes coming from web pages. Native clients send no Origin,
// the Tauri webview sends its own scheme; anything http(s) is a browser tab.
fn check_origin(req: &Request, resp: Response) -> Result<Response, ErrorResponse> {
    if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
        let allowed = origin.starts_with("tauri://")
            || origin.starts_with("http://tauri.localhost")
            || origin == "http://localhost:1420"; // `tauri dev`
        if !allowed {
            warn!("IPC: rejected connection from origin {}", origin);
            let mut resp = ErrorResponse::new(Some("forbidden origin".into()));
            *resp.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
            return Err(resp);
        }
    }
    Ok(resp)
}

// Send event to all connected clients
pub fn send(event: IpcEvent) {
    if let Some(tx) = BROADCAST_TX.get() {
        match tx.send(event.clone()) {
            Ok(n) => {
                if n > 0 {
                    debug!("IPC: Sent {:?} to {} client(s)", event, n);
                }
            }
            Err(_) => {
                // no receivers, that's fine
            }
        }
    }
}

// Register handler for incoming actions from GUI
pub fn set_action_handler<F>(handler: F)
where
    F: Fn(IpcAction) + Send + Sync + 'static,
{
    if let Some(h) = ACTION_HANDLER.get() {
        *h.write() = Some(Box::new(handler));
    }
}

fn handle_action(action: IpcAction) {
    info!("IPC: Received action {:?}", action);
    
    // handle ping internally
    if matches!(action, IpcAction::Ping) {
        send(IpcEvent::Pong);
        return;
    }
    
    // forward to registered handler
    if let Some(handler_lock) = ACTION_HANDLER.get() {
        let handler = handler_lock.read();
        if let Some(ref h) = *handler {
            h(action);
        }
    }
}

// Bind the IPC port synchronously. Done first thing at startup so that a second
// instance fails fast instead of running half-initialized without IPC.
pub fn bind() -> std::io::Result<std::net::TcpListener> {
    let addr = format!("{}:{}", IPC_ADDR, IPC_PORT);
    let listener = std::net::TcpListener::bind(&addr)?;
    listener.set_nonblocking(true)?;
    Ok(listener)
}

// Start the WebSocket server (blocking)
pub async fn start_server() {
    match bind() {
        Ok(l) => start_server_on(l).await,
        Err(e) => error!("IPC: Failed to bind to {}:{}: {}", IPC_ADDR, IPC_PORT, e),
    }
}

// Start the WebSocket server on an already bound listener (see `bind`)
pub async fn start_server_on(std_listener: std::net::TcpListener) {
    let listener = match TcpListener::from_std(std_listener) {
        Ok(l) => {
            info!("IPC: WebSocket server listening on ws://{}:{}", IPC_ADDR, IPC_PORT);
            l
        }
        Err(e) => {
            error!("IPC: Failed to register listener: {}", e);
            return;
        }
    };

    // notify that we're ready
    send(IpcEvent::Started { protocol: PROTOCOL_VERSION });

    while let Ok((stream, peer_addr)) = listener.accept().await {
        info!("IPC: Client connecting from {}", peer_addr);
        
        let rx = BROADCAST_TX
            .get()
            .map(|tx| tx.subscribe())
            .expect("IPC not initialized");

        tokio::spawn(handle_client(stream, peer_addr, rx));
    }
}

async fn handle_client(
    stream: TcpStream,
    peer_addr: SocketAddr,
    mut event_rx: broadcast::Receiver<IpcEvent>,
) {
    let ws_stream = match accept_hdr_async(stream, check_origin).await {
        Ok(ws) => {
            info!("IPC: Client connected: {}", peer_addr);
            ws
        }
        Err(e) => {
            error!("IPC: WebSocket handshake failed for {}: {}", peer_addr, e);
            return;
        }
    };

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // nothing is delivered or accepted until the client authenticated
    let mut authenticated = false;

    loop {
        tokio::select! {
            // forward events to client
            event_result = event_rx.recv(), if authenticated => {
                match event_result {
                    Ok(event) => {
                        let json = match serde_json::to_string(&event) {
                            Ok(j) => j,
                            Err(e) => {
                                error!("IPC: Failed to serialize event: {}", e);
                                continue;
                            }
                        };

                        if ws_tx.send(Message::Text(json.into())).await.is_err() {
                            info!("IPC: Client {} disconnected (send failed)", peer_addr);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("IPC: Client {} lagged {} events", peer_addr, n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("IPC: Broadcast channel closed");
                        break;
                    }
                }
            }

            // receive messages from client
            msg_result = ws_rx.next() => {
                match msg_result {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<IpcAction>(&text) {
                            Ok(IpcAction::Auth { token }) => {
                                if token_matches(&token) {
                                    authenticated = true;
                                    debug!("IPC: Client {} authenticated", peer_addr);
                                    // let the client know it may start sending actions
                                    let _ = ws_tx.send(Message::Text(
                                        serde_json::to_string(&IpcEvent::Started { protocol: PROTOCOL_VERSION })
                                            .unwrap_or_default().into()
                                    )).await;
                                } else {
                                    warn!("IPC: Client {} sent an invalid token, closing", peer_addr);
                                    break;
                                }
                            }
                            Ok(action) if !authenticated => {
                                warn!("IPC: Client {} sent {:?} before authenticating, closing", peer_addr, action);
                                break;
                            }
                            Ok(action) => handle_action(action),
                            Err(e) => {
                                warn!("IPC: Invalid action from {}: {} ({})", peer_addr, text, e);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if ws_tx.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("IPC: Client {} sent close frame", peer_addr);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("IPC: Error receiving from {}: {}", peer_addr, e);
                        break;
                    }
                    None => {
                        info!("IPC: Client {} stream ended", peer_addr);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    info!("IPC: Client disconnected: {}", peer_addr);
}

pub fn has_clients() -> bool {
    if let Some(tx) = BROADCAST_TX.get() {
        tx.receiver_count() > 0
    } else {
        false
    }
}
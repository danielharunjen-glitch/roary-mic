use log::{debug, error};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};

pub const AI_REPLY_WINDOW_LABEL: &str = "ai_reply";

const AI_REPLY_WIDTH: f64 = 480.0;
const AI_REPLY_HEIGHT: f64 = 320.0;

#[derive(Clone, serde::Serialize)]
pub struct AiReplyPayload {
    pub text: String,
}

/// Holds a reply payload while the window's JS is still initializing. Once the
/// frontend emits `ai-reply-window-ready`, `listener_ready` flips to true and
/// we skip the pending mailbox entirely (the listener is attached, emits are
/// delivered directly). Without this flag, every `show_ai_reply_window` would
/// store a copy of the text forever, leading to stale replays on any future
/// webview reload.
#[derive(Default)]
pub struct AiReplyPending {
    pub text: Mutex<Option<String>>,
    pub listener_ready: AtomicBool,
}

/// Create the AI reply window eagerly (hidden) and register the ready-handshake.
/// Called once at startup so the React listener is registered before any
/// `ai-mode-reply-ready` event fires.
pub fn create_ai_reply_window(app: &AppHandle) {
    app.manage(AiReplyPending::default());
    ensure_ai_reply_window(app);

    // Frontend emits this once `listen("ai-mode-reply-ready", ...)` has
    // resolved; if a payload was queued before mount, we replay it now, and
    // mark the listener as ready so future emits skip the mailbox.
    let handle = app.clone();
    app.listen("ai-reply-window-ready", move |_| {
        let pending = handle.state::<AiReplyPending>();
        let text = {
            let mut guard = pending.text.lock().unwrap_or_else(|p| p.into_inner());
            guard.take()
        };
        pending.listener_ready.store(true, Ordering::Release);
        if let Some(text) = text {
            if let Err(e) = handle.emit("ai-mode-reply-ready", AiReplyPayload { text }) {
                error!("Failed to replay ai-mode-reply-ready: {}", e);
            }
        }
    });
}

/// Shows the AI reply window. The window was pre-created at startup so the
/// React listener is already attached — the reply text is delivered via the
/// `ai-mode-reply-ready` event. If the window was recreated (or recovered from
/// a crash) and the listener isn't attached yet, `AiReplyPending` holds the
/// payload until `ai-reply-window-ready` fires.
pub fn show_ai_reply_window(app: &AppHandle, text: String) {
    ensure_ai_reply_window(app);

    // Only queue into the mailbox when the frontend listener hasn't signalled
    // readiness yet. Once the handshake has fired we know every future emit
    // lands, so skipping the mailbox prevents stale text from surviving to the
    // next webview reload.
    if let Some(pending) = app.try_state::<AiReplyPending>() {
        if !pending.listener_ready.load(Ordering::Acquire) {
            if let Ok(mut guard) = pending.text.lock() {
                *guard = Some(text.clone());
            }
        }
    }

    if let Err(e) = app.emit("ai-mode-reply-ready", AiReplyPayload { text }) {
        error!("Failed to emit ai-mode-reply-ready: {}", e);
    }

    if let Some(window) = app.get_webview_window(AI_REPLY_WINDOW_LABEL) {
        if let Err(e) = window.show() {
            error!("Failed to show AI reply window: {}", e);
        }
        if let Err(e) = window.set_focus() {
            debug!("Failed to focus AI reply window: {}", e);
        }
    }
}

/// Hide the AI reply window if it exists. Never destroys the window so that
/// subsequent shows are cheap.
pub fn hide_ai_reply_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(AI_REPLY_WINDOW_LABEL) {
        if let Err(e) = window.hide() {
            debug!("Failed to hide AI reply window: {}", e);
        }
    }
}

fn ensure_ai_reply_window(app: &AppHandle) {
    if app.get_webview_window(AI_REPLY_WINDOW_LABEL).is_some() {
        return;
    }

    let mut builder = WebviewWindowBuilder::new(
        app,
        AI_REPLY_WINDOW_LABEL,
        WebviewUrl::App("src/ai-reply/index.html".into()),
    )
    .title("Roary Mic — AI Reply")
    .inner_size(AI_REPLY_WIDTH, AI_REPLY_HEIGHT)
    .min_inner_size(AI_REPLY_WIDTH, AI_REPLY_HEIGHT)
    .resizable(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .center()
    .focused(true)
    .visible(false);

    if let Some(data_dir) = crate::portable::data_dir() {
        builder = builder.data_directory(data_dir.join("webview"));
    }

    match builder.build() {
        Ok(_) => debug!("AI reply window created (hidden)"),
        Err(e) => error!("Failed to create AI reply window: {}", e),
    }
}

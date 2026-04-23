use log::{debug, error};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const AI_REPLY_WINDOW_LABEL: &str = "ai_reply";

const AI_REPLY_WIDTH: f64 = 480.0;
const AI_REPLY_HEIGHT: f64 = 320.0;

#[derive(Clone, serde::Serialize)]
pub struct AiReplyPayload {
    pub text: String,
}

/// Shows the AI reply window, creating it on first use. The window comes up
/// frameless, always-on-top, and centered on screen. The provided reply text is
/// sent to the window via the `ai-mode-reply-ready` event.
pub fn show_ai_reply_window(app: &AppHandle, text: String) {
    ensure_ai_reply_window(app);

    // Emit before showing so the React app has the text on mount.
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

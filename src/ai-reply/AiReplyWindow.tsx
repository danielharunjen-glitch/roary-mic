import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { syncLanguageFromSettings } from "@/i18n";

type ReplyPayload = { text: string };

const AiReplyWindow: React.FC = () => {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const pasteButtonRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    (async () => {
      await syncLanguageFromSettings();
      const handler = await listen<ReplyPayload>(
        "ai-mode-reply-ready",
        (event) => {
          setText(event.payload?.text ?? "");
          setIsSpeaking(false);
          setErrorMessage(null);
          // Focus the default action once the text is in place.
          queueMicrotask(() => pasteButtonRef.current?.focus());
        },
      );
      if (cancelled) {
        handler();
        return;
      }
      unlisten = handler;
      // Handshake: tell the backend we're ready to receive payloads. If a
      // reply was queued before mount (first-use race), it replays now.
      await emit("ai-reply-window-ready");
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const handleCancel = useCallback(async () => {
    await commands.aiReplyCancel();
    try {
      await getCurrentWebviewWindow().hide();
    } catch {
      // Hide is best-effort — the backend also hides the window.
    }
  }, []);

  const handlePaste = useCallback(async () => {
    const result = await commands.aiReplyPaste(text);
    if (result.status !== "ok") {
      setErrorMessage(result.error ?? t("aiReplyWindow.pasteError"));
    }
  }, [text, t]);

  const handleSpeak = useCallback(async () => {
    setIsSpeaking(true);
    setErrorMessage(null);
    const result = await commands.aiReplySpeak(text);
    setIsSpeaking(false);
    if (result.status !== "ok") {
      setErrorMessage(result.error ?? t("aiReplyWindow.speakError"));
    }
  }, [text, t]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        handleCancel();
      } else if (e.key === "Enter" && !e.shiftKey) {
        // Textarea absorbs Enter by default — only handle when focus is
        // outside the textarea (e.g. on a button or the window chrome).
        const target = e.target as HTMLElement | null;
        if (target && target.tagName !== "TEXTAREA") {
          e.preventDefault();
          handlePaste();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handlePaste, handleCancel]);

  return (
    <div className="h-screen w-screen flex flex-col bg-background text-text border border-mid-gray/30 rounded-lg overflow-hidden">
      <header
        data-tauri-drag-region
        className="px-4 py-3 border-b border-mid-gray/20 flex items-center justify-between"
      >
        <h1 className="text-sm font-semibold">{t("aiReplyWindow.title")}</h1>
        <span className="text-xs text-mid-gray">
          {t("aiReplyWindow.subtitle")}
        </span>
      </header>
      <main className="flex-1 p-4 flex flex-col gap-3 min-h-0">
        <textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          className="flex-1 resize-none w-full text-sm bg-background border border-mid-gray/30 rounded-md px-3 py-2 focus:outline-none focus:border-logo-primary"
          aria-label={t("aiReplyWindow.textAreaLabel")}
        />
        {errorMessage && (
          <p className="text-xs text-red-600" role="alert">
            {errorMessage}
          </p>
        )}
      </main>
      <footer className="px-4 py-3 border-t border-mid-gray/20 flex justify-end gap-2">
        <button
          type="button"
          onClick={handleCancel}
          className="text-sm px-3 py-1.5 rounded-md border border-mid-gray/30 hover:border-mid-gray/60"
        >
          {t("aiReplyWindow.cancel")}
        </button>
        <button
          type="button"
          onClick={handleSpeak}
          disabled={isSpeaking || text.trim().length === 0}
          className="text-sm px-3 py-1.5 rounded-md border border-mid-gray/30 hover:border-logo-primary disabled:opacity-50"
        >
          {isSpeaking ? t("aiReplyWindow.speaking") : t("aiReplyWindow.speak")}
        </button>
        <button
          ref={pasteButtonRef}
          type="button"
          onClick={handlePaste}
          disabled={text.trim().length === 0}
          className="text-sm px-3 py-1.5 rounded-md bg-logo-primary text-text font-medium disabled:opacity-50"
        >
          {t("aiReplyWindow.paste")}
        </button>
      </footer>
    </div>
  );
};

export default AiReplyWindow;

import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import { ArrowRight, Check, Plus, Power, Trash2, X } from "lucide-react";
import { Button } from "../../ui/Button";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { events } from "@/bindings";
import {
  correctionCommands,
  type Correction,
  type PendingCorrectionEvent,
} from "@/lib/corrections";
import { formatDateTime } from "@/utils/dateFormat";
import { useSettings } from "@/hooks/useSettings";

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer disabled:cursor-not-allowed disabled:text-text/20 ${
      active
        ? "text-logo-primary hover:text-logo-primary/80"
        : "text-text/50 hover:text-logo-primary"
    }`}
    title={title}
  >
    {children}
  </button>
);

export const CorrectionsSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const autoCaptureEnabled =
    settings?.auto_capture_corrections_enabled ?? false;
  const [corrections, setCorrections] = useState<Correction[]>([]);
  const [pending, setPending] = useState<Correction[]>([]);
  const [loading, setLoading] = useState(true);
  const [newOriginal, setNewOriginal] = useState("");
  const [newCorrected, setNewCorrected] = useState("");
  const [adding, setAdding] = useState(false);

  const loadCorrections = useCallback(async () => {
    const result = await correctionCommands.listCorrections(200, "correction");
    if (result.status === "ok") {
      setCorrections(result.data);
    } else {
      console.error("Failed to load corrections:", result.error);
    }
    setLoading(false);
  }, []);

  const loadPending = useCallback(async () => {
    const result = await correctionCommands.listPendingAuto(50);
    if (result.status === "ok") {
      setPending(result.data);
    }
  }, []);

  useEffect(() => {
    loadCorrections();
    loadPending();
  }, [loadCorrections, loadPending]);

  // Reload when history entries change, since edits may create new corrections.
  useEffect(() => {
    const unlisten = events.historyUpdatePayload.listen((event) => {
      if (event.payload.action === "updated") {
        loadCorrections();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadCorrections]);

  // Refresh the pending list when new candidates arrive from a paste commit.
  useEffect(() => {
    const unlisten = listen<PendingCorrectionEvent>(
      "auto-correction-pending",
      () => {
        loadPending();
      },
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadPending]);

  const handleAutoCaptureToggle = async (next: boolean) => {
    const res = await correctionCommands.setAutoCaptureEnabled(next);
    if (res.status !== "ok") {
      toast.error(t("autoCapture.saveError"));
      return;
    }
    await refreshSettings();
  };

  const handleAcceptPending = async (id: number) => {
    const prev = pending;
    setPending((p) => p.filter((c) => c.id !== id));
    const res = await correctionCommands.promotePendingAuto(id);
    if (res.status === "ok") {
      await loadCorrections();
    } else {
      setPending(prev);
      toast.error(t("autoCapture.saveError"));
    }
  };

  const handleDiscardPending = async (id: number) => {
    const prev = pending;
    setPending((p) => p.filter((c) => c.id !== id));
    const res = await correctionCommands.discardPendingAuto(id);
    if (res.status !== "ok") {
      setPending(prev);
      toast.error(t("autoCapture.discardError"));
    }
  };

  const handleToggle = async (correction: Correction) => {
    // Optimistic update.
    setCorrections((prev) =>
      prev.map((c) =>
        c.id === correction.id ? { ...c, enabled: !c.enabled } : c,
      ),
    );
    const result = await correctionCommands.setCorrectionEnabled(
      correction.id,
      !correction.enabled,
    );
    if (result.status !== "ok") {
      // Revert.
      setCorrections((prev) =>
        prev.map((c) =>
          c.id === correction.id ? { ...c, enabled: correction.enabled } : c,
        ),
      );
      toast.error(t("settings.corrections.toggleError"));
    }
  };

  const handleAdd = async () => {
    const original = newOriginal.trim();
    const corrected = newCorrected.trim();
    if (original.length === 0 || corrected.length === 0) return;
    if (original === corrected) {
      toast.error(t("settings.corrections.addMustDiffer"));
      return;
    }
    setAdding(true);
    const result = await correctionCommands.insertCorrection(
      original,
      corrected,
      "correction",
    );
    if (result.status === "ok") {
      setCorrections((prev) => [result.data, ...prev]);
      setNewOriginal("");
      setNewCorrected("");
    } else {
      toast.error(t("settings.corrections.addError"));
    }
    setAdding(false);
  };

  const handleDelete = async (correction: Correction) => {
    // Optimistic removal.
    setCorrections((prev) => prev.filter((c) => c.id !== correction.id));
    const result = await correctionCommands.deleteCorrection(correction.id);
    if (result.status !== "ok") {
      loadCorrections();
      toast.error(t("settings.corrections.deleteError"));
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="space-y-2">
        <div className="px-4">
          <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide">
            {t("settings.corrections.title")}
          </h2>
          <p className="text-xs text-mid-gray mt-1">
            {t("settings.corrections.description")}
          </p>
        </div>

        <div className="bg-background border border-mid-gray/20 rounded-lg p-4 space-y-3">
          <ToggleSwitch
            checked={autoCaptureEnabled}
            onChange={handleAutoCaptureToggle}
            label={t("autoCapture.toastTitle")}
            description={t("autoCapture.toastBody", {
              original: "pasted",
              corrected: "edited",
            })}
            descriptionMode="inline"
          />
          {pending.length > 0 && (
            <ul className="divide-y divide-mid-gray/20 border-t border-mid-gray/20">
              {pending.map((p) => (
                <li
                  key={p.id}
                  className="pt-3 first:pt-3 pb-2 flex items-center gap-3"
                >
                  <div className="flex-1 min-w-0 flex items-center gap-2 text-sm">
                    <span
                      className="font-mono px-2 py-0.5 rounded bg-mid-gray/10 truncate"
                      title={p.original_text}
                    >
                      {p.original_text}
                    </span>
                    <ArrowRight className="w-4 h-4 shrink-0 text-text/40" />
                    <span
                      className="font-mono px-2 py-0.5 rounded bg-logo-primary/10 truncate"
                      title={p.corrected_text}
                    >
                      {p.corrected_text}
                    </span>
                  </div>
                  <div className="flex items-center">
                    <IconButton
                      onClick={() => handleAcceptPending(p.id)}
                      active
                      title={t("autoCapture.save")}
                    >
                      <Check className="w-4 h-4" />
                    </IconButton>
                    <IconButton
                      onClick={() => handleDiscardPending(p.id)}
                      title={t("autoCapture.dismiss")}
                    >
                      <X className="w-4 h-4" />
                    </IconButton>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="bg-background border border-mid-gray/20 rounded-lg px-4 py-3 flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-3">
          <input
            type="text"
            value={newOriginal}
            onChange={(e) => setNewOriginal(e.target.value)}
            placeholder={t("settings.corrections.addOriginalPlaceholder")}
            disabled={adding}
            className="flex-1 text-sm font-mono bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
          />
          <ArrowRight className="hidden sm:block w-4 h-4 shrink-0 text-text/40" />
          <input
            type="text"
            value={newCorrected}
            onChange={(e) => setNewCorrected(e.target.value)}
            placeholder={t("settings.corrections.addCorrectedPlaceholder")}
            disabled={adding}
            className="flex-1 text-sm font-mono bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
          />
          <Button
            onClick={handleAdd}
            disabled={
              adding ||
              newOriginal.trim().length === 0 ||
              newCorrected.trim().length === 0
            }
            size="sm"
            className="flex items-center gap-1 shrink-0"
          >
            <Plus className="w-4 h-4" />
            <span>{t("settings.corrections.addButton")}</span>
          </Button>
        </div>
        <div className="bg-background border border-mid-gray/20 rounded-lg overflow-hidden">
          {loading ? (
            <div className="px-4 py-3 text-center text-text/60">
              {t("settings.corrections.loading")}
            </div>
          ) : corrections.length === 0 ? (
            <div className="px-4 py-6 text-center text-text/60 text-sm">
              {t("settings.corrections.empty")}
            </div>
          ) : (
            <ul className="divide-y divide-mid-gray/20">
              {corrections.map((correction) => (
                <li
                  key={correction.id}
                  className="px-4 py-3 flex items-center gap-3"
                >
                  <div className="flex-1 min-w-0 flex items-center gap-2 text-sm">
                    <span
                      className={`font-mono px-2 py-0.5 rounded bg-mid-gray/10 truncate ${
                        correction.enabled ? "" : "opacity-40 line-through"
                      }`}
                      title={correction.original_text}
                    >
                      {correction.original_text}
                    </span>
                    <ArrowRight className="w-4 h-4 shrink-0 text-text/40" />
                    <span
                      className={`font-mono px-2 py-0.5 rounded bg-logo-primary/10 truncate ${
                        correction.enabled ? "" : "opacity-40 line-through"
                      }`}
                      title={correction.corrected_text}
                    >
                      {correction.corrected_text}
                    </span>
                  </div>
                  <span className="text-xs text-text/40 shrink-0 hidden sm:inline">
                    {formatDateTime(
                      String(correction.created_at),
                      i18n.language,
                    )}
                  </span>
                  <div className="flex items-center">
                    <IconButton
                      onClick={() => handleToggle(correction)}
                      active={correction.enabled}
                      title={
                        correction.enabled
                          ? t("settings.corrections.disable")
                          : t("settings.corrections.enable")
                      }
                    >
                      <Power className="w-4 h-4" />
                    </IconButton>
                    <IconButton
                      onClick={() => handleDelete(correction)}
                      title={t("settings.corrections.delete")}
                    >
                      <Trash2 className="w-4 h-4" />
                    </IconButton>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
};

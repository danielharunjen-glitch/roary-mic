import { invoke } from "@tauri-apps/api/core";
import type { HistoryEntry, Result } from "@/bindings";

export type CorrectionKind = "correction" | "reference" | "pending_auto";

export type PendingCorrectionRow = {
  id: number;
  original: string;
  corrected: string;
};

export type PendingCorrectionEvent = {
  candidates: PendingCorrectionRow[];
};

export type Correction = {
  id: number;
  original_text: string;
  corrected_text: string;
  history_id: number | null;
  created_at: number;
  enabled: boolean;
  kind: CorrectionKind;
};

async function wrap<T>(promise: Promise<T>): Promise<Result<T, string>> {
  try {
    return { status: "ok", data: await promise };
  } catch (e) {
    if (e instanceof Error) throw e;
    return { status: "error", error: e as unknown as string };
  }
}

export const correctionCommands = {
  updateHistoryEntryText: (id: number, newText: string) =>
    wrap<HistoryEntry>(invoke("update_history_entry_text", { id, newText })),
  listCorrections: (limit?: number, kind?: CorrectionKind) =>
    wrap<Correction[]>(
      invoke("list_corrections", {
        limit: limit ?? null,
        kind: kind ?? null,
      }),
    ),
  setCorrectionEnabled: (id: number, enabled: boolean) =>
    wrap<null>(invoke("set_correction_enabled", { id, enabled })),
  deleteCorrection: (id: number) =>
    wrap<null>(invoke("delete_correction", { id })),
  insertCorrection: (
    originalText: string,
    correctedText: string,
    kind?: CorrectionKind,
  ) =>
    wrap<Correction>(
      invoke("insert_correction", {
        originalText,
        correctedText,
        kind: kind ?? null,
      }),
    ),
  setAutoCaptureEnabled: (enabled: boolean) =>
    wrap<null>(
      invoke("change_auto_capture_corrections_setting", { enabled }),
    ),
  listPendingAuto: (limit?: number) =>
    wrap<Correction[]>(
      invoke("list_pending_auto_corrections", { limit: limit ?? null }),
    ),
  promotePendingAuto: (id: number) =>
    wrap<null>(invoke("promote_pending_auto_correction", { id })),
  discardPendingAuto: (id: number) =>
    wrap<null>(invoke("discard_pending_auto_correction", { id })),
};

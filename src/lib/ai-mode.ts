import { invoke } from "@tauri-apps/api/core";
import type { Result } from "@/bindings";

async function wrap<T>(promise: Promise<T>): Promise<Result<T, string>> {
  try {
    return { status: "ok", data: await promise };
  } catch (e) {
    if (e instanceof Error) throw e;
    return { status: "error", error: e as unknown as string };
  }
}

export const aiModeCommands = {
  setEnabled: (enabled: boolean) =>
    wrap<null>(invoke("change_ai_mode_enabled_setting", { enabled })),
  setProvider: (providerId: string) =>
    wrap<null>(invoke("change_ai_mode_provider_setting", { providerId })),
  setModel: (model: string) =>
    wrap<null>(invoke("change_ai_mode_model_setting", { model })),
  setPrompt: (prompt: string) =>
    wrap<null>(invoke("change_ai_mode_prompt_setting", { prompt })),
  setIncludeScreenshot: (enabled: boolean) =>
    wrap<null>(
      invoke("change_ai_mode_include_screenshot_setting", { enabled }),
    ),
};

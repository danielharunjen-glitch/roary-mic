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

export type AiModeOutputMode = "auto_paste" | "prompt_window";

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
  setOutputMode: (mode: AiModeOutputMode) =>
    wrap<null>(invoke("change_ai_mode_output_mode_setting", { mode })),
};

export const elevenLabsCommands = {
  setApiKey: (apiKey: string) =>
    wrap<null>(invoke("change_elevenlabs_api_key_setting", { apiKey })),
  setVoiceId: (voiceId: string) =>
    wrap<null>(invoke("change_elevenlabs_voice_id_setting", { voiceId })),
  setModelId: (modelId: string) =>
    wrap<null>(invoke("change_elevenlabs_model_id_setting", { modelId })),
};

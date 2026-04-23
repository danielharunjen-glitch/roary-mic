import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { aiModeCommands, elevenLabsCommands } from "@/lib/ai-mode";
import { useSettings } from "../../../hooks/useSettings";
import { useSettingsStore } from "../../../stores/settingsStore";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { ShortcutInput } from "../ShortcutInput";
import ScreenRecordingPermissions from "../../ScreenRecordingPermissions";
import { SectionHeader } from "../SectionHeader";

const CLAUDE_CODE_LOCAL_ID = "claude_code_local";

export const AiModeSettings: React.FC = () => {
  const { t } = useTranslation();
  const { settings } = useSettings();
  const refreshSettings = useSettingsStore((s) => s.refreshSettings);

  const enabled = settings?.ai_mode_enabled ?? false;
  const providerId = settings?.ai_mode_provider_id ?? "anthropic";
  const providers = settings?.post_process_providers ?? [];
  const currentProvider = providers.find((p) => p.id === providerId);
  const isClaudeCodeLocal = providerId === CLAUDE_CODE_LOCAL_ID;
  const apiKey = settings?.post_process_api_keys?.[providerId] ?? "";
  const model = settings?.ai_mode_model ?? "";
  const prompt = settings?.ai_mode_prompt ?? "";
  const includeScreenshot = settings?.ai_mode_include_screenshot ?? true;
  const outputMode = settings?.ai_mode_output_mode ?? "prompt_window";
  const elevenApiKey = settings?.elevenlabs_api_keys?.elevenlabs ?? "";
  const elevenVoiceId = settings?.elevenlabs_voice_id ?? "";
  const elevenModelId = settings?.elevenlabs_model_id ?? "";

  const [localModel, setLocalModel] = useState(model);
  const [localPrompt, setLocalPrompt] = useState(prompt);
  const [localApiKey, setLocalApiKey] = useState(apiKey);
  const [localElevenApiKey, setLocalElevenApiKey] = useState(elevenApiKey);
  const [localElevenVoiceId, setLocalElevenVoiceId] = useState(elevenVoiceId);
  const [localElevenModelId, setLocalElevenModelId] = useState(elevenModelId);
  const [claudeCodeAvailable, setClaudeCodeAvailable] = useState<
    boolean | null
  >(null);

  useEffect(() => {
    let cancelled = false;
    commands
      .checkClaudeCodeAvailable()
      .then((available) => {
        if (!cancelled) setClaudeCodeAvailable(available);
      })
      .catch(() => {
        if (!cancelled) setClaudeCodeAvailable(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    setLocalModel(model);
  }, [model]);
  useEffect(() => {
    setLocalPrompt(prompt);
  }, [prompt]);
  useEffect(() => {
    setLocalApiKey(apiKey);
  }, [apiKey]);
  useEffect(() => {
    setLocalElevenApiKey(elevenApiKey);
  }, [elevenApiKey]);
  useEffect(() => {
    setLocalElevenVoiceId(elevenVoiceId);
  }, [elevenVoiceId]);
  useEffect(() => {
    setLocalElevenModelId(elevenModelId);
  }, [elevenModelId]);

  const handleToggle = useCallback(
    async (next: boolean) => {
      const result = await aiModeCommands.setEnabled(next);
      if (result.status !== "ok") {
        toast.error(t("settings.aiMode.toggleError"));
      }
      await refreshSettings();
    },
    [refreshSettings, t],
  );

  const handleProviderChange = useCallback(
    async (id: string) => {
      const result = await aiModeCommands.setProvider(id);
      if (result.status !== "ok") {
        toast.error(t("settings.aiMode.providerError"));
      }
      await refreshSettings();
    },
    [refreshSettings, t],
  );

  const commitModel = useCallback(async () => {
    if (localModel === model) return;
    const result = await aiModeCommands.setModel(localModel);
    if (result.status !== "ok") {
      toast.error(t("settings.aiMode.modelError"));
      return;
    }
    await refreshSettings();
  }, [localModel, model, refreshSettings, t]);

  const commitPrompt = useCallback(async () => {
    if (localPrompt === prompt) return;
    const result = await aiModeCommands.setPrompt(localPrompt);
    if (result.status !== "ok") {
      toast.error(t("settings.aiMode.promptError"));
      return;
    }
    await refreshSettings();
  }, [localPrompt, prompt, refreshSettings, t]);

  const commitApiKey = useCallback(async () => {
    if (localApiKey === apiKey) return;
    const result = await commands.changePostProcessApiKeySetting(
      providerId,
      localApiKey,
    );
    if (result.status !== "ok") {
      toast.error(t("settings.aiMode.apiKeyError"));
      return;
    }
    await refreshSettings();
  }, [apiKey, localApiKey, providerId, refreshSettings, t]);

  const handleIncludeScreenshotToggle = useCallback(
    async (next: boolean) => {
      const result = await aiModeCommands.setIncludeScreenshot(next);
      if (result.status !== "ok") {
        toast.error(t("settings.aiMode.screenshotToggleError"));
      }
      await refreshSettings();
    },
    [refreshSettings, t],
  );

  const handleOutputModeChange = useCallback(
    async (mode: "auto_paste" | "prompt_window") => {
      const result = await aiModeCommands.setOutputMode(mode);
      if (result.status !== "ok") {
        toast.error(t("settings.aiMode.outputMode.error"));
      }
      await refreshSettings();
    },
    [refreshSettings, t],
  );

  const commitElevenApiKey = useCallback(async () => {
    if (localElevenApiKey === elevenApiKey) return;
    const result = await elevenLabsCommands.setApiKey(localElevenApiKey);
    if (result.status !== "ok") {
      toast.error(t("settings.aiMode.elevenlabs.apiKeyError"));
      return;
    }
    await refreshSettings();
  }, [elevenApiKey, localElevenApiKey, refreshSettings, t]);

  const commitElevenVoice = useCallback(async () => {
    if (localElevenVoiceId === elevenVoiceId) return;
    const result = await elevenLabsCommands.setVoiceId(localElevenVoiceId);
    if (result.status !== "ok") {
      toast.error(t("settings.aiMode.elevenlabs.voiceError"));
      return;
    }
    await refreshSettings();
  }, [elevenVoiceId, localElevenVoiceId, refreshSettings, t]);

  const commitElevenModel = useCallback(async () => {
    if (localElevenModelId === elevenModelId) return;
    const result = await elevenLabsCommands.setModelId(localElevenModelId);
    if (result.status !== "ok") {
      toast.error(t("settings.aiMode.elevenlabs.modelError"));
      return;
    }
    await refreshSettings();
  }, [elevenModelId, localElevenModelId, refreshSettings, t]);

  return (
    <div className="max-w-3xl w-full mx-auto pb-12">
      <SectionHeader
        number="07"
        title={t("settings.aiMode.title")}
        description={t("settings.aiMode.description")}
      />
      <div className="space-y-6">
        <div className="space-y-4">
          <ToggleSwitch
            checked={enabled}
            onChange={handleToggle}
            label={t("settings.aiMode.enableLabel")}
            description={t("settings.aiMode.enableDescription")}
            descriptionMode="inline"
          />

          {enabled && (
            <>
              <div className="space-y-1">
                <label className="text-sm font-medium">
                  {t("settings.aiMode.shortcutLabel")}
                </label>
                <p className="text-xs text-mid-gray">
                  {t("settings.aiMode.shortcutDescription")}
                </p>
                <div className="pt-1">
                  <ShortcutInput shortcutId="ai_mode" />
                </div>
              </div>

              <div className="space-y-1">
                <label className="text-sm font-medium">
                  {t("settings.aiMode.providerLabel")}
                </label>
                <select
                  value={providerId}
                  onChange={(e) => handleProviderChange(e.target.value)}
                  className="w-full text-sm bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
                >
                  {providers.map((p) => {
                    const disabled =
                      p.id === CLAUDE_CODE_LOCAL_ID &&
                      claudeCodeAvailable === false;
                    return (
                      <option
                        key={p.id}
                        value={p.id}
                        disabled={disabled}
                        title={
                          disabled
                            ? t("settings.aiMode.claudeCode.unavailableTooltip")
                            : undefined
                        }
                      >
                        {p.label}
                        {disabled
                          ? ` — ${t("settings.aiMode.claudeCode.unavailableSuffix")}`
                          : ""}
                      </option>
                    );
                  })}
                </select>
                {currentProvider && !isClaudeCodeLocal && (
                  <p className="text-xs text-mid-gray">
                    {currentProvider.base_url}
                  </p>
                )}
              </div>

              {isClaudeCodeLocal ? (
                <div className="rounded-md border border-mid-gray/20 bg-background/60 p-3 space-y-1">
                  <p className="text-xs text-mid-gray">
                    {t("settings.aiMode.claudeCode.description")}
                  </p>
                  <p className="text-xs text-mid-gray">
                    <a
                      href="https://docs.claude.com/en/docs/claude-code/overview"
                      target="_blank"
                      rel="noreferrer"
                      className="underline"
                    >
                      {t("settings.aiMode.claudeCode.docsLink")}
                    </a>
                  </p>
                  {claudeCodeAvailable === false && (
                    <p className="text-xs text-red-500">
                      {t("settings.aiMode.claudeCode.unavailableTooltip")}
                    </p>
                  )}
                </div>
              ) : (
                <div className="space-y-1">
                  <label className="text-sm font-medium">
                    {t("settings.aiMode.apiKeyLabel")}
                  </label>
                  <input
                    type="password"
                    value={localApiKey}
                    onChange={(e) => setLocalApiKey(e.target.value)}
                    onBlur={commitApiKey}
                    placeholder={t("settings.aiMode.apiKeyPlaceholder")}
                    className="w-full text-sm bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
                    autoComplete="off"
                  />
                  <p className="text-xs text-mid-gray">
                    {t("settings.aiMode.apiKeyDescription")}
                  </p>
                </div>
              )}

              <div className="space-y-1">
                <label className="text-sm font-medium">
                  {t("settings.aiMode.modelLabel")}
                </label>
                <input
                  type="text"
                  value={localModel}
                  onChange={(e) => setLocalModel(e.target.value)}
                  onBlur={commitModel}
                  placeholder={t("settings.aiMode.modelPlaceholder")}
                  className="w-full text-sm font-mono bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
                />
                <p className="text-xs text-mid-gray">
                  {t("settings.aiMode.modelDescription")}
                </p>
              </div>

              <div className="space-y-1">
                <label className="text-sm font-medium">
                  {t("settings.aiMode.promptLabel")}
                </label>
                <textarea
                  value={localPrompt}
                  onChange={(e) => setLocalPrompt(e.target.value)}
                  onBlur={commitPrompt}
                  rows={5}
                  className="w-full text-sm bg-background border border-mid-gray/30 rounded-md px-3 py-2 focus:outline-none focus:border-logo-primary resize-y"
                />
                <p className="text-xs text-mid-gray">
                  {t("settings.aiMode.promptDescription")}
                </p>
              </div>

              {!isClaudeCodeLocal && (
                <>
                  <ToggleSwitch
                    checked={includeScreenshot}
                    onChange={handleIncludeScreenshotToggle}
                    label={t("settings.aiMode.includeScreenshotLabel")}
                    description={t(
                      "settings.aiMode.includeScreenshotDescription",
                    )}
                    descriptionMode="inline"
                  />
                  {includeScreenshot && (
                    <ScreenRecordingPermissions compact />
                  )}
                </>
              )}

              <div className="space-y-1">
                <label className="text-sm font-medium">
                  {t("settings.aiMode.outputMode.label")}
                </label>
                <select
                  value={outputMode}
                  onChange={(e) =>
                    handleOutputModeChange(
                      e.target.value as "auto_paste" | "prompt_window",
                    )
                  }
                  className="w-full text-sm bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
                >
                  <option value="prompt_window">
                    {t("settings.aiMode.outputMode.promptWindow")}
                  </option>
                  <option value="auto_paste">
                    {t("settings.aiMode.outputMode.autoPaste")}
                  </option>
                </select>
                <p className="text-xs text-mid-gray">
                  {t("settings.aiMode.outputMode.description")}
                </p>
              </div>

              <div className="pt-2 border-t border-mid-gray/20 space-y-3">
                <div>
                  <h3 className="text-sm font-medium">
                    {t("settings.aiMode.elevenlabs.sectionTitle")}
                  </h3>
                  <p className="text-xs text-mid-gray">
                    {t("settings.aiMode.elevenlabs.sectionDescription")}
                  </p>
                </div>

                <div className="space-y-1">
                  <label className="text-sm font-medium">
                    {t("settings.aiMode.elevenlabs.apiKeyLabel")}
                  </label>
                  <input
                    type="password"
                    value={localElevenApiKey}
                    onChange={(e) => setLocalElevenApiKey(e.target.value)}
                    onBlur={commitElevenApiKey}
                    placeholder={t(
                      "settings.aiMode.elevenlabs.apiKeyPlaceholder",
                    )}
                    className="w-full text-sm bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
                    autoComplete="off"
                  />
                </div>

                <div className="space-y-1">
                  <label className="text-sm font-medium">
                    {t("settings.aiMode.elevenlabs.voiceLabel")}
                  </label>
                  <input
                    type="text"
                    value={localElevenVoiceId}
                    onChange={(e) => setLocalElevenVoiceId(e.target.value)}
                    onBlur={commitElevenVoice}
                    placeholder={t(
                      "settings.aiMode.elevenlabs.voicePlaceholder",
                    )}
                    className="w-full text-sm font-mono bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
                  />
                  <p className="text-xs text-mid-gray">
                    <a
                      href="https://elevenlabs.io/voices"
                      target="_blank"
                      rel="noreferrer"
                      className="underline"
                    >
                      {t("settings.aiMode.elevenlabs.voiceHelpText")}
                    </a>
                  </p>
                </div>

                <div className="space-y-1">
                  <label className="text-sm font-medium">
                    {t("settings.aiMode.elevenlabs.modelLabel")}
                  </label>
                  <input
                    type="text"
                    value={localElevenModelId}
                    onChange={(e) => setLocalElevenModelId(e.target.value)}
                    onBlur={commitElevenModel}
                    placeholder={t(
                      "settings.aiMode.elevenlabs.modelPlaceholder",
                    )}
                    className="w-full text-sm font-mono bg-background border border-mid-gray/30 rounded-md px-3 py-1.5 focus:outline-none focus:border-logo-primary"
                  />
                  <p className="text-xs text-mid-gray">
                    {t("settings.aiMode.elevenlabs.modelHelpText")}
                  </p>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
};

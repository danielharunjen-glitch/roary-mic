import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { aiModeCommands } from "@/lib/ai-mode";
import { useSettings } from "../../../hooks/useSettings";
import { useSettingsStore } from "../../../stores/settingsStore";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { ShortcutInput } from "../ShortcutInput";

export const AiModeSettings: React.FC = () => {
  const { t } = useTranslation();
  const { settings } = useSettings();
  const refreshSettings = useSettingsStore((s) => s.refreshSettings);

  const enabled = settings?.ai_mode_enabled ?? false;
  const providerId = settings?.ai_mode_provider_id ?? "anthropic";
  const providers = settings?.post_process_providers ?? [];
  const currentProvider = providers.find((p) => p.id === providerId);
  const apiKey = settings?.post_process_api_keys?.[providerId] ?? "";
  const model = settings?.ai_mode_model ?? "";
  const prompt = settings?.ai_mode_prompt ?? "";
  const includeScreenshot = settings?.ai_mode_include_screenshot ?? true;

  const [localModel, setLocalModel] = useState(model);
  const [localPrompt, setLocalPrompt] = useState(prompt);
  const [localApiKey, setLocalApiKey] = useState(apiKey);

  useEffect(() => {
    setLocalModel(model);
  }, [model]);
  useEffect(() => {
    setLocalPrompt(prompt);
  }, [prompt]);
  useEffect(() => {
    setLocalApiKey(apiKey);
  }, [apiKey]);

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

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="space-y-2">
        <div className="px-4">
          <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide">
            {t("settings.aiMode.title")}
          </h2>
          <p className="text-xs text-mid-gray mt-1">
            {t("settings.aiMode.description")}
          </p>
        </div>
        <div className="bg-background border border-mid-gray/20 rounded-lg p-4 space-y-4">
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
                  {providers.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.label}
                    </option>
                  ))}
                </select>
                {currentProvider && (
                  <p className="text-xs text-mid-gray">
                    {currentProvider.base_url}
                  </p>
                )}
              </div>

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

              <ToggleSwitch
                checked={includeScreenshot}
                onChange={handleIncludeScreenshotToggle}
                label={t("settings.aiMode.includeScreenshotLabel")}
                description={t("settings.aiMode.includeScreenshotDescription")}
                descriptionMode="inline"
              />
            </>
          )}
        </div>
      </div>
    </div>
  );
};

import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { AppDataDirectory } from "../AppDataDirectory";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { LogDirectory } from "../debug";

export const AboutSettings: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.8.2");
      }
    };

    fetchVersion();
  }, []);

  const handleDonateClick = async () => {
    try {
      await openUrl("https://handy.computer/donate");
    } catch (error) {
      console.error("Failed to open donate link:", error);
    }
  };

  return (
    <div className="max-w-3xl w-full mx-auto pb-12">
      {/* Editorial hero */}
      <section className="pt-14 pb-10 px-6">
        {/* eslint-disable i18next/no-literal-string */}
        <div className="font-mono text-[11px] tracking-[0.18em] mb-4" style={{ color: "var(--color-muted)" }}>ROARY MIC · v{version || "0.0.0"}</div>
        <h1 className="font-display text-[64px] leading-[0.95] tracking-tight" style={{ color: "var(--color-ink)" }}>
          <span className="italic" style={{ color: "var(--color-accent)" }}>Roary</span>{" "}
          <span>Mic</span>
        </h1>
        {/* eslint-enable i18next/no-literal-string */}
        <p
          className="font-sans text-[15px] leading-[1.6] mt-6 max-w-[46ch]"
          style={{ color: "var(--color-muted)" }}
        >
          {t("settings.about.version.description")}
        </p>
        <div
          aria-hidden
          className="text-[64px] leading-none select-none mt-10"
          style={{ opacity: 0.25 }}
          title="Roary Mic"
        >
          🦁
        </div>
      </section>

      <SettingsGroup title={t("settings.about.title")}>
        <AppLanguageSelector descriptionMode="tooltip" grouped={true} />
        <SettingContainer
          title={t("settings.about.version.title")}
          description={t("settings.about.version.description")}
          grouped={true}
        >
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="font-mono text-[12px] tracking-wider">
            v{version}
          </span>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.supportDevelopment.title")}
          description={t("settings.about.supportDevelopment.description")}
          grouped={true}
        >
          <Button variant="primary" size="md" onClick={handleDonateClick}>
            {t("settings.about.supportDevelopment.button")}
          </Button>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.sourceCode.title")}
          description={t("settings.about.sourceCode.description")}
          grouped={true}
        >
          <Button
            variant="secondary"
            size="md"
            onClick={() =>
              openUrl("https://github.com/danielharunjen-glitch/roary-mic")
            }
          >
            {t("settings.about.sourceCode.button")}
          </Button>
        </SettingContainer>

        <AppDataDirectory descriptionMode="tooltip" grouped={true} />
        <LogDirectory grouped={true} />
      </SettingsGroup>

      <SettingsGroup title={t("settings.about.acknowledgments.title")}>
        <SettingContainer
          title={t("settings.about.acknowledgments.whisper.title")}
          description={t("settings.about.acknowledgments.whisper.description")}
          grouped={true}
          layout="stacked"
        >
          <div
            className="text-[13px] leading-[1.6]"
            style={{ color: "var(--color-muted)" }}
          >
            {t("settings.about.acknowledgments.whisper.details")}
          </div>
        </SettingContainer>
      </SettingsGroup>
    </div>
  );
};

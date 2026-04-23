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

  return (
    <div className="max-w-3xl w-full mx-auto pb-16">
      {/* Editorial hero — magazine cover treatment. */}
      <section className="relative pt-20 pb-14 px-8 overflow-hidden">
        {/* Large decorative lion mark, subtly behind the type */}
        <div
          aria-hidden
          className="absolute select-none pointer-events-none"
          style={{
            top: "48%",
            right: "-32px",
            transform: "translateY(-50%)",
            fontSize: "260px",
            lineHeight: 1,
            opacity: 0.12,
            filter: "saturate(0.6)",
          }}
        >
          🦁
        </div>

        <div className="relative">
          {/* eslint-disable i18next/no-literal-string */}
          <div
            className="font-mono text-[10px] tracking-[0.24em] mb-5"
            style={{ color: "var(--color-muted)" }}
          >
            ROARY&nbsp;MIC &nbsp;·&nbsp; v{version || "—"} &nbsp;·&nbsp; MACOS
          </div>
          <h1
            className="font-display tracking-tight"
            style={{
              color: "var(--color-ink)",
              fontSize: "clamp(64px, 11vw, 104px)",
              lineHeight: 0.88,
              letterSpacing: "-0.03em",
            }}
          >
            <span
              className="italic"
              style={{ color: "var(--color-accent)" }}
            >
              Roary
            </span>
            <br />
            <span style={{ fontWeight: 400 }}>Mic.</span>
          </h1>
          {/* eslint-enable i18next/no-literal-string */}

          <div
            aria-hidden
            className="mt-8 mb-6"
            style={{
              height: "2px",
              width: "40px",
              background: "var(--color-accent)",
            }}
          />
          <p
            className="font-sans text-[15px] leading-[1.65] max-w-[44ch]"
            style={{ color: "var(--color-muted)" }}
          >
            {t("settings.about.version.description")}
          </p>
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

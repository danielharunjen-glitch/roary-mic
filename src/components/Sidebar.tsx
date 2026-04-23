import React from "react";
import { useTranslation } from "react-i18next";
import {
  Bot,
  BookOpen,
  Cog,
  FlaskConical,
  History,
  Info,
  Sparkles,
  Cpu,
  Replace,
} from "lucide-react";
import { useSettings } from "../hooks/useSettings";
import {
  GeneralSettings,
  AdvancedSettings,
  HistorySettings,
  DebugSettings,
  AboutSettings,
  PostProcessingSettings,
  ModelsSettings,
  CorrectionsSettings,
  ReferencesSettings,
  AiModeSettings,
} from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  [key: string]: any;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: (settings: any) => boolean;
}

export const SECTIONS_CONFIG = {
  general: {
    labelKey: "sidebar.general",
    icon: Cog,
    component: GeneralSettings,
    enabled: () => true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: Cog,
    component: AdvancedSettings,
    enabled: () => true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  corrections: {
    labelKey: "sidebar.corrections",
    icon: Replace,
    component: CorrectionsSettings,
    enabled: () => true,
  },
  references: {
    labelKey: "sidebar.references",
    icon: BookOpen,
    component: ReferencesSettings,
    enabled: () => true,
  },
  aiMode: {
    labelKey: "sidebar.aiMode",
    icon: Bot,
    component: AiModeSettings,
    enabled: () => true,
  },
  postprocessing: {
    labelKey: "sidebar.postProcessing",
    icon: Sparkles,
    component: PostProcessingSettings,
    enabled: (settings) => settings?.post_process_enabled ?? false,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode ?? false,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

const APP_VERSION = "0.8.2";

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const availableSections = Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  return (
    <aside className="flex flex-col w-[200px] h-full relative">
      {/* Right-edge hairline divider */}
      <div
        className="absolute top-0 right-0 bottom-0 w-px bg-rule"
        style={{ transform: "scaleX(0.5)", transformOrigin: "right" }}
      />

      {/* Brand nameplate */}
      <div className="px-6 pt-7 pb-5">
        {/* eslint-disable i18next/no-literal-string */}
        <div className="leading-none">
          <span className="font-display italic text-[28px] tracking-tight" style={{ color: "var(--color-accent)" }}>Roary</span>
          <span className="text-[28px] tracking-tight font-sans font-light ml-[2px]">Mic</span>
        </div>
        {/* eslint-enable i18next/no-literal-string */}
        <div
          className="label-mono mt-2"
          style={{ color: "var(--color-muted)" }}
        >
          {t("sidebar.tagline", "Speak · Type · Send")}
        </div>
      </div>

      <div
        className="mx-6 mb-4"
        style={{
          height: "1px",
          background: "var(--color-rule)",
          transform: "scaleY(0.5)",
          transformOrigin: "top",
        }}
      />

      {/* Section list */}
      <nav className="flex flex-col gap-px px-2 flex-1 overflow-y-auto">
        {availableSections.map((section, index) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;
          const sectionNumber = String(index + 1).padStart(2, "0");

          return (
            <button
              key={section.id}
              type="button"
              onClick={() => onSectionChange(section.id)}
              className={`group relative flex items-center gap-3 py-2 pl-5 pr-3 text-left cursor-pointer transition-all duration-200 rounded-sm ${
                isActive ? "" : "hover:bg-ink/[0.03]"
              }`}
              style={{
                color: isActive ? "var(--color-ink)" : "var(--color-muted)",
              }}
            >
              {/* Active-state left accent bar */}
              {isActive && (
                <span
                  aria-hidden
                  className="absolute left-0 top-1.5 bottom-1.5 w-[2px] rounded-full"
                  style={{ background: "var(--color-accent)" }}
                />
              )}

              <span
                className="font-mono text-[10px] tracking-widest tabular-nums shrink-0 transition-opacity"
                style={{
                  opacity: isActive ? 0.65 : 0.35,
                }}
              >
                {sectionNumber}
              </span>

              <Icon width={14} height={14} className="shrink-0 opacity-70" />

              <span
                className="text-[13px] tracking-tight font-medium truncate transition-opacity"
                style={{
                  opacity: isActive ? 1 : 0.9,
                }}
                title={t(section.labelKey)}
              >
                {t(section.labelKey)}
              </span>
            </button>
          );
        })}
      </nav>

      {/* Footer — lion sigil + version */}
      <div
        className="px-6 py-4 flex items-center justify-between"
        style={{
          borderTop: "1px solid var(--color-rule)",
        }}
      >
        <span
          className="text-[18px] select-none"
          style={{ opacity: 0.35 }}
          aria-hidden
          title="Roary Mic"
        >
          🦁
        </span>
        {/* eslint-disable-next-line i18next/no-literal-string */}
        <span className="font-mono text-[10px] tracking-wider" style={{ color: "var(--color-muted)" }}>v{APP_VERSION}</span>
      </div>
    </aside>
  );
};

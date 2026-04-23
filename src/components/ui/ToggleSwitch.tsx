import React from "react";
import { SettingContainer } from "./SettingContainer";

interface ToggleSwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  isUpdating?: boolean;
  label: string;
  description: string;
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  tooltipPosition?: "top" | "bottom";
}

/**
 * Contemporary-editorial toggle. Off-state is a hairline-outlined pill with
 * the ink color at low opacity. On-state is a solid accent pill with a
 * paper-colored knob. Subtle spring on transition for tactile feel.
 */
export const ToggleSwitch: React.FC<ToggleSwitchProps> = ({
  checked,
  onChange,
  disabled = false,
  isUpdating = false,
  label,
  description,
  descriptionMode = "tooltip",
  grouped = false,
  tooltipPosition = "top",
}) => {
  const pillStyle: React.CSSProperties = {
    width: 32,
    height: 18,
    borderRadius: 999,
    background: checked ? "var(--color-accent)" : "transparent",
    border: `1px solid ${checked ? "var(--color-accent)" : "var(--color-rule)"}`,
    boxShadow: checked ? "none" : "inset 0 0 0 1px var(--color-rule)",
    transition:
      "background-color 200ms cubic-bezier(0.34, 1.56, 0.64, 1), border-color 200ms ease",
    position: "relative",
    opacity: disabled || isUpdating ? 0.4 : 1,
  };

  const knobStyle: React.CSSProperties = {
    position: "absolute",
    top: 2,
    left: checked ? 16 : 2,
    width: 12,
    height: 12,
    borderRadius: 999,
    background: checked
      ? "var(--color-paper)"
      : "color-mix(in srgb, var(--color-ink) 55%, transparent)",
    transition: "left 220ms cubic-bezier(0.34, 1.56, 0.64, 1)",
  };

  return (
    <SettingContainer
      title={label}
      description={description}
      descriptionMode={descriptionMode}
      grouped={grouped}
      disabled={disabled}
      tooltipPosition={tooltipPosition}
    >
      <label
        className={`inline-flex items-center ${
          disabled || isUpdating ? "cursor-not-allowed" : "cursor-pointer"
        }`}
      >
        <input
          type="checkbox"
          value=""
          className="sr-only peer"
          checked={checked}
          disabled={disabled || isUpdating}
          onChange={(e) => onChange(e.target.checked)}
        />
        <div
          className="peer-focus-visible:ring-2 peer-focus-visible:ring-accent peer-focus-visible:ring-offset-0"
          style={pillStyle}
        >
          <span style={knobStyle} />
        </div>
      </label>
      {isUpdating && (
        <div className="absolute inset-0 flex items-center justify-center">
          <div
            className="w-4 h-4 rounded-full animate-spin"
            style={{
              border: "2px solid var(--color-accent)",
              borderTopColor: "transparent",
            }}
          />
        </div>
      )}
    </SettingContainer>
  );
};

import React, { useEffect, useRef, useState } from "react";
import { Tooltip } from "./Tooltip";

interface SettingContainerProps {
  title: string;
  description: string;
  children: React.ReactNode;
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
  layout?: "horizontal" | "stacked";
  disabled?: boolean;
  tooltipPosition?: "top" | "bottom";
}

/**
 * Contemporary-editorial setting row.
 *
 * Editorial-hairline style: each row is a horizontal strip with a
 * hairline bottom rule. The title is sans medium 14px; the description
 * (when inline) sits below in sans 13 muted. The control floats right.
 *
 * `grouped` prop is retained for back-compat but the visual difference
 * between grouped and standalone is now just whitespace — no cards, no
 * rounded borders, no stacked boxes.
 */
export const SettingContainer: React.FC<SettingContainerProps> = ({
  title,
  description,
  children,
  descriptionMode = "tooltip",
  grouped = false,
  layout = "horizontal",
  disabled = false,
  tooltipPosition = "top",
}) => {
  const [showTooltip, setShowTooltip] = useState(false);
  const tooltipRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        tooltipRef.current &&
        !tooltipRef.current.contains(event.target as Node)
      ) {
        setShowTooltip(false);
      }
    };
    if (showTooltip) {
      document.addEventListener("mousedown", handleClickOutside);
      return () =>
        document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [showTooltip]);

  const toggleTooltip = () => setShowTooltip(!showTooltip);

  // `grouped` rows don't render their own hairline — the SettingsGroup's
  // `divide` handles it. Standalone rows get their own top + bottom padding.
  const rowPadding = grouped ? "py-3.5 px-6" : "py-4 px-6";
  const titleCls = `text-[14px] font-medium tracking-tight ${
    disabled ? "opacity-40" : ""
  }`;
  const descCls = `text-[13px] leading-[1.5] mt-0.5 ${
    disabled ? "opacity-40" : ""
  }`;

  const tooltipIcon = (
    <div
      ref={tooltipRef}
      className="relative"
      onMouseEnter={() => setShowTooltip(true)}
      onMouseLeave={() => setShowTooltip(false)}
      onClick={toggleTooltip}
    >
      <svg
        className="w-[14px] h-[14px] cursor-help transition-colors duration-200 select-none"
        style={{ color: "var(--color-muted)" }}
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        aria-label="More information"
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            toggleTooltip();
          }
        }}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
        />
      </svg>
      {showTooltip && (
        <Tooltip targetRef={tooltipRef} position={tooltipPosition}>
          <p className="text-[13px] text-center leading-relaxed">
            {description}
          </p>
        </Tooltip>
      )}
    </div>
  );

  if (layout === "stacked") {
    return (
      <div className={rowPadding}>
        <div className="flex items-center gap-2 mb-2">
          <h3 className={titleCls}>{title}</h3>
          {descriptionMode === "tooltip" && tooltipIcon}
        </div>
        {descriptionMode === "inline" && (
          <p className={descCls} style={{ color: "var(--color-muted)" }}>
            {description}
          </p>
        )}
        <div className="w-full mt-3">{children}</div>
      </div>
    );
  }

  // Horizontal
  return (
    <div className={`flex items-center justify-between gap-6 ${rowPadding}`}>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <h3 className={titleCls}>{title}</h3>
          {descriptionMode === "tooltip" && tooltipIcon}
        </div>
        {descriptionMode === "inline" && (
          <p className={descCls} style={{ color: "var(--color-muted)" }}>
            {description}
          </p>
        )}
      </div>
      <div className="relative shrink-0">{children}</div>
    </div>
  );
};

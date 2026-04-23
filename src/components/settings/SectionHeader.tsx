import React from "react";

interface SectionHeaderProps {
  /** Two-digit section numeral shown in mono above the title, e.g., "03". */
  number?: string;
  /** Display title rendered in the serif. */
  title: string;
  /** Optional short description rendered in sans below the title. */
  description?: string;
  /** Optional trailing slot for controls on the right (e.g., a button). */
  right?: React.ReactNode;
}

/**
 * Contemporary-editorial section header. Every settings page opens with
 * this: mono numeral, serif display title, a thin accent rule, and a quiet
 * description. Whitespace rhythm is deliberate — do not tighten.
 */
export const SectionHeader: React.FC<SectionHeaderProps> = ({
  number,
  title,
  description,
  right,
}) => {
  return (
    <header className="pt-12 pb-8 px-6">
      <div className="flex items-start justify-between gap-6">
        <div className="min-w-0">
          {number && (
            <div
              className="font-mono text-[11px] tracking-[0.16em] tabular-nums mb-3"
              style={{ color: "var(--color-muted)" }}
            >
              {number}
            </div>
          )}
          <h2
            className="font-display text-[36px] leading-[1.05] tracking-tight"
            style={{ color: "var(--color-ink)" }}
          >
            {title}
          </h2>
          <div
            aria-hidden
            className="mt-4 mb-5"
            style={{
              height: "2px",
              width: "32px",
              background: "var(--color-accent)",
            }}
          />
          {description && (
            <p
              className="text-[14px] leading-[1.6] max-w-[52ch]"
              style={{ color: "var(--color-muted)" }}
            >
              {description}
            </p>
          )}
        </div>
        {right && <div className="shrink-0">{right}</div>}
      </div>
    </header>
  );
};

export default SectionHeader;

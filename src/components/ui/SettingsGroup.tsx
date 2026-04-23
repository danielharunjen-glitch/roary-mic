import React from "react";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

/**
 * Contemporary-editorial settings group.
 *
 * No outer border, no rounded card. Title renders as a small-caps mono
 * label; children are separated by a hairline rule. The group itself is
 * a vertical stack with generous top margin to establish rhythm between
 * sections.
 */
export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <section className="mt-4 first:mt-0">
      {title && (
        <div className="px-6 pb-3">
          <h3
            className="label-mono"
            style={{ color: "var(--color-muted)" }}
          >
            {title}
          </h3>
          {description && (
            <p
              className="text-[12px] leading-[1.5] mt-1.5 max-w-[52ch]"
              style={{ color: "var(--color-muted)" }}
            >
              {description}
            </p>
          )}
        </div>
      )}
      <div
        style={{
          borderTop: "1px solid var(--color-rule)",
        }}
      >
        <div className="divide-y" style={{ borderColor: "var(--color-rule)" }}>
          {React.Children.map(children, (child, idx) => (
            <div
              key={idx}
              style={{
                borderBottom:
                  idx ===
                  (Array.isArray(children) ? children.length - 1 : 0)
                    ? "none"
                    : "1px solid var(--color-rule)",
              }}
            >
              {child}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
};

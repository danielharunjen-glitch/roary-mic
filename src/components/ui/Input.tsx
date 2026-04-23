import React from "react";

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  variant?: "default" | "compact";
}

/**
 * Contemporary-editorial input. Hairline-underlined, no filled background
 * by default. On focus, the underline shifts to the accent color. Font is
 * sans by default; add `font-mono` via className for path-like inputs.
 */
export const Input: React.FC<InputProps> = ({
  className = "",
  variant = "default",
  disabled,
  ...props
}) => {
  const base =
    "w-full bg-transparent text-[13px] tracking-tight outline-none " +
    "transition-[border-color,color] duration-200 ease-out " +
    "placeholder:text-muted placeholder:opacity-70";

  const hairline =
    "border-0 border-b rounded-none border-rule " +
    "focus:border-accent hover:border-ink/40";

  const state = disabled
    ? "opacity-40 cursor-not-allowed"
    : "cursor-text";

  const variantClasses = {
    default: "px-0 py-2",
    compact: "px-0 py-1",
  } as const;

  return (
    <input
      className={`${base} ${hairline} ${state} ${variantClasses[variant]} ${className}`}
      disabled={disabled}
      {...props}
    />
  );
};

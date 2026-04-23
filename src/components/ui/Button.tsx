import React from "react";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?:
    | "primary"
    | "primary-soft"
    | "secondary"
    | "danger"
    | "danger-ghost"
    | "ghost"
    | "quiet";
  size?: "sm" | "md" | "lg";
}

/**
 * Contemporary-editorial button.
 *
 * Variants:
 *   primary       — solid accent background, paper text, tight radius
 *   primary-soft  — subtle accent wash on hover (legacy alias for bg-tint)
 *   secondary     — hairline-outlined, accent on hover
 *   ghost         — transparent until hovered; accent tint on hover
 *   quiet         — pure text, accent underline on hover
 *   danger        — solid red
 *   danger-ghost  — muted red text, light red hover tint
 *
 * All share a 200ms ease transition and accent-colored focus ring.
 */
export const Button: React.FC<ButtonProps> = ({
  children,
  className = "",
  variant = "primary",
  size = "md",
  ...props
}) => {
  const base =
    "inline-flex items-center justify-center gap-2 font-sans font-medium tracking-tight " +
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-0 " +
    "disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer " +
    "transition-[background-color,color,border-color,opacity] duration-200 ease-out";

  const variantClasses: Record<string, string> = {
    primary:
      "rounded-sm border text-paper bg-accent border-accent " +
      "hover:bg-accent-hover hover:border-accent-hover " +
      "focus-visible:ring-accent",
    "primary-soft":
      "rounded-sm border border-transparent text-ink bg-accent/10 " +
      "hover:bg-accent/20 focus-visible:ring-accent",
    secondary:
      "rounded-sm border text-ink border-rule bg-transparent " +
      "hover:border-accent hover:text-accent " +
      "focus-visible:ring-accent",
    ghost:
      "rounded-sm border border-transparent text-ink " +
      "hover:bg-ink/[0.04] " +
      "focus-visible:ring-accent",
    quiet:
      "rounded-none border-none text-ink bg-transparent underline-offset-4 " +
      "hover:text-accent hover:underline " +
      "focus-visible:ring-0 focus-visible:underline focus-visible:text-accent",
    danger:
      "rounded-sm border text-paper bg-red-600 border-red-600 " +
      "hover:bg-red-700 hover:border-red-700 " +
      "focus-visible:ring-red-500",
    "danger-ghost":
      "rounded-sm border border-transparent text-red-500 " +
      "hover:bg-red-500/10 focus-visible:ring-red-500",
  };

  const sizeClasses: Record<string, string> = {
    sm: "px-2.5 py-1 text-[12px]",
    md: "px-3.5 py-1.5 text-[13px]",
    lg: "px-5 py-2 text-[14px]",
  };

  return (
    <button
      className={`${base} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};

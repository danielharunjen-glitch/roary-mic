import React from "react";

interface TextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  variant?: "default" | "compact";
}

/**
 * Contemporary-editorial textarea. Hairline-bordered (1px all sides —
 * stronger than inputs because textareas benefit from a defined area),
 * accent on focus, subtle fill on hover.
 */
export const Textarea: React.FC<TextareaProps> = ({
  className = "",
  variant = "default",
  ...props
}) => {
  const base =
    "w-full bg-transparent text-[13px] tracking-tight outline-none " +
    "border rounded-xs transition-[border-color,background-color] duration-200 " +
    "placeholder:text-muted placeholder:opacity-70 resize-y " +
    "hover:bg-ink/[0.02] focus:border-accent focus:bg-transparent";

  const variantClasses = {
    default: "px-3 py-2 min-h-[100px]",
    compact: "px-2 py-1 min-h-[80px]",
  };

  return (
    <textarea
      className={`${base} ${variantClasses[variant]} ${className}`}
      style={{ borderColor: "var(--color-rule)" }}
      {...props}
    />
  );
};

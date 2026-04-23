import React from "react";

// Brand mark — intentionally not localized.
const BRAND_TEXT = ["Roary", "Mic"].join(" ");

const HandyTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox="0 0 930 328"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <text
        x="465"
        y="230"
        textAnchor="middle"
        fontFamily="ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif"
        fontSize="220"
        fontWeight="800"
        letterSpacing="-8"
        className="logo-primary"
        stroke="currentColor"
        strokeWidth="0"
      >
        {BRAND_TEXT}
      </text>
    </svg>
  );
};

export default HandyTextLogo;

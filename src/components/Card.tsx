import { ComponentChildren } from "preact";
import type { JSX } from "preact";
import { useState } from "preact/hooks";
import { tokens } from "../design-tokens.ts";

interface CardProps {
  children: ComponentChildren;
  className?: string;
  variant?: "primary" | "secondary";
  onClick?: () => void;
  style?: JSX.CSSProperties;
}

export const Card = ({
  children,
  className = "",
  variant = "secondary",
  onClick,
  style: styleOverride,
}: CardProps) => {
  const [hovered, setHovered] = useState(false);

  const style = {
    padding: tokens.spacing.lg,
    borderRadius: tokens.radii.panel,
    background: variant === "primary" ? tokens.colors.glassBgHeavy : tokens.colors.glassBg,
    backdropFilter: `blur(${tokens.colors.glassBlur}) saturate(1.35)`,
    WebkitBackdropFilter: `blur(${tokens.colors.glassBlur}) saturate(1.35)`,
    border: "1px solid rgba(255, 255, 255, 0.10)",
    boxShadow: tokens.shadows.sm,
    transition: tokens.transitions.normal,
    transform: hovered && onClick ? "translateY(-1px)" : "translateY(0)",
    cursor: onClick ? "pointer" : "default",
  } as const;

  return (
    <div
      className={className}
      onClick={onClick}
      style={{ ...style, ...styleOverride }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {children}
    </div>
  );
};

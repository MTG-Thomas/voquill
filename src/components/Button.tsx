import { ComponentChildren, VNode } from "preact";
import { useState } from "preact/hooks";
import { invoke } from "@tauri-apps/api/core";
import { tokens } from "../design-tokens.ts";

interface ButtonProps {
  children: ComponentChildren;
  onClick?: (e: MouseEvent) => void;
  variant?:
    | "primary"
    | "secondary"
    | "configAction"
    | "danger"
    | "ghost"
    | "icon"
    | "titlebarIcon"
    | "titlebarClose";
  size?: "sm" | "md" | "lg";
  disabled?: boolean;
  className?: string;
  title?: string;
  type?: "button" | "submit" | "reset";
  style?: Record<string, string | number>;
  logLabel?: string;
  disableClickLog?: boolean;
  pill?: boolean;
  floating?: boolean;
}

function extractText(children: ComponentChildren): string {
  if (typeof children === "string" || typeof children === "number") {
    return String(children).trim();
  }

  if (Array.isArray(children)) {
    return children
      .map((child) => extractText(child))
      .filter(Boolean)
      .join(" ")
      .trim();
  }

  if (children && typeof children === "object") {
    const vnode = children as VNode<{ children?: ComponentChildren }>;
    return extractText(vnode.props?.children);
  }

  return "";
}

export const Button = ({
  children,
  onClick,
  variant = "secondary",
  size = "md",
  disabled = false,
  className = "",
  title,
  type = "button",
  style,
  logLabel,
  disableClickLog = false,
  pill = false,
  floating = false,
}: ButtonProps) => {
  const [hovered, setHovered] = useState(false);
  const [pressed, setPressed] = useState(false);

  const variantStyles: Record<string, Record<string, string | number>> = {
    primary: {
      color: "#000000",
      background: tokens.colors.accentPrimary,
      border: "none",
    },
    secondary: {
      color: tokens.colors.textPrimary,
      background: tokens.colors.bgSecondary,
      border: "1px solid rgba(255, 255, 255, 0.10)",
    },
    configAction: {
      color: "#000000",
      background: tokens.colors.accentPrimary,
      border: "1px solid rgba(255, 255, 255, 0.10)",
      borderRadius: tokens.radii.button,
      padding: "10px 24px",
      fontWeight: 600,
      boxShadow: "none",
    },
    danger: {
      color: "#000000",
      background: tokens.colors.error,
      border: "none",
    },
    ghost: {
      border: "1px solid rgba(255, 255, 255, 0.10)",
      background: "rgba(255, 255, 255, 0.06)",
      color: tokens.colors.textPrimary,
    },
    icon: {
      border: "1px solid transparent",
      background: "rgba(255, 255, 255, 0.06)",
      color: tokens.colors.textPrimary,
      width: "34px",
      height: "34px",
      padding: tokens.spacing.sm,
      borderRadius: tokens.radii.button,
    },
    titlebarIcon: {
      border: "1px solid transparent",
      background: "transparent",
      color: tokens.colors.textPrimary,
      width: "46px",
      height: "32px",
      padding: 0,
      borderRadius: tokens.radii.button,
    },
    titlebarClose: {
      border: "1px solid transparent",
      background: "transparent",
      color: tokens.colors.textPrimary,
      width: "46px",
      height: "32px",
      padding: 0,
      borderRadius: tokens.radii.button,
    },
  };

  const sizeStyles: Record<string, Record<string, string | number>> = {
    sm: { padding: `6px ${tokens.spacing.sm}`, fontSize: tokens.typography.sizeXs },
    md: { padding: `10px ${tokens.spacing.md}`, fontSize: tokens.typography.sizeSm },
    lg: { padding: `14px ${tokens.spacing.lg}`, fontSize: tokens.typography.sizeMd },
  };

  const hoverStyles: Record<string, Record<string, string | number>> = {
    primary: { background: tokens.colors.accentHover },
    secondary: { background: tokens.colors.bgHover },
    configAction: { background: tokens.colors.accentHover },
    danger: { background: "#a4262c" },
    ghost: { background: tokens.colors.bgHover },
    icon: { background: tokens.colors.bgHover },
    titlebarIcon: {
      background: "rgba(255, 255, 255, 0.08)",
    },
    titlebarClose: {
      background: "#c42b1c",
      color: tokens.colors.textPrimary,
    },
  };

  const baseStyle: Record<string, string | number> = {
    background: "transparent",
    border: "2px solid transparent",
    borderRadius: tokens.radii.button,
    cursor: disabled ? "not-allowed" : "pointer",
    fontWeight: 600,
    transition: tokens.transitions.normal,
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    gap: tokens.spacing.sm,
    opacity: disabled ? 0.5 : 1,
    color: disabled ? tokens.colors.textMuted : tokens.colors.textPrimary,
  };

  const resolvedStyle: Record<string, string | number> = {
    ...baseStyle,
    ...sizeStyles[size],
    ...variantStyles[variant],
    ...(hovered && !disabled ? hoverStyles[variant] : {}),
    ...(pressed && !disabled && !["titlebarIcon", "titlebarClose"].includes(variant)
      ? { transform: "scale(0.98)", filter: "brightness(0.96)" }
      : {}),
    ...(pill ? { borderRadius: "40px" } : {}),
    ...(floating
      ? {
          pointerEvents: "auto",
          padding: "12px 32px",
          borderRadius: tokens.radii.button,
          backdropFilter: `blur(${tokens.colors.glassBlur})`,
          WebkitBackdropFilter: `blur(${tokens.colors.glassBlur})`,
          boxShadow: tokens.shadows.lg,
          border: "1px solid rgba(255, 255, 255, 0.10)",
        }
      : {}),
    ...(floating && hovered && !disabled
      ? {
          boxShadow: tokens.shadows.lg,
          filter: "brightness(1.02)",
        }
      : {}),
    ...style,
  };

  const handleClick = (e: MouseEvent) => {
    if (!disableClickLog) {
      const label = logLabel || title || extractText(children) || "Unnamed Button";
      invoke("log_ui_event", { message: `🖱️ Button clicked: ${label}` }).catch(() => {});
    }
    onClick?.(e);
  };

  return (
    <button
      type={type}
      className={className}
      onClick={handleClick}
      disabled={disabled}
      title={title}
      style={resolvedStyle}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => {
        setHovered(false);
        setPressed(false);
      }}
      onMouseDown={() => setPressed(true)}
      onMouseUp={() => setPressed(false)}
    >
      {children}
    </button>
  );
};

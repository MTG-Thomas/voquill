import type { JSX } from "preact";
import { tokens } from "../design-tokens.ts";

type Style = JSX.CSSProperties;

export const surfaceCardStyle: Style = {
  background: tokens.colors.glassBg,
  backdropFilter: `blur(${tokens.colors.glassBlur}) saturate(1.35)`,
  WebkitBackdropFilter: `blur(${tokens.colors.glassBlur}) saturate(1.35)`,
  border: "1px solid rgba(255, 255, 255, 0.10)",
  boxShadow: tokens.shadows.sm,
};

export const settingRowBaseStyle: Style = {
  marginBottom: tokens.spacing.md,
  display: "flex",
  flexDirection: "column",
  gap: "6px",
  alignItems: "flex-start",
  border: "1px solid rgba(255, 255, 255, 0.10)",
  borderRadius: tokens.radii.panel,
  background: "rgba(255, 255, 255, 0.04)",
  padding: "12px 14px",
  transition: "border-color 0.2s ease, background 0.2s ease",
};

export const getSettingRowStyle = ({ ready }: { ready: boolean }): Style => {
  if (ready) {
    return {
      ...settingRowBaseStyle,
      background: "rgba(16, 124, 16, 0.06)",
      borderColor: "rgba(16, 124, 16, 0.28)",
    };
  }

  return settingRowBaseStyle;
};

export const settingRowHeaderStyle: Style = {
  width: "100%",
  display: "flex",
  justifyContent: "space-between",
  alignItems: "flex-start",
  gap: tokens.spacing.sm,
};

export const settingRowStatusStyle: Style = {
  flexShrink: 0,
};

export const settingRowHeaderRightStyle: Style = {
  marginLeft: "auto",
  display: "flex",
  flexDirection: "column",
  alignItems: "flex-end",
  gap: "6px",
  flexShrink: 0,
};

export const settingRowLabelStyle: Style = {
  fontWeight: 600,
  color: tokens.colors.textPrimary,
  fontSize: tokens.typography.sizeSm,
  flex: 1,
  minWidth: 0,
  display: "block",
  textAlign: "left",
};

export const settingRowLabelBadgeStyle: Style = {
  fontSize: "10px",
  fontWeight: 800,
  letterSpacing: "0.06em",
  textTransform: "uppercase",
  color: "#fce100",
  border: "1px solid rgba(252, 225, 0, 0.28)",
  background: "rgba(252, 225, 0, 0.10)",
  borderRadius: "999px",
  padding: "2px 8px",
  lineHeight: 1.2,
};

export const settingRowDescriptionStyle: Style = {
  fontSize: tokens.typography.sizeXs,
  color: tokens.colors.textSecondary,
  margin: `0 0 ${tokens.spacing.sm} 0`,
  lineHeight: 1.4,
  textAlign: "left",
};

export const settingRowContentStyle: Style = {
  display: "flex",
  flexDirection: "column",
  gap: tokens.spacing.xs,
  width: "100%",
  alignItems: "center",
};

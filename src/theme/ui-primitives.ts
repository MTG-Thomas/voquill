import type { JSX } from "preact";
import { tokens } from "../design-tokens.ts";

export type Style = JSX.CSSProperties;

export const titleBarHeight = "42px";

export const appShellStyle: Style = {
  display: "flex",
  flexDirection: "column",
  width: "100%",
  height: "100%",
  position: "relative",
  background: `linear-gradient(180deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 42%, ${tokens.colors.bgGradientCool} 100%)`,
  color: tokens.colors.textPrimary,
};

export const titleBarStyle: Style = {
  height: titleBarHeight,
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
  padding: "0 8px 0 14px",
  background: "rgba(32, 32, 32, 0.78)",
  backdropFilter: `blur(${tokens.colors.glassBlur}) saturate(1.4)`,
  WebkitBackdropFilter: `blur(${tokens.colors.glassBlur}) saturate(1.4)`,
  borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
  userSelect: "none",
  WebkitUserSelect: "none",
};

export const titleBarTitleStyle: Style = {
  fontSize: "13px",
  fontWeight: 600,
  letterSpacing: 0,
  color: tokens.colors.textSecondary,
};

export const titleBarControlsStyle: Style = {
  display: "flex",
  alignItems: "center",
  gap: "2px",
  paddingRight: 0,
};

export const tabNavStyle: Style = {
  display: "flex",
  gap: "2px",
  padding: "6px 8px 0 8px",
  background: "rgba(32, 32, 32, 0.72)",
  backdropFilter: `blur(${tokens.colors.glassBlur}) saturate(1.2)`,
  WebkitBackdropFilter: `blur(${tokens.colors.glassBlur}) saturate(1.2)`,
  borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
  alignItems: "stretch",
};

export const tabContentStyle: Style = {
  flex: 1,
  minHeight: 0,
  overflow: "auto",
};

export const tabPanelStyle: Style = {
  width: "100%",
  minHeight: "100%",
  padding: "12px",
  display: "flex",
  flexDirection: "column",
};

export const tabPanelPaddedStyle: Style = {
  width: "100%",
  maxWidth: "900px",
  margin: "0 auto",
  display: "flex",
  flexDirection: "column",
  gap: "16px",
};

export const tabPanelContentStyle: Style = {
  width: "100%",
  maxWidth: "900px",
  margin: "0 auto",
  display: "flex",
  flexDirection: "column",
};

export const inputBaseStyle: Style = {
  width: "100%",
  background: tokens.colors.bgSecondary,
  color: tokens.colors.textPrimary,
  border: "1px solid rgba(255, 255, 255, 0.12)",
  borderBottom: `2px solid ${tokens.colors.accentPrimary}`,
  borderRadius: tokens.radii.input,
  padding: "8px 10px",
  fontSize: tokens.typography.sizeSm,
  outline: "none",
};

export const selectWrapperStyle: Style = {
  display: "flex",
  gap: tokens.spacing.sm,
  width: "100%",
  alignItems: "center",
};

export const helperTextStyle: Style = {
  fontSize: tokens.typography.sizeXs,
  color: tokens.colors.textSecondary,
  lineHeight: 1.4,
};

export const toastContainerStyle: Style = {
  position: "fixed",
  top: "60px",
  left: "50%",
  transform: "translateX(-50%)",
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  gap: "8px",
  zIndex: 1000,
  width: "min(420px, calc(100% - 24px))",
  padding: "0 12px",
  boxSizing: "border-box",
  pointerEvents: "none",
};

export const getToastStyle = (type: "success" | "error" | "info" | "saved"): Style => ({
  width: type === "saved" ? "auto" : "100%",
  maxWidth: type === "saved" ? "220px" : "100%",
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  padding: type === "saved" ? "6px 12px" : "10px 12px",
  borderRadius: type === "saved" ? "999px" : "10px",
  border: "none",
  background:
    type === "saved"
      ? tokens.colors.success
      : type === "success"
        ? tokens.colors.success
        : type === "error"
          ? tokens.colors.error
          : tokens.colors.bgSecondary,
  cursor: type === "saved" ? "default" : "pointer",
  pointerEvents: "auto",
  boxShadow: type === "saved" ? tokens.shadows.md : tokens.shadows.lg,
});

export const toastDotStyle: Style = {
  width: "8px",
  height: "8px",
  borderRadius: "999px",
  background: tokens.colors.accentPrimary,
  flexShrink: 0,
};

export const toastMessageStyle: Style = {
  fontSize: tokens.typography.sizeSm,
  color: tokens.colors.textPrimary,
};

export const getToastMessageStyle = (type: "success" | "error" | "info" | "saved"): Style => ({
  fontSize: type === "saved" ? tokens.typography.sizeXs : tokens.typography.sizeSm,
  color: tokens.colors.textPrimary,
  fontWeight: type === "saved" ? 700 : 500,
  letterSpacing: type === "saved" ? "0.01em" : "normal",
});

export const modalTextIntroStyle: Style = {
  ...helperTextStyle,
  marginBottom: "10px",
};

export const modalShortcutPathStyle: Style = {
  fontSize: tokens.typography.sizeSm,
  color: tokens.colors.textPrimary,
  fontWeight: 600,
  marginBottom: "8px",
};

export const modalShortcutNoteStyle: Style = {
  ...helperTextStyle,
};

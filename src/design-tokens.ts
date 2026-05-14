export const tokens = {
  colors: {
    // Backgrounds
    bgPrimary: "#202020",
    bgSecondary: "#2b2b2b",
    bgTertiary: "#1b1b1b",
    bgHover: "#3a3a3a",
    bgGradientWarm: "#242424",
    bgGradientCool: "#1b2430",

    // Text
    textPrimary: "#ffffff",
    textSecondary: "#d6d6d6",
    textMuted: "#a7a7a7",

    // Brand/Action
    accentPrimary: "#60cdff",
    accentHover: "#8ad7ff",
    success: "#6ccb5f",
    error: "#ff99a4",
    glassBg: "rgba(43, 43, 43, 0.74)",
    glassBgHeavy: "rgba(48, 48, 48, 0.96)",
    glassBlur: "24px",
  },

  spacing: {
    xs: "4px",
    sm: "8px",
    md: "16px",
    lg: "24px",
    xl: "32px",
  },

  radii: {
    input: "4px",
    panel: "8px",
    button: "4px",
  },

  shadows: {
    sm: "0 1px 2px rgba(0, 0, 0, 0.36)",
    md: "0 4px 12px rgba(0, 0, 0, 0.42)",
    lg: "0 18px 48px rgba(0, 0, 0, 0.54)",
    accent: "0 0 0 3px rgba(96, 205, 255, 0.24)",
  },

  transitions: {
    fast: "all 0.12s cubic-bezier(0.33, 0, 0.67, 1)",
    normal: "all 0.18s cubic-bezier(0.33, 0, 0.67, 1)",
    slow: "all 0.28s cubic-bezier(0.33, 0, 0.67, 1)",
  },

  typography: {
    fontMain: "Inter, 'Segoe UI', system-ui, -apple-system, sans-serif",
    fontMono: "'JetBrains Mono', 'Cascadia Code', 'Fira Code', monospace",
    sizeXs: "11px",

    sizeSm: "13px",
    sizeMd: "14px",
    sizeLg: "16px",
    sizeXl: "18px",
    sizeHuge: "32px",
  },
} as const;

export type DesignTokens = typeof tokens;

interface TokenTree {
  [key: string]: string | number | TokenTree;
}

// Helper to convert camelCase to kebab-case for CSS variables
export const tokensToCssVars = (obj: TokenTree, prefix = "--"): Record<string, string> => {
  const vars: Record<string, string> = {};

  const iterate = (current: TokenTree, currentPrefix: string) => {
    for (const key in current) {
      const value = current[key];
      const kebabKey = key.replace(/([a-z0-9])([A-Z])/g, "$1-$2").toLowerCase();
      const newPrefix = `${currentPrefix}${kebabKey}`;

      if (typeof value === "object" && value !== null) {
        iterate(value, `${newPrefix}-`);
      } else {
        vars[newPrefix] = String(value);
      }
    }
  };

  iterate(obj, prefix);
  return vars;
};

import { useEffect, useState } from "preact/hooks";
import { tokens } from "../design-tokens.ts";

const turboWarmPhases = [
  "Igniting NPU",
  "Loading Turbo weights",
  "Compiling NPU graph",
  "Priming decoder",
  "Running warm inference",
  "Ready soon",
] as const;

const TURBO_WARM_PHASE_MS = 30_000;

interface TurboWarmPhaseBarProps {
  compact?: boolean;
  startedAt?: number | null;
}

function getPhaseIndex(startedAt?: number | null) {
  if (!startedAt) {
    return 0;
  }

  const elapsedMs = Math.max(0, Date.now() - startedAt);
  return Math.min(Math.floor(elapsedMs / TURBO_WARM_PHASE_MS), turboWarmPhases.length - 1);
}

export function TurboWarmPhaseBar({ compact = false, startedAt = null }: TurboWarmPhaseBarProps) {
  const [phaseIndex, setPhaseIndex] = useState(() => getPhaseIndex(startedAt));

  useEffect(() => {
    setPhaseIndex(getPhaseIndex(startedAt));
    const interval = window.setInterval(() => {
      setPhaseIndex(getPhaseIndex(startedAt));
    }, 1000);

    return () => window.clearInterval(interval);
  }, [startedAt]);

  const progress = ((phaseIndex + 1) / turboWarmPhases.length) * 100;

  return (
    <div
      style={{
        width: compact ? "138px" : "260px",
        display: "flex",
        flexDirection: "column",
        gap: compact ? "4px" : "6px",
      }}
    >
      <style>{`
        @keyframes voquill-turbo-bar-shimmer {
          from { transform: translateX(-120%); }
          to { transform: translateX(220%); }
        }
        @keyframes voquill-turbo-dot-pulse {
          0%, 100% { opacity: 0.42; transform: scale(0.82); }
          45% { opacity: 1; transform: scale(1); }
        }
      `}</style>
      {!compact && (
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            gap: tokens.spacing.sm,
          }}
        >
          <span style={{ color: "#fff7c2", fontSize: tokens.typography.sizeXs, fontWeight: 800 }}>
            {turboWarmPhases[phaseIndex]}
          </span>
          <span
            style={{
              color: tokens.colors.textMuted,
              fontFamily: tokens.typography.fontMono,
              fontSize: tokens.typography.sizeXs,
            }}
          >
            phase {phaseIndex + 1}/{turboWarmPhases.length}
          </span>
        </div>
      )}
      <div
        aria-hidden="true"
        style={{
          position: "relative",
          height: compact ? "5px" : "7px",
          borderRadius: "999px",
          background: "rgba(255, 255, 255, 0.14)",
          overflow: "hidden",
          boxShadow: "inset 0 0 0 1px rgba(255, 255, 255, 0.08)",
        }}
      >
        <div
          style={{
            width: `${progress}%`,
            height: "100%",
            borderRadius: "999px",
            background: "linear-gradient(90deg, #f26d3d, #f8cf5a, #fff7c2)",
            transition: "width 0.5s ease",
          }}
        ></div>
        <div
          style={{
            position: "absolute",
            inset: 0,
            width: "42%",
            background: "linear-gradient(90deg, transparent, rgba(255,255,255,0.72), transparent)",
            animation: "voquill-turbo-bar-shimmer 1.15s ease-in-out infinite",
          }}
        ></div>
      </div>
      <div
        style={{ display: "flex", justifyContent: "space-between", gap: compact ? "3px" : "5px" }}
      >
        {turboWarmPhases.map((phase, index) => (
          <span
            key={phase}
            title={phase}
            style={{
              width: compact ? "5px" : "7px",
              height: compact ? "5px" : "7px",
              borderRadius: "50%",
              background: index <= phaseIndex ? "#f8cf5a" : "rgba(255,255,255,0.22)",
              boxShadow: index === phaseIndex ? "0 0 10px rgba(248, 207, 90, 0.75)" : "none",
              animation:
                index === phaseIndex ? "voquill-turbo-dot-pulse 1.1s ease-in-out infinite" : "none",
            }}
          ></span>
        ))}
      </div>
    </div>
  );
}

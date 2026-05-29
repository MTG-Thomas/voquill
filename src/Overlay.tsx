import { useState, useEffect, useRef } from "preact/hooks";
import { listen } from "@tauri-apps/api/event";
import StatusIcon from "./StatusIcon.tsx";
import { TurboWarmPhaseBar } from "./components/TurboWarmPhaseBar.tsx";
import { tokens } from "./design-tokens.ts";

interface StatusUpdatePayload {
  seq: number;
  status: string;
  turbo_warm?: boolean;
}

const TURBO_WARM_FALLBACK_TIMEOUT_MS = 190_000;

function Overlay() {
  const [status, setStatus] = useState<string>("Ready");
  const [isTurboWarmActive, setIsTurboWarmActive] = useState(false);
  const [turboWarmStartedAt, setTurboWarmStartedAt] = useState<number | null>(null);
  const lastStatusSeqRef = useRef<number>(0);
  const hasShownTurboWarmRef = useRef(false);
  const turboWarmTimeoutRef = useRef<number | null>(null);

  const beginTurboWarmUi = (startedAt = Date.now()) => {
    if (hasShownTurboWarmRef.current) {
      return;
    }

    hasShownTurboWarmRef.current = true;
    window.localStorage.setItem("voquill-turbo-warm-started-at", String(startedAt));
    setTurboWarmStartedAt(startedAt);
    setIsTurboWarmActive(true);
    if (turboWarmTimeoutRef.current !== null) {
      window.clearTimeout(turboWarmTimeoutRef.current);
    }
    turboWarmTimeoutRef.current = window.setTimeout(() => {
      setIsTurboWarmActive(false);
      turboWarmTimeoutRef.current = null;
    }, TURBO_WARM_FALLBACK_TIMEOUT_MS);
  };

  const endTurboWarmUi = () => {
    setIsTurboWarmActive(false);
    setTurboWarmStartedAt(null);
    window.localStorage.removeItem("voquill-turbo-warm-started-at");
  };

  const hasTauriRuntime =
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in (window as Window & { __TAURI_INTERNALS__?: unknown });
  const isPreviewMode = !hasTauriRuntime;

  useEffect(() => {
    if (isPreviewMode) {
      return;
    }

    let unlistenStatus: null | (() => void) = null;

    const setupEventListeners = async () => {
      try {
        unlistenStatus = await listen<string | StatusUpdatePayload>("status-update", (event) => {
          const payload = event.payload;
          const nextSeq = typeof payload === "string" ? lastStatusSeqRef.current + 1 : payload.seq;
          const newStatus = typeof payload === "string" ? payload : payload.status;

          if (newStatus !== "Recording" && newStatus !== "Transcribing") {
            return;
          }

          if (nextSeq < lastStatusSeqRef.current) {
            return;
          }

          lastStatusSeqRef.current = nextSeq;
          setStatus(newStatus);

          const turboWarmFromBackend = typeof payload === "object" && payload.turbo_warm === true;
          const turboWarmEligible =
            turboWarmFromBackend ||
            window.localStorage.getItem("voquill-turbo-warm-eligible") === "true";

          if (newStatus === "Transcribing" && turboWarmEligible) {
            const storedStartedAt = Number(
              window.localStorage.getItem("voquill-turbo-warm-started-at"),
            );
            const startedAt =
              Number.isFinite(storedStartedAt) && storedStartedAt > 0
                ? storedStartedAt
                : Date.now();
            beginTurboWarmUi(startedAt);
          } else if (newStatus !== "Transcribing") {
            endTurboWarmUi();
          }
        });
      } catch (error) {
        console.error("❌ Failed to setup overlay event listeners:", error);
      }
    };

    void setupEventListeners();

    return () => {
      if (unlistenStatus) {
        unlistenStatus();
      }
      if (turboWarmTimeoutRef.current !== null) {
        window.clearTimeout(turboWarmTimeoutRef.current);
      }
    };
  }, [isPreviewMode]);

  useEffect(() => {
    if (isPreviewMode) {
      return;
    }

    const htmlEl = document.documentElement;
    const bodyEl = document.body;
    const rootEl = document.getElementById("root");

    if (htmlEl) {
      htmlEl.style.background = "transparent";
    }
    if (bodyEl) {
      bodyEl.style.background = "transparent";
    }
    if (rootEl) {
      (rootEl as HTMLElement).style.background = "transparent";
    }
  }, [isPreviewMode]);

  if (isPreviewMode) {
    return (
      <div
        style={{
          minHeight: "100vh",
          width: "100%",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap: "16px",
          background: "#1f2125",
          color: "#fff",
          fontFamily: tokens.typography.fontMain,
        }}
      >
        <div style={{ display: "flex", gap: "8px" }}>
          <button
            type="button"
            onClick={() => setStatus("Recording")}
            style={{
              border: "none",
              borderRadius: "8px",
              padding: "8px 12px",
              background: status === "Recording" ? "#5865f2" : "rgba(255,255,255,0.12)",
              color: "#fff",
              cursor: "pointer",
            }}
          >
            Recording
          </button>
          <button
            type="button"
            onClick={() => setStatus("Transcribing")}
            style={{
              border: "none",
              borderRadius: "8px",
              padding: "8px 12px",
              background: status === "Transcribing" ? "#5865f2" : "rgba(255,255,255,0.12)",
              color: "#fff",
              cursor: "pointer",
            }}
          >
            Transcribing
          </button>
        </div>

        <div
          style={{
            width: "260px",
            height: "140px",
            border: "1px solid rgba(255,255,255,0.2)",
            borderRadius: "10px",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "flex-end",
            paddingBottom: "0",
            background: "rgba(255,255,255,0.04)",
          }}
        >
          <div
            key={`overlay-preview-${status}`}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "10px",
              isolation: "isolate",
              contain: "paint",
              overflow: "hidden",
              background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
              border: "1px solid rgba(255, 255, 255, 0.1)",
              borderRadius: "999px",
              padding: isTurboWarmActive ? "6px 14px 6px 8px" : "6px 12px 6px 8px",
              minWidth: isTurboWarmActive ? "242px" : "194px",
            }}
          >
            <StatusIcon
              status={status}
              size={40}
              variant={isTurboWarmActive ? "turboWarm" : "default"}
            />
            {isTurboWarmActive ? (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: "4px",
                  flex: 1,
                  minWidth: 0,
                }}
              >
                <span
                  style={{
                    color: "#fff",
                    fontFamily: tokens.typography.fontMain,
                    fontSize: "16px",
                    fontWeight: 700,
                    lineHeight: 1.1,
                    whiteSpace: "nowrap",
                    textShadow: "none",
                  }}
                >
                  Warming Turbo
                </span>
                <TurboWarmPhaseBar compact startedAt={turboWarmStartedAt} />
              </div>
            ) : (
              <span
                style={{
                  color: "#fff",
                  fontFamily: tokens.typography.fontMain,
                  fontSize: "18px",
                  fontWeight: 500,
                  textAlign: "center",
                  lineHeight: 1.2,
                  whiteSpace: "nowrap",
                  textShadow: "none",
                  flex: 1,
                }}
              >
                {status}
              </span>
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "flex-end",
        height: "100vh",
        width: "100vw",
        backgroundColor: "transparent",
        paddingBottom: "0",
      }}
    >
      <div
        key={`overlay-content-${status}`}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: "10px",
          isolation: "isolate",
          contain: "paint",
          overflow: "hidden",
          background: `linear-gradient(135deg, ${tokens.colors.bgGradientWarm} 0%, ${tokens.colors.bgPrimary} 50%, ${tokens.colors.bgGradientCool} 100%)`,
          border: "1px solid rgba(255, 255, 255, 0.1)",
          borderRadius: "999px",
          padding: isTurboWarmActive ? "6px 14px 6px 8px" : "6px 12px 6px 8px",
          minWidth: isTurboWarmActive ? "242px" : "194px",
        }}
      >
        <StatusIcon
          status={status}
          size={40}
          variant={isTurboWarmActive ? "turboWarm" : "default"}
        />
        {isTurboWarmActive ? (
          <div
            key={`overlay-status-${status}`}
            style={{ display: "flex", flexDirection: "column", gap: "4px", flex: 1, minWidth: 0 }}
          >
            <span
              style={{
                color: "#fff",
                fontFamily: tokens.typography.fontMain,
                fontSize: "16px",
                fontWeight: 700,
                lineHeight: 1.1,
                whiteSpace: "nowrap",
                textShadow: "none",
              }}
            >
              Warming Turbo
            </span>
            <TurboWarmPhaseBar compact startedAt={turboWarmStartedAt} />
          </div>
        ) : (
          <span
            key={`overlay-status-${status}`}
            style={{
              color: "#fff",
              fontFamily: tokens.typography.fontMain,
              fontSize: "18px",
              fontWeight: 500,
              textAlign: "center",
              lineHeight: 1.2,
              whiteSpace: "nowrap",
              textShadow: "none",
              flex: 1,
            }}
          >
            {status}
          </span>
        )}
      </div>
    </div>
  );
}

export default Overlay;

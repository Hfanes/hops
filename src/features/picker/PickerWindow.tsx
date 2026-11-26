import { useEffect, useState } from "react";
import { errorMessage } from "../../lib/errors";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FiLock, FiX } from "react-icons/fi";
import { getPickerState, hidePickerWindow, openUrl } from "../../services/picker";
import type { PickerSession, ThemePreference } from "../../types";
import { PickerBrowserList } from "./PickerBrowserList";
import "./picker.css";

// Also defined in src-tauri/src/lib.rs; a Rust test checks that the two match.
const PICKER_SESSION_EVENT = "picker-session";

function applyPickerTheme(themePreference: ThemePreference) {
  document.documentElement.classList.toggle("dark", themePreference === "dark");
}

function PickerWindow() {
  const [session, setSession] = useState<PickerSession | null>(null);
  const [statusText, setStatusText] = useState("");
  const [isOpening, setIsOpening] = useState(false);
  const [isAltPressed, setIsAltPressed] = useState(false);

  useEffect(() => {
    document.body.classList.add("picker-body");
    return () => {
      document.body.classList.remove("picker-body");
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const pickerWindow = getCurrentWindow();

    const applySession = (next: PickerSession) => {
      setSession(next);
      setStatusText("");
      setIsAltPressed(next.altPressed);
      applyPickerTheme(next.themePreference);
    };

    const registerUnlistener = (unlisten: () => void) => {
      if (disposed) {
        unlisten();
        return;
      }

      unlisteners.push(unlisten);
    };

    const load = async () => {
      try {
        const next = await getPickerState();
        if (disposed || !next) {
          return;
        }

        applySession(next);
      } catch (error) {
        if (disposed) {
          return;
        }

        const message = errorMessage(error);
        setStatusText(message);
      }
    };

    void load();

    void pickerWindow
      .listen<PickerSession>(PICKER_SESSION_EVENT, (event) => {
        applySession(event.payload);
        setIsOpening(false);
      })
      .then(registerUnlistener)
      .catch((error) => {
        if (!disposed) {
          setStatusText(errorMessage(error));
        }
      });

    void pickerWindow
      .onFocusChanged(({ payload }) => {
        if (!payload) {
          setIsAltPressed(false);
        }
      })
      .then(registerUnlistener)
      .catch((error) => {
        if (!disposed) {
          setStatusText(errorMessage(error));
        }
      });

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Alt") {
        setIsAltPressed(true);
      }

      if (event.key === "Escape") {
        event.preventDefault();
        void hidePickerWindow();
      }
    };

    const onKeyUp = (event: KeyboardEvent) => {
      if (event.key === "Alt") {
        setIsAltPressed(false);
      }
    };

    const onWindowBlur = () => {
      setIsAltPressed(false);
    };

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", onWindowBlur);

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => {
        unlisten();
      });
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", onWindowBlur);
    };
  }, []);

  async function handleOpen(browserId: string, privateMode: boolean) {
    if (!session) {
      return;
    }

    setIsOpening(true);
    setStatusText("");

    try {
      await openUrl({
        browserId,
        url: session.url,
        privateMode,
      });
      await hidePickerWindow();
    } catch (error) {
      const message = errorMessage(error);
      setStatusText(message);
    } finally {
      setIsOpening(false);
    }
  }

  return (
    <main className="picker-shell">
      <section className={`picker-menu ${session?.disableTransparency ? "solid" : ""}`}>
        <div className="picker-brand-row">
          <span className="picker-brand-mark">
            <img src="/hops.webp" alt="" aria-hidden="true" />
          </span>
          <div className="picker-brand-copy">
            <p>Hops Picker</p>
            <span>{session?.source === "manual" ? "Manual launch" : "Route decision"}</span>
          </div>
          {/* Mouse-only way out: Escape and click-outside need keyboard focus,
              which Windows does not always grant to the picker. */}
          <button
            type="button"
            className="picker-close"
            onClick={() => void hidePickerWindow()}
            aria-label="Close picker"
            title="Close (Esc)"
          >
            <FiX aria-hidden="true" />
          </button>
        </div>

        {session?.url ? (
          <header className="picker-menu-header" title={session.url}>
            <span>URL</span>
            {session.url}
          </header>
        ) : null}

        {/* One grid row for both, so the list keeps its row. */}
        {session?.notice || isAltPressed ? (
          <div className="picker-notices">
            {session?.notice ? (
              <p className="picker-status picker-notice" role="status">
                {session.notice}
              </p>
            ) : null}
            {isAltPressed ? (
              <p className="picker-hint">
                <FiLock aria-hidden="true" />
                Alt held: supported browsers will open in private mode.
              </p>
            ) : null}
          </div>
        ) : null}

        <PickerBrowserList
          browsers={session?.browsers ?? []}
          disabled={isOpening}
          isAltPressed={isAltPressed}
          usesRoutePrivate={session?.preferredPrivateMode ?? false}
          onOpen={(browserId, privateMode) =>
            void handleOpen(browserId, privateMode)
          }
        />

        {statusText ? <p className="picker-status">{statusText}</p> : null}
      </section>
    </main>
  );
}

export default PickerWindow;

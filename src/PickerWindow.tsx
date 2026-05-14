import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPickerState, hidePickerWindow, openUrl } from "./api";
import type { PickerSession } from "./types";
import "./PickerWindow.css";

const PICKER_SESSION_EVENT = "picker-session";

function supportsPrivateMode(privateFlag: string | null) {
  return !!privateFlag?.trim();
}

function PickerWindow() {
  const pickerWindow = getCurrentWindow();
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
    let mounted = true;

    const load = async () => {
      try {
        const next = await getPickerState();
        if (!mounted || !next) {
          return;
        }

        setSession(next);
        setStatusText("");
      } catch (error) {
        if (!mounted) {
          return;
        }

        const message = error instanceof Error ? error.message : String(error);
        setStatusText(message);
      }
    };

    void load();

    let unlistenSession: (() => void) | undefined;
    let unlistenFocus: (() => void) | undefined;

    void pickerWindow
      .listen<PickerSession>(PICKER_SESSION_EVENT, (event) => {
        setSession(event.payload);
        setStatusText("");
        setIsOpening(false);
      })
      .then((unlisten) => {
        unlistenSession = unlisten;
      });

    void pickerWindow
      .onFocusChanged(({ payload }) => {
        if (!payload) {
          void hidePickerWindow();
        }
      })
      .then((unlisten) => {
        unlistenFocus = unlisten;
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
      mounted = false;
      unlistenSession?.();
      unlistenFocus?.();
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", onWindowBlur);
    };
  }, [pickerWindow]);

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
      const message = error instanceof Error ? error.message : String(error);
      setStatusText(message);
    } finally {
      setIsOpening(false);
    }
  }

  return (
    <main className="picker-shell">
      <section className={`picker-menu ${session?.disableTransparency ? "solid" : ""}`}>
        {session?.url ? (
          <header className="picker-menu-header" title={session.url}>
            {session.url}
          </header>
        ) : null}

        {isAltPressed ? (
          <p className="picker-hint">Alt held: supported browsers will open in private mode.</p>
        ) : null}

        <div className="picker-menu-list" role="menu" aria-label="Open URL in browser">
          {session?.browsers.length ? (
            session.browsers.map((browser) => {
              const supportsPrivate = supportsPrivateMode(browser.privateFlag);
              const opensPrivate = isAltPressed && supportsPrivate;
              return (
                <button
                  key={browser.id}
                  type="button"
                  className={`picker-menu-item ${browser.isRunning ? "is-running" : ""} ${opensPrivate ? "private-mode" : ""}`}
                  onClick={(event) => {
                    const requestPrivate = supportsPrivate && (isAltPressed || event.altKey);
                    void handleOpen(browser.id, requestPrivate);
                  }}
                  disabled={isOpening}
                >
                  <span className="picker-menu-item-name">{browser.name}</span>
                  <span className="picker-menu-item-meta">
                    {browser.isDefault ? <span className="picker-chip">default</span> : null}
                    {browser.isRunning ? <span className="picker-chip running">running</span> : null}
                    {opensPrivate ? <span className="picker-chip private">private</span> : null}
                  </span>
                </button>
              );
            })
          ) : (
            <p className="picker-empty">No browsers are configured.</p>
          )}
        </div>

        {statusText ? <p className="picker-status">{statusText}</p> : null}
      </section>
    </main>
  );
}

export default PickerWindow;

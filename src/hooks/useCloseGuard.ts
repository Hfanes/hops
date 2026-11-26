import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ConfigStore } from "./useConfigStore";

/**
 * Keeps the settings window open until pending edits are saved. Closing
 * cancels the debounced saves, writes the latest config once, waits for the
 * save queue and then destroys the window. If saving fails the window stays
 * open and the settings save is scheduled again.
 */
export function useCloseGuard({
  store,
  cancelPendingBrowserSaves,
}: {
  store: ConfigStore;
  cancelPendingBrowserSaves: () => boolean;
}) {
  const {
    configRef,
    persistConfig,
    scheduleSettingsSave,
    cancelScheduledSettingsSave,
    waitForQueuedSaves,
  } = store;
  const isClosingRef = useRef(false);

  useEffect(() => {
    let disposed = false;
    let unlistenClose: (() => void) | null = null;

    void getCurrentWindow()
      .onCloseRequested((event) => {
        event.preventDefault();
        void closeSettingsWindow();
      })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlistenClose = unlisten;
      })
      .catch((error) => {
        console.warn("Could not listen for Hops close requests.", error);
      });

    return () => {
      disposed = true;
      unlistenClose?.();
    };
  }, []);

  async function closeSettingsWindow() {
    if (isClosingRef.current) {
      return;
    }

    isClosingRef.current = true;
    const hasPendingSettingsSave = cancelScheduledSettingsSave();
    const hasPendingBrowserSaves = cancelPendingBrowserSaves();

    try {
      const latestConfig = configRef.current;
      if (
        latestConfig &&
        (hasPendingSettingsSave || hasPendingBrowserSaves)
      ) {
        await persistConfig(latestConfig, {
          errorPrefix: "Could not save changes before closing",
          applySavedConfig: false,
        });
      }

      await waitForQueuedSaves();
      await getCurrentWindow().destroy();
    } catch {
      isClosingRef.current = false;
      if (configRef.current) {
        scheduleSettingsSave(configRef.current);
      }
    }
  }
}

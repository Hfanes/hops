import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  listMissingBrowserIds,
  listRunningBrowserIds,
} from "../../services/tauri";
import type { AppConfig, TabId } from "../../types";

const RUNNING_STATE_REFRESH_THROTTLE_MS = 1000;

/**
 * Which configured browsers are running, and which are no longer installed.
 * Refreshed when the Browsers tab opens and when the window regains focus
 * while it is open (throttled), never on a timer.
 */
export function useRunningBrowsers({
  activeTab,
  isLoading,
  configRef,
}: {
  activeTab: TabId;
  isLoading: boolean;
  configRef: { readonly current: AppConfig | null };
}) {
  const [runningBrowserIds, setRunningBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const [missingBrowserIds, setMissingBrowserIds] = useState<Set<string>>(
    new Set(),
  );
  const isRefreshInFlightRef = useRef(false);
  const lastRefreshAtRef = useRef(0);

  async function refreshRunningState({ force = false } = {}) {
    if (!configRef.current || isRefreshInFlightRef.current) {
      return;
    }

    const now = Date.now();
    if (
      !force &&
      now - lastRefreshAtRef.current < RUNNING_STATE_REFRESH_THROTTLE_MS
    ) {
      return;
    }

    isRefreshInFlightRef.current = true;
    lastRefreshAtRef.current = now;

    try {
      const [runningIds, missingIds] = await Promise.all([
        listRunningBrowserIds(),
        listMissingBrowserIds(),
      ]);
      setRunningBrowserIds(new Set(runningIds));
      setMissingBrowserIds(new Set(missingIds));
    } catch {
      // Running-state refresh should be non-blocking.
    } finally {
      isRefreshInFlightRef.current = false;
    }
  }

  useEffect(() => {
    if (activeTab !== "browsers" || isLoading) {
      return;
    }

    void refreshRunningState({ force: true });

    let disposed = false;
    let unlistenFocus: (() => void) | null = null;

    try {
      void getCurrentWindow()
        .onFocusChanged(({ payload }) => {
          if (payload) {
            void refreshRunningState();
          }
        })
        .then((unlisten) => {
          if (disposed) {
            unlisten();
            return;
          }

          unlistenFocus = unlisten;
        })
        .catch((error) => {
          console.warn("Could not listen for Hops focus changes.", error);
        });
    } catch (error) {
      console.warn("Could not access the current Hops window.", error);
    }

    return () => {
      disposed = true;
      unlistenFocus?.();
    };
  }, [activeTab, isLoading]);

  return { runningBrowserIds, missingBrowserIds, refreshRunningState };
}

import { useState } from "react";
import type { FeedbackMessage } from "../../components/common/FeedbackModal";
import { rejectAfter, ROUTE_OPEN_TIMEOUT_MS } from "../../lib/appTypes";
import { errorMessage } from "../../lib/errors";
import {
  previewRouteWithConfig,
  routeAndOpenWithConfig,
  showPickerForUrl,
} from "../../services/tauri";
import type { AppConfig, RouteDecision, StatusState } from "../../types";

/**
 * The Router tab: a URL to test, and either a preview of where it would go or
 * opening it for real (in the routed browser or through the picker).
 */
export function useRouteTester({
  config,
  setStatus,
  showFeedbackModal,
  refreshRunningState,
}: {
  config: AppConfig | null;
  setStatus: (status: StatusState) => void;
  showFeedbackModal: (feedback: FeedbackMessage) => void;
  refreshRunningState: (options?: { force?: boolean }) => Promise<void>;
}) {
  const [routeInput, setRouteInput] = useState(
    "https://github.com/openai/codex",
  );
  const [routeDecision, setRouteDecision] = useState<RouteDecision | null>(
    null,
  );
  const [isRouting, setIsRouting] = useState(false);

  async function runRoutePreview(openImmediately: boolean) {
    if (!config) {
      setStatus({ kind: "error", text: "Configuration is not loaded yet." });
      return;
    }

    if (!routeInput.trim()) {
      setStatus({ kind: "error", text: "Enter a URL to test routing." });
      return;
    }

    setIsRouting(true);
    try {
      const trimmedRouteInput = routeInput.trim();
      const decision = openImmediately
        ? await rejectAfter(
            routeAndOpenWithConfig(config, trimmedRouteInput),
            ROUTE_OPEN_TIMEOUT_MS,
            "Opening the routed browser timed out after 10 seconds.",
          )
        : await previewRouteWithConfig(config, trimmedRouteInput);

      setRouteDecision(decision);
      if (openImmediately && decision.action === "open_browser") {
        setStatus({
          kind: "success",
          text: `Opened in ${decision.browserName ?? "selected browser"}.`,
        });
      } else if (openImmediately && decision.action === "show_picker") {
        await rejectAfter(
          showPickerForUrl(
            trimmedRouteInput,
            decision.browserId,
            decision.privateMode,
          ),
          ROUTE_OPEN_TIMEOUT_MS,
          "Opening the picker timed out after 10 seconds.",
        );
        setStatus({
          kind: "success",
          text: "Routing requires the picker. The picker window was opened.",
        });
      } else {
        setStatus({ kind: "success", text: "Routing preview updated." });
      }
      await refreshRunningState({ force: true });
    } catch (error) {
      const message = errorMessage(error);
      setStatus({ kind: "error", text: `Routing failed: ${message}` });
      if (
        openImmediately &&
        error instanceof Error &&
        error.name === "TimeoutError"
      ) {
        showFeedbackModal({
          title: "Route opening timed out",
          message: `Hops could not open the route within 10 seconds.\n\n${message}`,
          kind: "warning",
        });
      }
    } finally {
      setIsRouting(false);
    }
  }

  return {
    routeInput,
    setRouteInput,
    routeDecision,
    clearRouteDecision: () => setRouteDecision(null),
    isRouting,
    runRoutePreview,
  };
}

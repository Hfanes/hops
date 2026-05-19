import type { RouteDecision } from "../../types";

export function RouterTester({
  routeDecision,
  routeInput,
  isRouting,
  onRouteInputChange,
  onRunRoutePreview,
}: {
  routeDecision: RouteDecision | null;
  routeInput: string;
  isRouting: boolean;
  onRouteInputChange: (value: string) => void;
  onRunRoutePreview: (shouldOpen: boolean) => void;
}) {
  return (
    <section className="tab-body">
      <p className="setting-help">
        Route tester simulates what Hops would do for this URL with your current
        in-app config. <strong>Preview route</strong> only shows the decision.{" "}
        <strong>Route and open</strong> also launches the chosen browser when
        action is <code>open_browser</code>.
      </p>
      <label>
        URL
        <input
          value={routeInput}
          onChange={(event) => onRouteInputChange(event.currentTarget.value)}
        />
      </label>
      <div className="inline-actions">
        <button
          type="button"
          className="secondary"
          disabled={isRouting}
          onClick={() => onRunRoutePreview(false)}
        >
          {isRouting ? "Checking..." : "Preview route"}
        </button>
        <button
          type="button"
          disabled={isRouting}
          onClick={() => onRunRoutePreview(true)}
        >
          {isRouting ? "Opening..." : "Route and open"}
        </button>
      </div>

      {routeDecision ? (
        <article className="card">
          <h3>Routing result</h3>
          <p>
            <strong>Action:</strong> {routeDecision.action}
          </p>
          <p>
            <strong>Reason:</strong> {routeDecision.reason}
          </p>
          <p>
            <strong>Browser:</strong>{" "}
            {routeDecision.browserName ?? "Picker required"}
          </p>
          <p>
            <strong>Private:</strong> {routeDecision.privateMode ? "Yes" : "No"}
          </p>
          <p>
            <strong>Matched rule:</strong>{" "}
            {routeDecision.matchedRuleId ?? "None"}
          </p>
        </article>
      ) : null}
    </section>
  );
}

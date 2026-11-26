import { FiPlus } from "react-icons/fi";
import type { TabId } from "../../types";

/** The buttons the Browsers and Rules tabs show in the top bar. */
export function TopbarActions({
  activeTab,
  isRefreshing,
  canAddRule,
  onRefreshBrowsers,
  onAddBrowser,
  onAddRule,
}: {
  activeTab: TabId;
  isRefreshing: boolean;
  canAddRule: boolean;
  onRefreshBrowsers: () => void;
  onAddBrowser: () => void;
  onAddRule: () => void;
}) {
  return (
    <>
      {activeTab === "browsers" ? (
        <>
          <button
            type="button"
            className="secondary"
            onClick={onRefreshBrowsers}
            disabled={isRefreshing}
          >
            {isRefreshing ? "Refreshing..." : "Refresh browsers"}
          </button>
          <button type="button" onClick={onAddBrowser}>
            <FiPlus aria-hidden="true" />
            Add browser
          </button>
        </>
      ) : null}
      {activeTab === "rules" ? (
        <button type="button" onClick={onAddRule} disabled={!canAddRule}>
          <FiPlus aria-hidden="true" />
          Add rule
        </button>
      ) : null}
    </>
  );
}

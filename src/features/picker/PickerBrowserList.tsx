import { FiCheck, FiExternalLink, FiLock, FiZap } from "react-icons/fi";
import type { PickerBrowserEntry } from "../../types";

function supportsPrivateMode(privateFlag: string | null) {
  return !!privateFlag?.trim();
}

function PickerBrowserItem({
  browser,
  disabled,
  isAltPressed,
  usesRoutePrivate,
  onOpen,
}: {
  browser: PickerBrowserEntry;
  disabled: boolean;
  isAltPressed: boolean;
  usesRoutePrivate: boolean;
  onOpen: (browserId: string, privateMode: boolean) => void;
}) {
  const supportsPrivate = supportsPrivateMode(browser.privateFlag);
  const opensPrivate = supportsPrivate && (isAltPressed || usesRoutePrivate);

  return (
    <button
      type="button"
      className={`picker-menu-item ${browser.isRunning ? "is-running" : ""} ${opensPrivate ? "private-mode" : ""}`}
      onClick={(event) => {
        const requestPrivate =
          supportsPrivate &&
          (isAltPressed || event.altKey || usesRoutePrivate);
        onOpen(browser.id, requestPrivate);
      }}
      disabled={disabled}
    >
      <span className="picker-menu-item-name">
        <FiExternalLink aria-hidden="true" />
        {browser.name}
      </span>
      <span className="picker-menu-item-meta">
        {browser.isDefault ? (
          <span className="picker-chip">
            <FiCheck aria-hidden="true" />
            default
          </span>
        ) : null}
        {browser.isRunning ? (
          <span className="picker-chip running">
            <FiZap aria-hidden="true" />
            running
          </span>
        ) : null}
        {opensPrivate ? (
          <span className="picker-chip private">
            <FiLock aria-hidden="true" />
            private
          </span>
        ) : null}
      </span>
    </button>
  );
}

export function PickerBrowserList({
  browsers,
  disabled,
  isAltPressed,
  usesRoutePrivate,
  onOpen,
}: {
  browsers: PickerBrowserEntry[];
  disabled: boolean;
  isAltPressed: boolean;
  usesRoutePrivate: boolean;
  onOpen: (browserId: string, privateMode: boolean) => void;
}) {
  return (
    <div className="picker-menu-list" role="menu" aria-label="Open URL in browser">
      {browsers.length ? (
        browsers.map((browser) => (
          <PickerBrowserItem
            key={browser.id}
            browser={browser}
            disabled={disabled}
            isAltPressed={isAltPressed}
            usesRoutePrivate={usesRoutePrivate}
            onOpen={onOpen}
          />
        ))
      ) : (
        <p className="picker-empty">No browsers are configured.</p>
      )}
    </div>
  );
}

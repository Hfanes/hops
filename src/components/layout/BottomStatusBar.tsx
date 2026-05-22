import {
  FiAlertTriangle,
  FiCheckCircle,
  FiFileText,
  FiRefreshCw,
} from "react-icons/fi";
import type { AppUpdateStatus } from "../../services/tauri";

export function BottomStatusBar({
  configPath,
  updateStatus,
  isCheckingUpdate,
  appVersion,
}: {
  configPath: string | null;
  updateStatus: AppUpdateStatus | null;
  isCheckingUpdate: boolean;
  appVersion: string | null;
}) {
  const version = updateStatus?.currentVersion ?? appVersion ?? "unknown";
  const updateLabel = isCheckingUpdate
    ? "Checking updates..."
    : updateStatus?.available
      ? `Update available ${updateStatus.version}`
      : updateStatus
        ? "Up to date"
        : "Update status unavailable";
  const UpdateIcon = isCheckingUpdate
    ? FiRefreshCw
    : updateStatus?.available || !updateStatus
      ? FiAlertTriangle
      : FiCheckCircle;
  const updateClassName =
    updateStatus?.available || !updateStatus ? "warning" : "running";

  return (
    <footer className="bottom-status-bar" aria-label="Application status">
      <div className="bottom-status-item bottom-status-config">
        <FiFileText aria-hidden="true" />
        <span>Config loaded:</span>
        <strong title={configPath ?? undefined} className="pt-1">
          {configPath ?? "Path unavailable"}
        </strong>
      </div>

      <div
        className={`bottom-status-item bottom-status-update ${updateClassName}`}
      >
        <UpdateIcon aria-hidden="true" />
        <strong>{updateLabel}</strong>
        <span>v{version}</span>
      </div>
    </footer>
  );
}

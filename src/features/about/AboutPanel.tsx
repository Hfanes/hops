import {
  FiDownload,
  FiGithub,
  FiGlobe,
  FiMail,
  FiRefreshCw,
} from "react-icons/fi";
import { RiTwitterXFill } from "react-icons/ri";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { MouseEvent } from "react";
import type { AppAboutInfo, AppUpdateStatus } from "../../services/tauri";

const creatorLinks = [
  {
    label: "GitHub",
    href: "https://github.com/Hfanes",
    icon: FiGithub,
  },
  {
    label: "Twitter",
    href: "https://x.com/hfa_dev",
    icon: RiTwitterXFill,
  },
  {
    label: "Website",
    href: "https://www.hfanes.com/",
    icon: FiGlobe,
  },
  {
    label: "Mail",
    href: "mailto:anesfh@gmail.com",
    icon: FiMail,
  },
];

export function AboutPanel({
  aboutInfo,
  updateStatus,
  isCheckingUpdate,
  isInstallingUpdate,
  onRefreshAbout,
  onUpdateApp,
}: {
  aboutInfo: AppAboutInfo | null;
  updateStatus: AppUpdateStatus | null;
  isCheckingUpdate: boolean;
  isInstallingUpdate: boolean;
  onRefreshAbout: () => void;
  onUpdateApp: () => void;
}) {
  async function handleCreatorLink(
    event: MouseEvent<HTMLAnchorElement>,
    href: string,
  ) {
    event.preventDefault();

    try {
      await openUrl(href);
    } catch (error) {
      console.warn("Could not open creator link.", error);
    }
  }

  return (
    <section className="tab-body">
      <article className="card">
        <div className="card-title">
          <h3>Hops</h3>
          <div className="badges">
            <span
              className={`badge ${updateStatus?.available ? "warning" : "running"}`}
            >
              {isCheckingUpdate
                ? "checking"
                : updateStatus?.available
                  ? "update available"
                  : updateStatus
                    ? "up to date"
                    : "not checked"}
            </span>
          </div>
        </div>

        <div className="about-grid">
          <div className="about-row">
            <span>Current version</span>
            <strong>
              {aboutInfo?.version ??
                updateStatus?.currentVersion ??
                "Loading..."}
            </strong>
          </div>
          <div className="about-row">
            <span>Release date</span>
            <strong>{aboutInfo?.releaseDate ?? "Loading..."}</strong>
          </div>
          <div className="about-row">
            <span>Update status</span>
            <strong>
              {isCheckingUpdate
                ? "Checking..."
                : updateStatus?.available
                  ? `Hops ${updateStatus.version} is available`
                  : updateStatus
                    ? "You are on the latest version"
                    : "Not checked yet"}
            </strong>
          </div>
          {updateStatus?.available && updateStatus.date ? (
            <div className="about-row">
              <span>Available release date</span>
              <strong>{updateStatus.date}</strong>
            </div>
          ) : null}
        </div>

        {updateStatus?.available && updateStatus.body ? (
          <div className="action-panel">
            <p className="setting-help">{updateStatus.body}</p>
          </div>
        ) : null}

        <div className="inline-actions">
          <button
            type="button"
            className="secondary"
            onClick={onRefreshAbout}
            disabled={isCheckingUpdate || isInstallingUpdate}
          >
            <FiRefreshCw aria-hidden="true" />
            {isCheckingUpdate ? "Checking..." : "Check for updates"}
          </button>
          {updateStatus?.available ? (
            <button
              type="button"
              onClick={onUpdateApp}
              disabled={isCheckingUpdate || isInstallingUpdate}
            >
              <FiDownload aria-hidden="true" />
              {isInstallingUpdate ? "Installing..." : "Update now"}
            </button>
          ) : null}
        </div>

        <p className="setting-help">
          Updates are checked against the latest GitHub Release. Hops downloads
          signed update artifacts and restarts after a successful install.
        </p>
      </article>

      <article className="card">
        <div className="card-title">
          <h3>By @hfa</h3>
        </div>

        <div className="about-links">
          {creatorLinks.map(({ label, href, icon: Icon }) => (
            <a
              key={label}
              className="about-link"
              href={href}
              rel="noreferrer"
              onClick={(event) => void handleCreatorLink(event, href)}
            >
              <Icon aria-hidden="true" />
              <span>{label}</span>
            </a>
          ))}
        </div>
      </article>
    </section>
  );
}

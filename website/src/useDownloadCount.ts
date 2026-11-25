import { useEffect, useState } from "react";

const releasesApiUrl =
  "https://api.github.com/repos/Hfanes/hops/releases?per_page=100";
const cacheKey = "hops:download-count";
// Keeps repeat visits and refreshes from hitting the GitHub API: the
// unauthenticated limit is 60 requests per hour per visitor IP.
const successTtlMs = 6 * 60 * 60 * 1000;
// A failure (rate limit, offline) is remembered too, so a visitor who keeps
// refreshing does not retry on every load.
const failureTtlMs = 60 * 60 * 1000;
// Only installers count; latest.json and .sig files are fetched by the
// updater and would inflate the total.
const installerNames = new Set(["hops.exe", "hops.msi"]);

type CachedCount = {
  count: number | null;
  fetchedAt: number;
};

type Release = {
  assets: { name: string; download_count: number }[];
};

function readCache(): CachedCount | null {
  try {
    const raw = localStorage.getItem(cacheKey);
    if (!raw) {
      return null;
    }

    const cached = JSON.parse(raw) as CachedCount;
    const ttl = cached.count === null ? failureTtlMs : successTtlMs;
    return Date.now() - cached.fetchedAt < ttl ? cached : null;
  } catch {
    return null;
  }
}

function writeCache(count: number | null) {
  try {
    const entry: CachedCount = { count, fetchedAt: Date.now() };
    localStorage.setItem(cacheKey, JSON.stringify(entry));
  } catch {
    // Storage can be full or disabled; the count is optional either way.
  }
}

/**
 * Total installer downloads across all GitHub releases, or null while loading
 * or when unavailable. Fetched at most once per cache window per browser.
 */
export function useDownloadCount(): number | null {
  const [count, setCount] = useState<number | null>(
    () => readCache()?.count ?? null,
  );

  useEffect(() => {
    if (readCache()) {
      return;
    }

    const controller = new AbortController();

    fetch(releasesApiUrl, {
      headers: { Accept: "application/vnd.github+json" },
      signal: controller.signal,
    })
      .then((response) => {
        if (!response.ok) {
          throw new Error(`GitHub API responded ${response.status}`);
        }
        return response.json() as Promise<Release[]>;
      })
      .then((releases) => {
        const total = releases
          .flatMap((release) => release.assets)
          .filter((asset) => installerNames.has(asset.name))
          .reduce((sum, asset) => sum + asset.download_count, 0);
        writeCache(total);
        setCount(total);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) {
          return;
        }
        console.warn("Could not load download count.", error);
        writeCache(null);
      });

    return () => controller.abort();
  }, []);

  return count;
}

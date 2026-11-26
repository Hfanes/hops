import { useEffect, useState } from "react";
import { errorMessage } from "../lib/errors";
import {
  getConfigFilePath,
  loadConfig,
  takeConfigRecoveryNotice,
} from "../services/tauri";
import type { AppConfig, StatusState } from "../types";

/**
 * Loads the config once when the settings window opens, reports whether it
 * was recovered from a broken file, and then loads the config path and the
 * caller's `loadStartupDetails` in parallel. `loadStartupDetails` resolves to
 * a function that applies what it loaded; it runs, with the config path, only
 * once everything has loaded. `isLoading` stays true until all of it is done
 * or one part fails.
 *
 * Hooks that need `isLoading` run before the ones that provide what
 * `loadStartupDetails` loads, so it is only read inside the effect, after the
 * first render.
 */
export function useAppBootstrap({
  applyLoadedConfig,
  setStatus,
  loadStartupDetails,
}: {
  applyLoadedConfig: (config: AppConfig) => void;
  setStatus: (status: StatusState) => void;
  loadStartupDetails: () => Promise<() => void>;
}) {
  const [isLoading, setIsLoading] = useState(true);
  const [configPath, setConfigPath] = useState<string | null>(null);

  useEffect(() => {
    const bootstrap = async () => {
      setIsLoading(true);
      try {
        const loaded = await loadConfig();
        applyLoadedConfig(loaded);
        const recoveryNotice = await takeConfigRecoveryNotice();
        setStatus(
          recoveryNotice
            ? { kind: "warning", text: recoveryNotice }
            : { kind: "success", text: "Configuration loaded." },
        );

        const [loadedConfigPath, applyStartupDetails] = await Promise.all([
          getConfigFilePath(),
          loadStartupDetails(),
        ]);
        setConfigPath(loadedConfigPath);
        applyStartupDetails();
      } catch (error) {
        const message = errorMessage(error);
        setStatus({ kind: "error", text: `Failed to load config: ${message}` });
      } finally {
        setIsLoading(false);
      }
    };

    void bootstrap();
  }, []);

  return { isLoading, configPath };
}

import { useCallback, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

/**
 * Media lifecycle plumbing behind the File menu and the media pool: offline
 * marking, moved-file relinking, changed-file refresh, colour info, and the
 * proxy state commands (attach/detach/toggle). Mutations run through the
 * Rust engine; the UI only collects gestures and reports results.
 */
export type ColorInfo = {
  media_id: number;
  name: string;
  colorspace: string | null;
  gamma: string | null;
  codec: string | null;
  width: number | null;
  height: number | null;
  fps: number | null;
  color_label: string | null;
};

export interface MediaHygieneOps {
  /** Bumped by searchFocus(); the pool focuses its search box. */
  searchFocusTick: number;
  searchFocus: () => void;
  /** Picks directories, then auto-relinks moved files by name. */
  relinkMoved: () => Promise<void>;
  refreshChanged: () => Promise<void>;
  offlineAll: () => Promise<void>;
  makeOffline: (mediaId: number) => Promise<void>;
  /** Picks a proxy file for the asset. */
  attachProxy: (mediaId: number) => Promise<void>;
  detachProxy: (mediaId: number) => Promise<void>;
  /** Flips the project-wide proxy playback flag. */
  toggleProxies: () => Promise<void>;
  colorInfo: (mediaId: number) => Promise<ColorInfo | null>;
}

export function useMediaHygiene({
  runCommand,
  refreshProject,
  setNotice,
  setBusy,
}: {
  runCommand: <T = unknown>(id: string, params?: unknown) => Promise<T>;
  refreshProject: () => Promise<unknown>;
  setNotice: (message: string) => void;
  setBusy: (busy: boolean) => void;
}): MediaHygieneOps {
  const [searchFocusTick, setSearchFocusTick] = useState(0);


  const searchFocus = useCallback(() => {
    setSearchFocusTick((tick) => tick + 1);
  }, []);

  const relinkMoved = useCallback(async () => {
    const picked = await open({
      directory: true,
      multiple: true,
      title: "Choose folders to scan for moved media",
    });
    const dirs = (Array.isArray(picked) ? picked : picked ? [picked] : []).filter(
      (entry): entry is string => typeof entry === "string",
    );
    if (dirs.length === 0) return;
    setBusy(true);
    try {
      const result = await runCommand<{ relinked: number; still_missing?: number }>(
        "media.auto_relink",
        { search_dirs: dirs },
      );
      await refreshProject();
      setNotice(
        `Relinked ${result.relinked} moved asset(s)` +
          (result.still_missing ? `, ${result.still_missing} still missing` : ""),
      );
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }, [runCommand, refreshProject, setNotice, setBusy]);

  const refreshChanged = useCallback(async () => {
    setBusy(true);
    try {
      const result = await runCommand<{
        changed: number[];
        missing: number[];
        baselined: number[];
      }>("media.refresh_changed", {});
      await refreshProject();
      const parts = [
        `${result.changed.length} changed`,
        `${result.missing.length} missing`,
      ];
      if (result.baselined.length) parts.push(`${result.baselined.length} newly tracked`);
      setNotice(`Media refresh: ${parts.join(", ")}`);
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }, [runCommand, refreshProject, setNotice, setBusy]);

  const offlineAll = useCallback(async () => {
    try {
      const result = await runCommand<{ offline: number }>("media.offline_all", {});
      await refreshProject();
      setNotice(`Marked ${result.offline} media offline`);
    } catch (error) {
      setNotice(String(error));
    }
  }, [runCommand, refreshProject, setNotice]);

  const makeOffline = useCallback(
    async (mediaId: number) => {
      try {
        await runCommand("media.make_offline", { media_ids: [mediaId] });
        await refreshProject();
        setNotice("Media marked offline");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const attachProxy = useCallback(
    async (mediaId: number) => {
      const picked = await open({
        multiple: false,
        title: "Choose the proxy file",
      });
      if (typeof picked !== "string" || !picked) return;
      try {
        await runCommand("media.attach_proxy", { media_id: mediaId, path: picked });
        await refreshProject();
        setNotice("Proxy attached");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const detachProxy = useCallback(
    async (mediaId: number) => {
      try {
        await runCommand("media.detach_proxy", { media_id: mediaId });
        await refreshProject();
        setNotice("Proxy detached");
      } catch (error) {
        setNotice(String(error));
      }
    },
    [runCommand, refreshProject, setNotice],
  );

  const toggleProxies = useCallback(async () => {
    try {
      const result = await runCommand<{ use_proxies: boolean }>(
        "media.toggle_proxies",
        {},
      );
      await refreshProject();
      setNotice(result.use_proxies ? "Proxy playback on" : "Proxy playback off");
    } catch (error) {
      setNotice(String(error));
    }
  }, [runCommand, refreshProject, setNotice]);

  const colorInfo = useCallback(
    async (mediaId: number) => {
      try {
        return await runCommand<ColorInfo>("media.color_info", { media_id: mediaId });
      } catch (error) {
        setNotice(String(error));
        return null;
      }
    },
    [runCommand, setNotice],
  );

  return {
    searchFocusTick,
    searchFocus,
    relinkMoved,
    refreshChanged,
    offlineAll,
    makeOffline,
    attachProxy,
    detachProxy,
    toggleProxies,
    colorInfo,
  };
}

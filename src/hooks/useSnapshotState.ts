import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { SNAPSHOT_EVENT } from "../constants";
import { getSnapshot } from "../lib/tauriApi";
import { formatInvokeError } from "../lib/utils";
import type { Snapshot } from "../types";

export async function fetchSnapshot() {
  return getSnapshot();
}

export function useSnapshotState() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        const current = await fetchSnapshot();
        if (mounted) {
          setSnapshot(current);
          setLoadError(null);
        }

        unlisten = await listen<Snapshot>(SNAPSHOT_EVENT, (event) => {
          setSnapshot(event.payload);
          setLoadError(null);
        });
      } catch (error) {
        if (mounted) {
          setLoadError(formatInvokeError(error));
        }
      }
    })();

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  return [snapshot, setSnapshot, loadError] as const;
}

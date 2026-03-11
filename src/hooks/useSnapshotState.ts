import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import { SNAPSHOT_EVENT } from "../constants";
import { getSnapshot } from "../lib/tauriApi";
import type { Snapshot } from "../types";

export async function fetchSnapshot() {
  return getSnapshot();
}

export function useSnapshotState() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);

  useEffect(() => {
    let mounted = true;
    let unlisten: (() => void) | undefined;

    void (async () => {
      const current = await fetchSnapshot();
      if (mounted) {
        setSnapshot(current);
      }

      unlisten = await listen<Snapshot>(SNAPSHOT_EVENT, (event) => {
        setSnapshot(event.payload);
      });
    })();

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  return [snapshot, setSnapshot] as const;
}

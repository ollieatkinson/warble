import { useEffect } from "react";

import { isIndicatorWindow } from "./constants";
import { ControlApp } from "./ControlApp";
import { AppErrorBoundary } from "./components/AppErrorBoundary";
import { IndicatorApp } from "./components/TranscriptionPill";
import { useSnapshotState } from "./hooks/useSnapshotState";

export default function App() {
  const [snapshot, setSnapshot, loadError] = useSnapshotState();

  useEffect(() => {
    document.documentElement.classList.toggle(
      "indicator-window",
      isIndicatorWindow,
    );
    document.body.classList.toggle("indicator-window", isIndicatorWindow);

    return () => {
      document.documentElement.classList.remove("indicator-window");
      document.body.classList.remove("indicator-window");
    };
  }, []);

  return (
    <AppErrorBoundary>
      {isIndicatorWindow ? (
        <IndicatorApp snapshot={snapshot} />
      ) : (
        <ControlApp
          snapshot={snapshot}
          setSnapshot={setSnapshot}
          loadError={loadError}
        />
      )}
    </AppErrorBoundary>
  );
}

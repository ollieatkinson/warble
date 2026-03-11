import { openPath } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";

import { ActionButton } from "../components/common";
import { CheckIcon, CopyIcon, ExternalIcon, RefreshIcon } from "../components/icons";
import { useButtonFeedback } from "../hooks/useButtonFeedback";
import { getDebugLogs } from "../lib/tauriApi";
import {
  buildSupportReport,
  formatCaptureInput,
  formatInvokeError,
} from "../lib/utils";
import type { DebugLogs, Snapshot } from "../types";

const EMPTY_LOGS: DebugLogs = {
  capture: "",
  livePreview: "",
};

export function DebugSection({ snapshot }: { snapshot: Snapshot }) {
  const [logs, setLogs] = useState<DebugLogs>(EMPTY_LOGS);
  const [supportNote, setSupportNote] = useState<string | null>(null);
  const {
    buttonFeedback,
    clearButtonFeedback,
    finishButtonFeedback,
    setButtonFeedbackState,
  } = useButtonFeedback();
  const selectedSource =
    snapshot.sources.find((source) => source.id === snapshot.settings.selectedSourceId) ?? null;

  async function loadLogs(actionId?: string) {
    if (actionId) {
      setButtonFeedbackState(actionId, "working");
    }

    try {
      setLogs(await getDebugLogs());
      if (actionId) {
        finishButtonFeedback(actionId, 900);
      }
    } catch (error) {
      if (actionId) {
        clearButtonFeedback(actionId);
      }
      setSupportNote(formatInvokeError(error));
    }
  }

  useEffect(() => {
    void loadLogs();
  }, [
    snapshot.captureDiagnostics.logPath,
    snapshot.captureDiagnostics.recentEvents.length,
    snapshot.previewDiagnostics.logPath,
    snapshot.previewDiagnostics.recentEvents.length,
  ]);

  async function copySupportReport() {
    const actionId = "debug-copy-report";
    setButtonFeedbackState(actionId, "working");

    try {
      await navigator.clipboard.writeText(buildSupportReport(snapshot));
      setSupportNote("Copied a support report with capture and live preview diagnostics.");
      finishButtonFeedback(actionId);
    } catch (error) {
      clearButtonFeedback(actionId);
      setSupportNote(formatInvokeError(error));
    }
  }

  async function openDiagnosticLog(
    path: string | null,
    actionId: string,
    label: string,
  ) {
    if (!path) {
      return;
    }

    setButtonFeedbackState(actionId, "working");

    try {
      await openPath(path);
      setSupportNote(`Opened the ${label} log.`);
      finishButtonFeedback(actionId, 900);
    } catch (error) {
      clearButtonFeedback(actionId);
      setSupportNote(formatInvokeError(error));
    }
  }

  return (
    <>
      <article className="surface debug-support-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Support</span>
          </div>
        </div>

        <div className="feature-unlock-list">
          <div className="feature-unlock-row">
            <strong>Platform</strong>
            <span>{snapshot.platform}</span>
          </div>
          <div className="feature-unlock-row">
            <strong>Status</strong>
            <span>{snapshot.statusMessage}</span>
          </div>
          <div className="feature-unlock-row">
            <strong>Error</strong>
            <span>{snapshot.errorMessage ?? "None"}</span>
          </div>
          <div className="feature-unlock-row">
            <strong>Source</strong>
            <span>
              {selectedSource
                ? `${selectedSource.name} · ${formatCaptureInput(
                    selectedSource.sampleRate,
                    selectedSource.channels,
                  )}`
                : "No source selected"}
            </span>
          </div>
          <div className="feature-unlock-row">
            <strong>Model</strong>
            <span>
              {snapshot.settings.selectedModelId} · {snapshot.settings.selectedModelKind}
            </span>
          </div>
        </div>

        <div className="inline-actions debug-actions">
          <ActionButton
            className="secondary small"
            state={buttonFeedback["debug-copy-report"]}
            idleLabel="Copy report"
            workingLabel="Copying"
            doneLabel="Copied"
            idleIcon={<CopyIcon className="small-icon" />}
            workingIcon={<CopyIcon className="small-icon" />}
            doneIcon={<CheckIcon className="small-icon" />}
            onClick={copySupportReport}
          />
          <ActionButton
            className="secondary small"
            state={buttonFeedback["debug-refresh-logs"]}
            idleLabel="Refresh logs"
            workingLabel="Refreshing"
            doneLabel="Updated"
            idleIcon={<RefreshIcon className="small-icon" />}
            workingIcon={<RefreshIcon className="small-icon" />}
            doneIcon={<CheckIcon className="small-icon" />}
            onClick={() => void loadLogs("debug-refresh-logs")}
          />
          <ActionButton
            className="secondary small"
            state={buttonFeedback["debug-open-capture-log"]}
            idleLabel="Open capture log"
            workingLabel="Opening"
            doneLabel="Opened"
            idleIcon={<ExternalIcon className="small-icon" />}
            workingIcon={<ExternalIcon className="small-icon" />}
            doneIcon={<CheckIcon className="small-icon" />}
            onClick={() =>
              void openDiagnosticLog(
                snapshot.captureDiagnostics.logPath,
                "debug-open-capture-log",
                "capture",
              )
            }
            disabled={!snapshot.captureDiagnostics.logPath}
          />
          <ActionButton
            className="secondary small"
            state={buttonFeedback["debug-open-preview-log"]}
            idleLabel="Open preview log"
            workingLabel="Opening"
            doneLabel="Opened"
            idleIcon={<ExternalIcon className="small-icon" />}
            workingIcon={<ExternalIcon className="small-icon" />}
            doneIcon={<CheckIcon className="small-icon" />}
            onClick={() =>
              void openDiagnosticLog(
                snapshot.previewDiagnostics.logPath,
                "debug-open-preview-log",
                "live preview",
              )
            }
            disabled={!snapshot.previewDiagnostics.logPath}
          />
        </div>

        {supportNote ? <p className="debug-support-note">{supportNote}</p> : null}
      </article>

      <section className="compact-grid-two">
        <article className="surface preview-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Capture</span>
            </div>
          </div>

          <div className="feature-unlock-list">
            <div className="feature-unlock-row">
              <strong>Status</strong>
              <span>{snapshot.captureDiagnostics.status}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Detail</strong>
              <span>{snapshot.captureDiagnostics.detail || "No capture detail yet"}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Source</strong>
              <span>{snapshot.captureDiagnostics.sourceName || "No capture yet"}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Input</strong>
              <span>
                {formatCaptureInput(
                  snapshot.captureDiagnostics.sampleRate,
                  snapshot.captureDiagnostics.channels,
                )}
              </span>
            </div>
            <div className="feature-unlock-row">
              <strong>Buffered</strong>
              <span>{snapshot.captureDiagnostics.lastBufferedSamples || 0} samples</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Log path</strong>
              <span>{snapshot.captureDiagnostics.logPath ?? "Unavailable"}</span>
            </div>
            <div className="feature-unlock-row feature-unlock-row-stack">
              <strong>Recent</strong>
              <span className="feature-unlock-events">
                {snapshot.captureDiagnostics.recentEvents.length > 0
                  ? snapshot.captureDiagnostics.recentEvents.join("\n")
                  : "No capture events yet"}
              </span>
            </div>
          </div>

          <pre className="debug-log-viewer">
            {logs.capture || "No capture log entries yet."}
          </pre>
        </article>

        <article className="surface preview-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Live Preview</span>
            </div>
          </div>

          <div className="feature-unlock-list">
            <div className="feature-unlock-row">
              <strong>Backend</strong>
              <span>{snapshot.previewDiagnostics.backend || "No session yet"}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Status</strong>
              <span>{snapshot.previewDiagnostics.status}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Detail</strong>
              <span>{snapshot.previewDiagnostics.detail || "No detail yet"}</span>
            </div>
            <div className="feature-unlock-row">
              <strong>Log path</strong>
              <span>{snapshot.previewDiagnostics.logPath ?? "Unavailable"}</span>
            </div>
            <div className="feature-unlock-row feature-unlock-row-stack">
              <strong>Recent</strong>
              <span className="feature-unlock-events">
                {snapshot.previewDiagnostics.recentEvents.length > 0
                  ? snapshot.previewDiagnostics.recentEvents.join("\n")
                  : "No live preview events yet"}
              </span>
            </div>
          </div>

          <pre className="debug-log-viewer">
            {logs.livePreview || "No live preview log entries yet."}
          </pre>
        </article>
      </section>
    </>
  );
}

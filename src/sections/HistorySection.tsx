import { useEffect, useState } from "react";

import { audioRetentionOptions } from "../constants";
import { ActionButton } from "../components/common";
import {
  AboutIcon,
  CheckIcon,
  CopyIcon,
  ExternalIcon,
  SearchIcon,
  TrashIcon,
} from "../components/icons";
import {
  formatAudioRetentionPolicy,
  formatCaptureInput,
  formatDuration,
  formatInferenceProvider,
} from "../lib/utils";
import type { ButtonFeedbackState, HistoryItem, SettingsDraft, Snapshot } from "../types";

export function HistorySection({
  snapshot,
  draft,
  historyQuery,
  filteredHistory,
  buttonFeedback,
  onSetHistoryQuery,
  onApplySettings,
  onOpenHistoryAudio,
  onCopyHistory,
  onRemoveHistoryItem,
  onClearHistory,
}: {
  snapshot: Snapshot;
  draft: SettingsDraft;
  historyQuery: string;
  filteredHistory: HistoryItem[];
  buttonFeedback: Record<string, ButtonFeedbackState>;
  onSetHistoryQuery: (value: string) => void;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
  onOpenHistoryAudio: (item: HistoryItem) => void | Promise<void>;
  onCopyHistory: (id: string, text: string) => void | Promise<void>;
  onRemoveHistoryItem: (id: string) => void | Promise<void>;
  onClearHistory: () => void | Promise<void>;
}) {
  const [expandedItems, setExpandedItems] = useState<Record<string, boolean>>({});
  const [confirmClearAll, setConfirmClearAll] = useState(false);

  useEffect(() => {
    if (!confirmClearAll) {
      return;
    }

    const timeout = window.setTimeout(() => {
      setConfirmClearAll(false);
    }, 2500);

    return () => {
      window.clearTimeout(timeout);
    };
  }, [confirmClearAll]);

  useEffect(() => {
    if (snapshot.history.length === 0) {
      setConfirmClearAll(false);
    }
  }, [snapshot.history.length]);

  return (
    <>
      <section className="compact-grid-two">
        <article className="surface preference-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Search</span>
            </div>
            {snapshot.history.length > 0 ? (
              <ActionButton
                className={`secondary small history-clear-button ${confirmClearAll ? "history-clear-button-confirm" : ""}`}
                state={buttonFeedback["history-clear-all"]}
                idleLabel={confirmClearAll ? "Confirm delete all" : "Delete all"}
                workingLabel="Deleting"
                doneLabel="Deleted"
                idleIcon={<TrashIcon className="small-icon" />}
                doneIcon={<CheckIcon className="small-icon" />}
                onClick={() => {
                  if (!confirmClearAll) {
                    setConfirmClearAll(true);
                    return;
                  }

                  setConfirmClearAll(false);
                  void onClearHistory();
                }}
              />
            ) : null}
          </div>

          <label className="search-field">
            <SearchIcon className="search-icon" />
            <input
              type="search"
              value={historyQuery}
              onChange={(event) => onSetHistoryQuery(event.currentTarget.value)}
              placeholder="Search transcripts"
            />
          </label>

          <div className="mini-meta-row">
            <span>{snapshot.history.length} saved</span>
            <span>{filteredHistory.length} visible</span>
          </div>
        </article>

        <article className="surface preference-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Audio clips</span>
            </div>
          </div>

          <div className="setting-list">
            <div className="field">
              <span>Keep original captured audio</span>
              <div className="segmented">
                {audioRetentionOptions.map((option) => (
                  <button
                    key={option.id}
                    type="button"
                    className={`segment ${draft.audioRetentionPolicy === option.id ? "segment-active" : ""}`}
                    onClick={() =>
                      void onApplySettings({
                        audioRetentionPolicy: option.id,
                      })
                    }
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </div>
          </div>

          <div className="mini-meta-row">
            <span>{formatAudioRetentionPolicy(draft.audioRetentionPolicy)}</span>
            <span>Transcript text stays until you remove it</span>
          </div>
        </article>
      </section>

      {filteredHistory.length === 0 ? (
        <section className="surface empty-state">
          {snapshot.history.length === 0 ? "No transcripts yet." : "No matches."}
        </section>
      ) : (
        <section className="history-list">
          {filteredHistory.map((item) => (
            <article
              className={`surface history-row ${expandedItems[item.id] ? "history-row-expanded" : ""}`}
              key={item.id}
            >
              <div className="history-row-head">
              <div className="history-meta">
                  <span>{new Date(item.createdAt).toLocaleString()}</span>
                  <span>{formatDuration(item.durationMs)}</span>
                </div>
                <div className="history-actions">
                  <button
                    className={`secondary small icon-only-button ${expandedItems[item.id] ? "history-details-toggle-active" : ""}`}
                    onClick={() =>
                      setExpandedItems((current) => ({
                        ...current,
                        [item.id]: !current[item.id],
                      }))
                    }
                    aria-label={expandedItems[item.id] ? "Hide details" : "Show details"}
                    title={expandedItems[item.id] ? "Hide details" : "Show details"}
                  >
                    <AboutIcon className="small-icon" />
                  </button>
                  {item.audioPath ? (
                    <button
                      className="secondary small icon-only-button"
                      onClick={() => {
                        void onOpenHistoryAudio(item);
                      }}
                      aria-label="Open audio"
                      title="Open audio"
                    >
                      <ExternalIcon className="small-icon" />
                    </button>
                  ) : null}
                  <ActionButton
                    className="secondary small"
                    state={buttonFeedback[`copy:${item.id}`]}
                    idleLabel="Copy"
                    doneLabel="Copied"
                    idleIcon={<CopyIcon className="small-icon" />}
                    doneIcon={<CheckIcon className="small-icon" />}
                    onClick={() => onCopyHistory(item.id, item.text)}
                    iconOnly
                  />
                  <ActionButton
                    className="secondary small"
                    state={buttonFeedback[`history-remove:${item.id}`]}
                    idleLabel="Remove"
                    workingLabel="Removing"
                    doneLabel="Removed"
                    idleIcon={<TrashIcon className="small-icon" />}
                    doneIcon={<CheckIcon className="small-icon" />}
                    onClick={() => onRemoveHistoryItem(item.id)}
                    iconOnly
                  />
                </div>
              </div>
              <p>{item.text}</p>
              {expandedItems[item.id] ? (
                <div className="history-details">
                  <div className="history-details-grid">
                    <div className="history-detail-item">
                      <span>Model</span>
                      <strong>{item.capture.modelName || "Unknown"}</strong>
                    </div>
                    <div className="history-detail-item">
                      <span>Inference</span>
                      <strong>
                        {formatInferenceProvider(item.capture.inferenceProvider)}
                      </strong>
                    </div>
                    <div className="history-detail-item">
                      <span>
                        {item.capture.sourceKind === "file" ? "Source file" : "Microphone"}
                      </span>
                      <strong>{item.sourceName}</strong>
                    </div>
                    <div className="history-detail-item">
                      <span>Input</span>
                      <strong>
                        {formatCaptureInput(
                          item.capture.inputSampleRate,
                          item.capture.inputChannels,
                        )}
                      </strong>
                    </div>
                    <div className="history-detail-item">
                      <span>Transcribe</span>
                      <strong>
                        {item.capture.transcriptionSampleRate > 0
                          ? `${Math.round(item.capture.transcriptionSampleRate / 100) / 10} kHz · mono`
                          : "Unknown"}
                      </strong>
                    </div>
                    <div className="history-detail-item">
                      <span>Audio clip</span>
                      <strong>{item.audioPath ? "Saved" : "Not saved"}</strong>
                    </div>
                  </div>
                </div>
              ) : null}
            </article>
          ))}
        </section>
      )}
    </>
  );
}

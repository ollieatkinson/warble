import { audioRetentionOptions } from "../constants";
import { ActionButton } from "../components/common";
import { CheckIcon, CopyIcon, ExternalIcon, SearchIcon, TrashIcon } from "../components/icons";
import { formatAudioRetentionPolicy, formatDuration } from "../lib/utils";
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
}) {
  return (
    <>
      <section className="compact-grid-two">
        <article className="surface preference-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Search</span>
            </div>
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
            <article className="surface history-row" key={item.id}>
              <div className="history-row-head">
                <div className="history-meta">
                  <span>{new Date(item.createdAt).toLocaleString()}</span>
                  <span>{item.sourceName}</span>
                  <span>{item.mode}</span>
                  <span>{formatDuration(item.durationMs)}</span>
                  {item.audioPath ? <span>Audio saved</span> : null}
                </div>
                <div className="history-actions">
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
            </article>
          ))}
        </section>
      )}
    </>
  );
}

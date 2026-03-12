import { CLEANUP_SUGGESTIONS } from "../constants";
import { ActionButton } from "../components/common";
import { CheckIcon } from "../components/icons";
import type { ButtonFeedbackState, SettingsDraft } from "../types";

export function CleanupSection({
  draft,
  cleanupInput,
  cleanupTerms,
  buttonFeedback,
  onSetCleanupInput,
  onApplySettings,
  onAddCleanupTerm,
  onRemoveCleanupTerm,
  onRestoreCleanupDefaults,
}: {
  draft: SettingsDraft;
  cleanupInput: string;
  cleanupTerms: string[];
  buttonFeedback: Record<string, ButtonFeedbackState>;
  onSetCleanupInput: (value: string) => void;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
  onAddCleanupTerm: (term?: string) => void | Promise<void>;
  onRemoveCleanupTerm: (term: string) => void | Promise<void>;
  onRestoreCleanupDefaults: () => void | Promise<void>;
}) {
  const availableCleanupSuggestions = CLEANUP_SUGGESTIONS.filter(
    (term) => !cleanupTerms.includes(term),
  );

  return (
    <section className="compact-grid-two">
      <article className="surface preference-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Cleanup vocabulary</span>
          </div>
        </div>

        <div className="setting-list">
          <label className="toggle-row toggle-row-card setting-toggle">
            <input
              type="checkbox"
              checked={draft.cleanupEnabled}
              onChange={(event) =>
                void onApplySettings({
                  cleanupEnabled: event.currentTarget.checked,
                })
              }
            />
            <div>
              <strong>Remove filler words</strong>
              <span>Clean transcripts before paste and before saving to history.</span>
            </div>
          </label>

          <div className="field">
            <span>Word or phrase</span>
            <div className="inline-actions cleanup-add-row">
              <input
                value={cleanupInput}
                onChange={(event) => onSetCleanupInput(event.currentTarget.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    void onAddCleanupTerm();
                  }
                }}
                placeholder="e.g. um, uh, you know"
              />
              <ActionButton
                state={buttonFeedback["cleanup-add"]}
                idleLabel="Add"
                workingLabel="Adding"
                doneLabel="Added"
                doneIcon={<CheckIcon className="small-icon" />}
                onClick={() => onAddCleanupTerm()}
              />
            </div>
          </div>

          {availableCleanupSuggestions.length > 0 ? (
            <div className="cleanup-suggestions">
              {availableCleanupSuggestions.map((term) => (
                <button
                  key={term}
                  type="button"
                  className="secondary cleanup-suggestion"
                  onClick={() => {
                    void onAddCleanupTerm(term);
                  }}
                >
                  {term}
                </button>
              ))}
            </div>
          ) : null}
        </div>

        <div className="mini-meta-row">
          <span>{draft.cleanupEnabled ? "Cleanup on" : "Cleanup off"}</span>
          <span>{cleanupTerms.length} terms</span>
        </div>
      </article>

      <article className="surface preference-surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Active phrases</span>
          </div>
          <ActionButton
            className="secondary small"
            state={buttonFeedback["cleanup-restore"]}
            idleLabel="Defaults"
            workingLabel="Restoring"
            doneLabel="Restored"
            doneIcon={<CheckIcon className="small-icon" />}
            onClick={onRestoreCleanupDefaults}
          />
        </div>

        {cleanupTerms.length === 0 ? (
          <div className="empty-state cleanup-empty-state">No cleanup phrases yet.</div>
        ) : (
          <div className="cleanup-list">
            {cleanupTerms.map((term) => (
              <div className="cleanup-chip" key={term}>
                <span>{term}</span>
                <ActionButton
                  className="secondary small cleanup-chip-remove"
                  state={buttonFeedback[`cleanup-remove:${term}`]}
                  idleLabel="Remove"
                  doneLabel="Removed"
                  doneIcon={<CheckIcon className="small-icon" />}
                  onClick={() => onRemoveCleanupTerm(term)}
                />
              </div>
            ))}
          </div>
        )}
      </article>
    </section>
  );
}

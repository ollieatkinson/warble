import { CleanupSection } from "./CleanupSection";
import { ActionButton } from "../components/common";
import { CheckIcon } from "../components/icons";
import type {
  ButtonFeedbackState,
  ReplacementRule,
  SettingsDraft,
} from "../types";

export function VocabularySection({
  draft,
  cleanupInput,
  cleanupTerms,
  replacementVariantsInput,
  replacementValueInput,
  replacementRules,
  buttonFeedback,
  onSetCleanupInput,
  onSetReplacementVariantsInput,
  onSetReplacementValueInput,
  onApplySettings,
  onAddCleanupTerm,
  onRemoveCleanupTerm,
  onRestoreCleanupDefaults,
  onAddReplacementRule,
  onRemoveReplacementRule,
}: {
  draft: SettingsDraft;
  cleanupInput: string;
  cleanupTerms: string[];
  replacementVariantsInput: string;
  replacementValueInput: string;
  replacementRules: ReplacementRule[];
  buttonFeedback: Record<string, ButtonFeedbackState>;
  onSetCleanupInput: (value: string) => void;
  onSetReplacementVariantsInput: (value: string) => void;
  onSetReplacementValueInput: (value: string) => void;
  onApplySettings: (update: Partial<SettingsDraft>) => void | Promise<void>;
  onAddCleanupTerm: (term?: string) => void | Promise<void>;
  onRemoveCleanupTerm: (term: string) => void | Promise<void>;
  onRestoreCleanupDefaults: () => void | Promise<void>;
  onAddReplacementRule: () => void | Promise<void>;
  onRemoveReplacementRule: (id: string) => void | Promise<void>;
}) {
  return (
    <>
      <section className="surface">
        <div className="surface-bar">
          <div className="surface-title">
            <span className="surface-title-label">Vocabulary</span>
          </div>
        </div>

        <div className="detail-copy">
          <p>
            Manage filler words and phrases that Warble should strip from finished transcripts
            before paste and history save, then teach it how specific words and phrases should be
            written in the final output.
          </p>
        </div>
      </section>

      <CleanupSection
        draft={draft}
        cleanupInput={cleanupInput}
        cleanupTerms={cleanupTerms}
        buttonFeedback={buttonFeedback}
        onSetCleanupInput={onSetCleanupInput}
        onApplySettings={onApplySettings}
        onAddCleanupTerm={onAddCleanupTerm}
        onRemoveCleanupTerm={onRemoveCleanupTerm}
        onRestoreCleanupDefaults={onRestoreCleanupDefaults}
      />

      <section className="compact-grid-two">
        <article className="surface preference-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Replacement rules</span>
            </div>
          </div>

          <div className="setting-list">
            <div className="field">
              <span>Spoken variants</span>
              <input
                value={replacementVariantsInput}
                onChange={(event) =>
                  onSetReplacementVariantsInput(event.currentTarget.value)
                }
                placeholder="e.g. github, git hub"
              />
            </div>

            <div className="field">
              <span>Write it as</span>
              <div className="inline-actions replacement-add-row">
                <input
                  value={replacementValueInput}
                  onChange={(event) =>
                    onSetReplacementValueInput(event.currentTarget.value)
                  }
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      void onAddReplacementRule();
                    }
                  }}
                  placeholder="e.g. GitHub"
                />
                <ActionButton
                  state={buttonFeedback["replacement-add"]}
                  idleLabel="Add"
                  workingLabel="Adding"
                  doneLabel="Added"
                  doneIcon={<CheckIcon className="small-icon" />}
                  onClick={() => onAddReplacementRule()}
                />
              </div>
            </div>
          </div>

          <div className="mini-meta-row">
            <span>{replacementRules.length} rules</span>
            <span>Applied after cleanup</span>
          </div>
        </article>

        <article className="surface preference-surface">
          <div className="surface-bar">
            <div className="surface-title">
              <span className="surface-title-label">Active replacements</span>
            </div>
          </div>

          {replacementRules.length === 0 ? (
            <div className="empty-state cleanup-empty-state">
              No custom replacements yet.
            </div>
          ) : (
            <div className="replacement-rule-list">
              {replacementRules.map((rule) => (
                <div className="replacement-rule" key={rule.id}>
                  <div className="replacement-rule-copy">
                    <strong>{rule.replacement}</strong>
                    <span>{rule.variants.join(" · ")}</span>
                  </div>
                  <ActionButton
                    className="secondary small cleanup-chip-remove"
                    state={buttonFeedback[`replacement-remove:${rule.id}`]}
                    idleLabel="Remove"
                    doneLabel="Removed"
                    doneIcon={<CheckIcon className="small-icon" />}
                    onClick={() => onRemoveReplacementRule(rule.id)}
                  />
                </div>
              ))}
            </div>
          )}
        </article>
      </section>
    </>
  );
}

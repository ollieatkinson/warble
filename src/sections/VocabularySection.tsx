import { CleanupSection } from "./CleanupSection";
import type { ButtonFeedbackState, SettingsDraft } from "../types";

export function VocabularySection({
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
            before paste and history save.
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
    </>
  );
}

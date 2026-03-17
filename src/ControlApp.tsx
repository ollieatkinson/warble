import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect } from "react";

import { sections, utilitySections, settingsPanes, SHELL_ACTION_EVENT } from "./constants";
import { NoticeBanner, StatusChip } from "./components/common";
import { CheckIcon, SectionIcon } from "./components/icons";
import { useControlApp } from "./hooks/useControlApp";
import { AboutSection } from "./sections/AboutSection";
import { CaptureSection } from "./sections/CaptureSection";
import { DebugSection } from "./sections/DebugSection";
import { GeneralSettingsPane } from "./sections/GeneralSettingsPane";
import { HistorySection } from "./sections/HistorySection";
import { InterfaceSection } from "./sections/InterfaceSection";
import { KeybindingsSection } from "./sections/KeybindingsSection";
import { ModelsSection } from "./sections/ModelsSection";
import { Sidebar } from "./sections/Sidebar";
import { ShellDialog } from "./components/ShellDialog";
import { VocabularySection } from "./sections/VocabularySection";
import type { ShellActionId, Snapshot } from "./types";

const allSections = [...sections, ...utilitySections];

export function ControlApp({
  snapshot,
  setSnapshot,
  loadError,
}: {
  snapshot: Snapshot | null;
  setSnapshot: (snapshot: Snapshot | null) => void;
  loadError?: string | null;
}) {
  const control = useControlApp({ snapshot, setSnapshot });
  const shellOpenAboutDialog = control.ready ? control.openAboutDialog : null;
  const shellCloseDialog = control.ready ? control.closeDialog : null;
  const shellSetActiveSection = control.ready ? control.setActiveSection : null;
  const shellTranscribeFile = control.ready ? control.transcribeFile : null;

  useEffect(() => {
    if (
      !control.ready ||
      !shellCloseDialog ||
      !shellOpenAboutDialog ||
      !shellSetActiveSection ||
      !shellTranscribeFile
    ) {
      return;
    }

    let unlisten: (() => void) | undefined;

    const handleShellAction = (action: ShellActionId) => {
      switch (action) {
        case "open-settings":
          shellSetActiveSection("settings");
          break;
        case "open-troubleshooting":
          shellSetActiveSection("help");
          break;
        case "open-about":
          shellOpenAboutDialog();
          break;
        case "navigate-capture":
          shellCloseDialog();
          shellSetActiveSection("capture");
          break;
        case "navigate-models":
          shellCloseDialog();
          shellSetActiveSection("models");
          break;
        case "navigate-vocabulary":
          shellCloseDialog();
          shellSetActiveSection("vocabulary");
          break;
        case "navigate-history":
          shellCloseDialog();
          shellSetActiveSection("history");
          break;
        case "transcribe-file":
          void shellTranscribeFile();
          break;
        default:
          break;
      }
    };

    const handleWindowShellAction = (event: Event) => {
      if (event instanceof CustomEvent && typeof event.detail === "string") {
        handleShellAction(event.detail as ShellActionId);
      }
    };

    window.addEventListener(SHELL_ACTION_EVENT, handleWindowShellAction);

    void (async () => {
      unlisten = await getCurrentWebviewWindow().listen<ShellActionId>(
        SHELL_ACTION_EVENT,
        (event) => {
          handleShellAction(event.payload);
        },
      );
    })();

    return () => {
      window.removeEventListener(SHELL_ACTION_EVENT, handleWindowShellAction);
      unlisten?.();
    };
  }, [
    shellCloseDialog,
    shellOpenAboutDialog,
    shellSetActiveSection,
    shellTranscribeFile,
    control.ready,
  ]);

  if (!control.ready) {
    return (
      <main className={`loading-shell ${loadError ? "loading-shell-error" : ""}`}>
        <div className="loading-shell-copy">
          <strong>{loadError ? "Warble couldn't load." : "Loading..."}</strong>
          {loadError ? <span>{loadError}</span> : null}
        </div>
      </main>
    );
  }

  const {
    snapshot: currentSnapshot,
    activeSection,
    setActiveSection,
    sidebarCollapsed,
    setSidebarCollapsed,
    activeDialog,
    activeSettingsPane,
    setActiveSettingsPane,
    message,
    setMessage,
    buttonFeedback,
    capturing,
    setCapturing,
    historyQuery,
    setHistoryQuery,
    cleanupInput,
    setCleanupInput,
    replacementVariantsInput,
    setReplacementVariantsInput,
    replacementValueInput,
    setReplacementValueInput,
    draft,
    dismissSnapshotError,
    applySettings,
    refreshDevices,
    startRecording,
    stopRecording,
    cancelCurrentOperation,
    transcribeFile,
    copyHistory,
    openHistoryAudio,
    removeHistoryItem,
    clearHistory,
    chooseDefaultModel,
    chooseLivePreviewModel,
    activateModel,
    downloadCatalogModel,
    removeCatalogModel,
    openModelReference,
    addCleanupTerm,
    removeCleanupTerm,
    restoreCleanupDefaults,
    addReplacementRule,
    removeReplacementRule,
    closeDialog,
    activeSource,
    sourceOptions,
    activeModel,
    batchModelRows,
    streamingModelRows,
    readyModelOptions,
    activeReadyModelId,
    livePreviewOptions,
    resolvedLivePreviewModel,
    effectiveLivePreviewModelId,
    recentFileTranscript,
    filteredHistory,
    previewTitle,
    previewDetail,
    cleanupTerms,
    replacementRules,
  } = control;

  const activePaneMeta =
    settingsPanes.find((pane) => pane.id === activeSettingsPane) ?? settingsPanes[0];

  return (
    <main
      className={`workspace-shell ${sidebarCollapsed ? "workspace-shell-collapsed" : ""}`}
    >
      <Sidebar
        activeSection={activeSection}
        collapsed={sidebarCollapsed}
        phase={currentSnapshot.phase}
        onSelect={setActiveSection}
        onToggleCollapsed={() => setSidebarCollapsed((value) => !value)}
      />

      <section className="workspace-main">
        <header className="workspace-header">
          <div className="workspace-title">
            <SectionIcon section={activeSection} className="workspace-title-icon" />
            <h1>{allSections.find((section) => section.id === activeSection)?.label}</h1>
          </div>

          <div className="header-actions">
            <StatusChip
              label={
                currentSnapshot.modelStatus === "ready"
                  ? `${activeModel?.family ?? "Model"} ready`
                  : `${activeModel?.family ?? "Model"} missing`
              }
              tone={currentSnapshot.modelStatus === "ready" ? "success" : "warning"}
              icon={<CheckIcon className="chip-icon-svg" />}
            />
            <StatusChip
              label={currentSnapshot.shortcutsActive ? "Keys active" : "Keys off"}
              tone={currentSnapshot.shortcutsActive ? "accent" : "warning"}
            />
          </div>
        </header>

        {message ? (
          <NoticeBanner
            kind={message.kind}
            text={message.text}
            onDismiss={() => setMessage(null)}
          />
        ) : null}
        {!message && currentSnapshot.errorMessage ? (
          <NoticeBanner
            kind="error"
            text={currentSnapshot.errorMessage}
            onDismiss={() => {
              void dismissSnapshotError();
            }}
          />
        ) : null}

        <div className="content-stack">
          {activeSection === "capture" ? (
            <CaptureSection
              snapshot={currentSnapshot}
              draft={draft}
              sourceOptions={sourceOptions}
              activeSource={activeSource}
              activeModel={activeModel}
              previewTitle={previewTitle}
              previewDetail={previewDetail}
              recentFileTranscript={recentFileTranscript}
              historyCount={currentSnapshot.history.length}
              cleanupTermsCount={cleanupTerms.length}
              fileActionState={buttonFeedback["transcribe-file"]}
              refreshActionState={buttonFeedback["refresh-inputs"]}
              onApplySettings={applySettings}
              onRefreshDevices={refreshDevices}
              onStartRecording={startRecording}
              onStopRecording={() => {
                void stopRecording();
              }}
              onCancel={() => {
                void cancelCurrentOperation();
              }}
              onTranscribeFile={() => {
                void transcribeFile();
              }}
              onNavigateToModels={() => setActiveSection("models")}
            />
          ) : null}

          {activeSection === "models" ? (
            <ModelsSection
              snapshot={currentSnapshot}
              batchModels={batchModelRows}
              streamingModels={streamingModelRows}
              activeModel={activeModel}
              activeReadyModelId={activeReadyModelId}
              readyModelOptions={readyModelOptions}
              livePreviewModel={resolvedLivePreviewModel}
              livePreviewOptions={livePreviewOptions}
              effectiveLivePreviewModelId={effectiveLivePreviewModelId}
              buttonFeedback={buttonFeedback}
              onChooseDefaultModel={chooseDefaultModel}
              onChooseLivePreviewModel={(value) => {
                void chooseLivePreviewModel(value);
              }}
              onActivateModel={(row) => activateModel(row.id)}
              onDownloadCatalogModel={(row) => downloadCatalogModel(row.id)}
              onRemoveCatalogModel={(row) => removeCatalogModel(row.id)}
              onOpenModelReference={(row) => openModelReference(row.hfUrl)}
            />
          ) : null}

          {activeSection === "vocabulary" ? (
            <VocabularySection
              draft={draft}
              cleanupInput={cleanupInput}
              cleanupTerms={cleanupTerms}
              replacementVariantsInput={replacementVariantsInput}
              replacementValueInput={replacementValueInput}
              replacementRules={replacementRules}
              buttonFeedback={buttonFeedback}
              onSetCleanupInput={setCleanupInput}
              onSetReplacementVariantsInput={setReplacementVariantsInput}
              onSetReplacementValueInput={setReplacementValueInput}
              onApplySettings={applySettings}
              onAddCleanupTerm={addCleanupTerm}
              onRemoveCleanupTerm={removeCleanupTerm}
              onRestoreCleanupDefaults={restoreCleanupDefaults}
              onAddReplacementRule={addReplacementRule}
              onRemoveReplacementRule={removeReplacementRule}
            />
          ) : null}

          {activeSection === "history" ? (
            <HistorySection
              snapshot={currentSnapshot}
              historyQuery={historyQuery}
              filteredHistory={filteredHistory}
              buttonFeedback={buttonFeedback}
              onSetHistoryQuery={setHistoryQuery}
              onOpenHistoryAudio={(item) => openHistoryAudio(item.audioPath)}
              onCopyHistory={copyHistory}
              onRemoveHistoryItem={removeHistoryItem}
              onClearHistory={clearHistory}
            />
          ) : null}

          {activeSection === "settings" ? (
            <div className="settings-shell">
              <aside className="settings-nav">
                {settingsPanes.map((pane) => (
                  <button
                    key={pane.id}
                    type="button"
                    className={`settings-nav-button ${pane.id === activeSettingsPane ? "settings-nav-button-active" : ""}`}
                    onClick={() => setActiveSettingsPane(pane.id)}
                  >
                    <strong>{pane.label}</strong>
                    <span>{pane.description}</span>
                  </button>
                ))}
              </aside>

              <div className="settings-pane">
                <div className="settings-pane-intro">
                  <strong>{activePaneMeta.label}</strong>
                  <span>{activePaneMeta.description}</span>
                </div>

                {activeSettingsPane === "general" ? (
                  <GeneralSettingsPane
                    snapshot={currentSnapshot}
                    draft={draft}
                    onApplySettings={applySettings}
                  />
                ) : null}

                {activeSettingsPane === "shortcuts" ? (
                  <KeybindingsSection
                    snapshot={currentSnapshot}
                    draft={draft}
                    capturing={capturing}
                    onSetCapturing={setCapturing}
                    onApplySettings={applySettings}
                  />
                ) : null}

                {activeSettingsPane === "appearance" ? (
                  <InterfaceSection draft={draft} onApplySettings={applySettings} platform={currentSnapshot.platform} />
                ) : null}
              </div>
            </div>
          ) : null}

          {activeSection === "help" ? (
            <DebugSection snapshot={currentSnapshot} />
          ) : null}
        </div>
      </section>

      {activeDialog === "about" ? (
        <ShellDialog
          title="About Warble"
          description="Local-first dictation with a tray-first workflow and on-device history."
          size="medium"
          onClose={closeDialog}
        >
          <AboutSection />
        </ShellDialog>
      ) : null}
    </main>
  );
}

import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect } from "react";

import { sections, SHELL_ACTION_EVENT } from "./constants";
import { NoticeBanner, StatusChip } from "./components/common";
import { CheckIcon, HelpIcon, SectionIcon, SettingsIcon } from "./components/icons";
import { useControlApp } from "./hooks/useControlApp";
import { AboutSection } from "./sections/AboutSection";
import { CaptureSection } from "./sections/CaptureSection";
import { DebugSection } from "./sections/DebugSection";
import { HistorySection } from "./sections/HistorySection";
import { ModelsSection } from "./sections/ModelsSection";
import { SettingsSheet } from "./sections/SettingsSheet";
import { Sidebar } from "./sections/Sidebar";
import { ShellDialog } from "./components/ShellDialog";
import { VocabularySection } from "./sections/VocabularySection";
import type { ShellActionId, Snapshot } from "./types";

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
  const shellOpenSettingsDialog = control.ready ? control.openSettingsDialog : null;
  const shellOpenTroubleshootingDialog = control.ready
    ? control.openTroubleshootingDialog
    : null;
  const shellCloseDialog = control.ready ? control.closeDialog : null;
  const shellSetActiveSection = control.ready ? control.setActiveSection : null;
  const shellTranscribeFile = control.ready ? control.transcribeFile : null;

  useEffect(() => {
    if (
      !control.ready ||
      !shellCloseDialog ||
      !shellOpenAboutDialog ||
      !shellOpenSettingsDialog ||
      !shellOpenTroubleshootingDialog ||
      !shellSetActiveSection ||
      !shellTranscribeFile
    ) {
      return;
    }

    let unlisten: (() => void) | undefined;

    const handleShellAction = (action: ShellActionId) => {
      switch (action) {
        case "open-settings":
          shellOpenSettingsDialog();
          break;
        case "open-troubleshooting":
          shellOpenTroubleshootingDialog();
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
    shellOpenSettingsDialog,
    shellOpenTroubleshootingDialog,
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
    openSettingsDialog,
    openTroubleshootingDialog,
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
  } = control;

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
            <h1>{sections.find((section) => section.id === activeSection)?.label}</h1>
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
            <button
              type="button"
              className="secondary small icon-button"
              onClick={() => openSettingsDialog()}
            >
              <SettingsIcon className="small-icon" />
              <span>Settings</span>
            </button>
            <button
              type="button"
              className="secondary small icon-button"
              onClick={openTroubleshootingDialog}
            >
              <HelpIcon className="small-icon" />
              <span>Help</span>
            </button>
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
              buttonFeedback={buttonFeedback}
              onSetCleanupInput={setCleanupInput}
              onApplySettings={applySettings}
              onAddCleanupTerm={addCleanupTerm}
              onRemoveCleanupTerm={removeCleanupTerm}
              onRestoreCleanupDefaults={restoreCleanupDefaults}
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
        </div>
      </section>

      {activeDialog === "settings" ? (
        <SettingsSheet
          snapshot={currentSnapshot}
          draft={draft}
          activePane={activeSettingsPane}
          capturing={capturing}
          onClose={closeDialog}
          onSetActivePane={setActiveSettingsPane}
          onSetCapturing={setCapturing}
          onApplySettings={applySettings}
        />
      ) : null}

      {activeDialog === "troubleshooting" ? (
        <ShellDialog
          title="Troubleshooting"
          description="Capture and live preview diagnostics stay here so the main IA stays focused on everyday use."
          size="wide"
          onClose={closeDialog}
        >
          <DebugSection snapshot={currentSnapshot} />
        </ShellDialog>
      ) : null}

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

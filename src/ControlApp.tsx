import { sections } from "./constants";
import { NoticeBanner, StatusChip } from "./components/common";
import { CheckIcon, SectionIcon } from "./components/icons";
import { useControlApp } from "./hooks/useControlApp";
import { AboutSection } from "./sections/AboutSection";
import { CleanupSection } from "./sections/CleanupSection";
import { HistorySection } from "./sections/HistorySection";
import { InputsSection } from "./sections/InputsSection";
import { InterfaceSection } from "./sections/InterfaceSection";
import { KeybindingsSection } from "./sections/KeybindingsSection";
import { ModelsSection } from "./sections/ModelsSection";
import { OverviewSection } from "./sections/OverviewSection";
import { Sidebar } from "./sections/Sidebar";
import type { Snapshot } from "./types";

export function ControlApp({
  snapshot,
  setSnapshot,
}: {
  snapshot: Snapshot | null;
  setSnapshot: (snapshot: Snapshot | null) => void;
}) {
  const control = useControlApp({ snapshot, setSnapshot });

  if (!control.ready) {
    return <main className="loading-shell">Loading...</main>;
  }

  const {
    snapshot: currentSnapshot,
    activeSection,
    setActiveSection,
    sidebarCollapsed,
    setSidebarCollapsed,
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
    activeSource,
    sourceOptions,
    activeModel,
    batchModelRows,
    streamingModelRows,
    readyModelOptions,
    activeReadyModelId,
    installedStreamingModels,
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
          {activeSection === "overview" ? (
            <OverviewSection
              activeModel={activeModel}
              activeSource={activeSource}
              animationStyle={draft.overlayAnimationStyle}
              showRecordingTimer={draft.showRecordingTimer}
              showLiveTranscription={draft.showLiveTranscription}
              liveTranscriptWidth={draft.liveTranscriptWidth}
              liveTranscriptLines={draft.liveTranscriptLines}
              phase={currentSnapshot.phase}
              historyCount={currentSnapshot.history.length}
              overlayTitle={currentSnapshot.overlay.title}
              previewTitle={previewTitle}
              previewDetail={previewDetail}
              levels={currentSnapshot.overlay.levels}
              elapsedMs={currentSnapshot.overlay.elapsedMs}
              limitMs={currentSnapshot.overlay.limitMs}
              recentFileTranscript={recentFileTranscript}
              shortcutsActive={currentSnapshot.shortcutsActive}
              fileActionState={buttonFeedback["transcribe-file"]}
              onStartRecording={startRecording}
              onStopRecording={() => {
                void stopRecording();
              }}
              onTranscribeFile={() => {
                void transcribeFile();
              }}
              onCancel={() => {
                void cancelCurrentOperation();
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

          {activeSection === "keybindings" ? (
            <KeybindingsSection
              snapshot={currentSnapshot}
              draft={draft}
              capturing={capturing}
              onSetCapturing={setCapturing}
              onApplySettings={applySettings}
            />
          ) : null}

          {activeSection === "interface" ? (
            <InterfaceSection
              snapshot={currentSnapshot}
              draft={draft}
              onApplySettings={applySettings}
              installedStreamingModels={installedStreamingModels}
              previewDiagnostics={currentSnapshot.previewDiagnostics}
            />
          ) : null}

          {activeSection === "cleanup" ? (
            <CleanupSection
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

          {activeSection === "inputs" ? (
            <InputsSection
              snapshot={currentSnapshot}
              draft={draft}
              sourceOptions={sourceOptions}
              activeSource={activeSource}
              previewTitle={previewTitle}
              previewDetail={previewDetail}
              buttonFeedback={buttonFeedback}
              elapsedMs={currentSnapshot.overlay.elapsedMs}
              limitMs={currentSnapshot.overlay.limitMs}
              liveTranscriptWidth={draft.liveTranscriptWidth}
              liveTranscriptLines={draft.liveTranscriptLines}
              onApplySettings={applySettings}
              onRefreshDevices={refreshDevices}
              onStartRecording={startRecording}
              onStopRecording={() => {
                void stopRecording();
              }}
              onCancel={() => {
                void cancelCurrentOperation();
              }}
            />
          ) : null}

          {activeSection === "history" ? (
            <HistorySection
              snapshot={currentSnapshot}
              draft={draft}
              historyQuery={historyQuery}
              filteredHistory={filteredHistory}
              buttonFeedback={buttonFeedback}
              onSetHistoryQuery={setHistoryQuery}
              onApplySettings={applySettings}
              onOpenHistoryAudio={(item) => openHistoryAudio(item.audioPath)}
              onCopyHistory={copyHistory}
              onRemoveHistoryItem={removeHistoryItem}
              onClearHistory={clearHistory}
            />
          ) : null}

          {activeSection === "about" ? <AboutSection /> : null}
        </div>
      </section>
    </main>
  );
}

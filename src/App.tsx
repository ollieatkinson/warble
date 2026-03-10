import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useRef, useState } from "react";

import { SIDEBAR_COLLAPSED_KEY, isIndicatorWindow, sections } from "./constants";
import { IndicatorApp } from "./components/TranscriptionPill";
import { NoticeBanner, StatusChip } from "./components/common";
import { CheckIcon, SectionIcon } from "./components/icons";
import { fetchSnapshot, useSnapshotState } from "./hooks/useSnapshotState";
import {
  buildModelRows,
  describeHardwareFit,
  formatModelAudioLimit,
  formatModelSizeLabel,
  matchesModel,
} from "./lib/modelCatalog";
import {
  buildSettingsUpdate,
  formatBytes,
  formatInvokeError,
  formatSystemProfile,
  matchesHistory,
  normalizeEditableOverlayPosition,
} from "./lib/utils";
import { AboutSection } from "./sections/AboutSection";
import { CleanupSection } from "./sections/CleanupSection";
import { HistorySection } from "./sections/HistorySection";
import { InputsSection } from "./sections/InputsSection";
import { InterfaceSection } from "./sections/InterfaceSection";
import { KeybindingsSection } from "./sections/KeybindingsSection";
import { ModelsSection } from "./sections/ModelsSection";
import { OverviewSection } from "./sections/OverviewSection";
import { Sidebar } from "./sections/Sidebar";
import type {
  ButtonFeedbackState,
  FlashMessage,
  ModelFilter,
  SectionId,
  SettingsDraft,
  ShortcutFieldName,
  Snapshot,
} from "./types";

const MEDIA_FILE_EXTENSIONS = [
  "wav",
  "mp3",
  "m4a",
  "aac",
  "flac",
  "ogg",
  "oga",
  "mp4",
  "mov",
  "mkv",
  "webm",
  "avi",
  "aif",
  "aiff",
];

function ControlApp({
  snapshot,
  setSnapshot,
}: {
  snapshot: Snapshot | null;
  setSnapshot: (snapshot: Snapshot | null) => void;
}) {
  const [activeSection, setActiveSection] = useState<SectionId>("overview");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => {
    try {
      return window.localStorage.getItem(SIDEBAR_COLLAPSED_KEY) === "1";
    } catch {
      return false;
    }
  });
  const [message, setMessage] = useState<FlashMessage>(null);
  const [capturing, setCapturing] = useState<ShortcutFieldName | null>(null);
  const [historyQuery, setHistoryQuery] = useState("");
  const [modelQuery, setModelQuery] = useState("");
  const [modelFilter, setModelFilter] = useState<ModelFilter>("all");
  const [cleanupInput, setCleanupInput] = useState("");
  const [selectedModelId, setSelectedModelId] = useState<string | null>(null);
  const [buttonFeedback, setButtonFeedback] = useState<
    Record<string, ButtonFeedbackState>
  >({});
  const buttonFeedbackTimersRef = useRef<Record<string, number>>({});
  const [draft, setDraft] = useState<SettingsDraft>({
    holdShortcut: "",
    toggleShortcut: "",
    selectedSourceId: "",
    autoPaste: true,
    cleanupEnabled: true,
    audioRetentionPolicy: "one-day",
    overlayPosition: "bottom-center",
    overlayAnimationStyle: "spectrum",
    showRecordingTimer: false,
    showLiveTranscription: false,
  });
  const draftRef = useRef(draft);
  const previousSnapshotRef = useRef<Snapshot | null>(null);

  useEffect(() => {
    draftRef.current = draft;
  }, [draft]);

  useEffect(() => {
    try {
      window.localStorage.setItem(
        SIDEBAR_COLLAPSED_KEY,
        sidebarCollapsed ? "1" : "0",
      );
    } catch {
      // Ignore local preference persistence issues.
    }
  }, [sidebarCollapsed]);

  useEffect(() => {
    return () => {
      Object.values(buttonFeedbackTimersRef.current).forEach((timer) =>
        window.clearTimeout(timer),
      );
    };
  }, []);

  useEffect(() => {
    if (!snapshot) {
      return;
    }

    setDraft({
      holdShortcut: snapshot.settings.holdShortcut,
      toggleShortcut: snapshot.settings.toggleShortcut,
      selectedSourceId:
        snapshot.settings.selectedSourceId ?? snapshot.sources[0]?.id ?? "",
      autoPaste: snapshot.settings.autoPaste,
      cleanupEnabled: snapshot.settings.cleanupEnabled,
      audioRetentionPolicy: snapshot.settings.audioRetentionPolicy,
      overlayPosition: normalizeEditableOverlayPosition(
        snapshot.settings.overlayPosition,
      ),
      overlayAnimationStyle: snapshot.settings.overlayAnimationStyle,
      showRecordingTimer: snapshot.settings.showRecordingTimer,
      showLiveTranscription: snapshot.settings.showLiveTranscription,
    });
  }, [snapshot]);

  useEffect(() => {
    if (!snapshot) {
      return;
    }

    const previous = previousSnapshotRef.current;
    if (
      previous &&
      previous.phase === "transcribing" &&
      snapshot.phase === "idle" &&
      snapshot.history.length > previous.history.length &&
      snapshot.history[0]?.capture.sourceKind === "file"
    ) {
      setHistoryQuery("");
      setActiveSection("history");
    }

    previousSnapshotRef.current = snapshot;
  }, [snapshot]);

  async function refreshSnapshot() {
    const current = await fetchSnapshot();
    setSnapshot(current);
  }

  async function sendSettingsUpdate(update: Record<string, unknown>) {
    setMessage(null);

    try {
      await invoke("update_settings_command", { update });
    } catch (error) {
      await refreshSnapshot();
      throw error;
    }
  }

  async function dismissSnapshotError() {
    try {
      await invoke("clear_error_message_command");
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  function clearButtonFeedback(actionId: string) {
    const timer = buttonFeedbackTimersRef.current[actionId];
    if (timer) {
      window.clearTimeout(timer);
      delete buttonFeedbackTimersRef.current[actionId];
    }

    setButtonFeedback((current) => {
      if (!(actionId in current)) {
        return current;
      }

      const next = { ...current };
      delete next[actionId];
      return next;
    });
  }

  function setButtonFeedbackState(actionId: string, state: ButtonFeedbackState) {
    const timer = buttonFeedbackTimersRef.current[actionId];
    if (timer) {
      window.clearTimeout(timer);
      delete buttonFeedbackTimersRef.current[actionId];
    }

    setButtonFeedback((current) => ({
      ...current,
      [actionId]: state,
    }));
  }

  function finishButtonFeedback(actionId: string, holdMs = 1200) {
    setButtonFeedbackState(actionId, "done");
    buttonFeedbackTimersRef.current[actionId] = window.setTimeout(() => {
      clearButtonFeedback(actionId);
    }, holdMs);
  }

  async function applySettings(update: Partial<SettingsDraft>) {
    if (!snapshot) {
      return;
    }

    const nextDraft = { ...draftRef.current, ...update };
    setDraft(nextDraft);
    draftRef.current = nextDraft;

    try {
      await sendSettingsUpdate(buildSettingsUpdate(nextDraft));
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function refreshDevices() {
    setMessage(null);
    setButtonFeedbackState("refresh-inputs", "working");

    try {
      await invoke("refresh_devices");
      await refreshSnapshot();
      finishButtonFeedback("refresh-inputs");
    } catch (error) {
      clearButtonFeedback("refresh-inputs");
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function startRecording(mode: "hold" | "toggle") {
    try {
      setMessage(null);
      await invoke("start_manual_recording", { mode });
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function stopRecording() {
    try {
      setMessage(null);
      await invoke("stop_manual_recording");
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function cancelCurrentOperation() {
    try {
      setMessage(null);
      await invoke("cancel_current_operation_command");
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function transcribeFile() {
    const actionId = "transcribe-file";
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      const selected = await openDialog({
        directory: false,
        multiple: false,
        filters: [
          {
            name: "Audio or video",
            extensions: MEDIA_FILE_EXTENSIONS,
          },
        ],
      });

      if (!selected || Array.isArray(selected)) {
        clearButtonFeedback(actionId);
        return;
      }

      await invoke("transcribe_media_file_command", {
        path: selected,
      });
      finishButtonFeedback(actionId, 900);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function copyHistory(id: string, text: string) {
    const actionId = `copy:${id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await navigator.clipboard.writeText(text);
      finishButtonFeedback(actionId, 1000);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function openHistoryAudio(audioPath: string | null) {
    if (!audioPath) {
      return;
    }

    try {
      setMessage(null);
      await openPath(audioPath);
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function removeHistoryItem(id: string) {
    const actionId = `history-remove:${id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await invoke("remove_history_item", { id });
      finishButtonFeedback(actionId, 900);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  function selectModel(modelId: string) {
    setSelectedModelId(modelId);
  }

  async function activateModel(modelId: string) {
    if (!snapshot) {
      return;
    }

    const row = buildModelRows(snapshot).find((candidate) => candidate.id === modelId);
    if (!row || !row.selectable || row.active) {
      return;
    }

    setMessage(null);
    try {
      await sendSettingsUpdate({
        selectedModelId: row.id,
        selectedModelKind: row.modelKind,
        selectedModelPath: row.source === "built-in" ? null : row.path ?? null,
      });
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function linkCatalogModel(modelId: string) {
    if (!snapshot) {
      return;
    }

    const row = buildModelRows(snapshot).find((candidate) => candidate.id === modelId);
    if (!row?.supportsInstall) {
      return;
    }

    const actionId = `model-link:${row.id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
      });

      if (!selected || Array.isArray(selected)) {
        clearButtonFeedback(actionId);
        return;
      }

      await invoke("install_catalog_model", {
        modelId: row.id,
        modelKind: row.modelKind,
        path: selected,
      });
      finishButtonFeedback(actionId);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function downloadCatalogModel(modelId: string) {
    if (!snapshot) {
      return;
    }

    const row = buildModelRows(snapshot).find((candidate) => candidate.id === modelId);
    if (!row?.supportsDownload) {
      return;
    }

    const actionId = `model-download:${row.id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await invoke("download_catalog_model", {
        modelId: row.id,
      });
      finishButtonFeedback(actionId, 1500);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function removeCatalogModel(modelId: string) {
    if (!snapshot) {
      return;
    }

    const row = buildModelRows(snapshot).find((candidate) => candidate.id === modelId);
    if (!row?.managed) {
      return;
    }

    const actionId = `model-remove:${row.id}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await invoke("remove_catalog_model", {
        modelId: row.id,
      });
      finishButtonFeedback(actionId, 1200);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function openModelReference(url: string | undefined) {
    if (!url) {
      return;
    }

    try {
      await openUrl(url);
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function openModelArtifact(url: string | undefined) {
    if (!url) {
      return;
    }

    try {
      await openUrl(url);
    } catch (error) {
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function addCleanupTerm(term = cleanupInput) {
    const trimmed = term.trim();
    if (!trimmed) {
      setMessage({
        kind: "error",
        text: "Enter a filler word or phrase to remove.",
      });
      return;
    }

    setMessage(null);
    setButtonFeedbackState("cleanup-add", "working");

    try {
      await invoke("add_cleanup_term", { term: trimmed });
      setCleanupInput("");
      finishButtonFeedback("cleanup-add");
    } catch (error) {
      clearButtonFeedback("cleanup-add");
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function removeCleanupTerm(term: string) {
    const actionId = `cleanup-remove:${term}`;
    setMessage(null);
    setButtonFeedbackState(actionId, "working");

    try {
      await invoke("remove_cleanup_term", { term });
      finishButtonFeedback(actionId, 900);
    } catch (error) {
      clearButtonFeedback(actionId);
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  async function restoreCleanupDefaults() {
    setMessage(null);
    setButtonFeedbackState("cleanup-restore", "working");

    try {
      await invoke("restore_default_cleanup_terms");
      finishButtonFeedback("cleanup-restore");
    } catch (error) {
      clearButtonFeedback("cleanup-restore");
      setMessage({
        kind: "error",
        text: formatInvokeError(error),
      });
    }
  }

  const modelRows = snapshot ? buildModelRows(snapshot) : [];
  const activeModelId = snapshot?.settings.selectedModelId ?? "parakeet";
  const selectedRowExists = selectedModelId
    ? modelRows.some((row) => row.id === selectedModelId)
    : false;
  const resolvedSelectedModelId = selectedRowExists && selectedModelId
    ? selectedModelId
    : modelRows.some((row) => row.id === activeModelId)
      ? activeModelId
      : modelRows[0]?.id ?? "parakeet";
  const selectedModel =
    modelRows.find((row) => row.id === resolvedSelectedModelId) ?? modelRows[0];
  const activeModel =
    modelRows.find((row) => row.active) ??
    modelRows.find((row) => row.id === activeModelId) ??
    null;
  const selectedModelFit =
    snapshot && selectedModel
      ? describeHardwareFit(selectedModel, snapshot.systemProfile)
      : null;
  const readyModelOptions = modelRows
    .filter((row) => row.selectable)
    .map((row) => ({
      id: row.id,
      label: row.name,
      description: `${row.speechMode} · ${formatModelSizeLabel(row)}`,
    }));
  const activeReadyModelId =
    activeModel?.selectable && activeModel
      ? activeModel.id
      : readyModelOptions[0]?.id ?? "";
  const selectedModelMeta: Array<[string, string]> = selectedModel
    ? [
        ["Family", selectedModel.family],
        ["Speech mode", selectedModel.speechMode],
        ["Architecture", selectedModel.architecture],
        ["Parameters", selectedModel.footprint],
        ["Runtime", selectedModel.runtime],
        [
          "Acceleration",
          selectedModel.directmlCapable
            ? snapshot?.systemProfile.directmlAvailable
              ? "DirectML GPU ready on this PC"
              : "DirectML-capable model"
            : "CPU / ONNX",
        ],
        ["Audio limit", formatModelAudioLimit(selectedModel)],
        [
          "Download size",
          selectedModel.downloadSizeBytes
            ? formatBytes(selectedModel.downloadSizeBytes)
            : "Included / n.a.",
        ],
        ["Size on disk", formatBytes(selectedModel.diskSizeBytes)],
        [
          "Unlocks",
          selectedModel.unlockedFeatures?.length
            ? selectedModel.unlockedFeatures.join(" · ")
            : "Default dictation engine",
        ],
        ["Speed", selectedModel.speed],
        ["Quality", selectedModel.quality],
        ["License", selectedModel.license],
        ["Best for", selectedModel.bestFor],
        ["Capabilities", selectedModel.capabilities.join(" · ")],
        ["This PC", selectedModelFit?.label ?? "Unknown"],
        [
          "Hardware",
          snapshot ? formatSystemProfile(snapshot.systemProfile) : "Unknown",
        ],
      ]
    : [];

  useEffect(() => {
    if (!selectedRowExists && modelRows[0]) {
      setSelectedModelId(activeModelId || modelRows[0].id);
    }
  }, [activeModelId, modelRows, selectedRowExists]);

  useEffect(() => {
    function handleCancelEscape(event: globalThis.KeyboardEvent) {
      if (event.key !== "Escape") {
        return;
      }

      if (
        !snapshot ||
        (snapshot.phase !== "recording" && snapshot.phase !== "transcribing")
      ) {
        return;
      }

      event.preventDefault();
      void cancelCurrentOperation();
    }

    document.addEventListener("keydown", handleCancelEscape);
    return () => {
      document.removeEventListener("keydown", handleCancelEscape);
    };
  }, [snapshot]);

  async function chooseDefaultModel(modelId: string) {
    setSelectedModelId(modelId);
    await activateModel(modelId);
  }

  if (!snapshot) {
    return <main className="loading-shell">Loading...</main>;
  }

  const activeSource =
    snapshot.sources.find((source) => source.id === draft.selectedSourceId) ??
    snapshot.sources.find((source) => source.isDefault) ??
    snapshot.sources[0] ??
    null;
  const sourceOptions = snapshot.sources.map((source) => ({
    id: source.id,
    label: source.name,
    description: `${source.sampleRate} Hz · ${source.channels} ch${
      source.isDefault ? " · default" : ""
    }`,
  }));
  const recentTranscript = snapshot.history[0] ?? null;
  const filteredHistory = snapshot.history.filter((item) =>
    matchesHistory(item, historyQuery),
  );
  const previewTitle =
    snapshot.phase === "recording"
      ? "Listening"
      : snapshot.phase === "transcribing"
        ? "Transcribing"
        : "Ready";
  const previewDetail = snapshot.overlay.detail || previewTitle;
  const filteredModels = modelRows.filter((row) =>
    matchesModel(row, modelQuery, modelFilter),
  );
  const cleanupTerms = snapshot.settings.cleanupTerms;
  const installedStreamingModels = modelRows.filter(
    (row) =>
      row.state === "ready" &&
      row.tags.includes("streaming") &&
      (row.unlockedFeatures?.length ?? 0) > 0,
  );

  return (
    <main
      className={`workspace-shell ${sidebarCollapsed ? "workspace-shell-collapsed" : ""}`}
    >
      <Sidebar
        activeSection={activeSection}
        collapsed={sidebarCollapsed}
        phase={snapshot.phase}
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
                snapshot.modelStatus === "ready"
                  ? `${activeModel?.family ?? "Model"} ready`
                  : `${activeModel?.family ?? "Model"} missing`
              }
              tone={snapshot.modelStatus === "ready" ? "success" : "warning"}
              icon={<CheckIcon className="chip-icon-svg" />}
            />
            <StatusChip
              label={snapshot.shortcutsActive ? "Keys active" : "Keys off"}
              tone={snapshot.shortcutsActive ? "accent" : "warning"}
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
        {!message && snapshot.errorMessage ? (
          <NoticeBanner
            kind="error"
            text={snapshot.errorMessage}
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
              phase={snapshot.phase}
              historyCount={snapshot.history.length}
              overlayTitle={snapshot.overlay.title}
              previewTitle={previewTitle}
              previewDetail={previewDetail}
              levels={snapshot.overlay.levels}
              elapsedMs={snapshot.overlay.elapsedMs}
              limitMs={snapshot.overlay.limitMs}
              recentTranscript={Boolean(recentTranscript)}
              shortcutsActive={snapshot.shortcutsActive}
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
              snapshot={snapshot}
              modelRows={modelRows}
              filteredModels={filteredModels}
              modelQuery={modelQuery}
              modelFilter={modelFilter}
              selectedModel={selectedModel}
              selectedModelFit={selectedModelFit}
              selectedModelMeta={selectedModelMeta}
              resolvedSelectedModelId={resolvedSelectedModelId}
              activeModel={activeModel}
              activeReadyModelId={activeReadyModelId}
              readyModelOptions={readyModelOptions}
              buttonFeedback={buttonFeedback}
              onSetModelQuery={setModelQuery}
              onSetModelFilter={setModelFilter}
              onSelectModel={(row) => selectModel(row.id)}
              onChooseDefaultModel={chooseDefaultModel}
              onActivateModel={(row) => activateModel(row.id)}
              onLinkCatalogModel={(row) => linkCatalogModel(row.id)}
              onDownloadCatalogModel={(row) => downloadCatalogModel(row.id)}
              onRemoveCatalogModel={(row) => removeCatalogModel(row.id)}
              onOpenModelReference={(row) => openModelReference(row.hfUrl)}
              onOpenModelArtifact={(row) => openModelArtifact(row.artifactUrl)}
            />
          ) : null}

          {activeSection === "keybindings" ? (
            <KeybindingsSection
              snapshot={snapshot}
              draft={draft}
              capturing={capturing}
              onSetCapturing={setCapturing}
              onApplySettings={applySettings}
            />
          ) : null}

          {activeSection === "interface" ? (
            <InterfaceSection
              draft={draft}
              onApplySettings={applySettings}
              installedStreamingModels={installedStreamingModels.map((row) => ({
                id: row.id,
                name: row.name,
                unlockedFeatures: row.unlockedFeatures ?? [],
                directmlCapable: Boolean(row.directmlCapable),
              }))}
              directmlAvailable={snapshot.systemProfile.directmlAvailable}
              previewDiagnostics={snapshot.previewDiagnostics}
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
              snapshot={snapshot}
              draft={draft}
              sourceOptions={sourceOptions}
              activeSource={activeSource}
              previewTitle={previewTitle}
              previewDetail={previewDetail}
              buttonFeedback={buttonFeedback}
              elapsedMs={snapshot.overlay.elapsedMs}
              limitMs={snapshot.overlay.limitMs}
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
              snapshot={snapshot}
              draft={draft}
              historyQuery={historyQuery}
              filteredHistory={filteredHistory}
              buttonFeedback={buttonFeedback}
              onSetHistoryQuery={setHistoryQuery}
              onApplySettings={applySettings}
              onOpenHistoryAudio={(item) => openHistoryAudio(item.audioPath)}
              onCopyHistory={copyHistory}
              onRemoveHistoryItem={removeHistoryItem}
            />
          ) : null}

          {activeSection === "about" ? <AboutSection /> : null}
        </div>
      </section>
    </main>
  );
}

export default function App() {
  const [snapshot, setSnapshot] = useSnapshotState();

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

  return isIndicatorWindow ? (
    <IndicatorApp snapshot={snapshot} />
  ) : (
    <ControlApp snapshot={snapshot} setSnapshot={setSnapshot} />
  );
}

import { invoke } from "@tauri-apps/api/core";

import type { DebugLogs, RecordingMode, Snapshot } from "../types";

export function getSnapshot() {
  return invoke<Snapshot>("get_snapshot");
}

export function refreshDevices() {
  return invoke<void>("refresh_devices");
}

export function primeMicrophoneAccess() {
  return invoke<void>("prime_microphone_access");
}

export function primeAutoPasteAccess() {
  return invoke<void>("prime_auto_paste_access");
}

export function getDebugLogs() {
  return invoke<DebugLogs>("get_debug_logs_command");
}

export function updateSettings(update: Record<string, unknown>) {
  return invoke<void>("update_settings_command", { update });
}

export function clearErrorMessage() {
  return invoke<void>("clear_error_message_command");
}

export function startManualRecording(mode: RecordingMode) {
  return invoke<void>("start_manual_recording", { mode });
}

export function stopManualRecording() {
  return invoke<void>("stop_manual_recording");
}

export function cancelCurrentOperation() {
  return invoke<void>("cancel_current_operation_command");
}

export function transcribeMediaFile(path: string) {
  return invoke<void>("transcribe_media_file_command", { path });
}

export function removeHistoryItem(id: string) {
  return invoke<void>("remove_history_item", { id });
}

export function clearHistory() {
  return invoke<void>("clear_history");
}

export function pasteLastTranscript() {
  return invoke<void>("paste_last_transcript_command");
}

export function downloadCatalogModel(modelId: string) {
  return invoke<void>("download_catalog_model", { modelId });
}

export function removeCatalogModel(modelId: string) {
  return invoke<void>("remove_catalog_model", { modelId });
}

export function addCleanupTerm(term: string) {
  return invoke<void>("add_cleanup_term", { term });
}

export function removeCleanupTerm(term: string) {
  return invoke<void>("remove_cleanup_term", { term });
}

export function restoreDefaultCleanupTerms() {
  return invoke<void>("restore_default_cleanup_terms");
}

export function addReplacementRule(variants: string[], replacement: string) {
  return invoke<void>("add_replacement_rule", { variants, replacement });
}

export function removeReplacementRule(id: string) {
  return invoke<void>("remove_replacement_rule", { id });
}

export function reportIndicatorLayout(width: number, height: number) {
  return invoke<void>("report_indicator_layout_command", { width, height });
}

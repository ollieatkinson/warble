use reqwest::blocking::Client;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::AppHandle;

use crate::constants::{PARAKEET_CTC_AUDIO_LIMIT_MS, PARAKEET_TDT_AUDIO_LIMIT_MS};
use crate::inference;
use crate::model_catalog::{catalog_download_spec, CatalogDownloadFile, CatalogDownloadSpec};
use crate::parakeet;
use crate::platform;
use crate::state::{
    LivePreviewModelPreference, ModelDownloadProgress, ModelPathInspection, ModelStatus, Settings,
    SharedState, TranscriptionModelKind,
};
use crate::storage::{
    emit_snapshot, is_managed_model_path, managed_model_dir_for_id, managed_models_dir,
    model_root_dir, path_size_bytes, persist_and_emit_settings_change,
};

pub(crate) fn parakeet_status_for_path(path: &Path) -> ModelStatus {
    if parakeet::model_ready_in_dir(path) || parakeet::model_ready_at(path) {
        ModelStatus::Ready
    } else {
        ModelStatus::Missing
    }
}

pub(crate) fn parakeet_ctc_status_for_path(path: &Path) -> ModelStatus {
    if parakeet::ctc_model_ready_in_dir(path) || parakeet::ctc_model_ready_at(path) {
        ModelStatus::Ready
    } else {
        ModelStatus::Missing
    }
}

pub(crate) fn path_matches_selected_model_kind(
    path: &Path,
    model_kind: TranscriptionModelKind,
) -> bool {
    match model_kind {
        TranscriptionModelKind::Parakeet => {
            parakeet::model_ready_in_dir(path) || parakeet::model_ready_at(path)
        }
        TranscriptionModelKind::ParakeetCtc => {
            parakeet::ctc_model_ready_in_dir(path) || parakeet::ctc_model_ready_at(path)
        }
    }
}

pub(crate) fn resolved_selected_model_path(settings: &Settings) -> Option<String> {
    settings
        .selected_model_path
        .clone()
        .filter(|path| {
            path_matches_selected_model_kind(Path::new(path), settings.selected_model_kind)
        })
        .or_else(|| {
            settings
                .installed_model_paths
                .get(&settings.selected_model_id)
                .filter(|path| {
                    path_matches_selected_model_kind(
                        Path::new(path.as_str()),
                        settings.selected_model_kind,
                    )
                })
                .cloned()
        })
}

pub(crate) fn installed_model_sizes(app: &AppHandle, settings: &Settings) -> BTreeMap<String, u64> {
    let mut sizes = BTreeMap::new();

    if let Ok(root) = model_root_dir(app) {
        if parakeet::model_ready_at(&root) {
            sizes.insert("parakeet".to_string(), path_size_bytes(&root));
        }
    }

    for (model_id, path) in &settings.installed_model_paths {
        let size = path_size_bytes(Path::new(path));
        if size > 0 {
            sizes.insert(model_id.clone(), size);
        }
    }

    if let Some(selected_path) = resolved_selected_model_path(settings) {
        let selected_size = path_size_bytes(Path::new(&selected_path));
        if selected_size > 0 {
            sizes
                .entry(settings.selected_model_id.clone())
                .or_insert(selected_size);
        }
    }

    sizes
}

pub(crate) fn model_kind_for_model_id(model_id: &str) -> TranscriptionModelKind {
    if model_id == "parakeet" {
        TranscriptionModelKind::Parakeet
    } else if model_id == "parakeet-ctc" {
        TranscriptionModelKind::ParakeetCtc
    } else {
        catalog_download_spec(model_id)
            .map(|spec| spec.model_kind)
            .unwrap_or(TranscriptionModelKind::Parakeet)
    }
}

pub(crate) fn choose_fallback_model_selection(app: &AppHandle, settings: &mut Settings) {
    if built_in_parakeet_status(app) == ModelStatus::Ready {
        settings.selected_model_id = "parakeet".to_string();
        settings.selected_model_kind = TranscriptionModelKind::Parakeet;
        settings.selected_model_path = None;
        return;
    }

    let preferred_ids = ["parakeet", "parakeet-ctc"];
    for model_id in preferred_ids {
        if let Some(path) = settings.installed_model_paths.get(model_id) {
            settings.selected_model_id = model_id.to_string();
            settings.selected_model_kind = model_kind_for_model_id(model_id);
            settings.selected_model_path = Some(path.clone());
            return;
        }
    }

    settings.selected_model_id = "parakeet".to_string();
    settings.selected_model_kind = TranscriptionModelKind::Parakeet;
    settings.selected_model_path = None;
}

pub(crate) fn selected_model_cache_key(settings: &Settings) -> String {
    let provider = inference::selected_provider_for_model(
        platform::current_platform(),
        settings,
        &inference::supported_acceleration_providers(),
        settings.selected_model_id.as_str(),
    )
    .to_string()
    .to_ascii_lowercase();

    match settings.selected_model_kind {
        TranscriptionModelKind::Parakeet => format!(
            "parakeet:{}:{provider}",
            resolved_selected_model_path(settings)
                .as_deref()
                .unwrap_or("builtin")
                .to_ascii_lowercase()
        ),
        TranscriptionModelKind::ParakeetCtc => format!(
            "parakeet-ctc:{}:{provider}",
            resolved_selected_model_path(settings)
                .as_deref()
                .unwrap_or("missing")
                .to_ascii_lowercase()
        ),
    }
}

pub(crate) fn built_in_parakeet_status(app: &AppHandle) -> ModelStatus {
    model_root_dir(app)
        .ok()
        .filter(|root| parakeet::model_ready_at(root))
        .map(|_| ModelStatus::Ready)
        .unwrap_or(ModelStatus::Missing)
}

pub(crate) fn current_model_status(app: &AppHandle, settings: &Settings) -> ModelStatus {
    match settings.selected_model_kind {
        TranscriptionModelKind::Parakeet => {
            if let Some(path) = resolved_selected_model_path(settings) {
                parakeet_status_for_path(Path::new(&path))
            } else {
                built_in_parakeet_status(app)
            }
        }
        TranscriptionModelKind::ParakeetCtc => resolved_selected_model_path(settings)
            .map(|path| parakeet_ctc_status_for_path(Path::new(&path)))
            .unwrap_or(ModelStatus::Missing),
    }
}

pub(crate) fn selected_model_audio_limit_ms(settings: &Settings) -> Option<u64> {
    match settings.selected_model_id.as_str() {
        "parakeet" => Some(PARAKEET_TDT_AUDIO_LIMIT_MS),
        "parakeet-ctc" => Some(PARAKEET_CTC_AUDIO_LIMIT_MS),
        _ => None,
    }
}

pub(crate) fn selected_model_display_name(settings: &Settings) -> String {
    match settings.selected_model_id.as_str() {
        "parakeet" => "Parakeet TDT".to_string(),
        "parakeet-ctc" => "Parakeet CTC".to_string(),
        model_id => catalog_download_spec(model_id)
            .map(|spec| spec.display_name.to_string())
            .unwrap_or_else(|| model_id.to_string()),
    }
}

pub(crate) fn inspect_model_candidate(path: &str) -> Result<ModelPathInspection, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Enter a local model file or folder path.".to_string());
    }

    let candidate = PathBuf::from(trimmed);
    let canonical = candidate
        .canonicalize()
        .map_err(|_| format!("Path not found: {trimmed}"))?;

    if canonical.is_file() {
        return Err("Choose a Parakeet model folder, not an individual file.".to_string());
    }

    if !canonical.is_dir() {
        return Err("That path is not a file or folder.".to_string());
    }

    let detected = parakeet::detect_model_dir(&canonical);
    let resolved_dir = detected
        .as_ref()
        .map(|(_, path)| path.clone())
        .unwrap_or_else(|| canonical.clone());
    let name = resolved_dir
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Custom model")
        .to_string();
    let ready = detected.is_some();
    let model_kind = detected
        .map(|(family, _)| match family {
            parakeet::TranscriptionFamily::Tdt => TranscriptionModelKind::Parakeet,
            parakeet::TranscriptionFamily::Ctc => TranscriptionModelKind::ParakeetCtc,
        })
        .unwrap_or(TranscriptionModelKind::Parakeet);

    Ok(ModelPathInspection {
        name,
        path: canonical.display().to_string(),
        model_kind,
        compatible: ready,
        ready,
    })
}

pub(crate) fn activate_catalog_model(
    app: &AppHandle,
    shared: &SharedState,
    model_id: String,
    model_kind: TranscriptionModelKind,
    inspection: ModelPathInspection,
) -> Result<(), String> {
    {
        let mut core = shared.lock();
        core.settings
            .installed_model_paths
            .insert(model_id.clone(), inspection.path.clone());
        core.settings.selected_model_id = model_id;
        core.settings.selected_model_kind = model_kind;
        core.settings.selected_model_path = Some(inspection.path.clone());
        core.status_message = format!("Using {}", inspection.name);
        core.error_message = None;
    }

    persist_and_emit_settings_change(app, shared)
}

pub(crate) fn download_catalog_model(
    app: &AppHandle,
    shared: &SharedState,
    model_id: String,
) -> Result<(), String> {
    let spec = catalog_download_spec(&model_id)
        .ok_or_else(|| format!("No managed download is configured for {model_id}"))?;
    {
        let core = shared.lock();
        if core.model_downloads.contains_key(&model_id) {
            return Err(format!("{} is already downloading.", spec.display_name));
        }
    }

    update_download_progress(
        app,
        shared,
        spec.model_id,
        spec.display_name,
        "Preparing download",
        0,
        None,
        Instant::now(),
    );

    let app = app.clone();
    let shared = shared.clone();
    std::thread::spawn(move || {
        if let Err(error) = run_catalog_model_download(&app, &shared, spec) {
            mark_download_failed(&app, &shared, spec.model_id, error);
        }
    });

    Ok(())
}

fn run_catalog_model_download(
    app: &AppHandle,
    shared: &SharedState,
    spec: &'static CatalogDownloadSpec,
) -> Result<(), String> {
    let model_dir = managed_models_dir(app)
        .map_err(|error| error.to_string())?
        .join(spec.model_id);
    fs::create_dir_all(&model_dir).map_err(|error| error.to_string())?;
    let download_root = model_dir.join(spec.model_dir_name);
    fs::create_dir_all(&download_root).map_err(|error| error.to_string())?;

    let client = Client::builder()
        .build()
        .map_err(|error| format!("Couldn't prepare download client: {error}"))?;

    let total_bytes = resolve_total_download_bytes(&client, &download_root, &spec.files);
    let started_at = Instant::now();
    let mut downloaded_bytes = existing_downloaded_bytes(&download_root, &spec.files);

    update_download_progress(
        app,
        shared,
        spec.model_id,
        spec.display_name,
        "Preparing download",
        downloaded_bytes,
        total_bytes,
        started_at,
    );

    for file in spec.files {
        let destination = download_root.join(file.file_name);
        let partial = destination.with_extension("part");
        let _ = fs::remove_file(&partial);

        if destination.exists() {
            continue;
        }

        let mut response = client
            .get(file.download_url)
            .send()
            .map_err(|error| format!("Couldn't download model: {error}"))?;
        if !response.status().is_success() {
            return Err(format!("Download failed with status {}", response.status()));
        }

        let mut output =
            fs::File::create(&partial).map_err(|error| format!("Couldn't create file: {error}"))?;
        let mut buffer = [0u8; 64 * 1024];
        let mut last_emit_at = Instant::now();
        let mut last_emit_bytes = downloaded_bytes;

        update_download_progress(
            app,
            shared,
            spec.model_id,
            spec.display_name,
            file.file_name,
            downloaded_bytes,
            total_bytes,
            started_at,
        );

        loop {
            let read = response
                .read(&mut buffer)
                .map_err(|error| format!("Download interrupted: {error}"))?;
            if read == 0 {
                break;
            }
            output
                .write_all(&buffer[..read])
                .map_err(|error| format!("Couldn't write model file: {error}"))?;
            downloaded_bytes += read as u64;

            if downloaded_bytes.saturating_sub(last_emit_bytes) >= 1_048_576
                || last_emit_at.elapsed() >= Duration::from_millis(220)
            {
                update_download_progress(
                    app,
                    shared,
                    spec.model_id,
                    spec.display_name,
                    file.file_name,
                    downloaded_bytes,
                    total_bytes,
                    started_at,
                );
                last_emit_bytes = downloaded_bytes;
                last_emit_at = Instant::now();
            }
        }
        output
            .flush()
            .map_err(|error| format!("Couldn't finalize model file: {error}"))?;
        fs::rename(&partial, &destination)
            .map_err(|error| format!("Couldn't move downloaded model into place: {error}"))?;

        update_download_progress(
            app,
            shared,
            spec.model_id,
            spec.display_name,
            file.file_name,
            downloaded_bytes,
            total_bytes,
            started_at,
        );
    }

    clear_download_progress(app, shared, spec.model_id);

    if spec.activates_as_default {
        let inspection = inspect_model_candidate(&model_dir.display().to_string())?;
        activate_catalog_model(
            app,
            shared,
            spec.model_id.to_string(),
            spec.model_kind,
            inspection,
        )
    } else {
        {
            let mut core = shared.lock();
            core.settings.installed_model_paths.insert(
                spec.model_id.to_string(),
                download_root.display().to_string(),
            );
            core.status_message = format!(
                "Installed {}. Streaming features are now available.",
                spec.display_name
            );
            core.error_message = None;
        }

        persist_and_emit_settings_change(app, shared)
    }
}

fn mark_download_failed(app: &AppHandle, shared: &SharedState, model_id: &str, error: String) {
    {
        let mut core = shared.lock();
        core.model_downloads.remove(model_id);
        core.status_message = "Model download failed".to_string();
        core.error_message = Some(error);
    }
    emit_snapshot(app, shared);
}

fn existing_downloaded_bytes(download_root: &Path, files: &[CatalogDownloadFile]) -> u64 {
    files
        .iter()
        .map(|file| {
            fs::metadata(download_root.join(file.file_name))
                .map(|metadata| metadata.len())
                .unwrap_or(0)
        })
        .sum()
}

fn resolve_total_download_bytes(
    client: &Client,
    download_root: &Path,
    files: &[CatalogDownloadFile],
) -> Option<u64> {
    let mut total = 0u64;

    for file in files {
        let destination = download_root.join(file.file_name);
        if let Ok(metadata) = fs::metadata(&destination) {
            total = total.saturating_add(metadata.len());
            continue;
        }

        let response = client.head(file.download_url).send().ok()?;
        if !response.status().is_success() {
            return None;
        }
        let content_length = response.content_length()?;
        total = total.saturating_add(content_length);
    }

    Some(total)
}

fn update_download_progress(
    app: &AppHandle,
    shared: &SharedState,
    model_id: &str,
    display_name: &str,
    file_name: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    started_at: Instant,
) {
    let elapsed_seconds = started_at.elapsed().as_secs_f64().max(0.001);
    let bytes_per_second = if downloaded_bytes == 0 {
        None
    } else {
        Some((downloaded_bytes as f64 / elapsed_seconds).round() as u64)
    };
    let seconds_remaining = match (total_bytes, bytes_per_second) {
        (Some(total), Some(speed)) if speed > 0 && total > downloaded_bytes => {
            Some(((total - downloaded_bytes) as f64 / speed as f64).ceil() as u64)
        }
        (Some(total), _) if total <= downloaded_bytes => Some(0),
        _ => None,
    };

    {
        let mut core = shared.lock();
        core.model_downloads.insert(
            model_id.to_string(),
            ModelDownloadProgress {
                display_name: display_name.to_string(),
                file_name: file_name.to_string(),
                downloaded_bytes,
                total_bytes,
                bytes_per_second,
                seconds_remaining,
            },
        );
        core.status_message = format!("Downloading {}…", display_name);
        core.error_message = None;
    }
    emit_snapshot(app, shared);
}

fn clear_download_progress(app: &AppHandle, shared: &SharedState, model_id: &str) {
    {
        let mut core = shared.lock();
        core.model_downloads.remove(model_id);
    }
    emit_snapshot(app, shared);
}

pub(crate) fn remove_catalog_model(
    app: &AppHandle,
    shared: &SharedState,
    model_id: String,
) -> Result<(), String> {
    let display_name = catalog_download_spec(&model_id)
        .map(|spec| spec.display_name)
        .unwrap_or(&model_id)
        .to_string();
    let stored_path = {
        let core = shared.lock();
        core.settings
            .installed_model_paths
            .get(&model_id)
            .cloned()
            .ok_or_else(|| "That model is not installed locally.".to_string())?
    };

    let stored_path_buf = PathBuf::from(&stored_path);
    if !is_managed_model_path(app, &stored_path_buf) {
        return Err("Only models downloaded inside Warble can be removed here.".to_string());
    }

    let model_root = managed_model_dir_for_id(app, &model_id).map_err(|error| error.to_string())?;
    if model_root.exists() {
        fs::remove_dir_all(&model_root)
            .map_err(|error| format!("Couldn't remove downloaded model: {error}"))?;
    } else if stored_path_buf.exists() {
        if stored_path_buf.is_dir() {
            fs::remove_dir_all(&stored_path_buf)
                .map_err(|error| format!("Couldn't remove downloaded model: {error}"))?;
        } else {
            fs::remove_file(&stored_path_buf)
                .map_err(|error| format!("Couldn't remove downloaded model: {error}"))?;
        }
    }

    {
        let mut core = shared.lock();
        core.settings.installed_model_paths.remove(&model_id);

        let selected_path_matches = core
            .settings
            .selected_model_path
            .as_ref()
            .map(|selected| {
                let selected_path = PathBuf::from(selected);
                selected_path == stored_path_buf || selected_path.starts_with(&model_root)
            })
            .unwrap_or(false);

        if core.settings.selected_model_id == model_id || selected_path_matches {
            choose_fallback_model_selection(app, &mut core.settings);
        }
        if matches!(
            (&core.settings.live_preview_model, model_id.as_str()),
            (
                LivePreviewModelPreference::NemotronStreaming,
                "nemotron-streaming"
            ) | (LivePreviewModelPreference::ParakeetEou, "parakeet-eou")
        ) {
            core.settings.live_preview_model = LivePreviewModelPreference::Auto;
        }

        core.status_message = format!("Removed {display_name}");
        core.error_message = None;
    }

    persist_and_emit_settings_change(app, shared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{PARAKEET_CTC_AUDIO_LIMIT_MS, PARAKEET_TDT_AUDIO_LIMIT_MS};

    #[test]
    fn model_kind_for_parakeet_id() {
        assert_eq!(
            model_kind_for_model_id("parakeet"),
            TranscriptionModelKind::Parakeet
        );
    }

    #[test]
    fn model_kind_for_parakeet_ctc_id() {
        assert_eq!(
            model_kind_for_model_id("parakeet-ctc"),
            TranscriptionModelKind::ParakeetCtc
        );
    }

    #[test]
    fn model_kind_for_unknown_defaults_to_parakeet() {
        let kind = model_kind_for_model_id("unknown-model");
        assert_eq!(kind, TranscriptionModelKind::Parakeet);
    }

    #[test]
    fn selected_model_cache_key_parakeet_builtin() {
        let settings = Settings::default();
        let key = selected_model_cache_key(&settings);
        let provider = inference::selected_provider_for_model(
            platform::current_platform(),
            &settings,
            &inference::supported_acceleration_providers(),
            settings.selected_model_id.as_str(),
        )
        .to_string()
        .to_ascii_lowercase();
        assert_eq!(key, format!("parakeet:builtin:{provider}"));
    }

    #[test]
    fn selected_model_cache_key_with_custom_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("vocab.txt"), "").unwrap();
        std::fs::write(dir.path().join("encoder-model.onnx"), "").unwrap();
        std::fs::write(dir.path().join("decoder_joint-model.onnx"), "").unwrap();
        let path_str = dir.path().display().to_string();

        let mut settings = Settings::default();
        settings.selected_model_path = Some(path_str.clone());
        let key = selected_model_cache_key(&settings);
        let provider = inference::selected_provider_for_model(
            platform::current_platform(),
            &settings,
            &inference::supported_acceleration_providers(),
            settings.selected_model_id.as_str(),
        )
        .to_string()
        .to_ascii_lowercase();
        assert!(
            key.starts_with("parakeet:"),
            "key should start with parakeet: got {key}"
        );
        assert!(
            !key.starts_with(&format!("parakeet:builtin:{provider}")),
            "key should not be builtin when path is set: {key}"
        );
        assert!(key.ends_with(&format!(":{provider}")));
    }

    #[test]
    fn selected_model_cache_key_ctc_without_path() {
        let mut settings = Settings::default();
        settings.selected_model_id = "parakeet-ctc".to_string();
        settings.selected_model_kind = TranscriptionModelKind::ParakeetCtc;
        let key = selected_model_cache_key(&settings);
        let provider = inference::selected_provider_for_model(
            platform::current_platform(),
            &settings,
            &inference::supported_acceleration_providers(),
            settings.selected_model_id.as_str(),
        )
        .to_string()
        .to_ascii_lowercase();
        assert_eq!(key, format!("parakeet-ctc:missing:{provider}"));
    }

    #[test]
    fn audio_limit_parakeet_tdt() {
        let settings = Settings::default();
        let limit = selected_model_audio_limit_ms(&settings);
        assert_eq!(limit, Some(PARAKEET_TDT_AUDIO_LIMIT_MS));
    }

    #[test]
    fn audio_limit_parakeet_ctc() {
        let mut settings = Settings::default();
        settings.selected_model_id = "parakeet-ctc".to_string();
        let limit = selected_model_audio_limit_ms(&settings);
        assert_eq!(limit, Some(PARAKEET_CTC_AUDIO_LIMIT_MS));
    }

    #[test]
    fn audio_limit_unknown_model() {
        let mut settings = Settings::default();
        settings.selected_model_id = "unknown".to_string();
        let limit = selected_model_audio_limit_ms(&settings);
        assert_eq!(limit, None);
    }

    #[test]
    fn display_name_parakeet() {
        let settings = Settings::default();
        assert_eq!(selected_model_display_name(&settings), "Parakeet TDT");
    }

    #[test]
    fn display_name_parakeet_ctc() {
        let mut settings = Settings::default();
        settings.selected_model_id = "parakeet-ctc".to_string();
        assert_eq!(selected_model_display_name(&settings), "Parakeet CTC");
    }

    #[test]
    fn display_name_unknown_falls_back_to_id() {
        let mut settings = Settings::default();
        settings.selected_model_id = "my-custom-model".to_string();
        assert_eq!(selected_model_display_name(&settings), "my-custom-model");
    }

    #[test]
    fn parakeet_status_for_path_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(parakeet_status_for_path(dir.path()), ModelStatus::Missing);
    }

    #[test]
    fn parakeet_status_for_path_ready() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("vocab.txt"), "").unwrap();
        std::fs::write(dir.path().join("encoder-model.onnx"), "").unwrap();
        std::fs::write(dir.path().join("decoder_joint-model.onnx"), "").unwrap();
        assert_eq!(parakeet_status_for_path(dir.path()), ModelStatus::Ready);
    }
}

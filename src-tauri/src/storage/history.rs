use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::fs;
use std::path::Path;
use tauri::AppHandle;

use super::paths::recordings_dir;
use crate::state::{AudioRetentionPolicy, HistoryItem, SharedState};

pub(crate) fn audio_retention_duration(policy: &AudioRetentionPolicy) -> ChronoDuration {
    match policy {
        AudioRetentionPolicy::OneDay => ChronoDuration::days(1),
        AudioRetentionPolicy::SevenDays => ChronoDuration::days(7),
        AudioRetentionPolicy::ThirtyDays => ChronoDuration::days(30),
    }
}

pub(crate) fn history_item_created_at(item: &HistoryItem) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&item.created_at)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

pub(crate) fn remove_history_audio_file(item: &HistoryItem) {
    let Some(path) = item.audio_path.as_ref() else {
        return;
    };

    let _ = fs::remove_file(path);
}

pub(crate) fn write_recording_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("failed to create wav file at {}", path.display()))?;

    for sample in samples {
        let scaled = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16;
        writer.write_sample(scaled)?;
    }

    writer.finalize()?;
    Ok(())
}

pub(crate) fn save_history_audio(
    app: &AppHandle,
    item_id: &str,
    samples: &[f32],
    sample_rate: u32,
) -> Result<String> {
    let path = recordings_dir(app)?.join(format!("{item_id}.wav"));
    write_recording_wav(&path, samples, sample_rate)?;
    Ok(path.display().to_string())
}

pub(crate) fn prune_history_audio(app: &AppHandle, shared: &SharedState) -> bool {
    let now = Utc::now();
    let mut removed_paths = Vec::new();
    let mut changed = false;

    {
        let mut core = shared.lock();
        let cutoff = now - audio_retention_duration(&core.settings.audio_retention_policy);

        for item in &mut core.history {
            let Some(path) = item.audio_path.as_ref() else {
                continue;
            };

            let should_expire = history_item_created_at(item)
                .map(|created_at| created_at < cutoff)
                .unwrap_or(false);
            let file_missing = !Path::new(path).exists();
            if should_expire || file_missing {
                if should_expire {
                    removed_paths.push(path.clone());
                }
                item.audio_path = None;
                changed = true;
            }
        }
    }

    for path in removed_paths {
        let _ = fs::remove_file(path);
    }

    if let Ok(directory) = recordings_dir(app) {
        let referenced_paths = {
            let core = shared.lock();
            core.history
                .iter()
                .filter_map(|item| item.audio_path.clone())
                .collect::<Vec<_>>()
        };

        if let Ok(entries) = fs::read_dir(directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                let path_string = path.display().to_string();
                if path.extension().and_then(|value| value.to_str()) == Some("wav")
                    && !referenced_paths
                        .iter()
                        .any(|existing| existing == &path_string)
                {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }

    changed
}

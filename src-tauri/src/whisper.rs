use anyhow::{bail, Context, Result};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub fn model_ready_at(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("bin"))
            .unwrap_or(false)
}

pub fn display_name_for(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Whisper model")
        .to_string()
}

pub struct WhisperTranscriber {
    context: WhisperContext,
}

impl WhisperTranscriber {
    pub fn load(path: &Path) -> Result<Self> {
        if !model_ready_at(path) {
            bail!("Whisper model file is missing at {}", path.display());
        }

        let context = WhisperContext::new_with_params(
            path.to_string_lossy().as_ref(),
            WhisperContextParameters::default(),
        )
        .with_context(|| format!("failed to load Whisper model at {}", path.display()))?;

        Ok(Self { context })
    }

    pub fn transcribe_audio(&mut self, audio: &[f32]) -> Result<String> {
        if audio.is_empty() {
            return Ok(String::new());
        }

        let mut state = self.context.create_state().context("failed to create Whisper state")?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_translate(false);

        state
            .full(params, audio)
            .context("Whisper inference failed")?;

        let segment_count = state.full_n_segments().context("failed to count Whisper segments")?;
        let mut transcript = String::new();
        for index in 0..segment_count {
            let segment = state
                .full_get_segment_text(index)
                .with_context(|| format!("failed to read Whisper segment {index}"))?;
            if !transcript.is_empty() {
                transcript.push(' ');
            }
            transcript.push_str(segment.trim());
        }

        Ok(transcript.trim().to_string())
    }
}

use anyhow::{anyhow, bail, Context, Result};
use std::fs::File;
use std::path::{Path, PathBuf};

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

#[derive(Debug, Clone)]
pub(crate) struct DecodedMedia {
    pub(crate) display_name: String,
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) samples: Vec<f32>,
}

pub(crate) fn decode_media_file(path: &Path) -> Result<DecodedMedia> {
    let file = File::open(path)
        .with_context(|| format!("failed to open media file {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }

    let mut probed = get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("failed to inspect media file {}", path.display()))?;
    let format = &mut probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow!("No supported audio track was found in {}", path.display()))?;
    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| anyhow!("Audio track did not report a sample rate"))?;
    let channels = track
        .codec_params
        .channels
        .map(|value| value.count() as u16)
        .unwrap_or(1);

    let mut decoder = get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .with_context(|| format!("failed to create media decoder for {}", path.display()))?;

    let mut mono = Vec::<f32>::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                bail!("This media file requires a decoder reset that is not supported yet");
            }
            Err(error) => return Err(error).context("failed while reading media packet"),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::ResetRequired) => {
                bail!("This media file requires a decoder reset that is not supported yet");
            }
            Err(error) => return Err(error).context("failed while decoding media audio"),
        };

        append_mono_samples(decoded, &mut mono)?;
    }

    if mono.is_empty() {
        bail!("No audio samples could be decoded from {}", path.display());
    }

    Ok(DecodedMedia {
        display_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("Imported file")
            .to_string(),
        sample_rate,
        channels,
        samples: mono,
    })
}

fn append_mono_samples(decoded: AudioBufferRef<'_>, destination: &mut Vec<f32>) -> Result<()> {
    let spec = *decoded.spec();
    let frames = u64::try_from(decoded.frames()).unwrap_or_default();
    let mut sample_buffer = SampleBuffer::<f32>::new(frames, spec);
    sample_buffer.copy_interleaved_ref(decoded);

    let samples = sample_buffer.samples();
    let channels = spec.channels.count();
    if channels == 0 {
        return Ok(());
    }

    if channels == 1 {
        destination.extend_from_slice(samples);
        return Ok(());
    }

    destination.extend(samples.chunks(channels).map(|frame| {
        let sum = frame.iter().copied().sum::<f32>();
        sum / channels as f32
    }));

    Ok(())
}

pub(crate) fn file_path_label(path: &Path) -> String {
    if let Some(name) = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
    {
        name.to_string()
    } else {
        path.to_string_lossy().into_owned()
    }
}

pub(crate) fn canonical_media_path(path: &str) -> Result<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        bail!("Choose an audio or video file first.");
    }

    let path = PathBuf::from(trimmed);
    let canonical = path
        .canonicalize()
        .with_context(|| format!("File not found: {trimmed}"))?;
    if !canonical.is_file() {
        bail!("That path is not a file.");
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_path_label_extracts_filename() {
        let path = Path::new("/some/dir/recording.wav");
        assert_eq!(file_path_label(path), "recording.wav");
    }

    #[test]
    fn file_path_label_falls_back_to_full_path() {
        let path = Path::new("/");
        let label = file_path_label(path);
        assert!(!label.is_empty());
    }

    #[test]
    fn canonical_media_path_rejects_empty() {
        let result = canonical_media_path("");
        assert!(result.is_err());
    }

    #[test]
    fn canonical_media_path_rejects_whitespace() {
        let result = canonical_media_path("   ");
        assert!(result.is_err());
    }

    #[test]
    fn canonical_media_path_rejects_nonexistent() {
        let result = canonical_media_path("/nonexistent/path/to/file.wav");
        assert!(result.is_err());
    }
}

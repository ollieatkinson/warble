use anyhow::{anyhow, bail, Context, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(wav_path) = args.next() else {
        bail!("usage: cargo run --example transcribe_whisper_wav -- <wav-path> <model-path>");
    };
    let Some(model_path) = args.next() else {
        bail!("usage: cargo run --example transcribe_whisper_wav -- <wav-path> <model-path>");
    };

    let audio = read_wav_mono(&PathBuf::from(&wav_path))?;
    let mut transcriber =
        transcribed_lib::whisper::WhisperTranscriber::load(&PathBuf::from(model_path))?;
    let text = transcriber.transcribe_audio(&audio)?;
    println!("{text}");
    Ok(())
}

fn read_wav_mono(path: &PathBuf) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("failed to open wav file at {}", path.display()))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));

    let samples = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|sample| sample.map(|value| value as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()
            .context("failed to read i16 wav samples")?,
        (hound::SampleFormat::Int, 24 | 32) => reader
            .samples::<i32>()
            .map(|sample| sample.map(|value| value as f32 / i32::MAX as f32))
            .collect::<Result<Vec<_>, _>>()
            .context("failed to read i32 wav samples")?,
        (hound::SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .context("failed to read f32 wav samples")?,
        _ => return Err(anyhow!("unsupported wav format: {:?}", spec)),
    };

    let mono = if channels == 1 {
        samples
    } else {
        samples
            .chunks(channels)
            .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
            .collect()
    };

    Ok(transcribed_lib::parakeet::resample_to_16khz(
        &mono,
        spec.sample_rate,
    ))
}

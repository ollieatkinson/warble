use anyhow::{bail, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(wav_path) = args.next() else {
        bail!("usage: cargo run --example transcribe_wav -- <wav-path> <model-root>");
    };
    let Some(model_root) = args.next() else {
        bail!("usage: cargo run --example transcribe_wav -- <wav-path> <model-root>");
    };

    eprintln!("loading model...");
    let mut transcriber = transcribed_lib::parakeet::ParakeetTdt::load(&PathBuf::from(model_root))?;
    eprintln!("model loaded");
    eprintln!("transcribing wav...");
    let text = transcriber.transcribe_wav_path(&PathBuf::from(wav_path))?;
    eprintln!("transcription finished");
    println!("{text}");
    Ok(())
}

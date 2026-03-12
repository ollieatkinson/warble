<p align="center">
  <img src="./src-tauri/icons/128x128.png" alt="Warble icon" width="96" />
</p>

<h1 align="center">Warble</h1>

<p align="center">
  Local-first desktop dictation and transcription with offline models, live preview, and one-step paste.
</p>

> [!WARNING]
> Warble is still alpha. Development and active testing have mostly been on Windows so far, so that is the path that currently gets the most attention. Other platforms may build, but expect rough edges.

Warble is built for fast speech-to-text without sending your audio to a cloud service. Pick a mic, choose a local model, dictate with a hotkey, and drop the result straight into the app you are already using.

## Highlights

- Local-first transcription, paste, and history flow.
- Hold-to-dictate and toggle-to-dictate capture modes.
- File transcription for existing audio and video files.
- Separate final transcription and live preview model selection.
- Searchable transcript history with optional saved audio clips.
- Cleanup vocabulary for stripping filler words before paste and save.
- Customizable HUD with position, animation style, timer, and live text options.
- Tray-first behavior so the app can stay ready in the background with global hotkeys.

## How It Works

1. Choose a microphone and the batch model you want to use for final text.
2. Start dictation from the main window or with global shortcuts.
3. Watch the live HUD for level feedback, timer, and optional draft text while you speak.
4. Warble cleans the transcript, pastes it into the active app when available, and saves it to History.

Live preview uses a separate streaming model. Final pasted text still comes from the selected batch model.

## Current Feature Set

### Capture

- Microphone input selection with quick device refresh.
- Hold-to-talk and toggle recording modes.
- File transcription from the same capture surface.
- Clipboard fallback when active-app paste is not available.

### Models

- Final transcription models:
  - `Parakeet TDT`
  - `Parakeet CTC`
- Live preview models:
  - `Parakeet Realtime EOU`
  - `Nemotron Streaming`
- In-app model downloads, activation, and removal.
- Windows-first acceleration path via DirectML where available.

### Cleanup and History

- Remove filler terms like `um`, `uh`, and custom phrases before output is pasted or saved.
- Search, copy, inspect, and delete previous transcripts.
- Optional retention of captured audio clips for later review.

### Settings

- Global shortcuts for hold and toggle dictation.
- Appearance controls for theme, HUD placement, and meter style.
- Optional live transcription expansion inside the floating HUD.
- Output and storage preferences in a single settings sheet.

## Status

- Alpha quality.
- Actively exercised on Windows.
- Cross-platform ambitions, but Windows is the only platform I would currently call tested with any confidence.

## Build From Source

Warble does not assume a hosted backend. Everything runs locally.

### Requirements

- Node.js `>= 20.19.0`
- `pnpm`
- Rust and Cargo
- Windows is the best-supported environment today

### Windows dev

If you are already in a Windows shell:

```powershell
pnpm install
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\dev-windows.ps1
```

### WSL to Windows

If you are working from WSL, use the wrapper so Windows PowerShell receives a Windows path:

```bash
pnpm install
make windows-dev
```

You can also use:

```bash
./scripts/dev-windows.sh
```

### Manual start

If you prefer to launch each side yourself:

```powershell
pnpm dev --host 0.0.0.0
```

Then in a second shell:

```powershell
cd .\src-tauri
cargo run
```

## Packaging

To build release artifacts for the current platform:

```bash
pnpm release:artifacts
```

On macOS, release artifacts must be signed with a stable code signing identity.
Ad hoc signing (`APPLE_SIGNING_IDENTITY=-`) is enough to run an app bundle locally, but
it does not produce reliable TCC privacy prompts for microphone access. Use an
`Apple Development` identity for normal local testing, a self-signed root code-signing
identity if you are the only tester, or a `Developer ID Application` identity for
distributable builds.

Warble's macOS bundle also relies on `src-tauri/Info.plist` and
`src-tauri/Entitlements.plist` for microphone access. The built app must carry
`NSMicrophoneUsageDescription` plus the `com.apple.security.device.audio-input`
entitlement.

You can inspect the available identities on your Mac with:

```bash
security find-identity -v -p codesigning
```

The GitHub macOS release job expects `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
and `APPLE_SIGNING_IDENTITY` repository secrets to be configured.

If you generate a local self-signed root code-signing `.p12` with OpenSSL 3 for CI
testing, export it with legacy-compatible PKCS#12 settings so macOS Keychain can
import it:

```bash
openssl pkcs12 -export \
  -legacy \
  -descert \
  -macalg sha1 \
  -inkey key.pem \
  -in cert.pem \
  -out certificate.p12 \
  -name "Warble Local Signing"
```

This runs the Tauri build and writes artifacts into:

```text
artifacts/linux/
artifacts/macos/
artifacts/windows/
```

On Windows, the release packaging step also creates a portable zip with the app binary and required runtime DLLs.

<details>
<summary>Developer shortcuts</summary>

If you prefer the existing wrappers:

```bash
make check
make web-build
make release-artifacts
make windows-dev
make dev-windows
make windows-release
```

Windows PowerShell release wrapper:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-windows-release.ps1
```

</details>

## Contributing

Contributions are welcome.

Useful areas right now:

- Windows polish and regression testing.
- First-pass validation on macOS and Linux.
- Packaging and installer work.
- Model UX, history UX, and cleanup workflow improvements.
- Documentation, screenshots, and onboarding.

Small focused PRs are ideal while the app is still moving quickly.

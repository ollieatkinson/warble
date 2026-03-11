# Transcribed

Local-first desktop transcription built with Tauri, React, and TypeScript.

## Windows Dev

For the most reliable local run, start the app from Windows rather than WSL:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\dev-windows.ps1
```

That opens one PowerShell window for Vite and a second for the Tauri app.

If you prefer manual commands:

```powershell
pnpm dev --host 0.0.0.0
```

and in a second shell:

```powershell
cd .\src-tauri
cargo run
```

## Windows Release Build

To produce downloadable Windows artifacts, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-windows-release.ps1
```

That script:

- runs `pnpm tauri build`
- creates a portable package with `transcribed.exe` plus the required ONNX/DirectML runtime DLLs
- zips the portable package
- copies any generated Tauri installer bundles (`.exe`, `.msi`) into `artifacts\windows\`

Expected output location:

```text
artifacts/windows/
```

Typical contents:

- `Transcribed_<version>_windows_x64_portable.zip`
- Tauri-generated installer artifacts from `src-tauri\target\release\bundle\`

## Make Targets

If you use `make`, there are convenience wrappers:

```bash
make check
make web-build
make windows-dev
make windows-release
```

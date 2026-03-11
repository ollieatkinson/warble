# Warble

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

## Release Artifacts

To produce release artifacts for the current platform, run:

```bash
pnpm release:artifacts
```

That script:

- runs `pnpm tauri build`
- copies the platform bundle output from `src-tauri/target/release/bundle/` into `artifacts/<platform>/bundle/`
- writes `artifacts/<platform>/manifest.json` with the generated file list
- on Windows, also creates a portable zip with the app binary plus the required ONNX/DirectML runtime DLLs

Expected output locations:

```text
artifacts/linux/
artifacts/macos/
artifacts/windows/
```

If you prefer the existing Windows PowerShell entrypoint, it now forwards to the same cross-platform script:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-windows-release.ps1
```

## Release CI

GitHub Actions now builds release artifacts on:

- every pushed tag matching `v*`
- manual `workflow_dispatch`

The workflow uploads one artifact bundle per platform runner:

- `release-linux`
- `release-macos`
- `release-windows`

## Make Targets

If you use `make`, there are convenience wrappers:

```bash
make check
make web-build
make release-artifacts
make windows-dev
make windows-release
```

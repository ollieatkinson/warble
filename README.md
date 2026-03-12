# Warble

Local-first desktop transcription built with Tauri, React, and TypeScript.

## Windows Dev

If you are already in a Windows shell, launch the dev environment with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\dev-windows.ps1
```

If you are in WSL, use a Windows-path wrapper instead of passing `/mnt/c/...` directly to `powershell.exe`:

```bash
make windows-dev
```

or:

```bash
./scripts/dev-windows.sh
```

That opens one Windows shell for Vite and a second for the Tauri app. Passing a Linux path such as `/mnt/c/.../scripts/dev-windows.ps1` to `powershell.exe -File` will fail because Windows PowerShell does not resolve WSL paths there.

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
make dev-windows
make windows-release
```

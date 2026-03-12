SHELL := bash

.PHONY: check web-build release-artifacts windows-dev dev-windows windows-release release-windows

check:
	cd src-tauri && cargo check

web-build:
	pnpm build

release-artifacts:
	pnpm release:artifacts

windows-dev:
	powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$$(wslpath -w "$$(pwd)")\\scripts\\dev-windows.ps1"

dev-windows: windows-dev

windows-release:
	powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$$(wslpath -w "$$(pwd)")\\scripts\\build-windows-release.ps1"

release-windows: windows-release

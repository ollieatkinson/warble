SHELL := bash

.PHONY: check web-build release-artifacts windows-dev windows-release

check:
	cd src-tauri && cargo check

web-build:
	pnpm build

release-artifacts:
	pnpm release:artifacts

windows-dev:
	powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$$(wslpath -w "$$(pwd)")\\scripts\\dev-windows.ps1"

windows-release:
	powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$$(wslpath -w "$$(pwd)")\\scripts\\build-windows-release.ps1"

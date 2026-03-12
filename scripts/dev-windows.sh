#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script_path="$repo_root/scripts/dev-windows.ps1"

if ! command -v wslpath >/dev/null 2>&1; then
  echo "This launcher is intended for WSL, where wslpath is available." >&2
  exit 1
fi

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w "$script_path")"

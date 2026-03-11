param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$tauriDir = Join-Path $repoRoot "src-tauri"

function Wait-ForDevServer {
  param([string]$Url, [int]$TimeoutSeconds = 30)

  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while ((Get-Date) -lt $deadline) {
    try {
      Invoke-WebRequest -Uri $Url -UseBasicParsing | Out-Null
      return
    } catch {
      Start-Sleep -Milliseconds 500
    }
  }

  throw "Timed out waiting for $Url"
}

Write-Host "Starting Vite dev server..." -ForegroundColor Cyan
Start-Process cmd.exe -WorkingDirectory $repoRoot -ArgumentList @(
  "/k",
  "cd /d `"$repoRoot`" && pnpm dev --host 0.0.0.0"
) | Out-Null

Wait-ForDevServer -Url "http://localhost:1420"

Write-Host "Starting Transcribed desktop app..." -ForegroundColor Cyan
Start-Process cmd.exe -WorkingDirectory $tauriDir -ArgumentList @(
  "/k",
  "cd /d `"$tauriDir`" && cargo run"
) | Out-Null

Write-Host "Windows dev environment started." -ForegroundColor Green
Write-Host "Vite: http://localhost:1420"
Write-Host "App: cargo run in a new PowerShell window"

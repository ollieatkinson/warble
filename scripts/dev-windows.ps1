param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$viteScript = Join-Path $repoRoot "node_modules\vite\bin\vite.js"
$tauriCliScript = Join-Path $repoRoot "node_modules\@tauri-apps\cli\tauri.js"
$tauriExternalDevConfig = Join-Path $repoRoot "src-tauri\tauri.windows.external-dev.json"

function Resolve-ToolPath {
  param(
    [string]$DisplayName,
    [string[]]$Candidates,
    [string[]]$CommandNames = @()
  )

  foreach ($candidate in $Candidates) {
    if ([string]::IsNullOrWhiteSpace($candidate)) {
      continue
    }

    if (Test-Path -LiteralPath $candidate) {
      return (Resolve-Path -LiteralPath $candidate).Path
    }
  }

  foreach ($commandName in $CommandNames) {
    $command = Get-Command $commandName -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command -and $command.Source) {
      return $command.Source
    }
  }

  throw "Could not find $DisplayName. Install it or add it to PATH."
}

function Convert-ToSingleQuotedLiteral {
  param([string]$Value)

  return "'" + $Value.Replace("'", "''") + "'"
}

function Start-ToolWindow {
  param(
    [string]$Title,
    [string]$WorkingDirectory,
    [string]$Executable,
    [string[]]$Arguments = @(),
    [string[]]$PathPrefixes = @()
  )

  $windowTitleLiteral = Convert-ToSingleQuotedLiteral $Title
  $workingDirectoryLiteral = Convert-ToSingleQuotedLiteral $WorkingDirectory
  $executableLiteral = Convert-ToSingleQuotedLiteral $Executable
  $argumentLiterals = @($Arguments | ForEach-Object { Convert-ToSingleQuotedLiteral ([string]$_) })
  $pathLiteral = if ($PathPrefixes.Count -gt 0) {
    Convert-ToSingleQuotedLiteral (($PathPrefixes -join ";") + ";" + $env:Path)
  } else {
    $null
  }
  $invocation = "& $executableLiteral"

  if ($argumentLiterals.Count -gt 0) {
    $invocation += " " + ($argumentLiterals -join " ")
  }

  $commandParts = @(
    "`$host.UI.RawUI.WindowTitle = $windowTitleLiteral"
  )

  if ($pathLiteral) {
    $commandParts += "`$env:Path = $pathLiteral"
  }

  $commandParts += "Set-Location -LiteralPath $workingDirectoryLiteral"
  $commandParts += $invocation

  $command = "& { " + ($commandParts -join "; ") + " }"

  Start-Process powershell.exe -WorkingDirectory $WorkingDirectory -ArgumentList @(
    "-NoExit",
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-Command",
    $command
  ) | Out-Null
}

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

$nodeExe = Resolve-ToolPath -DisplayName "Node.js" -Candidates @(
  (Join-Path $env:ProgramFiles "nodejs\node.exe"),
  (Join-Path ${env:ProgramFiles(x86)} "nodejs\node.exe"),
  (Join-Path $env:LOCALAPPDATA "Programs\nodejs\node.exe")
) -CommandNames @("node")

$cargoExe = Resolve-ToolPath -DisplayName "cargo" -Candidates @(
  (Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe")
) -CommandNames @("cargo")
$cargoDir = Split-Path -Parent $cargoExe

if (-not (Test-Path -LiteralPath $viteScript)) {
  throw "Could not find the local Vite entrypoint at $viteScript. Run pnpm install first."
}

if (-not (Test-Path -LiteralPath $tauriCliScript)) {
  throw "Could not find the local Tauri CLI at $tauriCliScript. Run pnpm install first."
}

if (-not (Test-Path -LiteralPath $tauriExternalDevConfig)) {
  throw "Could not find the external dev config override at $tauriExternalDevConfig."
}

Write-Host "Starting Vite dev server..." -ForegroundColor Cyan
Start-ToolWindow -Title "Warble Vite" -WorkingDirectory $repoRoot -Executable $nodeExe -Arguments @(
  $viteScript,
  "dev",
  "--host",
  "0.0.0.0"
)

Wait-ForDevServer -Url "http://localhost:1420"

Write-Host "Starting Warble desktop app..." -ForegroundColor Cyan
Start-ToolWindow -Title "Warble App" -WorkingDirectory $repoRoot -Executable $nodeExe -Arguments @(
  $tauriCliScript,
  "dev",
  "--no-watch",
  "--config",
  $tauriExternalDevConfig,
  "--runner",
  $cargoExe
) -PathPrefixes @(
  $cargoDir
)

Write-Host "Windows dev environment started." -ForegroundColor Green
Write-Host "Vite: http://localhost:1420"
Write-Host "App: tauri dev in a new PowerShell window"

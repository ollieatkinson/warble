param(
  [switch]$SkipTauriBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$nodeScript = Join-Path $repoRoot "scripts/build-release-artifacts.mjs"
$arguments = @("scripts/build-release-artifacts.mjs", "--platform", "windows")

if ($SkipTauriBuild) {
  $arguments += "--skip-tauri-build"
}

function Resolve-NodePath {
  $candidates = @(
    (Join-Path $env:ProgramFiles "nodejs\node.exe"),
    (Join-Path ${env:ProgramFiles(x86)} "nodejs\node.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\nodejs\node.exe")
  )

  foreach ($candidate in $candidates) {
    if ([string]::IsNullOrWhiteSpace($candidate)) {
      continue
    }

    if (Test-Path -LiteralPath $candidate) {
      return (Resolve-Path -LiteralPath $candidate).Path
    }
  }

  $command = Get-Command node -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($command -and $command.Source) {
    return $command.Source
  }

  throw "Could not find Node.js. Install it or add it to PATH."
}

$nodeExe = Resolve-NodePath

if (-not (Test-Path -LiteralPath $nodeScript)) {
  throw "Could not find $nodeScript."
}

Push-Location $repoRoot
try {
  & $nodeExe @arguments
  $exitCode = if (Test-Path variable:global:LASTEXITCODE) {
    $global:LASTEXITCODE
  } else {
    0
  }

  if ($exitCode -ne 0) {
    throw "Release artifact build failed."
  }
} finally {
  Pop-Location
}

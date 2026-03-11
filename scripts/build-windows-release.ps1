param(
  [switch]$SkipTauriBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$arguments = @("scripts/build-release-artifacts.mjs", "--platform", "windows")

if ($SkipTauriBuild) {
  $arguments += "--skip-tauri-build"
}

Push-Location $repoRoot
try {
  & node @arguments
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

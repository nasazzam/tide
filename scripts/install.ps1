[CmdletBinding()]
param(
    [string]$Prefix = (Join-Path $HOME '.local\bin'),
    [switch]$Debug,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Profile = if ($Debug) { 'debug' } else { 'release' }

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Rust/Cargo is required. Install it from https://rustup.rs'
}
if (-not $SkipBuild) {
    $cargoArgs = @('build', '--manifest-path', (Join-Path $Root 'Cargo.toml'), '--locked')
    if (-not $Debug) { $cargoArgs += '--release' }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed.' }
}

New-Item -ItemType Directory -Force -Path $Prefix | Out-Null
Copy-Item (Join-Path $Root "target\$Profile\tide-editor.exe") (Join-Path $Prefix 'tide-editor.exe') -Force
Copy-Item (Join-Path $Root 'bin\tide.ps1') (Join-Path $Prefix 'tide.ps1') -Force
Copy-Item (Join-Path $Root 'bin\tide.cmd') (Join-Path $Prefix 'tide.cmd') -Force
Write-Host "Installed TIDE to $Prefix"
Write-Host 'Use `tide -Editor .` natively, or use the full workspace through WSL.'

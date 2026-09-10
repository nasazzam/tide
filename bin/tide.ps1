#!/usr/bin/env pwsh
[CmdletBinding(PositionalBinding = $false)]
param(
    [switch]$Editor,
    [switch]$Version,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Arguments
)

$ErrorActionPreference = 'Stop'

if ($Version) {
    Write-Output 'tide 0.3.0'
    exit 0
}

$editorBinary = Join-Path $PSScriptRoot 'tide-editor.exe'
if (-not (Test-Path $editorBinary)) {
    $command = Get-Command tide-editor.exe -ErrorAction SilentlyContinue
    if ($command) { $editorBinary = $command.Source }
}

if ($Editor) {
    if (-not (Test-Path $editorBinary)) {
        throw 'tide-editor.exe is not installed.'
    }
    $path = if ($Arguments.Count -gt 0) { $Arguments[0] } else { (Get-Location).Path }
    & $editorBinary $path
    exit $LASTEXITCODE
}

Write-Host @'
TIDE's multi-pane workspace uses tmux. On Windows, launch it through WSL:

    wsl tide <agent> [args...]

The native editor is available with:

    tide -Editor [path]
'@
exit 1

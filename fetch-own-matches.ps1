# Recover salts for your own Deadlock matches via the Steam Game Coordinator.
# Usage: irm https://raw.githubusercontent.com/DTV-GG/dtv-ingest/master/fetch-own-matches.ps1 | iex
$ErrorActionPreference = 'Stop'
# The progress bar makes Invoke-WebRequest extremely slow on Windows PowerShell 5.1.
$ProgressPreference = 'SilentlyContinue'

$bin = Join-Path $env:TEMP 'dtv-ingest-own-matches.exe'
Invoke-WebRequest -UseBasicParsing -OutFile $bin -Uri 'https://github.com/DTV-GG/dtv-ingest/releases/latest/download/dtv-ingest-windows-latest.exe'
$prevRustLog = $env:RUST_LOG
$env:RUST_LOG = 'info'
try {
    & $bin --own-matches
} finally {
    $env:RUST_LOG = $prevRustLog
    Remove-Item $bin -ErrorAction SilentlyContinue
}

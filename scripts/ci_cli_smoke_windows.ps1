$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
$bashScript = Join-Path $scriptDir "ci_cli_smoke.sh"
$bashCommand = Get-Command bash -ErrorAction SilentlyContinue

if (-not $bashCommand) {
    throw "bash is not available in PATH"
}

if (-not (Test-Path $bashScript)) {
    throw "Smoke script not found: $bashScript"
}

$compilerInput = if ($env:APEX_COMPILER_PATH) {
    $env:APEX_COMPILER_PATH
} else {
    Join-Path $repoRoot "target\release\apex-compiler.exe"
}

if (-not [System.IO.Path]::IsPathRooted($compilerInput)) {
    $compilerInput = Join-Path $repoRoot $compilerInput
}

if (-not (Test-Path $compilerInput)) {
    throw "Compiler binary not found: $compilerInput"
}

$bashScript = (Resolve-Path $bashScript).Path
$compilerInput = (Resolve-Path $compilerInput).Path
$repoRoot = (Resolve-Path $repoRoot).Path

$bashScriptUnix = (& $bashCommand.Source -lc "cygpath -u '$bashScript'").Trim()
$compilerUnix = (& $bashCommand.Source -lc "cygpath -u '$compilerInput'").Trim()
$repoRootUnix = (& $bashCommand.Source -lc "cygpath -u '$repoRoot'").Trim()

Write-Host "bashScriptUnix: $bashScriptUnix"
Write-Host "compilerUnix: $compilerUnix"
Write-Host "repoRootUnix: $repoRootUnix"

if (-not $bashScriptUnix) {
    throw "Failed to convert smoke script path for bash: $bashScript"
}
if (-not $compilerUnix) {
    throw "Failed to convert compiler path for bash: $compilerInput"
}
if (-not $repoRootUnix) {
    throw "Failed to convert repo root path for bash: $repoRoot"
}

$ciSkip = if ($env:CI_SKIP_COMPILER_BUILD) { $env:CI_SKIP_COMPILER_BUILD } else { "0" }
$bashRun = @"
set -euo pipefail
cd '$repoRootUnix'
chmod +x '$bashScriptUnix' '$compilerUnix'
export APEX_COMPILER_PATH='$compilerUnix'
export CI_SKIP_COMPILER_BUILD='$ciSkip'
echo "=== Running smoke script ==="
bash -x '$bashScriptUnix' 2>&1
echo "=== Smoke script done ==="
"@

$bashOutput = & $bashCommand.Source --noprofile --norc -lc $bashRun 2>&1
$exitCode = $LASTEXITCODE
Write-Host "Bash output:"
Write-Host $bashOutput
Write-Host "Bash exit code: $exitCode"
if ($exitCode -ne 0) {
    Write-Error "Windows CLI smoke wrapper failed with exit code $exitCode"
    exit $exitCode
}

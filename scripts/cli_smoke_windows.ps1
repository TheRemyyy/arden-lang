$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
$bashScript = Join-Path $scriptDir "cli_smoke.sh"
$bashCommand = Get-Command bash -ErrorAction SilentlyContinue

if (-not $bashCommand) {
    throw "bash is not available in PATH"
}

if (-not (Test-Path $bashScript)) {
    throw "Smoke script not found: $bashScript"
}

$compilerInput = if ($env:ARDEN_COMPILER_PATH) {
    $env:ARDEN_COMPILER_PATH
} else {
    Join-Path $repoRoot "target\release\arden.exe"
}

if (-not [System.IO.Path]::IsPathRooted($compilerInput)) {
    $compilerInput = Join-Path $repoRoot $compilerInput
}

if (-not (Test-Path $compilerInput)) {
    throw "Compiler binary not found: $compilerInput"
}

function ConvertTo-BashSingleQuoted([string]$value) {
    return "'" + ($value -replace "'", "'""'""'") + "'"
}

function ConvertTo-UnixPath([string]$windowsPath, [string]$label) {
    $escapedPath = ConvertTo-BashSingleQuoted $windowsPath
    $unixPath = (& $bashCommand.Source -lc "cygpath -u $escapedPath" 2>$null).Trim()
    if (-not $unixPath) {
        throw "Failed to convert $label path for bash: $windowsPath"
    }
    return $unixPath
}

$bashScript = (Resolve-Path $bashScript).Path
$compilerInput = (Resolve-Path $compilerInput).Path
$repoRoot = (Resolve-Path $repoRoot).Path

$bashScriptUnix = ConvertTo-UnixPath $bashScript "smoke script"
$compilerUnix = ConvertTo-UnixPath $compilerInput "compiler"
$repoRootUnix = ConvertTo-UnixPath $repoRoot "repo root"

Write-Host "bashScriptUnix: $bashScriptUnix"
Write-Host "compilerUnix: $compilerUnix"
Write-Host "repoRootUnix: $repoRootUnix"

$bashScriptEscaped = ConvertTo-BashSingleQuoted $bashScriptUnix
$compilerEscaped = ConvertTo-BashSingleQuoted $compilerUnix
$repoRootEscaped = ConvertTo-BashSingleQuoted $repoRootUnix

$ciSkip = if ($env:CI_SKIP_COMPILER_BUILD) { $env:CI_SKIP_COMPILER_BUILD } else { "0" }
$ciSkipEscaped = ConvertTo-BashSingleQuoted $ciSkip
$tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
$logPath = Join-Path $tempRoot "arden-cli-smoke-windows.log"
$bashRun = @"
set -euo pipefail
cd $repoRootEscaped
chmod +x $bashScriptEscaped $compilerEscaped
export ARDEN_COMPILER_PATH=$compilerEscaped
export CI_SKIP_COMPILER_BUILD=$ciSkipEscaped
echo "=== Running smoke script ==="
bash -x $bashScriptEscaped 2>&1
echo "=== Smoke script done ==="
"@

$bashOutput = & $bashCommand.Source --noprofile --norc -lc $bashRun 2>&1
$exitCode = $LASTEXITCODE
$joinedOutput = ($bashOutput | Out-String)
$joinedOutput | Set-Content -Path $logPath -Encoding UTF8
Write-Host "Bash output:"
Write-Host $joinedOutput
Write-Host "Bash exit code: $exitCode"
Write-Host "Bash log: $logPath"
if ($exitCode -ne 0) {
    Write-Host "Last 80 log lines:"
    Get-Content -Path $logPath -Tail 80 | ForEach-Object { Write-Host $_ }
    Write-Error "Windows CLI smoke wrapper failed with exit code $exitCode"
    exit $exitCode
}

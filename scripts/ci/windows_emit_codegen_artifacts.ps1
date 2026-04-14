param(
    [string]$CompilerPath = "",
    [string]$Source = "",
    [string]$Context = "windows-failure",
    [string]$OutputRoot = ""
)

$ErrorActionPreference = "Continue"
Set-StrictMode -Version Latest

function Resolve-CompilerPath {
    param([string]$Hint)

    if ($Hint -and (Test-Path -LiteralPath $Hint)) {
        return (Resolve-Path -LiteralPath $Hint).Path
    }

    $candidates = @(
        (Join-Path $PWD "target\release\arden.exe"),
        (Join-Path $PWD "target\debug\arden.exe")
    )

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }

    return ""
}

function Resolve-LlcPath {
    $pathFromCommand = (Get-Command llc.exe -ErrorAction SilentlyContinue | Select-Object -First 1).Path
    if ($pathFromCommand) {
        return $pathFromCommand
    }

    if ($env:LLVM_SYS_221_PREFIX) {
        $fromEnv = Join-Path $env:LLVM_SYS_221_PREFIX "bin\llc.exe"
        if (Test-Path -LiteralPath $fromEnv) {
            return $fromEnv
        }
    }

    return ""
}

function Get-SourcesToDump {
    param([string]$SingleSource)

    if ($SingleSource) {
        return @($SingleSource)
    }

    if ($env:ARDEN_FAILURE_SOURCES) {
        return ($env:ARDEN_FAILURE_SOURCES -split ";" | Where-Object { $_ -and $_.Trim().Length -gt 0 })
    }

    return @(
        "examples\single_file\stdlib_and_system\18_file_io\18_file_io.arden",
        "examples\demos\demo_notes\demo_notes.arden"
    )
}

function Sanitize-FileStem {
    param([string]$Raw)
    return ($Raw -replace "[^A-Za-z0-9._-]", "_")
}

if (-not $OutputRoot) {
    if ($env:GITHUB_WORKSPACE) {
        $OutputRoot = Join-Path $env:GITHUB_WORKSPACE "artifacts\windows-failure"
    } else {
        $OutputRoot = Join-Path $PWD "artifacts\windows-failure"
    }
}

$compiler = Resolve-CompilerPath -Hint $CompilerPath
if (-not $compiler) {
    Write-Host "::warning::No arden compiler binary found (checked hint + target\\release + target\\debug). Skipping LLVM dump."
    exit 0
}

$llc = Resolve-LlcPath
if (-not $llc) {
    Write-Host "::warning::llc.exe was not found on PATH or under LLVM_SYS_221_PREFIX; .obj emission will be skipped."
}

$contextDir = Join-Path $OutputRoot $Context
New-Item -ItemType Directory -Force -Path $contextDir | Out-Null

$sources = Get-SourcesToDump -SingleSource $Source
$anyAttempted = $false

foreach ($src in $sources) {
    if (-not $src) {
        continue
    }

    $resolvedSource = $src
    if (-not [System.IO.Path]::IsPathRooted($resolvedSource)) {
        $resolvedSource = Join-Path $PWD $resolvedSource
    }

    if (-not (Test-Path -LiteralPath $resolvedSource)) {
        Write-Host "::warning::Skipping missing source: $src"
        continue
    }

    $anyAttempted = $true
    $resolvedSource = (Resolve-Path -LiteralPath $resolvedSource).Path
    $relativeLabel = $resolvedSource
    if ($env:GITHUB_WORKSPACE) {
        $workspace = (Resolve-Path -LiteralPath $env:GITHUB_WORKSPACE).Path
        if ($resolvedSource.StartsWith($workspace, [System.StringComparison]::OrdinalIgnoreCase)) {
            $relativeLabel = $resolvedSource.Substring($workspace.Length).TrimStart('\', '/')
        }
    }

    $stem = Sanitize-FileStem -Raw ($relativeLabel -replace "[\\/]", "__")
    $llPath = Join-Path $contextDir "$stem.ll"
    $objPath = Join-Path $contextDir "$stem.obj"
    $logPath = Join-Path $contextDir "$stem.log.txt"
    $metaPath = Join-Path $contextDir "$stem.meta.txt"

    Write-Host ""
    Write-Host "[windows-dump] source: $resolvedSource"
    Write-Host "[windows-dump] llvm:   $llPath"
    Write-Host "[windows-dump] object: $objPath"

    @(
        "context=$Context",
        "source=$resolvedSource",
        "compiler=$compiler",
        "llc=$llc",
        "timestamp_utc=$([DateTime]::UtcNow.ToString('o'))"
    ) | Set-Content -Path $metaPath -Encoding UTF8

    & $compiler compile $resolvedSource --emit-llvm $llPath *>&1 | Tee-Object -FilePath $logPath -Append | Out-Host
    if ($LASTEXITCODE -ne 0) {
        Write-Host "::warning::Failed to emit LLVM for $resolvedSource (exit $LASTEXITCODE)"
        continue
    }

    Write-Host "[windows-dump] generated .ll => $llPath"

    if ($llc) {
        & $llc -mtriple=x86_64-pc-windows-msvc $llPath -filetype=obj -o $objPath *>&1 | Tee-Object -FilePath $logPath -Append | Out-Host
        if ($LASTEXITCODE -eq 0) {
            Write-Host "[windows-dump] generated .obj => $objPath"
        } else {
            Write-Host "::warning::llc failed for $resolvedSource (exit $LASTEXITCODE)"
        }
    }
}

if (-not $anyAttempted) {
    Write-Host "::warning::No valid source files were available for LLVM/object dump."
}

Write-Host "[windows-dump] output root: $contextDir"
exit 0

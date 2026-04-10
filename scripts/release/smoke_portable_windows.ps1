param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath
)

$ErrorActionPreference = "Stop"

$TempRoot = Join-Path $env:RUNNER_TEMP "arden-portable-smoke"
if (Test-Path $TempRoot) {
    Remove-Item -Recurse -Force $TempRoot
}
New-Item -ItemType Directory -Path $TempRoot | Out-Null

Expand-Archive -Path $ArchivePath -DestinationPath $TempRoot -Force
$BundleDir = Get-ChildItem -Path $TempRoot -Directory | Select-Object -First 1
if (-not $BundleDir) {
    throw "Portable bundle directory not found after extraction"
}

$RequiredBundledLibs = @(
    (Join-Path $BundleDir.FullName "toolchain\windows-libs\vc\legacy_stdio_definitions.lib"),
    (Join-Path $BundleDir.FullName "toolchain\windows-libs\ucrt\ucrt.lib"),
    (Join-Path $BundleDir.FullName "toolchain\windows-libs\um\kernel32.lib"),
    (Join-Path $BundleDir.FullName "toolchain\windows-libs\builtins\clang_rt.builtins-x86_64.lib")
)
foreach ($RequiredBundledLib in $RequiredBundledLibs) {
    if (-not (Test-Path $RequiredBundledLib)) {
        throw "Portable bundle is missing required Windows import lib: $RequiredBundledLib"
    }
}

$OriginalPath = $env:Path
$env:Path = "C:\Windows\System32;C:\Windows"
$env:LLVM_SYS_221_PREFIX = ""
$env:LLVM_SYS_211_PREFIX = ""
$env:LLVM_CONFIG_PATH = ""
$env:LIB = ""
$env:LIBPATH = ""
$env:INCLUDE = ""

& (Join-Path $BundleDir.FullName "arden.cmd") --version

$WorkDir = Join-Path $TempRoot "work"
New-Item -ItemType Directory -Path $WorkDir | Out-Null
$HelloFile = Join-Path $WorkDir "hello.arden"
@'
import std.io.*;

function main(): None {
    println("Hello from portable Arden!");
    return None;
}
'@ | Set-Content -Path $HelloFile -Encoding UTF8

$RunOutput = & (Join-Path $BundleDir.FullName "arden.cmd") run $HelloFile
$RunOutput | Write-Host
if (-not ($RunOutput -match "Hello from portable Arden!")) {
    throw "Portable run output did not contain the expected hello string"
}

$env:USERPROFILE = Join-Path $TempRoot "profile"
New-Item -ItemType Directory -Path $env:USERPROFILE -Force | Out-Null
& (Join-Path $BundleDir.FullName "install.ps1")
& (Join-Path $env:USERPROFILE "AppData\Local\Arden\bin\arden.cmd") --version

$env:Path = $OriginalPath

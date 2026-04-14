@echo off
setlocal EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "REPO_ROOT=%%~fI"

pushd "%REPO_ROOT%" >nul

echo ========================================
echo   Arden Example Smoke Runner
echo ========================================

if defined ARDEN_COMPILER_PATH (
    set "COMPILER=%ARDEN_COMPILER_PATH%"
) else (
    set "COMPILER=%REPO_ROOT%\target\release\arden.exe"
)

echo.
echo [1/5] Preparing Compiler...
if not "%CI_SKIP_COMPILER_BUILD%"=="1" (
    cargo build --release
    if %ERRORLEVEL% NEQ 0 (
        echo Build failed!
        popd >nul
        exit /b 1
    )
)

if not exist "%COMPILER%" (
    echo Compiler binary not found at %COMPILER%
    popd >nul
    exit /b 1
)

echo Build successful. Using %COMPILER%

set FAIL_COUNT=0
set PASS_COUNT=0

REM Test single-file examples
echo.
echo [2/5] Running Single-File Examples...
echo.

for /R "%REPO_ROOT%\examples\single_file" %%f in (*.arden) do (
    echo ----------------------------------------
    echo Testing %%f...

    "%COMPILER%" run "%%f"

    if !ERRORLEVEL! EQU 0 (
        echo [PASS] %%~nxf
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] %%~nxf
        pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%REPO_ROOT%\scripts\ci\windows_emit_codegen_artifacts.ps1" -CompilerPath "%COMPILER%" -Source "%%f" -Context "examples-smoke-single-file" -OutputRoot "%REPO_ROOT%\artifacts\windows-failure"
        set /a FAIL_COUNT+=1
    )
)

for /R "%REPO_ROOT%\examples\demos" %%f in (*.arden) do (
    echo ----------------------------------------
    echo Testing %%f...

    "%COMPILER%" run "%%f"

    if !ERRORLEVEL! EQU 0 (
        echo [PASS] %%~nxf
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] %%~nxf
        pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%REPO_ROOT%\scripts\ci\windows_emit_codegen_artifacts.ps1" -CompilerPath "%COMPILER%" -Source "%%f" -Context "examples-smoke-demos" -OutputRoot "%REPO_ROOT%\artifacts\windows-failure"
        set /a FAIL_COUNT+=1
    )
)

REM Test starter project
echo.
echo [3/5] Testing Starter Project...
echo.

if exist "%REPO_ROOT%\examples\starter_project\arden.toml" (
    pushd "%REPO_ROOT%\examples\starter_project" >nul
    "%COMPILER%" run
    set TEST_EXIT=!ERRORLEVEL!
    popd >nul

    if !TEST_EXIT! EQU 0 (
        echo [PASS] starter_project
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] starter_project
        pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%REPO_ROOT%\scripts\ci\windows_emit_codegen_artifacts.ps1" -CompilerPath "%COMPILER%" -Context "examples-smoke-starter-project" -OutputRoot "%REPO_ROOT%\artifacts\windows-failure"
        set /a FAIL_COUNT+=1
    )
) else (
    echo starter_project not found, skipping...
)

REM Test nested package project
echo.
echo [4/5] Testing Nested Package Project...
echo.

if exist "%REPO_ROOT%\examples\nested_package_project\arden.toml" (
    pushd "%REPO_ROOT%\examples\nested_package_project" >nul
    "%COMPILER%" run
    set TEST_EXIT=!ERRORLEVEL!
    popd >nul

    if !TEST_EXIT! EQU 0 (
        echo [PASS] nested_package_project
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] nested_package_project
        pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%REPO_ROOT%\scripts\ci\windows_emit_codegen_artifacts.ps1" -CompilerPath "%COMPILER%" -Context "examples-smoke-nested-project" -OutputRoot "%REPO_ROOT%\artifacts\windows-failure"
        set /a FAIL_COUNT+=1
    )
) else (
    echo nested_package_project not found, skipping...
)

REM Test minimal project
echo.
echo [5/5] Testing Minimal Project...
echo.

if exist "%REPO_ROOT%\examples\minimal_project\arden.toml" (
    pushd "%REPO_ROOT%\examples\minimal_project" >nul
    "%COMPILER%" run
    set TEST_EXIT=!ERRORLEVEL!
    popd >nul

    if !TEST_EXIT! EQU 0 (
        echo [PASS] minimal_project
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] minimal_project
        pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%REPO_ROOT%\scripts\ci\windows_emit_codegen_artifacts.ps1" -CompilerPath "%COMPILER%" -Context "examples-smoke-minimal-project" -OutputRoot "%REPO_ROOT%\artifacts\windows-failure"
        set /a FAIL_COUNT+=1
    )
) else (
    echo minimal_project not found, skipping...
)

echo.
echo ========================================
echo Test Summary
echo ========================================
echo Passed: %PASS_COUNT%
echo Failed: %FAIL_COUNT%

popd >nul

if %FAIL_COUNT% EQU 0 (
    echo ALL TESTS PASSED
    exit /b 0
) else (
    echo SOME TESTS FAILED
    exit /b 1
)

@echo off
setlocal EnableDelayedExpansion

set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%..") do set "REPO_ROOT=%%~fI"

pushd "%REPO_ROOT%" >nul

echo ========================================
echo      Arden Test Runner
echo ========================================

if defined APEX_COMPILER_PATH (
    set "COMPILER=%APEX_COMPILER_PATH%"
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

for %%f in ("%REPO_ROOT%\examples\*.arden") do (
    echo ----------------------------------------
    echo Testing %%f...

    "%COMPILER%" run "%%f"

    if !ERRORLEVEL! EQU 0 (
        echo [PASS] %%~nxf
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] %%~nxf
        set /a FAIL_COUNT+=1
    )
)

REM Test multi-file project (basic)
echo.
echo [3/5] Testing Multi-File Project (Basic)...
echo.

if exist "%REPO_ROOT%\examples\multi_file_project\arden.toml" (
    pushd "%REPO_ROOT%\examples\multi_file_project" >nul
    "%COMPILER%" run
    set TEST_EXIT=!ERRORLEVEL!
    popd >nul

    if !TEST_EXIT! EQU 0 (
        echo [PASS] multi_file_project
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] multi_file_project
        set /a FAIL_COUNT+=1
    )
) else (
    echo multi_file_project not found, skipping...
)

REM Test multi-file project with Java-style namespaces
echo.
echo [4/5] Testing Java-Style Namespace Project...
echo.

if exist "%REPO_ROOT%\examples\multi_file_depth_project\arden.toml" (
    pushd "%REPO_ROOT%\examples\multi_file_depth_project" >nul
    "%COMPILER%" run
    set TEST_EXIT=!ERRORLEVEL!
    popd >nul

    if !TEST_EXIT! EQU 0 (
        echo [PASS] multi_file_depth_project
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] multi_file_depth_project
        set /a FAIL_COUNT+=1
    )
) else (
    echo multi_file_depth_project not found, skipping...
)

REM Test no-import project
echo.
echo [5/5] Testing No-Import Project (Global Scope)...
echo.

if exist "%REPO_ROOT%\examples\test_no_import\arden.toml" (
    pushd "%REPO_ROOT%\examples\test_no_import" >nul
    "%COMPILER%" run
    set TEST_EXIT=!ERRORLEVEL!
    popd >nul

    if !TEST_EXIT! EQU 0 (
        echo [PASS] test_no_import
        set /a PASS_COUNT+=1
    ) else (
        echo [FAIL] test_no_import
        set /a FAIL_COUNT+=1
    )
) else (
    echo test_no_import not found, skipping...
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

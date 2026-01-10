@echo off
REM CicadaGallery Installer Build Script
REM Requires: Inno Setup 6 installed and ISCC.exe in PATH

echo ============================================
echo CicadaGallery Installer Build Script
echo ============================================
echo.

REM Check if we're in the right directory
if not exist "Cargo.toml" (
    echo Error: Please run this script from the project root directory.
    exit /b 1
)

REM Step 1: Build release version
echo [1/3] Building release version...
cargo build --release
if errorlevel 1 (
    echo Error: Cargo build failed!
    exit /b 1
)
echo Build successful!
echo.

REM Step 2: Create dist directory
echo [2/3] Preparing distribution...
if not exist "dist" mkdir dist

REM Step 3: Build installer
echo [3/3] Building installer...

REM Try to find Inno Setup
set ISCC_PATH=
if exist "C:\Program Files (x86)\Inno Setup 6\ISCC.exe" (
    set "ISCC_PATH=C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
) else if exist "C:\Program Files\Inno Setup 6\ISCC.exe" (
    set "ISCC_PATH=C:\Program Files\Inno Setup 6\ISCC.exe"
) else (
    REM Try to find in PATH
    where ISCC.exe >nul 2>&1
    if errorlevel 1 (
        echo Error: Inno Setup not found!
        echo Please install Inno Setup 6 from: https://jrsoftware.org/isdl.php
        exit /b 1
    )
    set "ISCC_PATH=ISCC.exe"
)

echo Using Inno Setup: %ISCC_PATH%
"%ISCC_PATH%" installer\CicadaGallery.iss
if errorlevel 1 (
    echo Error: Installer build failed!
    exit /b 1
)

echo.
echo ============================================
echo Build complete!
echo Installer created in: dist\
echo ============================================
dir dist\*.exe

pause

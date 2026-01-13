# CicadaGallery Portable Distribution Build Script
# Creates a ZIP file with all necessary components

param(
    [switch]$SkipBuild = $false
)

$ErrorActionPreference = "Stop"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "CicadaGallery Portable Distribution Builder" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Check if we're in the right directory
if (!(Test-Path "Cargo.toml")) {
    Write-Host "Error: Please run this script from the project root directory." -ForegroundColor Red
    exit 1
}

# Step 1: Build release version
if (!$SkipBuild) {
    Write-Host "[1/4] Building release version..." -ForegroundColor Yellow
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Error: Cargo build failed!" -ForegroundColor Red
        exit 1
    }
    Write-Host "Build successful!" -ForegroundColor Green
} else {
    Write-Host "[1/4] Skipping build (using existing release)" -ForegroundColor Yellow
}
Write-Host ""

# Step 2: Prepare distribution directory
Write-Host "[2/4] Preparing distribution..." -ForegroundColor Yellow
$distDir = "dist"
$appDir = "$distDir\CicadaGallery"

# Clean and create directories
if (Test-Path $appDir) {
    Remove-Item -Recurse -Force $appDir
}
New-Item -ItemType Directory -Path $appDir -Force | Out-Null
New-Item -ItemType Directory -Path "$appDir\image" -Force | Out-Null
New-Item -ItemType Directory -Path "$appDir\mpv" -Force | Out-Null
New-Item -ItemType Directory -Path "$appDir\mpv\glsl_shaders" -Force | Out-Null
New-Item -ItemType Directory -Path "$appDir\ffmpeg" -Force | Out-Null
New-Item -ItemType Directory -Path "$appDir\ffmpeg\bin" -Force | Out-Null

# Step 3: Copy files
Write-Host "[3/4] Copying files..." -ForegroundColor Yellow

# Main application
Write-Host "  - cicada_gallery.exe"
Copy-Item "target\release\cicada_gallery.exe" "$appDir\"

# Application icon
Write-Host "  - image\cicadaGallery.ico"
Copy-Item "image\cicadaGallery.ico" "$appDir\image\"

# Documentation
Write-Host "  - README.md"
Copy-Item "README.md" "$appDir\"
Write-Host "  - LICENSE"
Copy-Item "LICENSE" "$appDir\"

# MPV - Essential components
Write-Host "  - mpv\mpv.exe"
Copy-Item "mpv\mpv.exe" "$appDir\mpv\"
Write-Host "  - mpv\mpv.com"
Copy-Item "mpv\mpv.com" "$appDir\mpv\"
Write-Host "  - mpv\d3dcompiler_43.dll"
Copy-Item "mpv\d3dcompiler_43.dll" "$appDir\mpv\"
Write-Host "  - mpv\glsl_shaders\*"
Copy-Item "mpv\glsl_shaders\*" "$appDir\mpv\glsl_shaders\" -Recurse

# FFmpeg - Essential components
Write-Host "  - ffmpeg\bin\ffmpeg.exe"
Copy-Item "ffmpeg\bin\ffmpeg.exe" "$appDir\ffmpeg\bin\"
Write-Host "  - ffmpeg\bin\ffprobe.exe"
Copy-Item "ffmpeg\bin\ffprobe.exe" "$appDir\ffmpeg\bin\"
Write-Host "  - ffmpeg\LICENSE.txt"
Copy-Item "ffmpeg\LICENSE.txt" "$appDir\ffmpeg\"

# Step 4: Create ZIP archive
Write-Host "[4/4] Creating ZIP archive..." -ForegroundColor Yellow

$version = "1.0.0"
$zipName = "CicadaGallery-$version-portable.zip"
$zipPath = "$distDir\$zipName"

if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}

Compress-Archive -Path $appDir -DestinationPath $zipPath -CompressionLevel Optimal

Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "Build complete!" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Output files:" -ForegroundColor Yellow
Get-ChildItem "$distDir\*.zip" | ForEach-Object {
    $size = [math]::Round($_.Length / 1MB, 2)
    Write-Host "  $($_.Name) ($size MB)" -ForegroundColor White
}
Write-Host ""
Write-Host "Distribution folder: $appDir" -ForegroundColor Yellow

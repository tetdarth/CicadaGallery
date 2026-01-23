[English](README.md) | [日本語](README.ja.md)

# CicadaGallery - Video Gallery Player

A video management and playback application built with Rust. Achieves high-quality video playback using the MPV player.

📖 **Documentation**: [https://www.cicadagallery.net/en/](https://www.cicadagallery.net/en/)

## Features

### Basic Features (Free Version)
- 📁 **Add Files/Directories**: Add video files individually or by folder
- 🖱️ **Drag & Drop**: Add videos or folders via drag and drop
- 🏷️ **Tagging**: Organize videos with tags
- ⭐ **Favorites**: Mark your favorite videos
- 🔍 **Search**: Search videos by title or tags
- 💾 **SQLite Database**: Manage video information with lightweight SQLite
- 🖼️ **Thumbnail Generation**: Automatic thumbnail generation with FFmpeg
- 🎬 **Scene Detection**: Automatically detect scene changes and generate thumbnails
- 🌍 **Multi-language Support**: English, Japanese, and Chinese
- 🎨 **Customizable**: Dark mode, thumbnail size adjustment, and more
- 📺 **MPV Integration**: Play videos with the high-performance MPV player
- 🔊 **Volume Settings**: Set default volume

### Premium Features
- ♾️ **Unlimited Video Registration**: Register more than 100 videos
- ⭐ **1-5 Star Rating**: 5-level rating system
- 🎨 **GLSL Shaders**: Support for custom shaders like Anime4K
- 🖥️ **GPU High-Quality Rendering**: MPV's gpu-hq profile
- ✅ **Multiple Folder/Tag Selection**: Filter by multiple folders or tags
- 🔀 **AND/OR Filter Mode**: Toggle between AND/OR conditions for tags

## Supported Video Formats

MP4, AVI, MKV, MOV, WMV, FLV, WebM, M4V, MPG, MPEG

## Dependencies

This application requires the following external tools:

### MPV (Required)

MPV is used for video playback. It is bundled in the `mpv/` folder of the project.

### FFmpeg (Recommended)

Used for thumbnail generation and video analysis. It is bundled in the `ffmpeg/` folder of the project.

## How to Run

```bash
cd CicadaGallery
cargo run
```

## Build

### Free Version (Open Source)
```bash
cargo build --release
```

> Premium features are not available when built from the public repository.

The executable will be generated at `target/release/cicada_gallery.exe`.

## Usage

### Basic Operations
1. **Add Files**: Click the "📁 Add Files" button in the top bar, or drag & drop from Explorer
2. **Add Folder**: Click the "📂 Add Folder" button in the top bar, or drag & drop from Explorer
3. **Play Video**: Click a thumbnail (plays with MPV)
4. **Multiple Selection**: Ctrl+Click for multiple selection, Shift+Click for range selection
5. **Favorites**: Click the ⭐ button to add to favorites
6. **Filtering**: Filter by folders or tags in the sidebar
7. **View Toggle**: Switch between grid view and list view

### Management Features
- **Folder Management**: Options → Manage Folders... to add/remove folders
- **Tag Management**: Options → Manage Tags... to add/remove tags
- **Shader Management**: Options → Manage Shaders... to select GLSL shaders (Premium feature)

### Keyboard Shortcuts (Inside MPV Player)
- `Space`: Play/Pause
- `←/→`: Rewind/Fast forward 5 seconds
- `↑/↓`: Volume adjustment
- `f`: Toggle fullscreen
- `q`: Quit player

## Tech Stack

- **GUI**: egui / eframe
- **Database**: SQLite (rusqlite)
- **Serialization**: serde / serde_json
- **File Dialog**: rfd
- **Date/Time**: chrono
- **File Watching**: notify
- **License Authentication**: ed25519-dalek

## Data Storage Locations

- **Database**: `%LOCALAPPDATA%\CicadaGallery\database.db`
- **Settings File**: `%LOCALAPPDATA%\CicadaGallery\settings.json`
- **Scene Cache**: `cache/scenes/`

## Language Settings

1. Click the "Options" button in the top bar
2. Select your preferred language in the "Language" section
   - English
   - 日本語 (Japanese)
3. Settings are saved automatically

## License Activation

A license key is required to use premium features.

1. Click the "Enter License Key" button
2. Paste your license key and click "Activate"
3. Premium features will be enabled upon successful authentication

## Future Plans

- Playlist functionality
- More detailed search and filter features
- Smart collections

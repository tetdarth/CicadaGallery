---
layout: default
title: FAQ
lang: en
---

[🇯🇵 日本語](../faq.md) | **🇺🇸 English**

# Frequently Asked Questions (FAQ)

## General Questions

### Q: Is CicadaGallery free to use?

**A:** Yes, basic features are free. The free version is limited to 100 videos and 5 scenes for scene detection. Purchase a premium license to unlock all features without limits.

### Q: What operating systems are supported?

**A:** Currently, only Windows 10/11 (64-bit) is supported. macOS and Linux versions are planned for future releases.

### Q: Is installation required?

**A:** No, it's a portable application. Just extract the ZIP file and you're ready to go.

---

## Video Playback

### Q: Videos won't play

**A:** Please check the following:

1. The bundled mpv folder is correctly placed
2. The video file is not corrupted
3. The video format is supported (MP4, AVI, MKV, MOV, WMV, FLV, WebM, M4V, MPG, MPEG)

### Q: Thumbnails are not generated

**A:** Make sure FFmpeg is working correctly. The bundled ffmpeg folder is required.

### Q: The player doesn't stay on top during playback

**A:** Go to Options → Player Settings → Enable "Always on Top".

---

## Scene Detection

### Q: Scene detection takes a long time

**A:** Scene detection analyzes the entire video, so longer videos take more time. Videos over 1 hour may take several minutes.

### Q: No scenes are detected

**A:** Possible causes:

1. The video has few scene changes
2. FFmpeg is not working correctly
3. Free version limit (5 scenes max) has been reached

---

## Licensing

### Q: How do I enter my license key?

**A:** Go to Options → Premium License section and click "Enter License Key", then enter your license key.

### Q: Can I use my license on multiple PCs?

**A:** A personal license is valid for one PC only. For multiple PCs, please purchase additional licenses.

### Q: Does the license expire?

**A:** No, it's a one-time purchase with no expiration date.

---

## Troubleshooting

### Q: The application won't start

**A:** Try the following:

1. Make sure Visual C++ Redistributable is installed
2. Try running as administrator
3. Check if antivirus software is falsely detecting it

### Q: Settings keep resetting

**A:** Make sure you have write permission to the settings file (settings.json). Settings won't save in read-only folders.

### Q: Database seems corrupted

**A:** Delete the `database.db` file and restart. A new database will be created, but registered video information will be lost.

---

## Contact

If the above doesn't solve your issue, please contact us via [GitHub Issues](https://github.com/tetdarth/CicadaGallery/issues).

---

[← Back to Home](index.md)

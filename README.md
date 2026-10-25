![ArtCraft Android](artcraft-android-1.png)

# ArtCraft for Android

**Beautiful, powerful creative tools — now on your Android device.**

The ArtCraft suite brings professional-grade creative applications to Android with full native performance, Material design integration, and seamless file handling. Each app has been carefully adapted from its desktop counterpart with Android-specific optimizations for touch, stylus, and keyboard input.

## The ArtCraft Suite

ArtCraft is a collection of professional creative applications, each designed for a specific creative workflow. All apps are available as signed Android APKs with full support for Android 11-15.

| App | Description | Key Features |
|---|---|---|
| **[PhotoCraft](https://getartcraft.com/apps/photocraft)** | Professional photo editing and compositing | Layer-based editing, advanced masks, filters, RAW support, PSD compatibility |
| **[VectorCraft](https://getartcraft.com/apps/vectorcraft)** | Vector illustration and design | Precision paths, bezier curves, shapes, typography, SVG support |
| **[FilmCraft](https://getartcraft.com/apps/filmcraft)** | Video editing and post-production | Timeline editing, effects, color grading, multi-track audio, export in multiple formats |
| **[LightCraft](https://getartcraft.com/apps/lightcraft)** | Photo organization and color grading | Non-destructive editing, presets, batch processing, library management |
| **[EffectCraft](https://getartcraft.com/apps/effectcraft)** | Motion graphics and visual effects | Compositing, animation, particle effects, keyframing, GPU-accelerated rendering |
| **[DesignCraft](https://getartcraft.com/apps/designcraft)** | Page layout and publishing | Multi-page documents, master pages, typography, print-ready exports, PDF output |
| **[PDFCraft](https://getartcraft.com/apps/pdfcraft)** | PDF editing and annotation | Edit text and images, forms, annotations, signatures, merge/split documents |
| **[WordCraft](https://getartcraft.com/apps/wordcraft)** | Rich text editing and word processing | Advanced formatting, styles, tables, footnotes, DOCX compatibility |
| **[CADCraft](https://getartcraft.com/apps/cadcraft)** | 2D/3D CAD and technical drawing | Precision drafting, dimensions, layers, DXF/DWG support, technical documentation |
| **[GridCraft](https://getartcraft.com/apps/gridcraft)** | Spreadsheet and data analysis | Formulas, charts, pivot tables, CSV/Excel compatibility, advanced calculations |
| **[DeckCraft](https://getartcraft.com/apps/deckcraft)** | Presentation and slideshow design | Slide layouts, animations, transitions, speaker notes, PPTX export |
| **[FigCraft](https://getartcraft.com/apps/figcraft)** | UI/UX design and prototyping | Components, auto-layout, prototyping, design systems, collaborative workflows |
| **[SoundCraft](https://getartcraft.com/apps/soundcraft)** | Digital Audio Workstation (DAW) | Multi-track recording, mixing, effects, MIDI support, professional audio routing |
| **[SubCraft](https://getartcraft.com/apps/subcraft)** | Subtitle and caption editing | Timeline-based editing, waveform display, SRT/ASS/VTT support, video preview |

## Android-Native Features

Every ArtCraft app has been extensively adapted for Android with a comprehensive set of platform integrations:

### File Handling & Storage
- **Storage Access Framework (SAF)** - Open and save files from any location on your device, including cloud storage
- **Intent Integration** - Open files directly from other apps, file managers, or email attachments
- **Drag & Drop** - Drag files from other apps directly into ArtCraft apps (Android 11+)
- **App-Private Storage** - Automatic staging and management of files with smart cleanup
- **File Associations** - All supported file formats are registered with Android for seamless opening

### Input & Interaction
- **Touch Optimized** - Redesigned for touch with intuitive gestures and mobile-friendly UI scaling
- **Stylus Support** - Full pressure sensitivity and palm rejection for precise drawing and editing
- **Trackpad & Mouse** - Complete support for external input devices with proper cursor handling
- **Hardware Keyboard** - Full keyboard shortcut support matching desktop workflows
- **Soft Keyboard Integration** - Custom IME bridge for seamless text input on all Android keyboards
- **Gesture Controls** - Two-finger scrolling, pinch-to-zoom, and momentum scrolling

### System Integration
- **Immersive Fullscreen** - Edge-to-edge display with automatic system bar theming
- **Dark/Light Mode** - Automatic theme synchronization with system settings
- **Permissions Handling** - Smart runtime permission requests with graceful fallbacks
- **Audio Routing** - Professional audio routing for SoundCraft with telephony/speakerphone support
- **Printing** - Direct printing to any Android printer for supported apps
- **Clipboard** - Full copy/paste integration with Android clipboard
- **Update Checker** - Automatic notifications for new app versions

### Performance & Reliability
- **GPU Acceleration** - Hardware-accelerated rendering with Vulkan/OpenGL ES
- **Crash Recovery** - Automatic autosave and recovery for PhotoCraft and other apps
- **Offline Capable** - All apps work completely offline with no account required
- **ARM64 Optimized** - Native performance on modern Android devices
- **Adaptive UI Scaling** - Smart scaling for phones and tablets

## Download

Get the latest APKs from the [Releases page](https://github.com/MostafaXS1/artcraft-android/releases/).

All builds are locally signed and tested on real Android devices. Each release includes all 14 ArtCraft applications.

## Building from Source

Want to build the apps yourself? The ArtCraft Android Builder provides a complete, automated build system.

### Requirements
- Linux, macOS, or WSL2
- Git, Python 3.11+, Rust (latest stable)
- Android SDK/NDK (automatically downloaded if missing)
- Java 17-23 (automatically downloaded if missing)
- ~10-20GB of free disk space
- Stable internet connection for first build

### Quick Start

```bash
# Clone the builder
git clone https://github.com/MostafaXS1/artcraft-android.git
cd artcraft-android

# Build a release APK for any app
bash build.sh -photocraft
bash build.sh -vectorcraft
bash build.sh -soundcraft
# ... any of the 14 apps

# Build a debug APK for testing
bash build-debug.sh -photocraft
```

### Build Options
- `-stable` (default) - Build from latest stable tag
- `-main` or `-latest` - Build from latest main branch
- `-tag=<tag>` - Build from specific git tag
- `-branch=<branch>` - Build from specific branch

### Output
Built APKs are placed in the `output/` directory as `<app>-<version>.apk`. Build logs are saved to `logs/`.

### Testing on Device
```bash
# Install APK
adb install -r output/photocraft-*.apk

# View logs
adb logcat -s com.artcraft.photocraft

# Check permissions
adb shell dumpsys package com.artcraft.photocraft | grep -A 15 "requested permissions"
```

## Links

- **Website:** [getartcraft.com](https://getartcraft.com/)
- **Apps:** [getartcraft.com/apps](https://getartcraft.com/apps)
- **Releases:** [GitHub Releases](https://github.com/MostafaXS1/artcraft-android/releases/)
- **Source:** [GitHub Repository](https://github.com/MostafaXS1/artcraft-android)

---

*All apps are built from official Storytold repositories with extensive Android platform adaptations. No upstream source is modified - all Android integration is done through templates, adapters, and vendored patches.*

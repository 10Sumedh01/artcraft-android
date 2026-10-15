# Changelog

## v13 — 2026-10-15

### Fixed
- Crash on hardware keyboard/mouse connect/disconnect (configChanges fix)
- LightCraft SAF image import (missing `.with_fs()` file hooks)
- Right/middle-click also triggering left-click (winit Android button mapping)

### Added
- Android system clipboard via JNI (vendored arboard + egui-winit)
- Ctrl+C/X/V shortcut handling in the shell
- Immersive fullscreen + Ctrl/Alt modifier-key fix (carried from v12)

### Supported apps
photocraft, vectorcraft, filmcraft, lightcraft, pdfcraft, effectcraft,
designcraft, figcraft, wordcraft, soundcraft, deckcraft, cadcraft, gridcraft

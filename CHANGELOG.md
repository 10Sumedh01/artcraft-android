# Changelog

## v20 — 2026-10-17

### Added
- Every app can now be opened from another app ("Open with") or receive a shared file. Each
  launcher and both adapters declare Android `VIEW` and `SEND`/`SEND_MULTIPLE` intent filters for
  the document types that app handles — PDF with pdfcraft, designcraft and vectorcraft, Word
  documents with wordcraft, spreadsheets with gridcraft, images and camera raw with photocraft and
  lightcraft, video and audio with filmcraft and soundcraft, and so on. The table lives in one place
  (`lib/file_associations.py`) and is applied to the generated launchers and to the adapters'
  copied manifests, so the associations cannot drift. Filters cover `content://` URIs by MIME type,
  `file://`/`content://` URIs by file-name pattern (for the apps' own project formats, which have no
  registered MIME), and `http`/`https` links for browser-opened documents.
- A file handed to the app is copied into app-private staging off the UI thread and opened exactly
  like a file chosen in the app's own open picker: the async-inbox apps (designcraft, deckcraft,
  gridcraft, wordcraft) receive it in their import inbox, the others are asked to open the staged
  path through their document API (`open_path`, `import::open`, `session.open`, …). This also works
  while the app is already running, through `onNewIntent`.

### Fixed
- A transient upstream compile error on an app's `main` can no longer block the build. Before
  preparing the launcher, the builder runs a small repair step that fills in the `export_presets`
  field PhotoCraft's preset loader is missing (the regression introduced by "load each persisted
  preset set independently"). The step only edits the file while the broken text is present, so it
  is a no-op once upstream ships the fix, and the entry can then be deleted; the checkout is still
  never reset.

### Supported apps
photocraft, vectorcraft, filmcraft, lightcraft, pdfcraft, effectcraft,
designcraft, figcraft, wordcraft, soundcraft, deckcraft, cadcraft, gridcraft

## v19 — 2026-10-17

### Fixed
- The Android soft keyboard is now reliable in every app. Previously it appeared only sometimes,
  flashed in and out, and never delivered typed text: a bare `NativeActivity` has no editable view,
  so `InputMethodManager.showSoftInput` on the decor view was unreliable, and egui's own text input
  had no `InputConnection` to receive commits. Every Activity now installs a tiny, invisible but
  focusable `EditText` (`templates/android/SafActivity.java` and both adapter Activities) whose real
  `InputConnection` makes the keyboard show/dismiss on request and reports every
  commit/composition/delete/editor-action. A shared Rust module
  (`templates/android/ime_bridge.rs`, copied into every generated launcher and both adapters) drains
  those events each frame into egui `ImeEvent`s for the focused `TextEdit`.
- The vendored `android-activity` now forwards `show_soft_input`/`hide_soft_input` to the Activity's
  `imeShow`/`imeHide` when it provides them, instead of always targeting the decor view; and the
  vendored `egui-winit` no longer toggles `set_ime_allowed(false)+(true)` to interrupt a composition
  on Android, which mapped to a visible hide+show flash. The Activity also sets
  `SOFT_INPUT_ADJUST_NOTHING` so the keyboard overlays the fullscreen native surface instead of
  resizing or panning it.
- The keyboard host stays invisible to every other input path. It never consumes touch or hardware key
  events, so those keep flowing to the native surface and `winit` exactly as before (physical
  keyboards still work). Its Enter/newline key and editor action are translated to an egui
  `Key::Enter`, which egui applies per field: a newline in multiline fields, confirm-and-dismiss in
  single-line ones (a bare committed newline would otherwise be dropped by egui). On devices that
  ignore an immediate `showSoftInput`, the Activity retries once after focus and layout settle.

### Supported apps
photocraft, vectorcraft, filmcraft, lightcraft, pdfcraft, effectcraft,
designcraft, figcraft, wordcraft, soundcraft, deckcraft, cadcraft, gridcraft

## v18 — 2026-10-17

### Added
- Android system fonts are now used, alongside each app's own embedded fonts, so non-Latin scripts
  (CJK, Arabic, Hebrew, Devanagari and the other Indic scripts, Thai, Khmer, Myanmar, Georgian,
  Armenian, Ethiopic, …) render instead of appearing as blank boxes. A shared module
  (`templates/android/system_fonts.rs`, copied into every generated launcher and both adapters)
  installs the device's `/system/fonts` (`/product/fonts`) faces as *lowest*-priority epaint
  fallbacks, so they are only consulted for glyphs the app's own fonts lack. A script is skipped when
  the app can already draw it, and each face is read at most once.
- The system font folder is also linked into the per-user locations the apps' own font scanners
  expect (`$HOME/.fonts`, `$HOME/.local/share/fonts`, `$XDG_DATA_HOME/fonts`) and a minimal
  `fonts.conf` is written with `FONTCONFIG_FILE`/`FONTCONFIG_PATH`, so their document/palette
  rendering (wordcraft, gridcraft, deckcraft, designcraft, vectorcraft, pdfcraft, …) sees the same
  fonts as the interface.

### Fixed
- Generated launcher and adapter `ui` methods install the fallbacks each frame, so an app that
  replaces its font definitions with `set_fonts` (e.g. on a language switch) does not drop them.

### Supported apps
photocraft, vectorcraft, filmcraft, lightcraft, pdfcraft, effectcraft,
designcraft, figcraft, wordcraft, soundcraft, deckcraft, cadcraft, gridcraft

## v16 — 2026-10-16

### Added
- Settings, preferences and recent-file lists are saved on Android and restored at startup for
  every app except PhotoCraft (whose adapter already saved). Each app's own load/save calls are
  reproduced in the generated launcher and pointed at the app's private files directory
  (`<internal data>/config`), which survives restarts and app updates. Saves happen on a short
  timer, when the window loses focus (the app is being backgrounded), and at exit.
- The platform path helpers (`HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`) are
  pointed at app-private storage before the app starts, so files the apps write themselves
  (autosave, crash-recovery copies, caches) land there too.

### Fixed
- FigCraft launcher against the current `DialogFuture` picker API.
- DesignCraft launcher against the current `Services::download` signature (it now returns a
  `Result`, so a picker that fails to start is reported instead of only logged).

### Supported apps
photocraft, vectorcraft, filmcraft, lightcraft, pdfcraft, effectcraft,
designcraft, figcraft, wordcraft, soundcraft, deckcraft, cadcraft, gridcraft

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

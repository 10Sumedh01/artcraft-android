# Changelog

## v37 — 2026-10-20

### Added (in progress — hover not yet reaching winit)
- An Android pointer-icon bridge in the vendored `egui-winit`: `pointer_icon.rs` translates the
  `egui::CursorIcon` the interface resolves each frame into an `android.view.PointerIcon` and
  pushes it to the Activity's `InputEnabledSurfaceView` over JNI, so a mouse could show the
  native I-beam over text, a pointing hand over buttons, and resize arrows over splitters.
  winit's Android `set_cursor` is a no-op, so without this the icon never reaches the screen.
  The Activity handle is published once by each launcher (`android_activity.rs`) because
  `ndk-context` only carries the *Application*, and `Activity.getWindow()`/`startActivity()` are
  Activity-only. Verified on-device: the bridge runs and sets the icon on the surface view.
- A native link opener in the vendored `egui-winit`: `open_url.rs` opens a URL with an
  `ACTION_VIEW` intent on the Activity. egui's link widgets emit an `OpenUrl` command that
  `egui-winit` forwards to `webbrowser::open`, but that is compiled only under the `links`
  feature, which the ArtCraft apps leave off — so on Android a clicked link did nothing. The
  app's own `open_url` service is likewise unset on Android, so every link path ends here.

### Fixed
- EffectCraft could not import or export at all: every import said "media import is not available in
  this build" and every export "export is not available in this build". The generated launcher
  started a bare `Session::new()`, which has no media importer, no Render Queue exporter, no
  expression engine and no scripting; the desktop app instead builds `effectcraft_host::session()`.
  The launcher now depends on `effectcraft-host` and constructs `EffectcraftApp::new(
  effectcraft_host::session())`, so its import and export paths work. `effectcraft-host` compiles
  for `aarch64-linux-android`; the APK grows (64 MB → 87 MB) because the media decoders and the
  export encoders are now linked in. (This was mis-diagnosed earlier as a blocking-picker ANR; the
  picker itself was fine — the staged file even landed in `media/` — but the session had nothing to
  import it with.)
- SoundCraft's launcher icon came out as a white background with a nearly all-black bird, instead
  of the white bird on teal it should be. Two bugs in `lib/app_icons.py` combined. (1) Its
  `icon.svg` draws the artwork inside a rounded-square clip (transparent corners), and `magick` was
  rasterising the SVG onto an opaque white canvas: those corners came back white, which then won the
  brand-colour vote, so the adaptive background layer was generated `#FFFFFF` and the legacy mipmaps
  carried white corner wedges; `_raster`/`_write_legacy` now pass `-background none`. (2) The
  adaptive foreground is the artwork with its background flood-filled away, seeded at the image
  edges — but SoundCraft's bird runs off the bottom and right edges (its paths reach `520,520` and
  `530,540`, past the 512 viewBox), so the flood reached the *cream body* and erased it, leaving only
  the dark outline. `_strip_background` now identifies the background colour (the most common opaque
  border colour) and clears a flooded pixel only when it actually *was* that colour, so a flood that
  bleeds into edge-touching artwork leaves the artwork intact. Result: background `#14A9C4`, a cream
  `#EFE9DC` bird on it, and transparent legacy corners. Other apps are unchanged (their artwork does
  not touch the edge); the white-corner fix still applies to every rounded-square icon.

### Known gap
- The pointer icon still does not change on a real mouse hover. `apply_cursor` (and
  `on_cursor_moved`) are never reached, so hover is not arriving at winit even though the
  Activity's window receives it (the framework logs `updatePointerIcon`). GameActivity attaches
  an `OnGenericMotionListener` to the `InputEnabledSurfaceView`, but the view appears not to
  receive hover; to be continued.

## v36 — 2026-10-19

### Fixed
- The API-33 guard on `AMotionEvent_getActionButton` is back in the vendored
  `android-activity` crate. It had been reverted to the committed crate during the mouse
  input rollback, which restored the direct `ndk_sys` reference to a function Android only
  added in 13 — so native-activity builds again died at launch on Android 12L and below with
  `cannot locate symbol "AMotionEvent_getActionButton"` (the report came from gridcraft,
  0.8.0). The call is resolved once through `libc::dlsym` against `RTLD_DEFAULT` and cached
  in a `OnceLock`; a device without the symbol reports an unknown button, exactly as the
  v33 fix behaved. GameActivity builds never read the symbol (the button arrives from the
  Java event), so nothing changes there.

### Changed
- One more mouse-wheel fix, in the other backend: the GameActivity pointer struct only
  carries the axes the app asks for, and only X/Y are on by default, so the scroll wheels
  read as zero and no `MouseWheel` was ever emitted. `EventLoop::new` now enables
  `Axis::Vscroll` and `Axis::Hscroll` before the loop starts (lock-safe, unlike a call from
  inside the event callback), which restores wheel scrolling.
- Rebuilt gridcraft, photocraft, vectorcraft and designcraft. All four launch cold with no
  `UnsatisfiedLinkError`, and `readelf` finds zero references to `AMotionEvent_getActionButton`
  in the shipped libraries.

## v35 — 2026-10-19

### Fixed
- The nine apps that had not been rebuilt since the v33 symbol fix still refused to start on
  Android 12L and below, dying at launch with "cannot locate symbol
  AMotionEvent_getActionButton". Their shipped APKs predated the guard (and, on those trees,
  the GameActivity migration), so lifting them onto the current builder was the whole fix —
  every build re-vendors the patched `android-activity` crate fresh. All nine, plus the four
  already-current gate apps rebuilt in the same pass, now carry **zero** references to the
  symbol, checked with `readelf` against the libraries inside the shipped APKs.

### Restored
- Pointer-icon support is back — the v34 removal is undone — now living in the shared vendored
  `egui-winit`, which is where the hook always belonged: each build vendors that one crate, so
  a per-app copy is just a build artifact the next build overwrites, which is exactly how the
  five wired copies were quietly dropped in the first place. A mouse or trackpad over any app
  again gets the interface's cursor shape (text I-beam over text, resize arrows on an edge,
  grabbing hand while dragging, and so on). Every app was rebuilt with it, so all thirteen
  APKs carry the bridge.

## v34 — 2026-10-19

### Fixed
- Light/dark following never worked on a device. v31's per-frame poll reads
  `Configuration.uiMode`, which is a Java **`int`**, but the JNI field lookup asked for a
  `byte` — so `GetFieldID` failed with `NoSuchFieldError` on every frame and the theme read
  always came back `None`, leaving the interface on its start-up theme. The field is now read
  with its real signature (`I`), so an app opened in dark mode is dark from the first frame,
  and switching the phone's appearance while the app runs re-themes it live.
- PhotoCraft's canvas fell back to the CPU compositor and Help › System Info reported no
  adapter ("unknown"). The adapter never handed eframe's render state to the app. It now
  requests the GPU renderer explicitly (Vulkan first, GLES as fallback), widens the device to
  the adapter's own texture and buffer limits like the desktop build, records the adapter for
  System Info, and starts the wgpu canvas — a device that reports a CPU adapter still falls
  back honestly.

### Removed
- The pointer-icon experiment (forwarding egui's cursor shape to Android's system pointer
  icon over JNI). It had already been dropped from the shared vendored `egui-winit`, but the
  five app trees vendored while it lived (vectorcraft, lightcraft, effectcraft, designcraft,
  soundcraft) still carried it wired — those copies are back to the committed crate, and the
  duplicated empty "Mouse and trackpad" heading it left in the README is gone too.
  *(Reverted in v35: the hook now sits in the shared vendored crate, so no build can drop it
  again.)*

## v33 — 2026-10-19

### Fixed
- The apps no longer refuse to start on Android 12L and below, dying at launch with a
  "cannot locate symbol AMotionEvent_getActionButton" link error. The mouse/scroll input
  path reads the button a press or release refers to through the NDK function
  `AMotionEvent_getActionButton`, which Android only added in API 33 (Android 13). The
  direct reference kept that symbol in the library's dynamic table, so on every older
  device the dynamic linker refused to load the whole app before a single frame was
  drawn. The call is now resolved once through `dlsym` — the same way the platform
  resolves it — and devices without it report an unknown button: no button identity was
  reported there before either (the field cannot exist on those versions), so click
  handling behaves exactly as it did, and every other input field keeps working.
  GameActivity builds were never affected — that backend receives the button from the
  Java event, which exists since API 23 — but the guard covers the vendored
  android-activity crate's native-activity path too, and all shipped APKs now carry no
  reference to the symbol at all (verified with `readelf`).

## v32 — 2026-10-19

### Changed
- Every app now runs its Android lifecycle through GameActivity (`androidx.games:games-activity`
  4.4.0) instead of a framework NativeActivity, and the Gradle Android Plugin assembles the APK
  instead of cargo-apk. GameActivity is the Activity Google maintains for native apps (NativeActivity
  receives no updates), and its Java half is what hosts the soft keyboard's input connection against
  a real view — the decor-view path never did that reliably. The Rust side of android-activity gains
  the matching `game-activity` backend, into which the builder's earlier patches were ported: the
  sync-picker ANR fix now bounds all three of the glue's JVM-thread waits (window handoff,
  activity-state handoff and save-state — each a two-second timed wait), so a blocking file pick
  still cannot freeze the UI thread's callbacks into an ANR; `show_soft_input`/`hide_soft_input`
  prefer the Activity's imeShow/imeHide bridge before falling through to the stock calls; and
  `set_nonblocking_callbacks` — the switch the SAF bridge flips around each pick — is exported for
  the new backend too. The three Java activities extend GameActivity accordingly and gain the
  kill-on-finish guard GameActivity needs: its destroy handshake waits for a native loop that a
  stopped eframe can no longer service, so a finishing activity takes the process down with it
  after waiting out a bounded grace period (a non-finishing destroy — the configuration-change
  path — stays on the normal flow).
- Because GameActivity extends AppCompatActivity, it refuses to inflate under the framework-only
  theme every app used (the launch would die with "You need to use a Theme.AppCompat theme"). The
  generated manifest now points at `Theme.Artcraft`, a generated AppCompat theme that keeps
  `windowFullscreen` — the flag the edge-to-edge code builds on — plus a black first frame before
  the Rust UI draws.
- The build itself: cargo-ndk compiles the native library into a `jniLibs` tree, and a generated
  single-module Gradle project (Gradle 8.11.1 wrapper, AGP 8.7.3) packages it with the Activity,
  the resources and a freshly generated `AndroidManifest.xml` — parsed from the same Cargo metadata
  tables cargo-apk2 read, so intent filters, permissions and activity attributes carry over one to
  one, and libraries are stored uncompressed and page-aligned for direct loading. The APKs keep the
  exact `versionCode` formula, `applicationId` and keystore of the cargo-apk era, so the in-app
  Updater accepts them as ordinary updates of the same app; signing moved from cargo-apk
  environment variables into Gradle's signingConfigs, fed the same builder key through the
  environment at build time. Launchers also request the GPU renderer explicitly — Vulkan first,
  GLES as the fallback for devices without usable Vulkan — and the adapter's real texture and
  buffer limits (the values the desktop apps use), so wide and tall documents stay on the GPU
  compositor instead of falling to egui's 8192 px CPU path. The generator version moves to **31**
  so every generated launcher regenerates.
- Gradle needs a JDK in the 17–23 range; the builder uses the first one it finds and, when there
  is none, downloads Temurin 21 into `.cache/jdk/` once (the signing keystore itself is still
  created with the system `keytool`).
- The photocraft and vectorcraft adapters now declare the same full `configChanges` set as the
  generated launchers, `uiMode` included: v31's live light/dark switching polls the theme every
  frame, but without `uiMode` in the manifest a quick-settings toggle would instead have recreated
  those two apps' activities mid-session.

### Removed
- The cargo-apk/cargo-apk2 dependency: neither tool is invoked or checked for any more.

## v31 — 2026-10-19

### Added
- The interface follows the phone's light/dark setting. Every app defaults to `Appearance`
  → system (the upstream default), which egui resolves against the OS theme it receives —
  but on Android that theme never arrived: every other platform delivers it as a winit
  `ThemeChanged` event, and Android's backend doesn't emit it, so an app opened in the
  wrong mode stayed there for the whole session. The vendored egui-winit now reads the
  Activity's `Configuration.uiMode` over JNI once per frame — the same bridge pattern the
  clipboard backend uses — and fills `RawInput::system_theme`, the identical field the
  desktop path fills, so the cold start is already correct. A quick-settings toggle then
  switches live: `uiMode` is in the manifest's `configChanges` (no restart), and the
  focus round-trip through the shade schedules the frame that carries the new value
  before the app is visible again — with the per-frame system-bar icon sync following it
  into the new theme. The mapping from the `uiMode` night bits to egui's theme is a pure
  function, unit-tested alongside the module it lives in.

## v30 — 2026-10-19

### Added
- Adaptive launcher icons for every app. Modern launchers mask icons into circles, squircles and
  rounded squares, and Android 13+ can tint them with the wallpaper's palette (Material You
  themed icons) — neither works properly with a flat square PNG, which is why the icons looked
  out of place when themed. Every app now ships the two-layer adaptive icon the platform expects:
  the brand colour as a solid `ic_launcher_background` vector, the artwork as
  `ic_launcher_foreground` with its own solid background flood-filled away from the edges (so the
  layer is the artwork silhouette on transparency, scaled to 66.56/108 of the canvas — inside the
  launcher's visible safe zone), and the same foreground registered as the `<monochrome>` layer,
  which gives themed icons a clean single-tint silhouette instead of a tinted square. The flat
  density mipmaps remain for file managers and other surfaces that still read them. The artwork
  source is the app's own vector icon where one exists (rasterised through ImageMagick), else its
  largest own PNG; an explicit `-appicon` goes through the same pipeline. A new shared module,
  `lib/app_icons.py`, owns the whole pipeline, and photocraft's adapter and the generated
  launchers (plus vectorcraft's committed `res/`) all produce identical resources from it. The
  generator version moves to **30** so every generated launcher regenerates.

### Removed
- The mouse pointer-shape experiment from the unreleased v29 work is gone again: it never showed
  the contextual shapes on real devices, so the vendored egui-winit is back to its prior state
  (the clipboard backend untouched) and mouse hover, wheel and secondary-click *events* remain
  the supported pointer surface.

## v29 — 2026-10-19

### Added
- Files can be dragged onto a running app. Dragging a file out of the Files app (or a browser)
  onto the window in split-screen opens it in the app, through the identical path a share-sheet
  delivery uses: the Activity's drag listener collects the dropped URIs, copies them into
  app-private staging off the UI thread with the same helpers, and queues the paths for the Rust
  shell's `intentPoll()`. Drags that carry no files are declined so they don't interfere with
  anything else on the window, and a persistable read grant is requested when the source offered
  one — drag grants are otherwise temporary, so without it the copy can race the end of the drag
  (the system file picker's drags do offer persistable access).
- WordCraft prints through Android's native print system. The print flow gains a printer named
  "Android print system" — the one entry a device-side printer list cannot have, because Android
  keeps enumeration inside the system print dialog so that Wi-Fi, network and cloud print
  services are all reachable. Selecting it (auto-selected on first discovery) and submitting
  opens the real dialog, where printer, paper size, duplex, color and page range live; the PDF
  WordCraft already collated — range, copies and settings baked in — is staged in the cache
  directory and served to the spooler unchanged by a `PrintDocumentAdapter` that reports the page
  count from the system's own `PdfRenderer` and streams the full document on write (a sub-range
  picked in the dialog itself falls back to the full document, which the app's own range settings
  have already shaped). The submission answers synchronously over the SAF bridge, so the app
  reports "Print job submitted." or the failure text where the desktop spooler's feedback
  appears. `templates/android/saf_bridge.rs` grew `print_document`, and the bridge marker moves
  to **V3**.

### Changed
- Apps present edge-to-edge: the interface draws behind transparent system bars, and the bar
  icons (clock, battery, navigation) switch between light and dark with the app's own theme each
  frame, so a light document no longer sits under white-on-white status icons. This is the
  platform equivalent of AndroidX's `enableEdgeToEdge`, without the AndroidX dependency:
  `Window.setDecorFitsSystemWindows(false)` with transparent bar colors on Android 11+, the
  legacy layout flags below it. The Rust shell reports the active egui theme to the Activity at
  the end of each frame (`sync_bar_appearance`; egui 0.36's `Context::theme()` is the source of
  truth, `Context::style()` no longer exists), and the Activity applies the matching
  `APPEARANCE_LIGHT_STATUS_BAR`/`APPEARANCE_LIGHT_NAVIGATION_BAR` window flags — or the legacy
  `SYSTEM_UI_FLAG_*` equivalents on Android 9 and 10, carried across the replace-style set
  instead of dropped. Selection handles and the long-press toolbar are suppressed over the
  native surface, where they otherwise appear on every touch with no text field to belong to.
- Hardware acceleration needed no intervention; this release verifies that rather than changing
  it. cargo-apk2 emits no `android:hardwareAccelerated` attribute, so the platform default — on
  for every `targetSdkVersion` of 14 and later, and every app here targets at least 28 — applies,
  and the fullscreen `DeviceDefault` theme doesn't disable it. The rendering path was always
  native (a `SurfaceView` window surface driven by wgpu); the builder has no WebView anywhere,
  so WebView-only tuning such as `offscreenPreRaster` has nothing to apply to. Glyphs are
  rasterized by epaint on the CPU as on every other egui platform and composited on the GPU
  after that.
- The generator version moves to 29, so every generated launcher regenerates on its next build
  and picks all of this up. The two adapters (PhotoCraft, VectorCraft) ship their own launcher
  sources and were untouched by the template changes.

Everything above is build-verified (each stage gated on three randomly chosen apps, including
WordCraft for the print work); on-device confirmation is pending — the test device was unplugged
during this batch.

## v26 — 2026-10-18

### Fixed
- GridCraft's text renders in Arabic, Hebrew, CJK and the other non-Latin scripts. Every launcher
  hands epaint the device's own faces (`/system/fonts`) as the last fallback of each active font
  *family*, but the fallback was appended only to the families that existed on the pass it ran, and
  a family born later was never given it: `Context::add_font` does nothing for a face whose key the
  definition set already holds, so once the face was recorded it could no longer reach a family
  introduced afterwards. Most apps draw with egui's default `Proportional` family, which exists from
  the first pass and therefore got the fallback; GridCraft draws the interface and every cell with
  its own named families (`FontFamily::Name("ui")`, `"cell"`, the per-workbook `fnt:…` roles), which
  it binds in a `set_fonts` call applied on the *next* pass — after the fallback had already been
  appended to the defaults. The shell now checks the fallback per family each frame and re-adds it
  (under a fresh key) to any family still missing it, so a family bound at any point ends up with
  the system faces as its lowest-priority entries. The change lives in the shared
  `templates/android/system_fonts.rs`, which every launcher — generated and adapter — compiles.

## v25 — 2026-10-18

### Fixed
- Arabic, Hebrew, Chinese/Japanese/Korean and the other non-Latin scripts render again. v23 moved the
  *font environment* to the external app directory along with the app's data, but Android's emulated
  external storage (`/storage/emulated/0/...`, a FUSE/sdcardfs mount) refuses `symlink`, so the links
  that expose `/system/fonts` as `$HOME/.fonts`, `$HOME/.local/share/fonts` and `$XDG_DATA_HOME/fonts`
  were never created — silently, because the failure is discarded. Every app whose font scanner reads
  those directories (which is how the apps see any system face on Android, since `/usr/share/fonts`
  does not exist there) found no font for a script its own embedded faces lack, and the interface and
  documents showed blank boxes. Verified on device: the links were missing in every installed app, and
  `symlink` succeeds under `/data/...` but fails with `Permission denied` under `/sdcard`.
- The font environment is now built on **internal** storage, where the links can be made. Fonts are
  not user data, so nothing user-visible moves: the settings/preferences/recent list keep living in
  `.../Android/data/<pkg>/files/config` and the SAF working copies in `.../files/media`, both still
  reachable over USB/MTP. Only `HOME` and `XDG_DATA_HOME` (the roots the font scanners read, plus
  per-app dotfiles and fontconfig's cache) stay internal; `XDG_CONFIG_HOME` and `XDG_CACHE_HOME` stay
  external. The generated launchers and both adapters set `HOME`/`XDG_DATA_HOME` in one place
  (`system_fonts::install_font_env`) so the two can no longer disagree about which storage they use.

## v24 — 2026-10-17

### Changed
- Clearing the recent-files list now removes the document's staged working copy as well. Importing
  a PDF (or any file) through Storage Access Framework copies it to
  `/storage/emulated/0/Android/data/com.artcraft.<app>/files/media/<document name>`, and it used to
  stay there forever after the app forgot it. The shell now keeps a snapshot of the recent list each
  frame: when an entry leaves it — Clear Recent, the × on one entry, or the capped list aging it
  out — and no open document is showing that file, the staged copy under `media/` is deleted. Only
  direct children of the app's own `media/` directory are ever touched, so nothing else on the
  device can be lost. The `uri:` mapping in SharedPreferences is deliberately kept: re-opening the
  same document reuses the same path (the SAF original is recopied) and a save still reaches the
  document the user picked. The staged copy is a cache — the original lives at its SAF location —
  so nothing but the cache is ever lost.
- A one-time sweep, half a minute after start, deletes staged copies no longer referred to by
  anything: leftovers from before this change, or from an app Android killed mid-session. It waits
  for session restore, spares files written in the last minute (a document may be mid-import),
  skips files the recent list or an open document still names, and — like the per-entry pruning —
  only ever deletes direct children of `media/`.
- A name whose staged copy was deleted is free again: `newStageFile` reuses the path instead of
  adding a ` (2)` suffix when the previously mapped file no longer exists, so a cleared document
  that is imported again lands on its original, suffix-free path. A different, still-existing file
  with the same name keeps the suffix as before.
- Enabled for pdfCraft, PhotoCraft and VectorCraft, and for CadCraft's next build; the remaining
  generated apps switch it on one by one as their recent-list recipes are added (an app without a
  recipe simply keeps everything under `media/`, the old behaviour).

## v23 — 2026-10-17

### Changed
- Release and debug APKs are now named after the version the app's own source declares, e.g.
  `output/vectorcraft-0.12.0.apk` (debug: `output/vectorcraft-0.12.0-debug.apk`). The builder reads
  the workspace `[workspace.package] version` from the checkout — the same value cargo writes into
  the APK's `versionName` — so the file name always matches the build without anyone remembering to
  pass it in. Sources that do not pin a workspace version fall back to the launcher crate's version.
- The Android package is `com.artcraft.<app>` for all thirteen apps (the eleven generated launchers
  and the two adapters, PhotoCraft and VectorCraft). Because the package id changes, Android treats
  each build as a new app: uninstall the previous `ai.storyteller.*` installation before testing, or
  it will sit next to the new one.
- App data now lives in the app-specific external directory
  `/storage/emulated/0/Android/data/com.artcraft.<app>/files` (settings, preferences, recent files,
  autosaves, caches, the fontconfig cache and the working copy of every document opened through
  Storage Access Framework) instead of the internal `/data/user/0/<pkg>/files`,
  which a phone file manager or a connected computer cannot reach. The internal directory is kept as
  a fallback for devices without external storage. Android 11+ still hides `Android/data` from the
  on-device file picker; it is visible over USB/MTP.

### Fixed
- Two builds running at the same time no longer destroy each other. They share the launcher crate
  under `source/<app>/apps/<app>-android`: `prepare.sh` replaces it, and the other run's `javac`
  inherits its working directory from that same directory, so the second build deleted the first
  one's working directory and it died with `java.lang.Error: Properties init: Could not determine
  current working directory`. A lock now refuses the second build up front and names the build that
  holds it.
- A document opened "with" an ArtCraft app is now staged at
  `/storage/emulated/0/Android/data/com.artcraft.<app>/files/media/<document name>` (internal storage
  only as the fallback). It used to be hidden in the internal `/data/user/0/<pkg>/files/...`
  directory — invisible to a phone file manager or a connected computer — inside a random per-open
  subdirectory such as `.../files/PhotoCraft/saf/<uuid>/example.psd`, so the working copy piled up
  under meaningless names. The same document now always maps to the same path; if the name is already
  taken by a different document, a ` (2)`-style suffix is added rather than sharing the file, because
  the staged path is what maps a save back to the document the user picked.
- "Open with" now actually offers the right ArtCraft app. The intent filters were present all along,
  but many MIME types were spelled the way the IANA register names them rather than the way Android's
  MIME database does, and Android matches types literally: `.psd` is `image/x-photoshop` (not
  `image/vnd.adobe.photoshop`), `.bmp` is `image/x-ms-bmp`, `.raf` is `image/x-fuji-raf`, `.rtf` is
  `text/rtf`, `.csv` is `text/comma-separated-values`, the `.docm`/`.dotm`/`.xlsm`/`.xltm`/`.xlsb`
  macro formats use a lower-case `macroenabled`, and so on. `lib/file_associations.py` now lists the
  Android-reported type for every extension plus the registered alias, and the mapping is documented
  against AOSP's `android.mime.types`/`MimeUtils` and a live device's `MediaStore`.
- "Open with" offered every ArtCraft app for *every* file, APKs included. A `<data>` element that
  declares a scheme and a `pathPattern` but no `android:host` does not constrain the path at all:
  AOSP `IntentFilter.matchData` only performs the path check inside its `if (mDataAuthorities !=
  null)` branch, so the element degenerated into a scheme-only match and claimed every `file://`
  and `content://` URI. Path elements now also declare `android:host="*"`, which matches any
  authority while the path pattern does the real work, and the extension glob escapes its dot
  (`.*\.psd`) so it cannot match an unrelated name that merely ends in the same letters.
- Files whose extension Android does not know (`.psb`, `.cr3`, `.ora`, the apps' own project
  formats) are reported as `application/octet-stream`, which a filter that declares only a path
  pattern does not match, because a filter that declares a MIME type makes the intent have to carry
  one. A second, separate VIEW-only filter pairs the same path patterns with `mimeType="*/*"`, so a
  typed chooser entry matches while the path pattern keeps the app off unrelated files. It is
  VIEW-only because a share intent carries its file in `EXTRA_STREAM` and has no data URI, so it can
  never match a filter that declares a scheme; shares of known types are served by the type-only
  `SEND` filter.

### Supported apps
photocraft, vectorcraft, filmcraft, lightcraft, pdfcraft, effectcraft,
designcraft, figcraft, wordcraft, soundcraft, deckcraft, cadcraft, gridcraft

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

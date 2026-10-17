# Per-app Android adapters

The builder auto-generates a thin eframe/NativeActivity wrapper when an app has the conventional
`crates/engine`, `crates/ui-egui`, public `*App` type, `Session::new()`, and `Services::default()` API.
This only supplies a native launch shell; it does not automatically implement Android document pickers,
shared storage, clipboard, audio devices, or touch-specific UI behavior.

If an app has a different constructor or needs platform-specific services, add a complete adapter at:

    adapters/<app>/Cargo.toml
    adapters/<app>/src/lib.rs
    adapters/<app>/res/...

The builder copies that adapter into `source/<app>/apps/<app>-android/` on the first preparation. Include
`[package.metadata.android]` and the NativeActivity metadata in the adapter manifest. Use relative paths
from the Android crate to the root app workspace, normally `../../crates/...`. Keep custom adapters here,
not only in `source/`, so deleting/re-fetching a source checkout does not delete your work.

To regenerate an auto-generated wrapper, first preserve any custom edits, then remove only the marked
`source/<app>/apps/<app>-android/` directory and run the build again. The builder refuses to overwrite
unmarked directories.

## Android file open/save (SAF bridge)

`adapters/photocraft` and `adapters/vectorcraft` are complete adapters that add Android Storage Access
Framework support so the apps' File ▸ Open/Save buttons actually work on-device. Each pairs a Rust
platform-service bridge (`src/lib.rs`) with a Java `NativeActivity` subclass under
`adapters/<app>/java/...` that runs the SAF pickers, stages picked documents in app-private storage,
and copies saved bytes back through `ContentResolver`. Upstream app crates are never edited.

Building these adapters needs `cargo-apk2` (Java Activity support); plain `cargo-apk` ignores
`java_sources`. The builder selects `cargo-apk2` automatically for any adapter whose manifest declares
`java_sources`, and after packaging it verifies the launcher activity and the compiled Java are in the
APK (`verify_android_bridge` in `lib/build-common.sh`).

Manifest notes for a Java-bearing adapter:

- set `java_sources = "java"` under `[package.metadata.android]`;
- name the activity (`name = ".MyActivity"`) and give it `meta_data` `android.app.lib_name`;
- declare the launcher filter **inline**, because `cargo-apk2` does not add one itself and its serde
  `flatten` rejects an `[[...activity.intent_filter]]` table:

      intent_filter = [{ actions = ["android.intent.action.MAIN"], categories = ["android.intent.category.LAUNCHER"] }]

  Without this the APK has no launcher icon and cannot be started from the home screen, even though
  the build succeeds.

## Modifier keys (Ctrl/Alt) and Android winit

On Android, `winit` 0.30 delivers `KeyboardInput` for alphanumeric keys but does not emit
`ModifiersChanged` (it discards the `KeyEvent` meta state). The physical modifier keys
(`ControlLeft/Right`, `AltLeft/Right`, `ShiftLeft/Right`) do reach egui as `Event::Key` events, so
the adapters mirror `egui-winit` by tracking those keys in a per-app `modifiers` field and patching
`raw.events` (key, pointer button, mouse wheel modifiers) and pushing `Event::ModifiersChanged` when
they change. Plain keys work as-is; shortcuts that rely on `Ctrl`/`Alt` (e.g., `Ctrl+K`) are fixed.
Text emitted alongside a modifier chord is suppressed.

## Fullscreen

Both adapters add Java lifecycle hooks in their activity subclasses: in `onCreate` and
`onWindowFocusChanged`, they enter sticky immersive mode (hide status and navigation bars). The
theme's `Theme.DeviceDefault.NoActionBar.Fullscreen` alone hides the status bar on many Android
versions; the navigation bar often remains visible until immersive mode is explicitly set.
Re-applying on window focus ensures the bars stay hidden after system gestures or returning from the
SAF picker.

## System fonts

Adapters include the shared `mod system_fonts;`, whose source lives in
`templates/android/system_fonts.rs`. The builder copies that file next to the adapter's `src/lib.rs`
when it stages the Android crate (`prepare.sh` for PhotoCraft; the generator's custom-adapter path for
VectorCraft), so the one copy stays in step with the generated launchers. The module installs the
device's `/system/fonts` (`/product/fonts`) faces as lowest-priority egui fallbacks for scripts the
app's own fonts cannot draw, and links that folder into the user font locations the app's scanners
read. See the changelog for details. An adapter's `ui` must call
`system_fonts::install_system_fonts(ui.ctx())` and its `android_main` should call
`system_fonts::install_font_env(&android_app)` before the app starts.

## Soft keyboard (IME)

Adapters include the shared `mod ime_bridge;`, whose source lives in
`templates/android/ime_bridge.rs`. The builder copies that file next to the adapter's `src/lib.rs`
when it stages the Android crate (`prepare.sh` for PhotoCraft; the generator's custom-adapter path for
VectorCraft), so the one copy stays in step with the generated launchers. `android-activity`'s
`show_soft_input`/`hide_soft_input` forward to the Activity's `imeShow`/`imeHide` (see the vendored
`vendor/android-activity`), and `ime_bridge::ime::ImeBridge::poll` drains the Activity's queued
commit/composition/delete events into egui `ImeEvent`s each frame. An adapter's `raw_input_hook` must
call `self.ime.poll(raw)`, and its Activity must install the hidden `EditText` (the `imeSetup()` plus
`imeShow`/`imeHide`/`imePoll` methods in the Java subclass).

## Inspecting a build on-device

Both adapters log a marker at startup (`ARTCRAFT_<APP>_ANDROID_SHELL_V2_MODIFIERS_FULLSCREEN`) and
the SAF bridge logs its own marker, so a device log confirms which shell is running:

    adb logcat | grep -E 'ARTCRAFT_|PhotoCraftSAF|VectorCraftSAF'

Installed system-font fallbacks log `Android: installed system fallback font <key> (U+XXXX)`, and a
missing script is not an error (the device simply ships no font for it):

    adb logcat | grep -E 'system fallback font|no system font found'


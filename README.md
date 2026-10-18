# ArtCraft Android Builder
## Go to [Releases](https://github.com/MostafaXS1/artcraft-android/releases/) to download the apps.

Build locally signed, 64-bit ARM (arm64) Android APKs from the official Storytold repositories for the ArtCraft apps.

The builder provides a repeatable path from an existing app repository to an installable Android package. It includes release and debug builds, optional app metadata overrides, a shared file-association registry, and persistent app-specific adapters for projects that need more than the generic launcher.

Project links: [ArtCraft](https://getartcraft.com/) · [The Crafting Apps](https://getartcraft.com/apps)

## Quick start

Run the scripts from any directory. Each script resolves its own location and switches to the project directory before building.

```sh
bash build.sh -pdfcraft          # Optimized release APK
bash build-debug.sh -pdfcraft    # Debug APK; larger and slower
```

Replace `-pdfcraft` with any supported app name.

## Supported apps

```text
photocraft vectorcraft filmcraft lightcraft pdfcraft effectcraft
designcraft figcraft wordcraft soundcraft deckcraft cadcraft gridcraft
```

App names are passed as command-line flags, such as `-photocraft` or `-vectorcraft`.

## Options

| Option | Description |
| --- | --- |
| `-appname=<name>` | Override the Android display name. By default, the app's built-in title is used. |
| `-packagename=<pkg>` | Override the Android package name. By default, the package is `com.artcraft.<app>`. |
| `-appicon=<path>` | Use a custom launcher icon PNG. By default, the app's own icon is used. |
| `-h`, `--help` | Show command help. |

## Examples

```sh
# Build PhotoCraft
bash build.sh -photocraft

# Override the display name
bash build.sh -photocraft -appname=notPhotoshop

# Override the Android package name
bash build.sh -photocraft -packagename=com.adobe.photoshop

# Use a custom launcher icon
bash build.sh -photocraft -appicon=/path/to/icon.png

# Set multiple options
bash build.sh -photocraft \
  -appname=MyApp \
  -packagename=com.example.myapp \
  -appicon=icon.png
```

## Build outputs

| Path | Contents |
| --- | --- |
| `output/<app>-<version>.apk` | Optimized release APK, versioned from the app's own source manifest (e.g. `output/vectorcraft-0.12.0.apk`) |
| `output/<app>-<version>-debug.apk` | Debug APK |
| `source/<app>/` | Source checkout and generated NativeActivity crate |
| `build/<app>/target/` | Isolated Cargo build output for the app |
| `logs/` | Build logs for diagnosis |
| `keys/artcraft-builder.jks` | Persistent local signing keystore |

Build output is separated by app so that Cargo artifacts and source changes do not need to be shared between targets.

## Source checkout behavior

The builder is designed to preserve local work. It does not delete an existing source checkout, reset it, or pull over local changes. Subsequent builds reuse the existing checkout and any modifications made to the Android port.

Update app sources deliberately with Git after preserving your changes. Review the diff before switching versions or incorporating upstream changes.

## Porting model and adapter boundary

The generic launcher targets native Rust/egui applications that expose `Session`, `Services`, and a public `*App` type from `crates/engine` and `crates/ui-egui`.

Some applications need an app-specific adapter rather than the generic startup path. This can apply to apps with:

- Specialized startup requirements, such as audio or media backends.
- Services that are not available through the default launcher setup.
- Dependencies that do not compile for Android without changes.

A failed build is recorded under `logs/`. Use [`adapters/README.md`](adapters/README.md) to add a persistent adapter for an app. A successful APK build confirms that the Android launch shell compiled; it does not, by itself, establish that every app workflow is ready for mobile use. Verify file access, save and export behavior, audio, clipboard, lifecycle handling, and touch interaction on a device.

## Runtime integration

### Preferences and recovery files

Each launcher persists the app's preferences under `config` in the app's private storage and restores them at startup. The path helpers an app writes through — `XDG_CONFIG_HOME`, `XDG_CACHE_HOME` and the app's own config folder — are pointed at the same location, so app-written autosave and recovery files are stored there as well. `HOME` and `XDG_DATA_HOME` are the one exception: they are the roots the font scanners read (see [System fonts](#system-fonts)) and stay on internal storage, the only place the links to `/system/fonts` can be created.

That storage is the app-specific external directory, `/storage/emulated/0/Android/data/com.artcraft.<app>/files`, rather than the internal `/data/user/0/<pkg>/files`. External storage is reachable from a phone file manager and over USB/MTP, which makes settings, autosaves and exports findable; internal storage is invisible without root. The internal directory is the fallback when external storage is unavailable (and is always used for the font environment, which needs links external storage cannot hold). Android 11+ hides `Android/data` from the on-device document picker, so use USB/MTP or `adb` to browse it.

A document opened through Storage Access Framework (SAF) is staged as `<files>/media/<document name>`, so the working copy the app edits is easy to find and the same document always maps to the same path. If that name is already taken by another document, a ` (2)`-style suffix is added rather than sharing the file (a name whose staged copy has since been deleted is free again — no suffix for a cleared document that comes back). When the app lets go of a document — the recent list is cleared, an entry is removed, or the capped list ages it out — and no open document shows it, its staged copy under `media/` is deleted; the original still lives at its SAF location, and the `uri:` mapping is kept so re-opening it reuses the same path. A one-time sweep, half a minute after start, also clears out staged copies that nothing refers to any more (leftovers from an older session or a crash).

PhotoCraft and VectorCraft save through their adapters, which use the same external location.

### System fonts

The launcher uses Android's system fonts alongside each app's embedded fonts. It installs device font faces under `/system/fonts` and `/product/fonts` as the lowest-priority entry of **every** egui font family an app binds — re-checked each frame, so a family an app creates later (GridCraft draws with its own named `ui`/`cell`/`fnt:…` families rather than egui's default `Proportional`) still receives the fallback. It then links the font directory into the user font locations read by app-level scanners:

- `$HOME/.fonts`
- `$HOME/.local/share/fonts`
- `$XDG_DATA_HOME/fonts`

A minimal `fonts.conf` provides the font configuration. The links and that file live in the **internal** private directory (`/data/user/0/<pkg>/files`), not the external one: Android's emulated external storage refuses `symlink`, so building them there leaves `$HOME/.fonts` and friends missing and the scanners find no system face — the reason non-Latin scripts broke under v23. `HOME` and `XDG_DATA_HOME` therefore point at the internal directory even though the rest of the app's data is external. This supports scripts including CJK, Arabic, Hebrew, Indic scripts, Thai, Khmer, and Myanmar when suitable fonts are present on the device. Existing app rendering is left unchanged where the app already has glyph coverage. Exact coverage depends on the fonts included by the device manufacturer.

### Soft keyboard and text input

The Android soft keyboard is wired into every app. Each Activity installs an invisible, focusable `EditText`; its `InputConnection` controls keyboard visibility and forwards text commits, composition updates, deletions, and Enter to the Rust shell. The shell converts these into egui input-method events for the currently focused text field.

The keyboard overlays the fullscreen native surface rather than resizing it.

### Open-with and share-sheet integration

Every app is registered with Android's **Open with** and share sheets for the file types it handles. The current associations include:

| File types | Apps |
| --- | --- |
| PDF | PdfCraft, DesignCraft, VectorCraft |
| Word documents | WordCraft |
| Spreadsheets | GridCraft |
| Presentations | DeckCraft |
| Images and camera raw | PhotoCraft, LightCraft |
| Video and audio | FilmCraft, SoundCraft |
| CAD drawings | CADCraft |
| App-specific project formats | The corresponding app |

File associations are centralized in [`lib/file_associations.py`](lib/file_associations.py) and applied to launcher and adapter manifests to keep them consistent. Extensions Android's MIME database knows are matched by the type it actually reports (for example `.psd` is `image/x-photoshop`, not `image/vnd.adobe.photoshop`), and every extension is also matched by file name so that a provider which stamps a generic `application/octet-stream` still offers the app. Those by-name elements pair the escaped-dot pattern with `android:host="*"` — Android only evaluates a `pathPattern` when the element also declares an authority, and without one a scheme-plus-path element matches every `file://`/`content://` URI — and the same pattern is repeated in a second, VIEW-only element declaring `mimeType="*/*"`, because a filter that declares a MIME type only matches intents that carry one. An app is therefore offered for its own extensions and types and for nothing else.

When a file is opened, shared, or passed through a link, the launcher copies it into app-private storage off the UI thread and opens it through the same path used by the app's own file picker. This is supported while the app is already running as well as on initial launch.

## Signing and distribution

The keystore in `keys/` is intended for local builds only. A randomly generated local password is stored in `keys/.keystore-password`.

Keep the keystore and password safe and stable if you need to install later APKs as updates over an earlier build from this builder. Do not use this key for a public release. Public distribution requires a separate release-signing process and appropriate key management.

## Troubleshooting

Start with the relevant file in `logs/`. If the generic launcher cannot initialize an app or one of its dependencies does not support Android, review the adapter guidance in [`adapters/README.md`](adapters/README.md).

For device-level validation, test the workflows that matter for the target app, including opening files from other apps, saving and exporting, keyboard input, audio or media playback, lifecycle transitions, and touch interaction.

## Related projects

- [ArtCraft website](https://getartcraft.com/)
- [The Crafting Apps](https://getartcraft.com/apps)
- [Storytold repositories on GitHub](https://github.com/storytold)


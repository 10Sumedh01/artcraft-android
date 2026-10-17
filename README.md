# ArtCraft Android Builder

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
| `-packagename=<pkg>` | Override the Android package name. By default, the package is `ai.storyteller.<app>`. |
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
| `output/<app>-release.apk` | Optimized release APK |
| `output/<app>-debug.apk` | Debug APK |
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

Each generated launcher persists the app's preferences to its private files directory under `config` and restores them at startup. Platform path helpers such as `HOME` and `XDG_*` are pointed at the same app-private location so that app-written autosave and recovery files are stored there as well.

PhotoCraft already saves through its adapter.

### System fonts

The launcher uses Android's system fonts alongside each app's embedded fonts. It installs device font faces under `/system/fonts` and `/product/fonts` as lower-priority egui fallbacks, then links the font directory into the user font locations read by app-level scanners:

- `$HOME/.fonts`
- `$HOME/.local/share/fonts`
- `$XDG_DATA_HOME/fonts`

A minimal `fonts.conf` provides the font configuration. This supports scripts including CJK, Arabic, Hebrew, Indic scripts, Thai, Khmer, and Myanmar when suitable fonts are present on the device. Existing app rendering is left unchanged where the app already has glyph coverage. Exact coverage depends on the fonts included by the device manufacturer.

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

File associations are centralized in [`lib/file_associations.py`](lib/file_associations.py) and applied to launcher and adapter manifests to keep them consistent.

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


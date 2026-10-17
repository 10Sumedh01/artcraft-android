# ArtCraft Android Builder

Builds locally signed, arm64 Android ports of ArtCraft apps from official Storytold app repositories

## Usage

    bash build.sh -pdfcraft          # optimized release APK
    bash build-debug.sh -pdfcraft    # debug APK (larger and slower)

### Options

    -appname=<name>     Override the display name (default: app's built-in title)
    -packagename=<pkg>  Override the Android package (default: ai.storyteller.<app>)
    -appicon=<path>     Use a custom launcher icon PNG (default: app's own icon)
    -h, --help          Show help

### Examples

    bash build.sh -photocraft
    bash build.sh -photocraft -appname=notPhotoshop
    bash build.sh -photocraft -packagename=com.adobe.photoshop
    bash build.sh -photocraft -appicon=/path/to/icon.png
    bash build.sh -photocraft -appname=MyApp -packagename=com.example.myapp -appicon=icon.png

Supported repositories:

    photocraft vectorcraft filmcraft lightcraft pdfcraft effectcraft designcraft figcraft wordcraft soundcraft deckcraft cadcraft gridcraft

## Output

    output/<app>-release.apk
    output/<app>-debug.apk
    source/<app>/                       source checkout and generated NativeActivity crate
    build/<app>/target/                 isolated Cargo build output
    logs/                               build logs
    keys/artcraft-builder.jks           persistent local signing key

Run commands from anywhere; scripts resolve their own directory and change to it. They never delete an
existing source checkout, reset it, or pull over local changes. On later builds the existing checkout and
its port changes are reused; update sources deliberately with Git after preserving your changes.

## Important porting boundary

The generic launcher assumes a native Rust/egui app exposes `Session`, `Services`, and a public `*App` type
from `crates/engine` and `crates/ui-egui`. Apps with specialised startup (for example audio/media backends),
non-default service requirements, or dependencies that do not compile on Android may require an adapter.
Such a build failure is recorded in `logs/`; use `adapters/README.md` to add a persistent app-specific adapter.
A successful APK build means the Android launch shell compiled; verify the app's file access, save/export, audio,
clipboard, lifecycle and touch workflows separately before treating it as a complete mobile port.

Settings, preferences and recent-file lists are persisted: the generated launcher saves each app's own
preferences to its private files directory (`internal data path` `/config`) and restores them at startup, and
points the platform path helpers (`HOME`, `XDG_*`) at that directory so app-written autosave and recovery files
land there too. PhotoCraft already saved through its adapter.

Fonts use the device's own system fonts in addition to each app's embedded ones. The launcher installs the
faces under `/system/fonts` (`/product/fonts`) as lowest-priority egui fallbacks for the scripts the app
cannot already draw, and links that folder into the user font locations the apps' own scanners read
(`$HOME/.fonts`, `$HOME/.local/share/fonts`, `$XDG_DATA_HOME/fonts`) with a minimal `fonts.conf`. This covers
CJK, Arabic, Hebrew, the Indic scripts, Thai, Khmer, Myanmar and more; a script the app can already render is
left untouched. Exact coverage still depends on the fonts the device ships.

The Android soft keyboard works in every app. Each Activity installs a small, invisible but focusable
`EditText` whose real `InputConnection` makes the keyboard show and dismiss on request and delivers every
commit, composition, deletion and Enter to the Rust shell, which feeds them to the focused text field as egui
IME events. The keyboard overlays the fullscreen native surface instead of resizing it.

Every app appears in Android's "Open with" and share sheets for the file types it handles: PDF with pdfcraft,
designcraft and vectorcraft, Word documents with wordcraft, spreadsheets with gridcraft, presentations with
deckcraft, images and camera raw with photocraft and lightcraft, video and audio with filmcraft and soundcraft,
CAD drawings with cadcraft, and each app's own project formats. The associations live in one place
(`lib/file_associations.py`) and are written into every launcher's and adapter's manifest, so they cannot
drift between apps. A file handed to the app — by opening it, sharing it, or following a link — is copied into
app-private storage off the UI thread and then opened exactly like a file chosen in the app's own open picker,
including while the app is already running.

The keystore in `keys/` is for local builds only, with a randomly generated local password stored in `keys/.keystore-password`. Keep it safe and stable
if you need to install updates over an earlier APK from this builder. Do not use it for a public release.

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

The keystore in `keys/` is for local builds only, with a randomly generated local password stored in `keys/.keystore-password`. Keep it safe and stable
if you need to install updates over an earlier APK from this builder. Do not use it for a public release.
# artcraft-android
# artcraft-android
# artcraft-android

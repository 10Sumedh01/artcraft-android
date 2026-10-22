#!/usr/bin/env bash
# The soft keyboard's return after the user dismissed it — the app-side patch.
#
# Android takes the back key as "hide the keyboard" and tells the app nothing about it: the
# app goes on believing the keyboard is up, so it never asks for it again, and the keyboard
# stays down through the rest of the session — the next edit (a tap in the text, a caret moved
# by key) goes unseen by it. The keyboard only came back with a restart.
#
# Two things are needed for it to come back, and neither is in the app's own code:
#
#  * the Activity has to *know* the keyboard was dismissed. It watches the keyboard's own
#    window insets: a keyboard that was on and then went away while the app still wanted it
#    was dismissed by the user;
#  * the shell has to *report* that the document is being worked on again. It sees the edits
#    (a pointer press, a key that is not the back key) and tells the Activity, which then
#    asks for the keyboard again.
#
# Only WordCraft gets this: the other apps keep the keyboard where the app puts it.
#
# Every edit is guarded and idempotent: it applies only while the original text is still
# present, so it no-ops once the code is there, and it is re-applied on a fresh checkout.
#
# To test WITHOUT the patch, comment out the call to this script in lib/build-common.sh.
#
# Usage: ime.sh <source-dir> <app> <title>
set -euo pipefail

SRC="${1:?source dir}"
APP="${2:?app}"
TITLE="${3:?title}"

if [[ "$APP" != "wordcraft" ]]; then
    exit 0
fi

python3 - "$SRC" "$APP" <<'PY'
from pathlib import Path
import os
import sys

src, app = Path(sys.argv[1]), Path(sys.argv[2])
android_dir = src / "apps" / f"{app}-android"

fixed = []


def fix(rel: str, old: str, new: str) -> None:
    """Replace the single `old` in `rel` with `new`, unless it is already gone or already fixed."""
    path = android_dir / rel
    try:
        text = path.read_text()
    except OSError:
        return
    if old not in text or new in text or text.count(old) != 1:
        return
    path.write_text(text.replace(old, new, 1))
    fixed.append(rel)


# Instrumentation for work on the keyboard: every event the shell drains is logged, so a
# device's logcat shows whether one press queued two events. Off unless ARTCRAFT_IME_TRACE=1.
def trace(path_rel: str) -> None:
    path = android_dir / path_rel
    try:
        text = path.read_text()
    except OSError:
        return
    marker = "    public String imePoll() {\n        return imeEvents.poll();\n    }"
    traced = (
        "    public String imePoll() {\n"
        "        String event = imeEvents.poll();\n"
        "        if (event != null) Log.i(TAG, \"imePoll -> \" + event);\n"
        "        return event;\n"
        "    }"
    )
    if marker in text and "imePoll ->" not in text:
        path.write_text(text.replace(marker, traced, 1))
        fixed.append(path_rel)
        text = path.read_text()
    # Every queued event, at the source.
    marker = "    private void queueImeEvent(String event) {\n        imeEvents.offer(event);"
    traced = "    private void queueImeEvent(String event) {\n        Log.i(TAG, \"queueImeEvent \" + event);\n        imeEvents.offer(event);"
    if marker in text and "queueImeEvent \" + event" not in text:
        path.write_text(text.replace(marker, traced, 1))
        fixed.append(path_rel + " (queue)")
        text = path.read_text()
    # Whether the shell ever asks for the keyboard, and what the Activity does about it.
    for signature in ("public void imeShow() {", "public void imeHide() {", "public void imeInteraction() {"):
        if signature in text:
            traced = signature + "\n        Log.i(TAG, \"" + signature.split()[2] + "\");"
            if traced not in text:
                path.write_text(text.replace(signature, traced, 1))
                fixed.append(path_rel + " (" + signature.split()[2] + ")")
                text = path.read_text()


# --- The Activity: notice the dismissal, and bring the keyboard back on the next edit. ---

# The Activity's own file name is not known here (the class is the app's), so it is found by
# the one thing every one of them has: the soft-keyboard host.
activity = None
for candidate in sorted((android_dir / "java").rglob("*.java")):
    if "public void imeShow()" in candidate.read_text(errors="replace"):
        activity = candidate
        break
if activity is None:
    raise SystemExit("Could not find the launcher Activity with the soft-keyboard host")

rel = str(activity.relative_to(android_dir))

fix(
    rel,
    """    // True between imeShow() and imeHide(); suppresses a deferred show that a quick hide overtook.
    private volatile boolean imeWanted = false;
""",
    """    // True between imeShow() and imeHide(); suppresses a deferred show that a quick hide overtook.
    private volatile boolean imeWanted = false;
    // The keyboard was on and then went away while the app still wanted it: the user dismissed
    // it with the back key. It comes back the next time the user works on the document.
    private volatile boolean imeDismissed = false;
    // Whether the keyboard was on at the last window-insets change, so a dismissal (the keyboard
    // going away) can be told apart from a keyboard that was never shown.
    private volatile boolean imeWasVisible = false;
""",
)

fix(
    rel,
    """        Window window = getWindow();
        if (window != null) {
            window.getDecorView().post(this::applyImmersiveFullscreen);
        }
""",
    """        Window window = getWindow();
        if (window != null) {
            window.getDecorView().post(this::applyImmersiveFullscreen);
            // Watch the keyboard's own insets: when it goes away while the app still wants it,
            // the user dismissed it with the back key, and the next work on the document brings
            // it back (see imeInteraction()).
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                window.getDecorView().setOnApplyWindowInsetsListener((v, insets) -> {
                    boolean visible = insets.isVisible(WindowInsets.Type.ime());
                    if (visible) {
                        imeDismissed = false;
                    } else if (imeWasVisible && imeWanted) {
                        imeDismissed = true;
                        Log.i(TAG, "the keyboard was dismissed; it returns with the next edit");
                    }
                    imeWasVisible = visible;
                    return insets;
                });
            }
        }
""",
)

fix(
    rel,
    """    /**
     * Hand the Rust shell at most one queued keyboard event per call.""",
    """    /**
     * The user worked on the document again — a tap, a caret moved by key, anything the shell
     * sees as an edit. When the keyboard was dismissed with the back key while the app still
     * wanted it, bring it back: the document is being edited, so the keyboard belongs on screen.
     *
     * Called by the Rust shell over JNI, only when something happened.
     */
    public void imeInteraction() {
        if (!imeWanted || !imeDismissed) {
            return;
        }
        imeDismissed = false;
        final EditText view = imeView;
        if (view == null) {
            return;
        }
        runOnUiThread(() -> {
            try {
                view.setFocusable(true);
                view.setFocusableInTouchMode(true);
                view.requestFocus();
                InputMethodManager imm =
                        (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
                if (imm != null) {
                    boolean shown = imm.showSoftInput(view, InputMethodManager.SHOW_IMPLICIT);
                    Log.i(TAG, "the keyboard was dismissed; asked for it again (shown=" + shown + ")");
                }
            } catch (Throwable error) {
                Log.w(TAG, "Could not show the soft keyboard again", error);
            }
        });
    }

    /**
     * Hand the Rust shell at most one queued keyboard event per call.""",
)

# --- The shared bridge: report the edits to the Activity. ---

fix(
    "src/ime_bridge.rs",
    """        fn next_event(&self) -> Result<Option<String>, String> {""",
    """        /// Tell the Activity that the user worked on the document again.
        ///
        /// Android takes the back key as "hide the keyboard" and the app hears nothing of it, so
        /// the app keeps believing the keyboard is up while it is not — and the next edit (a tap,
        /// a caret moved by key) would go unseen by the keyboard. The Activity watches the
        /// keyboard's own insets and knows when it was dismissed; this call tells it that the
        /// document is being worked on again, so it can bring the keyboard back.
        ///
        /// Only edits count: a pointer press, or a key that is not the back key (which the app
        /// receives as Escape, and which must not count as an edit — it is what dismissed the
        /// keyboard in the first place).
        pub fn interaction(&mut self, raw: &egui::RawInput) {
            let edited = raw.events.iter().any(|event| match event {
                egui::Event::PointerButton { .. } => true,
                egui::Event::Key { key, .. } => *key != egui::Key::Escape,
                _ => false,
            });
            if !edited {
                return;
            }
            if let Err(error) = self.request_interaction() {
                log::warn!("Android soft-keyboard bridge cannot report an edit: {error}");
            }
        }

        /// Ask the Activity to bring the keyboard back after the user dismissed it.
        fn request_interaction(&self) -> Result<(), String> {
            self.vm
                .attach_current_thread(|env| -> jni::errors::Result<()> {
                    let activity_raw = self.app.activity_as_ptr() as jni::sys::jobject;
                    // SAFETY: AndroidApp retains this global Activity reference; as_cast_raw borrows
                    // it without taking ownership or attempting to delete the reference.
                    let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };
                    match env.call_method(
                        &*activity,
                        jni_str!("imeInteraction"),
                        jni_sig!("()V"),
                        &[],
                    ) {
                        Ok(_) => Ok(()),
                        Err(error) => {
                            env.exception_clear();
                            Err(error)
                        }
                    }
                })
                .map_err(|error| error.to_string())
        }

        fn next_event(&self) -> Result<Option<String>, String> {""",
)

# --- The shell: report the edits it sees. ---

fix(
    "src/lib.rs",
    """        self.probe.observe(raw);
        self.ime.poll(raw);
""",
    """        self.probe.observe(raw);
        self.ime.poll(raw);
        self.ime.interaction(raw);
""",
)

if os.environ.get("ARTCRAFT_IME_TRACE") == "1":
    trace(rel)

if fixed:
    print(f"  Patched the soft keyboard's return for {app}: {', '.join(fixed)}")
PY

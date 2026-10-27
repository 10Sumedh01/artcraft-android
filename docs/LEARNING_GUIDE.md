# Mobile Systems & Cross-Platform Rust: Comprehensive Learning Guide

This guide breaks down the core engineering concepts, architecture patterns, and systems programming practices found in the **ArtCraft Android Builder** project. It is structured as an educational reference for engineers learning Rust, native Android development, cross-platform graphics, and desktop-to-mobile porting.

---

## Table of Contents
1. [Rust & Java Interop: High-Performance Safe JNI](#1-rust--java-interop-high-performance-safe-jni)
2. [Desktop-to-Mobile UI Adaptation Patterns](#2-desktop-to-mobile-ui-adaptation-patterns)
3. [Multi-Touch Signal Processing & Gesture State Machines](#3-multi-touch-signal-processing--gesture-state-machines)
4. [Bridging Android Scoped Storage to POSIX Filesystems](#4-bridging-android-scoped-storage-to-posix-filesystems)
5. [The Headless Soft Keyboard (IME) Engine](#5-the-headless-soft-keyboard-ime-engine)
6. [Mobile Graphics Lifecycles & ANR Prevention](#6-mobile-graphics-lifecycles--anr-prevention)
7. [Non-Invasive Upstream Code Patching](#7-non-invasive-upstream-code-patching)
8. [Hands-On Practice Exercises](#8-hands-on-practice-exercises)

---

## 1. Rust & Java Interop: High-Performance Safe JNI

In hybrid applications where native code (Rust) drives the render loop and the operating system host (Java/Kotlin) controls windowing and OS services, the **Java Native Interface (JNI)** is the critical boundary.

### 1.1 Thread Attachment and JVM Lifecycles
On Android, threads created in Rust (such as worker pools or `android_main`) are **not** automatically registered with the ART (Android Runtime) garbage collector or virtual machine. Calling JNI from an unattached thread leads to undefined behavior or immediate process aborts.

#### The Pattern:
```rust
use jni::{JavaVM, objects::{JObject, JValue}, jni_str, jni_sig};

// 1. Acquire raw pointer from AndroidApp safely
let jvm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };

// 2. Attach the current thread temporarily for the duration of the closure
let result = jvm.attach_current_thread(|env| -> jni::errors::Result<String> {
    let activity_raw = app.activity_as_ptr() as jni::sys::jobject;
    let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };

    // 3. Call method using compiled type signatures
    let str_val = env.call_method(
        &*activity,
        jni_str!("getPackageName"),
        jni_sig!("()Ljava/lang/String;"),
        &[],
    )?.l()?;

    // 4. Convert JString to Rust String safely
    let rust_string = env.get_string(&str_val.into())?.into();
    Ok(rust_string)
});
```

### 1.2 Safe Reference Management
* **Local References (`JObject`)**: Valid only for the duration of the native method frame. The JVM provides a default table size (usually 512). Loops that allocate JNI references without manual scoping will overflow the local reference table.
* **Global References (`GlobalRef`)**: Retained across function returns and thread boundaries until explicitly dropped.
* **Borrowing via `as_cast_raw`**: Used when borrowing an existing reference owned elsewhere (like `AndroidApp`'s activity reference) without adding an unnecessary global reference increment.

### 1.3 Exception Boundaries
A pending Java exception prevents further JNI calls. In Rust, you must clear exceptions immediately:
```rust
match env.call_method(...) {
    Ok(val) => Ok(val),
    Err(err) => {
        // ALWAYS clear before recovery or logging
        env.exception_clear();
        Err(err.to_string())
    }
}
```

---

## 2. Desktop-to-Mobile UI Adaptation Patterns

Desktop GUI toolkits (such as `egui`, `Qt`, or `Dear ImGui`) assume:
1. A hardware mouse with hover capability and right-click context menus.
2. A full physical keyboard with modifier keys (`Ctrl`, `Alt`, `Shift`, `Cmd`).
3. Dedicated window resize borders.

On mobile devices, none of these assumptions hold. The project solves these challenges through several patterns:

### 2.1 Virtual Modifier Synthesis
On Android, `winit` discards the `KeyEvent`'s meta state and never emits `ModifiersChanged`. As a result, shortcuts like `Ctrl+Z` or `Ctrl+C` fail.

#### Architectural Fix:
In [`source/wordcraft/apps/wordcraft-android/src/lib.rs`](file:///home/sumedh/artcraft-android/source/wordcraft/apps/wordcraft-android/src/lib.rs#L154-L200), physical modifier keys (`Key::ControlLeft`, `Key::AltLeft`, etc.) are tracked manually. Each frame's `RawInput`:
1. Updates an internal `egui::Modifiers` struct.
2. Stretches modifier flags onto all subsequent pointer/mouse events.
3. **Drops synthetic ASCII text**: When `Ctrl` or `Command` is held, standard text input characters are suppressed so that pressing `Ctrl+C` does not write the literal letter `c` into your document.

### 2.2 Touch Long-Press Right Click
To provide right-click context menus without requiring an attached mouse:
* When a single touch (`PointerButton::Primary`) lands, record `(timestamp, position)`.
* If the finger moves less than `8.0` pixels and remains pressed for `450ms`:
  1. Synthesize a `PointerButton::Secondary { pressed: true }` event.
  2. Follow immediately with `PointerButton::Secondary { pressed: false }`.
  3. Mark the press as converted so the subsequent physical touch release does not trigger an unwanted primary click.

### 2.3 The Floating Mobile Accessory Dock
Implemented in [`templates/android/mobile_overlay.rs`](file:///home/sumedh/artcraft-android/templates/android/mobile_overlay.rs):
* Rendered in `egui::Order::Foreground` with persistent positioning.
* Provides sticky virtual modifier toggles (`[Ctrl]`, `[Alt]`, `[Shift]`, `[✋ Pan]`).
* Provides quick zoom controls (`[🔍+]`, `[🔍-]`) and soft keyboard controls (`[⌨ Keyboard]`, `[🔒 Lock]`).
* Can be collapsed into a minimal floating bubble (`📱 Touch`) that stays out of the user's workflow.

---

## 3. Multi-Touch Signal Processing & Gesture State Machines

Distinguishing between two-finger scrolling (pan) and two-finger zooming (pinch) on a glass touchscreen requires signal processing to filter out finger jitter.

### 3.1 The Gesture Disambiguation Problem
When two fingers touch a screen:
* The **midpoint** $\vec{M} = \frac{\vec{P}_1 + \vec{P}_2}{2}$ measures displacement (translation/scroll).
* The **spread** $S = \|\vec{P}_1 - \vec{P}_2\|$ measures distance between fingers (scaling/zoom).

If both metrics are evaluated without deadzones, small finger tremors cause accidental zooming when the user only wants to scroll down a page, or accidental panning during zoom.

```
                           +----------------------+
                           |   Gesture::Undecided |
                           +----------------------+
                                  /        \
          spread_change > PINCH_DEADZONE    midpoint_delta > PAN_DEADZONE
         && spread >= RATIO * moved         && moved >= spread * 0.75
                                /            \
                               v              v
                     +---------------+  handover   +-------------+
                     | Gesture::Pinch| <=========> | Gesture::Pan|
                     +---------------+             +-------------+
```

### 3.2 Key Parameters in [`vendor/winit/src/platform_impl/android/mod.rs`](file:///home/sumedh/artcraft-android/vendor/winit/src/platform_impl/android/mod.rs):
* `PAN_DEADZONE_PX = 14.0`: Physical pixels the midpoint must travel before triggering Pan.
* `PINCH_DEADZONE_PX = 14.0`: Physical pixels the distance must change before triggering Pinch.
* `PINCH_SPREAD_RATIO = 0.75`: A ratio allowing one-finger anchored pinches (where one finger stays still and the other moves) to be recognized as Pinch rather than Pan.
* **Continuous Handover**: If a user starts scrolling and then separates their fingers widely, the state machine transitions smoothly from `Pan` to `Pinch` without requiring the user to lift their fingers.
* **Midpoint Anchor Emission**: By emitting `WindowEvent::CursorMoved` at the fingers' midpoint during pinch, the canvas calculates the exact mathematical anchor point for the zoom transformation matrix.

---

## 4. Bridging Android Scoped Storage to POSIX Filesystems

Desktop engines rely heavily on Rust's standard library:
```rust
let mut file = std::fs::File::open("/path/to/my_file.docx")?;
```
On Android 10+ (API 29+), accessing arbitrary file paths throws permission denials (`EACCES`). Android requires using the **Storage Access Framework (SAF)** and `content://` URIs.

### 4.1 The Staging Solution
To run desktop engines without rewriting every `std::fs` call:
1. **Open Intent**: The Java Activity invokes `ACTION_OPEN_DOCUMENT`.
2. **Staging Stream**: The Java side reads the file stream from the `ContentResolver` and writes it into the app's private external cache directory:
   `/storage/emulated/0/Android/data/com.artcraft.<app>/files/media/<filename>`
3. **Delivery**: The resulting POSIX file path is delivered to the Rust engine over a JNI JSON queue.
4. **Direct Execution**: The engine opens and reads the staged file using standard `std::fs`.
5. **Flush-on-Save**: When the user clicks "Save", Rust alerts the Activity with the staged path. The Java side opens an `OutputStream` to the original `content://` URI and copies the staged bytes back.
6. **Automatic Deduplication & Cleanup**: File collisions get safe suffixes (`doc (2).docx`), and a background sweep cleans up unreferenced staged files 30 seconds after startup.

---

## 5. The Headless Soft Keyboard (IME) Engine

Standard Android apps use standard `View` hierarchies with `EditText` widgets where Android's `InputMethodManager` (IMM) handles soft keyboard popups automatically.

Native apps using `GameActivity` or `NativeActivity` draw directly to an `ANativeWindow` via OpenGL/Vulkan. They have **no Android View hierarchy**. This creates three severe problems:
1. `imm.showSoftInput` fails because there is no focused `View`.
2. Hardware back button presses dismiss the keyboard, but the native app is never notified.
3. Android IMEs often emit duplicate backspace events (`deleteSurroundingText` + `sendKeyEvent(KEYCODE_DEL)`).

### 5.1 The Architecture
1. **Invisible Host View**: [`SafActivity.java`](file:///home/sumedh/artcraft-android/templates/android/SafActivity.java) creates a hidden, focusable `EditText` attached to the window decor view.
2. **Custom `InputConnectionWrapper`**: Intercepts `commitText`, `setComposingText`, and `deleteSurroundingText`, packaging them into a thread-safe `ConcurrentLinkedQueue<String>`.
3. **Deduplication Filter**: Dropping backspace pairs arriving within `30ms` of each other:
   ```java
   private static final long IME_DELETE_PAIR_MS = 30L;
   // Drops redundant key event if deleteSurroundingText already fired
   ```
4. **ALooper Wakeup**: The native event loop sleeps in `ALooper_pollOnce(-1)`. To prevent typed characters from lagging until the next frame, each IME event synthesizes a dummy key event to wake up the Looper immediately.
5. **Keyboard Lock**: Accidental taps on the screen are prevented from opening the keyboard by setting `imeLocked = true` in Java and clearing `egui`'s IME output request.

---

## 6. Mobile Graphics Lifecycles & ANR Prevention

### 6.1 Surface Invalidation
On desktop OSes, an application window minimizes, but its GPU framebuffer remains valid. On Android, when an application is sent to the background (e.g. user presses Home or switches apps):
1. The OS calls `onSurfaceDestroyed`.
2. The `ANativeWindow` pointer becomes **completely invalid**.
3. Any attempt to draw to or present a `wgpu::Surface` targeting the old handle triggers a fatal crash.

### 6.2 The Event-Driven Lifecycle Pattern
```rust
match event {
    Event::Resumed => {
        // Safe to acquire ANativeWindow and initialize/reconfigure wgpu surface
    }
    Event::Suspended => {
        // MUST drop the wgpu surface or pause all render pipelines
    }
    _ => {}
}
```
Failing to release surfaces promptly when receiving `Suspended` can cause the Android system watchdog to kill the process with an **ANR (Application Not Responding)**.

---

## 7. Non-Invasive Upstream Code Patching

A major architectural achievement of this project is its ability to build 10+ different desktop applications without maintaining separate git forks for each upstream app.

### 7.1 Idempotent Source Patching
Located in [`lib/ime.sh`](file:///home/sumedh/artcraft-android/lib/ime.sh) and [`lib/scrolling.sh`](file:///home/sumedh/artcraft-android/lib/scrolling.sh):
* Rather than maintaining fragile `.patch` or `.diff` files that fail when line numbers change, Python scripts perform **idempotent textual AST/token substitutions**.
* Every substitution checks:
  1. Does the target code exist?
  2. Has the patch already been applied?
  3. Is the replacement unique?
* This allows rebuilding repeatedly against fresh checkouts or modified checkouts without conflicts.

---

## 8. Hands-On Practice Exercises

To solidify your understanding of these concepts, try the following exercises:

### Exercise 1: Implement a Shake-to-Undo Detector
* **Goal**: Use Android's accelerometer to trigger an `Undo` event in egui.
* **Steps**:
  1. Register a `SensorEventListener` for `Sensor.TYPE_ACCELEROMETER` in `SafActivity.java`.
  2. When acceleration exceeds a threshold ($> 15 m/s^2$), queue a `"SHAKE"` string in an event queue.
  3. Expose a JNI method `pollSensorEvent()` in `saf_bridge.rs`.
  4. In `mobile_overlay.rs`, poll the event and synthesize a `Ctrl+Z` key event.

### Exercise 2: Add a Custom Touch Accessory Button
* **Goal**: Add a `[📋 Paste]` button to the mobile touch overlay.
* **Steps**:
  1. Open [`templates/android/mobile_overlay.rs`](file:///home/sumedh/artcraft-android/templates/android/mobile_overlay.rs).
  2. Locate the expanded toolbar in `ui()`.
  3. Add a button that reads text via `crate::android_clipboard_get(&self.app)` and pushes an `egui::Event::Paste(text)` event into `raw.events`.

### Exercise 3: Inspect Multi-Touch Events with Logcat
* **Goal**: Observe the gesture deadzone state transitions in real time.
* **Steps**:
  1. Connect an Android device or emulator with `adb logcat`.
  2. Filter by `winit` or the app's tag: `adb logcat -s ArtCraft WordCraft`.
  3. Place two fingers on screen and slowly pinch vs. pan to see the transitions between `Gesture::Pan` and `Gesture::Pinch`.

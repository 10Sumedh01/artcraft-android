//! Builder-owned Android shell for VectorCraft.
//!
//! Platform file operations use Android's Storage Access Framework (SAF). The Java
//! `VectorCraftActivity` runs the pickers, stages picked documents inside app-private storage and
//! remembers a save destination's URI; the Rust side reads staged paths with the standard file API,
//! writes bytes to them, and copies them back through ContentResolver. The upstream VectorCraft
//! crates remain unchanged.
//!
//! VectorCraft shows its file dialogs off the UI thread through [`Services::start_pick`]: a command
//! (or a dialog's OK) runs as an entry, asking returns no path for now, and when the picker answers
//! the entry runs again with the picked path. The picker is launched through JNI and its answer is
//! delivered into the receiver each frame by [`poll_answers`].
//!
//! ARTCRAFT_VECTORCRAFT_SAF_BRIDGE_V1

use android_activity::AndroidApp;
use jni::{
    JavaVM, jni_sig, jni_str,
    objects::{JObject, JString, JValue},
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use vectorcraft_engine::Session;
use vectorcraft_ui_egui::picks::PickRequest;
use vectorcraft_ui_egui::{FilePick, Services, VectorcraftApp};

/// Emitted at startup so the builder can confirm the bridge is present in the packaged `.so`.
const SAF_BRIDGE_MARKER: &str = "ARTCRAFT_VECTORCRAFT_SAF_BRIDGE_V1";

/// Emitted at startup so a device log (and the packaged `.so` strings) can confirm this shell —
/// including the Android modifier-key fix and immersive fullscreen — is the one that is running.
const ANDROID_SHELL_MARKER: &str = "ARTCRAFT_VECTORCRAFT_ANDROID_SHELL_V2_MODIFIERS_FULLSCREEN";

struct AndroidFileBridge {
    app: AndroidApp,
    vm: Arc<JavaVM>,
}

impl AndroidFileBridge {
    fn new(app: AndroidApp) -> Result<Self, String> {
        // AndroidApp owns the live VM pointer for the lifetime of the activity.
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
        Ok(Self {
            app,
            vm: Arc::new(vm),
        })
    }

    /// Keep the JavaVM and the AndroidApp cheap to clone into the platform-service closures.
    fn clone_for_services(&self) -> Self {
        Self {
            app: self.app.clone(),
            vm: self.vm.clone(),
        }
    }

    fn request_open(&self, id: u64, multiple: bool, mime: &str) -> Result<(), String> {
        self.vm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                // AndroidApp returns an unowned global ref. Do not delete it when this wrapper drops.
                let activity_raw = self.app.activity_as_ptr() as jni::sys::jobject;
                // SAFETY: AndroidApp retains this global Activity reference; as_cast_raw borrows it
                // without taking ownership or attempting to delete the reference.
                let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };
                let mime = env.new_string(mime)?;
                let mime_obj = JObject::from(mime);
                env.call_method(
                    &*activity,
                    jni_str!("vcRequestOpen"),
                    jni_sig!("(JZLjava/lang/String;)V"),
                    &[
                        JValue::Long(id as i64),
                        JValue::Bool(multiple),
                        JValue::Object(&mime_obj),
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| e.to_string())
    }

    fn request_save(&self, id: u64, name: &str, mime: &str) -> Result<(), String> {
        self.vm
            .attach_current_thread(|env| -> jni::errors::Result<()> {
                let activity_raw = self.app.activity_as_ptr() as jni::sys::jobject;
                // SAFETY: AndroidApp retains this global Activity reference; as_cast_raw borrows it
                // without taking ownership or attempting to delete the reference.
                let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };
                let name = env.new_string(name)?;
                let name_obj = JObject::from(name);
                let mime = env.new_string(mime)?;
                let mime_obj = JObject::from(mime);
                env.call_method(
                    &*activity,
                    jni_str!("vcRequestSave"),
                    jni_sig!("(JLjava/lang/String;Ljava/lang/String;)V"),
                    &[
                        JValue::Long(id as i64),
                        JValue::Object(&name_obj),
                        JValue::Object(&mime_obj),
                    ],
                )?;
                Ok(())
            })
            .map_err(|e| e.to_string())
    }

    fn poll_result(&self) -> Result<Option<String>, String> {
        self.vm
            .attach_current_thread(|env| -> jni::errors::Result<Option<String>> {
                let activity_raw = self.app.activity_as_ptr() as jni::sys::jobject;
                // SAFETY: AndroidApp retains this global Activity reference; as_cast_raw borrows it
                // without taking ownership or attempting to delete the reference.
                let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };
                let value = env
                    .call_method(
                        &*activity,
                        jni_str!("vcPollPickerResult"),
                        jni_sig!("()Ljava/lang/String;"),
                        &[],
                    )?
                    .l()?;
                if value.is_null() {
                    return Ok(None);
                }
                let value = JString::cast_local(env, value)?;
                let value: String = value.try_to_string(env)?;
                Ok(Some(value))
            })
            .map_err(|e| e.to_string())
    }

    fn files_dir(&self) -> Result<String, String> {
        self.vm
            .attach_current_thread(|env| -> jni::errors::Result<String> {
                let activity_raw = self.app.activity_as_ptr() as jni::sys::jobject;
                // SAFETY: AndroidApp retains this global Activity reference; as_cast_raw borrows it
                // without taking ownership or attempting to delete the reference.
                let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };
                let value = env
                    .call_method(
                        &*activity,
                        jni_str!("vcFilesDir"),
                        jni_sig!("()Ljava/lang/String;"),
                        &[],
                    )?
                    .l()?;
                let value = JString::cast_local(env, value)?;
                let value: String = value.try_to_string(env)?;
                Ok(value)
            })
            .map_err(|e| e.to_string())
    }

    /// `NONE` for ordinary private paths, `OK` after committing a selected SAF destination, or
    /// `ERROR:<message>` when the Android provider refused the write.
    fn commit_if_mapped(&self, path: &str) -> Result<String, String> {
        self.vm
            .attach_current_thread(|env| -> jni::errors::Result<String> {
                let activity_raw = self.app.activity_as_ptr() as jni::sys::jobject;
                // SAFETY: AndroidApp retains this global Activity reference; as_cast_raw borrows it
                // without taking ownership or attempting to delete the reference.
                let activity = unsafe { env.as_cast_raw::<JObject>(&activity_raw)? };
                let path = env.new_string(path)?;
                let path_obj = JObject::from(path);
                let value = env
                    .call_method(
                        &*activity,
                        jni_str!("vcCommitIfMapped"),
                        jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                        &[JValue::Object(&path_obj)],
                    )?
                    .l()?;
                let value = JString::cast_local(env, value)?;
                let value: String = value.try_to_string(env)?;
                Ok(value)
            })
            .map_err(|e| e.to_string())
    }
}

/// The picker requests in flight: their answer receivers, keyed by request id.
#[derive(Default)]
struct PickState {
    next_id: AtomicU64,
    senders: Mutex<HashMap<u64, Sender<Vec<String>>>>,
}

impl PickState {
    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }

    fn insert(&self, id: u64, sender: Sender<Vec<String>>) {
        self.senders
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, sender);
    }

    fn remove(&self, id: u64) -> Option<Sender<Vec<String>>> {
        self.senders
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id)
    }
}

/// A MIME type for an open dialog, from its first file-type filter (every file when it has none).
fn mime_for_filters(pick: &FilePick) -> String {
    pick.filters
        .first()
        .and_then(|(_, exts)| exts.first().copied())
        .map(mime_for_extension)
        .unwrap_or_else(|| "*/*".to_string())
}

fn mime_for_name(name: &str) -> String {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    mime_for_extension(ext)
}

fn mime_for_extension(ext: &str) -> String {
    match ext.to_ascii_lowercase().as_str() {
        "svg" | "svgz" => "image/svg+xml",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "tif" | "tiff" => "image/tiff",
        "bmp" => "image/bmp",
        _ => "*/*",
    }
    .to_string()
}

/// Show `request`'s Android picker off the UI thread; the answer arrives on the receiver. `None`
/// when this host can't (a folder picker): the request is then dropped as cancelled.
fn start_pick(
    state: &Arc<PickState>,
    bridge: &AndroidFileBridge,
    request: PickRequest,
) -> Option<Receiver<Vec<String>>> {
    let id = state.next_id();
    let (tx, rx) = mpsc::channel();
    let launch = match request {
        PickRequest::Open(pick) => {
            let mime = mime_for_filters(&pick);
            state.insert(id, tx);
            bridge.request_open(id, false, &mime)
        }
        PickRequest::OpenMany => {
            state.insert(id, tx);
            bridge.request_open(id, true, "*/*")
        }
        PickRequest::Save(pick) => {
            let name = if pick.name.trim().is_empty() {
                "Untitled.svg".to_string()
            } else {
                pick.name.clone()
            };
            let mime = mime_for_name(&name);
            state.insert(id, tx);
            bridge.request_save(id, &name, &mime)
        }
        PickRequest::Folder => return None,
    };
    if let Err(error) = launch {
        state.remove(id);
        log::error!("Could not start the Android file picker: {error}");
        return None;
    }
    Some(rx)
}

/// Each frame: read every picker answer Android queued and send it to the request that asked.
fn poll_answers(state: &Arc<PickState>, bridge: &AndroidFileBridge, ctx: &egui::Context) {
    // Each answer is one short JSON object queued by VectorCraftActivity after picker I/O ends.
    for _ in 0..8 {
        let raw = match bridge.poll_result() {
            Ok(Some(raw)) => raw,
            Ok(None) => break,
            Err(error) => {
                log::error!("Android file picker polling failed: {error}");
                break;
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(error) => {
                log::error!("Invalid Android picker result: {error}: {raw}");
                continue;
            }
        };
        let Some(id) = value.get("id").and_then(serde_json::Value::as_u64) else {
            log::error!("Android picker result has no request id: {raw}");
            continue;
        };
        let Some(sender) = state.remove(id) else {
            log::warn!("Ignoring stale Android picker reply {id}");
            continue;
        };
        let status = value
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("error");
        let paths: Vec<String> = match status {
            "cancel" => Vec::new(),
            "ok" => value
                .get("paths")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect(),
            _ => {
                log::error!(
                    "Android file picker failed: {}",
                    value
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown error")
                );
                Vec::new()
            }
        };
        let _ = sender.send(paths);
        ctx.request_repaint();
    }
}

/// Write `bytes` to `path`, then copy them to the selected SAF document when `path` is staged.
fn write_mobile(bridge: &AndroidFileBridge, path: &str, bytes: &[u8]) -> Result<(), String> {
    let target = Path::new(path);
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    fs::write(target, bytes).map_err(|e| format!("Could not write {}: {e}", target.display()))?;
    match bridge.commit_if_mapped(path)? {
        state if state == "NONE" || state == "OK" => Ok(()),
        state if state.starts_with("ERROR:") => Err(state.trim_start_matches("ERROR:").to_owned()),
        state => Err(format!("Unexpected Android save result: {state}")),
    }
}

/// Reconstruct egui's modifier state on Android.
///
/// winit's Android backend delivers `KeyboardInput` events but never emits `ModifiersChanged`
/// (it discards the `KeyEvent`'s meta state), so egui's modifier state stays empty and
/// `Ctrl`/`Alt` shortcuts are ignored while plain keys continue to work. Physical modifier keys
/// do arrive as `Event::Key` events (`Key::ControlLeft`, `Key::AltLeft`, ...), so we track them
/// and stamp the resulting state onto this frame's events. This mirrors `egui-winit`'s
/// `ModifiersChanged` handling and suppresses the text character that would otherwise be emitted
/// alongside a Ctrl/Command shortcut.
fn apply_android_modifiers(state: &mut egui::Modifiers, raw: &mut egui::RawInput) {
    let before = *state;
    let mut kept: Vec<egui::Event> = Vec::with_capacity(raw.events.len());
    for mut event in raw.events.drain(..) {
        match &mut event {
            egui::Event::Key {
                key,
                physical_key,
                pressed,
                modifiers,
                ..
            } => {
                let tracked = physical_key.unwrap_or(*key);
                match tracked {
                    egui::Key::ShiftLeft | egui::Key::ShiftRight => state.shift = *pressed,
                    egui::Key::ControlLeft | egui::Key::ControlRight => state.ctrl = *pressed,
                    egui::Key::AltLeft | egui::Key::AltRight => state.alt = *pressed,
                    _ => {}
                }
                state.mac_cmd = false;
                state.command = state.ctrl;
                *modifiers = *state;
                kept.push(event);
            }
            egui::Event::PointerButton { modifiers, .. } => {
                *modifiers = *state;
                kept.push(event);
            }
            egui::Event::MouseWheel { modifiers, .. } => {
                *modifiers = *state;
                kept.push(event);
            }
            egui::Event::Text(_) if state.ctrl || state.command || state.mac_cmd => {
                // Suppress text emitted with a modifier chord.
            }
            egui::Event::WindowFocused(false) => {
                *state = egui::Modifiers::default();
                kept.push(event);
            }
            _ => kept.push(event),
        }
    }
    raw.events = kept;
    if *state != before {
        // `RawInput` has no modifier field in this egui version: `InputState.modifiers` is driven
        // purely by this event (plus its own per-event copies), so it must be emitted on change.
        raw.events.push(egui::Event::ModifiersChanged(*state));
    }
}

fn services(bridge: AndroidFileBridge, state: Arc<PickState>) -> Services {
    let write_bridge = bridge.clone_for_services();
    let pick_bridge = bridge.clone_for_services();
    let pick_state = state.clone();
    Services {
        // A staged document is an ordinary file in app-private storage.
        read: Some(Box::new(|path: &str| {
            fs::read(path).map_err(|e| format!("Could not read {path}: {e}"))
        })),
        write: Some(Box::new(move |path: &str, bytes: &[u8]| {
            write_mobile(&write_bridge, path, bytes)
        })),
        // Shown off the UI thread, so `ask` never calls these; they make the dialogs count as
        // available (`picks::can`).
        pick_open: Some(Box::new(|_pick: &FilePick| None)),
        pick_save: Some(Box::new(|_pick: &FilePick| None)),
        pick_open_multi: Some(Box::new(Vec::new)),
        pick_folder: Some(Box::new(|| None)),
        start_pick: Some(Box::new(move |request: PickRequest| {
            start_pick(&pick_state, &pick_bridge, request)
        })),
        ..Services::default()
    }
}

struct VectorCraftAndroidApp {
    inner: VectorcraftApp,
    bridge: AndroidFileBridge,
    picks: Arc<PickState>,
    modifiers: egui::Modifiers,
}

impl eframe::App for VectorCraftAndroidApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Deliver picker answers before the app's own logic runs, so `picks::poll` resumes the
        // entry that asked in this same frame.
        poll_answers(&self.picks, &self.bridge, ctx);
        self.inner.logic(ctx);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        apply_android_modifiers(&mut self.modifiers, raw);
        self.inner.raw_input_hook(raw);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.inner.ui(ui);
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: AndroidApp) {
    log::info!("{SAF_BRIDGE_MARKER} ready; {ANDROID_SHELL_MARKER}");
    let bridge = match AndroidFileBridge::new(android_app.clone()) {
        Ok(bridge) => bridge,
        Err(error) => {
            log::error!("VectorCraft Android startup failed: {error}");
            return;
        }
    };
    if let Ok(dir) = bridge.files_dir() {
        log::info!("VectorCraft app-private storage: {dir}");
    }
    let picks = Arc::new(PickState::default());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VectorCraft")
            .with_drag_and_drop(true),
        android_app: Some(android_app),
        ..Default::default()
    };
    let app_bridge = bridge.clone_for_services();
    let app_picks = picks.clone();
    let result = eframe::run_native(
        "VectorCraft",
        options,
        Box::new(move |_cc| {
            let app = VectorcraftApp::new(
                Session::new(),
                services(app_bridge.clone_for_services(), app_picks.clone()),
            );
            Ok(Box::new(VectorCraftAndroidApp {
                inner: app,
                bridge: app_bridge.clone_for_services(),
                picks: app_picks.clone(),
                modifiers: egui::Modifiers::default(),
            }))
        }),
    );
    if let Err(error) = result {
        log::error!("VectorCraft Android exited with an error: {error}");
    }
}

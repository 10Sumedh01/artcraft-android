//! Android system fonts, shared by every generated launcher and by the hand-written adapter
//! launchers.
//!
//! The desktop builds render every script with the fonts the operating system ships. On Android
//! those fonts live in `/system/fonts` (and `/product/fonts`), which none of the desktop font
//! helpers look at, so non-Latin text came out as blank boxes ("tofu"). This module closes that gap
//! in two ways:
//!
//! * [`install_system_fonts`] hands epaint the Android faces for the scripts the app's own fonts do
//!   not cover, as *lowest*-priority fallbacks, so they are only consulted for glyphs nothing else
//!   has. It is called every frame (cheaply) because an app that installs its own fonts replaces the
//!   whole definition set and would otherwise drop the fallbacks.
//! * [`install_font_env`] links the system font folder into the per-user locations the apps' own
//!   font scanners (`wordcraft`, `gridcraft`, `deckcraft`, `designcraft`, `vectorcraft`,
//!   `pdfcraft`, …) expect on a desktop, and writes a minimal fontconfig file for the ones that read
//!   one. This makes their document/palette rendering see the same fonts as the interface.
//!
//! Everything is best-effort and lazy: a missing, unreadable or already-covered font is skipped, and
//! a font is only read from disk the first time it is actually needed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// Directories Android keeps its system fonts in, in the order they are preferred.
const FONT_DIRS: &[&str] = &["/system/fonts", "/product/fonts"];

/// One non-Latin script worth falling back to, its probe character and the system font files that
/// draw it (best candidate first, with the face index for `.ttc` collections).
struct SystemScript {
    /// The epaint font key. Also used to tell our fallbacks from the app's own fonts.
    key: &'static str,
    /// A letter only this script uses. When the active font chain can already draw it, the script is
    /// left to the app's own fonts and no system face is installed.
    probe: char,
    /// `(file name, face index)` candidates, in preference order.
    candidates: &'static [(&'static str, u32)],
}

/// The scripts Android ships fonts for. The probes are letters the embedded Latin faces cannot
/// draw, so a script is only loaded when it is actually missing. OEM font sets vary, so each script
/// lists the several names the same face is shipped under (AOSP `Noto*`, the older `Droid*`, and the
/// per-region CJK files).
const SYSTEM_SCRIPTS: &[SystemScript] = &[
    SystemScript {
        key: "artcraft-cjk",
        probe: '中',
        candidates: &[
            ("NotoSansCJK-Regular.ttc", 0),
            ("NotoSansCJKjp-Regular.otf", 0),
            ("NotoSansHans-Regular.otf", 0),
            ("NotoSansCJKsc-Regular.otf", 0),
            ("DroidSansFallback.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-hangul",
        probe: '가',
        candidates: &[
            ("NotoSansCJKkr-Regular.otf", 0),
            ("NotoSansKR-Regular.otf", 0),
            ("NotoSansKR-VF.ttf", 0),
            ("NotoSansCJK-Regular.ttc", 1),
            ("DroidSansFallback.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-emoji",
        probe: '😀',
        candidates: &[
            ("NotoColorEmoji.ttf", 0),
            ("NotoColorEmojiCompat.ttf", 0),
            ("NotoColorEmojiLegacy.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-arabic",
        probe: 'ا',
        candidates: &[
            ("NotoSansArabic-Regular.ttf", 0),
            ("NotoSansArabicUI-Regular.ttf", 0),
            ("NotoNaskhArabic-Regular.ttf", 0),
            ("DroidSansArabic.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-hebrew",
        probe: 'א',
        candidates: &[
            ("NotoSansHebrew-Regular.ttf", 0),
            ("NotoSansHebrewUI-Regular.ttf", 0),
            ("DroidSansHebrew-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-devanagari",
        probe: 'क',
        candidates: &[
            ("NotoSansDevanagari-Regular.ttf", 0),
            ("NotoSansDevanagariUI-Regular.ttf", 0),
            ("DroidSansDevanagari-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-bengali",
        probe: 'ক',
        candidates: &[
            ("NotoSansBengali-Regular.ttf", 0),
            ("NotoSansBengaliUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-tamil",
        probe: 'க',
        candidates: &[
            ("NotoSansTamil-Regular.ttf", 0),
            ("NotoSansTamilUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-telugu",
        probe: 'క',
        candidates: &[
            ("NotoSansTelugu-Regular.ttf", 0),
            ("NotoSansTeluguUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-kannada",
        probe: 'ಕ',
        candidates: &[
            ("NotoSansKannada-Regular.ttf", 0),
            ("NotoSansKannadaUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-malayalam",
        probe: 'ക',
        candidates: &[
            ("NotoSansMalayalam-Regular.ttf", 0),
            ("NotoSansMalayalamUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-gujarati",
        probe: 'ક',
        candidates: &[
            ("NotoSansGujarati-Regular.ttf", 0),
            ("NotoSansGujaratiUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-gurmukhi",
        probe: 'ਕ',
        candidates: &[
            ("NotoSansGurmukhi-Regular.ttf", 0),
            ("NotoSansGurmukhiUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-oriya",
        probe: 'କ',
        candidates: &[
            ("NotoSansOriya-Regular.ttf", 0),
            ("NotoSansOriyaUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-sinhala",
        probe: 'ක',
        candidates: &[
            ("NotoSansSinhala-Regular.ttf", 0),
            ("NotoSansSinhalaUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-thai",
        probe: 'ก',
        candidates: &[
            ("NotoSansThai-Regular.ttf", 0),
            ("NotoSansThaiUI-Regular.ttf", 0),
            ("DroidSansThai.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-lao",
        probe: 'ກ',
        candidates: &[
            ("NotoSansLao-Regular.ttf", 0),
            ("NotoSansLaoUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-khmer",
        probe: 'ក',
        candidates: &[
            ("NotoSansKhmer-Regular.ttf", 0),
            ("NotoSansKhmerUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-myanmar",
        probe: 'က',
        candidates: &[
            ("NotoSansMyanmar-Regular.ttf", 0),
            ("NotoSansMyanmarUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-ethiopic",
        probe: 'ሀ',
        candidates: &[
            ("NotoSansEthiopic-Regular.ttf", 0),
            ("NotoSansEthiopicUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-georgian",
        probe: 'ა',
        candidates: &[
            ("NotoSansGeorgian-Regular.ttf", 0),
            ("NotoSansGeorgianUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-armenian",
        probe: 'Ա',
        candidates: &[
            ("NotoSansArmenian-Regular.ttf", 0),
            ("NotoSansArmenianUI-Regular.ttf", 0),
        ],
    },
    SystemScript {
        key: "artcraft-symbols",
        probe: '✓',
        candidates: &[
            ("NotoSansSymbols-Regular.ttf", 0),
            ("NotoSansSymbols2-Regular.ttf", 0),
        ],
    },
];

/// Whether `key` names one of our system faces for `script`: `script.key` itself, or a later
/// generation `script.key#n` (see [`install_system_fonts`]).
fn is_our_face(key: &str, script: &SystemScript) -> bool {
    key == script.key || key.strip_prefix(script.key).is_some_and(|rest| rest.starts_with('#'))
}

/// Bumped each time our faces are re-appended for families that appeared after the last install, so
/// the same face can be added again under a fresh key (`Context::add_font` skips a key it already
/// holds, which is exactly what would keep a family born later from ever receiving the fallback).
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Install any missing system fallbacks into every active font family.
///
/// Called from the app's `ui` (inside an egui pass, where fonts exist) each frame: it is one cheap
/// lookup per family when everything is already in place, and it *repairs* the definition set after
/// an app replaces it wholesale (`Context::set_fonts`).
///
/// Presence in `font_data` is *not* enough to decide there is nothing to do. An app that draws with
/// its own families (`FontFamily::Name("cell-…")`, as GridCraft does) builds them in a `set_fonts`
/// call whose definitions are applied on the *next* pass, so our faces are first appended to the
/// families that already existed and `Context::add_font` then refuses to re-add them to the
/// families that appear afterwards — leaving every non-Latin glyph drawn in those families a blank
/// box. We therefore check *coverage per family* and, when one is missing a face, append it under a
/// fresh generation key. Generations share the same borrowed font bytes, and a key no family
/// references is never rasterized, so this costs no measurable memory.
pub fn install_system_fonts(ctx: &egui::Context) {
    // The families bound this pass, and, per script, those whose chain carries none of our faces.
    let (families, missing) = ctx.fonts(|view| {
        let definitions = view.definitions();
        let families: Vec<egui::FontFamily> = view.families();
        let missing: Vec<Vec<usize>> = SYSTEM_SCRIPTS
            .iter()
            .map(|script| {
                families
                    .iter()
                    .enumerate()
                    .filter(|(_, family)| {
                        !definitions
                            .families
                            .get(*family)
                            .is_some_and(|chain| chain.iter().any(|key| is_our_face(key, script)))
                    })
                    .map(|(index, _)| index)
                    .collect()
            })
            .collect();
        (families, missing)
    });
    if families.is_empty() || missing.iter().all(Vec::is_empty) {
        return;
    }

    let mut added = false;
    for (script, missing) in SYSTEM_SCRIPTS.iter().zip(&missing) {
        if missing.is_empty() {
            continue;
        }
        let Some(data) = script_font(script) else { continue };
        // Append as the last fallback, so an app's own faces always win and only glyphs they lack
        // come from the system face.
        let inserts = missing
            .iter()
            .map(|&index| egui::epaint::text::InsertFontFamily {
                family: families[index].clone(),
                priority: egui::epaint::text::FontPriority::Lowest,
            })
            .collect();
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
        let key = format!("{}#{generation}", script.key);
        ctx.add_font(egui::epaint::text::FontInsert::new(&key, data, inserts));
        log::info!(
            "Android: added system fallback font {} (U+{:04X}) to {} families that lacked it",
            script.key,
            script.probe as u32,
            missing.len()
        );
        added = true;
    }
    if added {
        // `add_font` only queues the change; egui applies it at the start of the next pass, so ask
        // for that pass rather than waiting for the next input event.
        ctx.request_repaint();
    }
}

/// The font for `script`, read from disk the first time it is asked for (`None` when the device
/// ships no usable file for it).
fn script_font(script: &'static SystemScript) -> Option<egui::FontData> {
    static READ: OnceLock<Mutex<HashMap<&'static str, Option<egui::FontData>>>> = OnceLock::new();
    let mut read = READ
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(cached) = read.get(script.key) {
        return cached.clone();
    }

    let font = script.candidates.iter().find_map(|&(name, index)| {
        let path = font_path(name)?;
        let bytes = file_bytes(&path)?;
        // Guard the face index so an out-of-range one is skipped instead of panicking epaint.
        if index >= font_face_count(bytes) {
            return None;
        }
        let mut data = egui::FontData::from_static(bytes);
        data.index = index;
        Some(data)
    });
    if font.is_none() {
        log::info!("Android: no system font found for {}", script.key);
    }
    read.insert(script.key, font.clone());
    font
}

/// The bytes of a system font file, read once per path (several scripts may name the same
/// collection) and kept for the life of the process.
fn file_bytes(path: &Path) -> Option<&'static [u8]> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<&'static [u8]>>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(cached) = cache.get(path) {
        return *cached;
    }

    let bytes = match std::fs::read(path) {
        Ok(bytes) if looks_like_font(&bytes) => {
            let leaked: &'static mut [u8] = Box::leak(bytes.into_boxed_slice());
            Some(&*leaked)
        }
        Ok(_) => {
            log::warn!("Skipping system font {}: not a font file", path.display());
            None
        }
        Err(error) => {
            log::warn!("Could not read system font {}: {error}", path.display());
            None
        }
    };
    cache.insert(path.to_path_buf(), bytes);
    bytes
}

/// Where a system font file lives, if the device ships it.
fn font_path(name: &str) -> Option<PathBuf> {
    FONT_DIRS
        .iter()
        .map(|dir| Path::new(dir).join(name))
        .find(|path| path.is_file())
}

/// Whether `bytes` starts like a font epaint can parse. `Context::add_font` panics on data it cannot
/// parse, so a damaged or unexpected system file is skipped rather than allowed to crash the app.
fn looks_like_font(bytes: &[u8]) -> bool {
    matches!(
        bytes.first_chunk::<4>(),
        Some([0x00, 0x01, 0x00, 0x00]) // TrueType outlines
            | Some([b'O', b'T', b'T', b'O']) // CFF/OpenType outlines
            | Some([b't', b't', b'c', b'f']) // font collection
            | Some([b't', b'r', b'u', b'e']) // legacy TrueType
    )
}

/// How many faces a font file holds (`1` for a single-face font, the collection's count for a
/// `.ttc`).
fn font_face_count(bytes: &[u8]) -> u32 {
    if bytes.starts_with(b"ttcf") {
        bytes
            .get(8..12)
            .and_then(|count| <[u8; 4]>::try_from(count).ok())
            .map(u32::from_be_bytes)
            .unwrap_or(1)
    } else {
        1
    }
}

/// Make Android's system fonts visible to the app's own font scanners.
///
/// Their desktop code looks under the user's home (`$HOME/.fonts`, `$HOME/.local/share/fonts`) and
/// the XDG data directory (`$XDG_DATA_HOME/fonts`), none of which exist on Android. Symlinking the
/// system font folder in under those names — and writing a minimal `fonts.conf` for the fontconfig
/// readers — is enough for the existing scanners, without editing any app.
///
/// The links can only be created on **internal** storage: Android's emulated external storage
/// (`/storage/emulated/0/...`, a FUSE/sdcardfs mount) refuses `symlink` with `EPERM`. Creating them
/// under the external app directory therefore left `$HOME/.fonts` and friends missing, and the
/// scanners found no system face for Arabic, Hebrew, CJK and the other non-Latin scripts (blank
/// boxes). Fonts are not user data, so this environment stays in the internal private directory even
/// though the settings, working copies and exports live on external storage.
pub fn install_font_env(app: &android_activity::AndroidApp) {
    let Some(base) = app.internal_data_path() else { return };
    let home = base.join("home");
    let _ = std::fs::create_dir_all(home.join(".local/share"));

    #[cfg(unix)]
    for link in [
        home.join(".fonts"),
        home.join(".local/share/fonts"),
        base.join("fonts"),
    ] {
        if let Some(parent) = link.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if !link.exists() {
            let _ = std::os::unix::fs::symlink("/system/fonts", &link);
        }
    }

    let conf_dir = base.join("fontconfig");
    let _ = std::fs::create_dir_all(conf_dir.join("cache"));
    let conf = conf_dir.join("fonts.conf");
    if !conf.exists() {
        let dirs: String = FONT_DIRS.iter().map(|dir| format!("<dir>{dir}</dir>")).collect();
        let cache = conf_dir.join("cache");
        let xml = format!(
            "<?xml version=\"1.0\"?>\n<!DOCTYPE fontconfig SYSTEM \"fonts.dtd\">\n\
             <fontconfig>\n<cachedir>{}</cachedir>\n{dirs}\n</fontconfig>\n",
            cache.display()
        );
        let _ = std::fs::write(&conf, xml);
    }

    // SAFETY: `android_main` runs before the app starts any worker thread, so no other thread is
    // reading the environment here. `HOME`/`XDG_DATA_HOME` are the roots the apps' font scanners
    // read, so they are pointed at the internal links above *unconditionally*: the app-data redirect
    // (`install_persist_env`) may already have chosen external storage for the settings and caches,
    // and external storage cannot hold these links.
    #[allow(unused_unsafe)]
    unsafe {
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_DATA_HOME", &base);
        std::env::set_var("FONTCONFIG_FILE", &conf);
        std::env::set_var("FONTCONFIG_PATH", &conf_dir);
    }
}

//! Shared mobile touch overlay and virtual accessory toolbar for ArtCraft Android apps.
//!
//! Provides a floating, draggable mobile dock containing:
//! - Soft keyboard summon button (IME trigger)
//! - Virtual modifier toggles (`Ctrl`, `Alt`, `Shift`, `Space` for canvas pan)
//! - One-shot right-click mode (transforms next tap into secondary click)
//! - Quick action buttons (`Undo`, `Redo`, `Esc`, `Tab`, `Enter`)

#[derive(Clone, Debug)]
pub struct MobileOverlay {
    pub expanded: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub space_pan: bool,
    pub right_click_mode: bool,
    pub keyboard_locked: bool,
    pub pos: egui::Pos2,
    pub pos_initialized: bool,
    pub overlay_rect: egui::Rect,
    queued_keys: Vec<(egui::Key, egui::Modifiers)>,
    queued_zoom: Option<f32>,
}

impl Default for MobileOverlay {
    fn default() -> Self {
        Self {
            expanded: false,
            ctrl: false,
            alt: false,
            shift: false,
            space_pan: false,
            right_click_mode: false,
            keyboard_locked: false,
            pos: egui::pos2(16.0, 16.0),
            pos_initialized: false,
            overlay_rect: egui::Rect::NOTHING,
            queued_keys: Vec::new(),
            queued_zoom: None,
        }
    }
}

impl MobileOverlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process raw input each frame: applies virtual modifiers, handles right-click translation,
    /// space/pan tool behavior, and drains queued synthetic keys.
    pub fn process_raw_input(&mut self, raw: &mut egui::RawInput) {
        let mut active_mods = egui::Modifiers::default();
        if self.ctrl {
            active_mods.ctrl = true;
            active_mods.command = true;
        }
        if self.shift {
            active_mods.shift = true;
        }
        if self.alt {
            active_mods.alt = true;
        }

        // Apply active virtual modifiers to key and pointer events
        if self.ctrl || self.shift || self.alt {
            for event in &mut raw.events {
                match event {
                    egui::Event::Key { modifiers, .. } => {
                        if self.ctrl {
                            modifiers.ctrl = true;
                            modifiers.command = true;
                        }
                        if self.shift {
                            modifiers.shift = true;
                        }
                        if self.alt {
                            modifiers.alt = true;
                        }
                    }
                    egui::Event::PointerButton { modifiers, .. } => {
                        if self.ctrl {
                            modifiers.ctrl = true;
                            modifiers.command = true;
                        }
                        if self.shift {
                            modifiers.shift = true;
                        }
                        if self.alt {
                            modifiers.alt = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        // When space_pan is active, touching and dragging on the canvas acts as the Pan/Hand tool.
        if self.space_pan {
            for event in &raw.events {
                if let egui::Event::PointerButton { pressed, pos, .. } = event {
                    if !self.overlay_rect.contains(*pos) {
                        raw.events.push(egui::Event::Key {
                            key: egui::Key::Space,
                            physical_key: None,
                            pressed: *pressed,
                            repeat: false,
                            modifiers: active_mods,
                        });
                        break;
                    }
                }
            }
        }

        // Right-Click Mode: convert the next primary click outside the overlay area to secondary click.
        if self.right_click_mode {
            let mut reset_mode = false;
            for event in &mut raw.events {
                if let egui::Event::PointerButton { button, pos, pressed, .. } = event {
                    if !self.overlay_rect.contains(*pos) {
                        if *button == egui::PointerButton::Primary {
                            *button = egui::PointerButton::Secondary;
                        }
                        if !*pressed {
                            reset_mode = true;
                        }
                    }
                }
            }
            if reset_mode {
                self.right_click_mode = false;
            }
        }

        // Drain any synthetic key events queued from button clicks (Undo, Redo, Esc, Tab, Enter)
        for (key, mods) in self.queued_keys.drain(..) {
            raw.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: mods,
            });
            raw.events.push(egui::Event::Key {
                key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: mods,
            });
        }

        // Apply any synthetic zoom factors queued from quick zoom buttons
        if let Some(factor) = self.queued_zoom.take() {
            raw.events.push(egui::Event::Zoom(factor));
        }
    }

    /// Render the floating mobile touch overlay toolbar.
    pub fn ui(&mut self, ctx: &egui::Context, ime: &crate::ime_bridge::ime::ImeBridge) {
        // When keyboard lock is active, suppress any automatic IME output requests
        // from egui (e.g. from canvas focus or accidental clicks on screen)
        if self.keyboard_locked {
            ctx.output_mut(|o| o.ime = None);
        }

        let screen_rect = ctx.content_rect();
        if !self.pos_initialized {
            // Default position near bottom-left, safely away from system bars
            self.pos = egui::pos2(16.0, (screen_rect.max.y - 100.0).max(16.0));
            self.pos_initialized = true;
        }

        let area_id = egui::Id::new("artcraft_mobile_touch_overlay");
        let area = egui::Area::new(area_id)
            .order(egui::Order::Foreground)
            .movable(true)
            .current_pos(self.pos);

        let response = area.show(ctx, |ui| {
            egui::Frame::default()
                .fill(egui::Color32::from_rgba_premultiplied(24, 28, 36, 235))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(50)))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    if !self.expanded {
                        // Collapsed floating pill (shows lock status when active)
                        let pill_text = if self.keyboard_locked {
                            "📱 Touch 🔒"
                        } else {
                            "📱 Touch"
                        };
                        let pill_bg = if self.keyboard_locked {
                            egui::Color32::from_rgb(160, 45, 45)
                        } else {
                            egui::Color32::from_white_alpha(35)
                        };
                        let btn = egui::Button::new(
                            egui::RichText::new(pill_text)
                                .size(13.0)
                                .color(egui::Color32::WHITE)
                                .strong(),
                        )
                        .min_size(egui::vec2(68.0, 36.0))
                        .fill(pill_bg);
                        if ui.add(btn).clicked() {
                            self.expanded = true;
                        }
                    } else {
                        // Expanded touch accessory toolbar
                        ui.horizontal_wrapped(|ui| {
                            // Collapse button
                            if ui.add(
                                egui::Button::new(egui::RichText::new("✕").size(14.0).color(egui::Color32::LIGHT_GRAY))
                                    .min_size(egui::vec2(28.0, 34.0))
                            ).clicked() {
                                self.expanded = false;
                            }

                            // Force Soft Keyboard show button (always shows keyboard when tapped)
                            if ui.add(
                                egui::Button::new(egui::RichText::new("⌨ Keyboard").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(84.0, 34.0))
                                    .fill(egui::Color32::from_rgb(45, 95, 175))
                            ).clicked() {
                                let _ = ime.show_ime_forced();
                            }

                            // Keyboard Lock button: suppresses accidental screen clicks from opening the keyboard
                            let lock_color = if self.keyboard_locked {
                                egui::Color32::from_rgb(180, 45, 45)
                            } else {
                                egui::Color32::from_white_alpha(30)
                            };
                            let lock_label = if self.keyboard_locked { "🔒 Lock: ON" } else { "🔓 Lock: OFF" };
                            if ui.add(
                                egui::Button::new(egui::RichText::new(lock_label).size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(82.0, 34.0))
                                    .fill(lock_color)
                            ).clicked() {
                                self.keyboard_locked = !self.keyboard_locked;
                                let _ = ime.set_locked(self.keyboard_locked);
                                if self.keyboard_locked {
                                    let _ = ime.hide_ime();
                                }
                            }

                            // Quick Zoom Buttons
                            if ui.add(
                                egui::Button::new(egui::RichText::new("🔍+").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(38.0, 34.0))
                                    .fill(egui::Color32::from_white_alpha(30))
                            ).clicked() {
                                self.queued_zoom = Some(1.2);
                            }

                            if ui.add(
                                egui::Button::new(egui::RichText::new("🔍-").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(38.0, 34.0))
                                    .fill(egui::Color32::from_white_alpha(30))
                            ).clicked() {
                                self.queued_zoom = Some(0.833);
                            }

                            ui.separator();

                            // Modifier buttons (toggleable)
                            let ctrl_color = if self.ctrl { egui::Color32::from_rgb(50, 160, 90) } else { egui::Color32::from_white_alpha(30) };
                            if ui.add(
                                egui::Button::new(egui::RichText::new("Ctrl").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(44.0, 34.0))
                                    .fill(ctrl_color)
                            ).clicked() {
                                self.ctrl = !self.ctrl;
                            }

                            let alt_color = if self.alt { egui::Color32::from_rgb(50, 160, 90) } else { egui::Color32::from_white_alpha(30) };
                            if ui.add(
                                egui::Button::new(egui::RichText::new("Alt").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(40.0, 34.0))
                                    .fill(alt_color)
                            ).clicked() {
                                self.alt = !self.alt;
                            }

                            let shift_color = if self.shift { egui::Color32::from_rgb(50, 160, 90) } else { egui::Color32::from_white_alpha(30) };
                            if ui.add(
                                egui::Button::new(egui::RichText::new("Shift").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(48.0, 34.0))
                                    .fill(shift_color)
                            ).clicked() {
                                self.shift = !self.shift;
                            }

                            let pan_color = if self.space_pan { egui::Color32::from_rgb(200, 120, 30) } else { egui::Color32::from_white_alpha(30) };
                            if ui.add(
                                egui::Button::new(egui::RichText::new("✋ Pan").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(52.0, 34.0))
                                    .fill(pan_color)
                            ).clicked() {
                                self.space_pan = !self.space_pan;
                            }

                            let rclick_color = if self.right_click_mode { egui::Color32::from_rgb(210, 60, 60) } else { egui::Color32::from_white_alpha(30) };
                            if ui.add(
                                egui::Button::new(egui::RichText::new("🖱 R-Click").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(68.0, 34.0))
                                    .fill(rclick_color)
                            ).clicked() {
                                self.right_click_mode = !self.right_click_mode;
                            }

                            ui.separator();

                            // Action shortcuts
                            if ui.add(
                                egui::Button::new(egui::RichText::new("↶ Undo").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(56.0, 34.0))
                                    .fill(egui::Color32::from_white_alpha(30))
                            ).clicked() {
                                self.queued_keys.push((egui::Key::Z, egui::Modifiers { ctrl: true, ..Default::default() }));
                            }

                            if ui.add(
                                egui::Button::new(egui::RichText::new("↷ Redo").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(56.0, 34.0))
                                    .fill(egui::Color32::from_white_alpha(30))
                            ).clicked() {
                                self.queued_keys.push((egui::Key::Y, egui::Modifiers { ctrl: true, ..Default::default() }));
                            }

                            if ui.add(
                                egui::Button::new(egui::RichText::new("Esc").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(38.0, 34.0))
                                    .fill(egui::Color32::from_white_alpha(30))
                            ).clicked() {
                                self.queued_keys.push((egui::Key::Escape, egui::Modifiers::default()));
                            }

                            if ui.add(
                                egui::Button::new(egui::RichText::new("Tab").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(38.0, 34.0))
                                    .fill(egui::Color32::from_white_alpha(30))
                            ).clicked() {
                                self.queued_keys.push((egui::Key::Tab, egui::Modifiers::default()));
                            }

                            if ui.add(
                                egui::Button::new(egui::RichText::new("Enter").size(13.0).color(egui::Color32::WHITE))
                                    .min_size(egui::vec2(44.0, 34.0))
                                    .fill(egui::Color32::from_white_alpha(30))
                            ).clicked() {
                                self.queued_keys.push((egui::Key::Enter, egui::Modifiers::default()));
                            }
                        });
                    }
                });
        });

        self.overlay_rect = response.response.rect;
        self.pos = response.response.rect.min;
        let margin = 10.0;
        self.pos.x = self.pos.x.clamp(margin, (screen_rect.max.x - response.response.rect.width() - margin).max(margin));
        self.pos.y = self.pos.y.clamp(margin, (screen_rect.max.y - response.response.rect.height() - margin).max(margin));
    }
}

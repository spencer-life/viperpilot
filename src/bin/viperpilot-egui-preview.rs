#![cfg_attr(windows, windows_subsystem = "windows")]

//! Hardware-free exploration of an egui settings editor.
//! All state in this binary is invented in memory. It is not connected to the
//! production tray, profile storage, device worker, or HID interfaces.

use eframe::egui::{self, Color32, RichText, Stroke, Vec2, Visuals};

const ROSE: Color32 = Color32::from_rgb(255, 91, 151);
const ROSE_DARK: Color32 = Color32::from_rgb(105, 39, 67);
const CANVAS: Color32 = Color32::from_rgb(19, 20, 25);
const PANEL: Color32 = Color32::from_rgb(28, 30, 37);
const PANEL_RAISED: Color32 = Color32::from_rgb(38, 40, 49);
const MUTED: Color32 = Color32::from_rgb(156, 160, 174);
const GREEN: Color32 = Color32::from_rgb(107, 210, 165);

const POLLING_OPTIONS: [u16; 7] = [125, 250, 500, 1_000, 2_000, 4_000, 8_000];
const MAPPING_OPTIONS: [&str; 8] = [
    "Disabled",
    "Left click",
    "Right click",
    "Middle click",
    "Back",
    "Forward",
    "Keyboard key (intent)",
    "Media action (intent)",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewMode {
    Quick,
    Detailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ButtonMappingDraft {
    button: &'static str,
    action: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EditorDraft {
    profile_name: String,
    dpi_stages: Vec<u16>,
    active_stage: usize,
    polling_hz: u16,
    lift_off: String,
    sleep_minutes: u16,
    mappings: Vec<ButtonMappingDraft>,
    view_mode: ViewMode,
}

impl Default for EditorDraft {
    fn default() -> Self {
        Self {
            profile_name: "My Profile".to_owned(),
            dpi_stages: vec![800, 1_600, 3_200],
            active_stage: 1,
            polling_hz: 1_000,
            lift_off: "Standard (intent)".to_owned(),
            sleep_minutes: 5,
            mappings: vec![
                ButtonMappingDraft {
                    button: "Primary",
                    action: "Left click".to_owned(),
                },
                ButtonMappingDraft {
                    button: "Secondary",
                    action: "Right click".to_owned(),
                },
                ButtonMappingDraft {
                    button: "Wheel click",
                    action: "Middle click".to_owned(),
                },
                ButtonMappingDraft {
                    button: "Mouse4 / Side button 1",
                    action: "Back".to_owned(),
                },
                ButtonMappingDraft {
                    button: "Mouse5 / Side button 2",
                    action: "Forward".to_owned(),
                },
                ButtonMappingDraft {
                    button: "DPI button",
                    action: "Disabled".to_owned(),
                },
            ],
            view_mode: ViewMode::Quick,
        }
    }
}

impl EditorDraft {
    #[cfg(test)]
    fn active_dpi(&self) -> Option<u16> {
        self.dpi_stages.get(self.active_stage).copied()
    }

    fn can_apply() -> bool {
        // This prototype has no device capability evidence or write path.
        false
    }

    #[cfg(test)]
    fn change_mapping(&mut self, button: &str, action: &str) {
        if let Some(mapping) = self.mappings.iter_mut().find(|item| item.button == button) {
            mapping.action = action.to_owned();
        }
    }
}

struct EguiPreview {
    draft: EditorDraft,
}

impl EguiPreview {
    fn new(context: &eframe::CreationContext<'_>) -> Self {
        configure_theme(&context.egui_ctx);
        Self {
            draft: EditorDraft::default(),
        }
    }
}

impl eframe::App for EguiPreview {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        preview_top_bar(ui);
        preview_navigation(ui);
        editor_content(ui, &mut self.draft);
    }
}

fn preview_top_bar(ui: &mut egui::Ui) {
    egui::Panel::top("top_bar")
        .frame(
            egui::Frame::new()
                .fill(PANEL)
                .inner_margin(egui::Margin::symmetric(22, 15)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("V").strong().size(23.0).color(ROSE));
                ui.vertical(|ui| {
                    ui.label(RichText::new("ViperPilot").strong().size(16.0));
                    ui.label(
                        RichText::new("SETTINGS EDITOR PREVIEW")
                            .size(10.0)
                            .color(MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("PREVIEW - NO DEVICE WRITES")
                            .strong()
                            .size(10.0)
                            .color(ROSE),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new("Viper V4 Pro - capability unverified")
                            .size(11.0)
                            .color(MUTED),
                    );
                });
            });
        });
}

fn preview_navigation(ui: &mut egui::Ui) {
    egui::Panel::left("navigation")
        .exact_size(202.0)
        .frame(
            egui::Frame::new()
                .fill(CANVAS)
                .inner_margin(egui::Margin::symmetric(16, 20)),
        )
        .show(ui, |ui| {
            ui.label(RichText::new("DEVICE").strong().size(10.0).color(MUTED));
            ui.add_space(10.0);
            nav_item(ui, "01", "Overview", false);
            nav_item(ui, "02", "Profiles", true);
            nav_item(ui, "03", "Button mapping", false);
            nav_item(ui, "04", "Device settings", false);
            ui.add_space(22.0);
            ui.separator();
            ui.add_space(12.0);
            ui.label(
                RichText::new("LOCAL PROFILE")
                    .strong()
                    .size(10.0)
                    .color(MUTED),
            );
            ui.add_space(7.0);
            ui.label(
                RichText::new("Draft edits stay in this preview session.")
                    .size(11.0)
                    .color(MUTED),
            );
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.label(
                    RichText::new("No hardware connection")
                        .size(10.0)
                        .color(MUTED),
                );
                ui.label(
                    RichText::new("[SIM]  SIMULATED DEVICE")
                        .strong()
                        .size(10.0)
                        .color(ROSE),
                );
            });
        });
}

fn editor_content(ui: &mut egui::Ui, draft: &mut EditorDraft) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(CANVAS).inner_margin(egui::Margin::same(24)))
        .show(ui, |ui| {
            let footer_height = 82.0;
            let scroll_height = (ui.available_height() - footer_height - 14.0).max(160.0);
            egui::ScrollArea::vertical()
                .max_height(scroll_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Profiles").strong().size(26.0));
                            ui.label(RichText::new("Shape a profile around your setup. Every value here is an unverified intent.").size(12.0).color(MUTED));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.selectable_value(&mut draft.view_mode, ViewMode::Detailed, "All settings");
                            ui.selectable_value(&mut draft.view_mode, ViewMode::Quick, "Quick edit");
                        });
                    });

                    ui.add_space(18.0);
                    warning_banner(ui);
                    ui.add_space(14.0);
                    profile_card(ui, draft);
                    ui.add_space(14.0);
                    if draft.view_mode == ViewMode::Quick {
                        quick_controls(ui, draft);
                    } else {
                        detailed_controls(ui, draft);
                    }
                });
            ui.add_space(12.0);
            apply_footer(ui);
        });
}

fn configure_theme(context: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = CANVAS;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(15, 16, 20);
    visuals.faint_bg_color = PANEL_RAISED;
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(57, 44, 55);
    visuals.widgets.active.bg_fill = ROSE_DARK;
    visuals.selection.bg_fill = ROSE_DARK;
    visuals.selection.stroke = Stroke::new(1.0, ROSE);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(222, 224, 231);
    visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;
    visuals.hyperlink_color = ROSE;
    context.set_visuals(visuals);
    context.global_style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(10.0, 9.0);
        style.spacing.button_padding = Vec2::new(11.0, 7.0);
    });
}

fn nav_item(ui: &mut egui::Ui, icon: &str, title: &str, selected: bool) {
    let color = if selected { ROSE } else { MUTED };
    let frame = if selected {
        egui::Frame::new()
            .fill(ROSE_DARK)
            .stroke(Stroke::new(1.0, ROSE_DARK))
    } else {
        egui::Frame::new()
            .fill(CANVAS)
            .stroke(Stroke::new(1.0, CANVAS))
    };
    frame
        .inner_margin(egui::Margin::symmetric(10, 8))
        .corner_radius(8)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(15.0).color(color));
                ui.label(RichText::new(title).size(12.0).color(if selected {
                    Color32::WHITE
                } else {
                    MUTED
                }));
            });
        });
}

fn warning_banner(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(Color32::from_rgb(51, 35, 45))
        .stroke(Stroke::new(1.0, ROSE_DARK))
        .inner_margin(egui::Margin::symmetric(14, 11))
        .corner_radius(9)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("i").strong().size(15.0).color(ROSE));
                ui.vertical(|ui| {
                    ui.label(RichText::new("Preview values are not validated for this device").strong().size(12.0));
                    ui.label(RichText::new("You can explore the editor. Apply stays disabled until device support and safe write behavior are verified.").size(11.0).color(MUTED));
                });
            });
        });
}

fn card<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, Color32::from_rgb(53, 55, 64)))
        .inner_margin(egui::Margin::same(16))
        .corner_radius(12)
        .show(ui, add_contents)
}

fn section_header(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong().size(14.0));
            ui.label(RichText::new(detail).size(10.0).color(MUTED));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new("INTENT - UNVERIFIED")
                    .strong()
                    .size(9.0)
                    .color(ROSE),
            );
        });
    });
}

fn profile_card(ui: &mut egui::Ui, draft: &mut EditorDraft) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let name_label = ui.label(
                    RichText::new("PROFILE NAME")
                        .strong()
                        .size(10.0)
                        .color(MUTED),
                );
                ui.add_space(5.0);
                let name_field = ui.add_sized(
                    [ui.available_width().min(390.0), 34.0],
                    egui::TextEdit::singleline(&mut draft.profile_name)
                        .hint_text("Name this profile"),
                );
                name_field.clone().labelled_by(name_label.id);
                ui.ctx().accesskit_node_builder(name_field.id, |node| {
                    node.set_label("Profile name");
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("DRAFT").strong().size(10.0).color(ROSE));
            });
        });
        ui.add_space(7.0);
        ui.label(RichText::new("This label is personal to your setup; it does not change supported hardware capabilities.").size(10.0).color(MUTED));
    });
}

fn quick_controls(ui: &mut egui::Ui, draft: &mut EditorDraft) {
    card(ui, |ui| {
        section_header(
            ui,
            "Sensitivity",
            "DPI and report rate are editable preview values",
        );
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            for (index, dpi) in draft.dpi_stages.iter_mut().enumerate() {
                let selected = draft.active_stage == index;
                let response = ui.selectable_label(selected, format!("{dpi} DPI"));
                if response.clicked() {
                    draft.active_stage = index;
                }
            }
            if ui.button("+ Add stage").clicked() {
                draft.dpi_stages.push(2_400);
                draft.active_stage = draft.dpi_stages.len() - 1;
            }
            if draft.dpi_stages.len() > 1 && ui.button("Remove selected").clicked() {
                draft.dpi_stages.remove(draft.active_stage);
                draft.active_stage = draft.active_stage.min(draft.dpi_stages.len() - 1);
            }
        });
        let active_stage = draft.active_stage;
        if let Some(slot) = draft.dpi_stages.get_mut(active_stage) {
            ui.horizontal(|ui| {
                let label = ui.label("Active stage DPI");
                let slider = ui.add(
                    egui::Slider::new(slot, 100..=32_000)
                        .step_by(50.0)
                        .suffix(" DPI"),
                );
                slider.clone().labelled_by(label.id);
                ui.ctx().accesskit_node_builder(slider.id, |node| {
                    node.set_label("Active stage DPI");
                });
            });
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let label = ui.label("Polling rate");
            egui::ComboBox::from_id_salt("quick_polling_rate")
                .selected_text(format!("{} Hz (intent)", draft.polling_hz))
                .show_ui(ui, |ui| {
                    for rate in POLLING_OPTIONS {
                        ui.selectable_value(&mut draft.polling_hz, rate, format!("{rate} Hz"));
                    }
                })
                .response
                .labelled_by(label.id);
        });
    });

    ui.add_space(14.0);
    card(ui, |ui| {
        section_header(
            ui,
            "Quick button mapping",
            "Mouse4 and Mouse5 - action intents only",
        );
        ui.add_space(10.0);
        for mapping in draft.mappings.iter_mut().filter(|mapping| {
            matches!(
                mapping.button,
                "Mouse4 / Side button 1" | "Mouse5 / Side button 2"
            )
        }) {
            mapping_row(ui, mapping);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.selectable_value(&mut draft.view_mode, ViewMode::Detailed, "Edit all buttons");
        });
    });
}

fn detailed_controls(ui: &mut egui::Ui, draft: &mut EditorDraft) {
    dpi_stage_controls(ui, draft);
    ui.add_space(14.0);
    button_mapping_controls(ui, draft);
    ui.add_space(14.0);
    other_settings_controls(ui, draft);
}

fn dpi_stage_controls(ui: &mut egui::Ui, draft: &mut EditorDraft) {
    card(ui, |ui| {
        section_header(
            ui,
            "Sensitivity",
            "Each value represents an editable intent",
        );
        ui.add_space(12.0);
        ui.label(RichText::new("DPI stages").strong().size(11.0));
        for (index, dpi) in draft.dpi_stages.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut draft.active_stage,
                    index,
                    format!("Stage {}", index + 1),
                );
                ui.add(
                    egui::DragValue::new(dpi)
                        .range(100..=32_000)
                        .speed(50)
                        .suffix(" DPI"),
                );
                if draft.active_stage == index {
                    ui.label(RichText::new("ACTIVE - PREVIEW").size(9.0).color(GREEN));
                }
            });
        }
        ui.horizontal(|ui| {
            if ui.button("+ Add DPI stage").clicked() {
                draft.dpi_stages.push(2_400);
                draft.active_stage = draft.dpi_stages.len() - 1;
            }
            if draft.dpi_stages.len() > 1 && ui.button("Remove selected stage").clicked() {
                draft.dpi_stages.remove(draft.active_stage);
                draft.active_stage = draft.active_stage.min(draft.dpi_stages.len() - 1);
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let label = ui.label("Polling rate");
            egui::ComboBox::from_id_salt("detailed_polling_rate")
                .selected_text(format!("{} Hz (intent)", draft.polling_hz))
                .show_ui(ui, |ui| {
                    for rate in POLLING_OPTIONS {
                        ui.selectable_value(&mut draft.polling_hz, rate, format!("{rate} Hz"));
                    }
                })
                .response
                .labelled_by(label.id);
        });
    });
}

fn button_mapping_controls(ui: &mut egui::Ui, draft: &mut EditorDraft) {
    card(ui, |ui| {
        section_header(
            ui,
            "Button mapping",
            "Mapping options are ideas; device support is not established",
        );
        ui.add_space(10.0);
        for mapping in &mut draft.mappings {
            mapping_row(ui, mapping);
        }
    });
}

fn other_settings_controls(ui: &mut egui::Ui, draft: &mut EditorDraft) {
    card(ui, |ui| {
        section_header(
            ui,
            "Other settings",
            "Example editor controls, pending capability research",
        );
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label("Lift-off distance");
            egui::ComboBox::from_id_salt("lift_off_distance")
                .selected_text(&draft.lift_off)
                .show_ui(ui, |ui| {
                    for value in ["Low (intent)", "Standard (intent)", "High (intent)"] {
                        ui.selectable_value(&mut draft.lift_off, value.to_owned(), value);
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Idle sleep timer");
            egui::ComboBox::from_id_salt("sleep_timer")
                .selected_text(format!("{} minutes (intent)", draft.sleep_minutes))
                .show_ui(ui, |ui| {
                    for minutes in [1, 2, 5, 10, 15] {
                        ui.selectable_value(
                            &mut draft.sleep_minutes,
                            minutes,
                            format!("{minutes} minutes"),
                        );
                    }
                });
        });
        ui.label(
            RichText::new(
                "These controls do not imply that firmware exposes or accepts these settings.",
            )
            .size(10.0)
            .color(MUTED),
        );
    });
}

fn mapping_row(ui: &mut egui::Ui, mapping: &mut ButtonMappingDraft) {
    ui.horizontal(|ui| {
        let label = ui.label(RichText::new(mapping.button).size(11.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let combo = egui::ComboBox::from_id_salt(format!("mapping_{}", mapping.button))
                .selected_text(&mapping.action)
                .width(188.0)
                .show_ui(ui, |ui| {
                    for option in MAPPING_OPTIONS {
                        ui.selectable_value(&mut mapping.action, option.to_owned(), option);
                    }
                });
            combo.response.clone().labelled_by(label.id);
            ui.ctx().accesskit_node_builder(combo.response.id, |node| {
                node.set_label(mapping.button);
            });
        });
    });
}

fn apply_footer(ui: &mut egui::Ui) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Ready to apply?").strong().size(12.0));
                ui.label(
                    RichText::new("No validated device path is connected to this preview.")
                        .size(10.0)
                        .color(MUTED),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled(EditorDraft::can_apply(), egui::Button::new("Apply changes"));
                ui.add_enabled(
                    false,
                    egui::Button::new("Save draft (unavailable)").fill(PANEL_RAISED),
                );
            });
        });
    });
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("ViperPilot Settings PREVIEW")
            .with_inner_size([1_080.0, 790.0])
            .with_min_inner_size([850.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ViperPilot Settings PREVIEW",
        options,
        Box::new(|context| Ok(Box::new(EguiPreview::new(context)))),
    )
}

#[cfg(test)]
mod tests {
    use super::{EditorDraft, MAPPING_OPTIONS, POLLING_OPTIONS};

    #[test]
    fn draft_values_are_editable_but_never_enable_device_apply() {
        let mut draft = EditorDraft {
            profile_name: "My own profile".to_owned(),
            ..EditorDraft::default()
        };
        draft.dpi_stages[draft.active_stage] = 2_150;
        draft.polling_hz = 2_000;
        draft.change_mapping("Mouse4 / Side button 1", "Keyboard key (intent)");

        assert_eq!(draft.profile_name, "My own profile");
        assert_eq!(draft.active_dpi(), Some(2_150));
        assert_eq!(draft.polling_hz, 2_000);
        assert_eq!(draft.mappings[3].action, "Keyboard key (intent)");
        assert!(!EditorDraft::can_apply());
    }

    #[test]
    fn widgets_expose_a_named_edit_field_and_disabled_apply_to_accesskit() {
        use eframe::egui::accesskit::Role;
        use egui_kittest::{
            Harness,
            kittest::{NodeT as _, Queryable as _},
        };

        let mut draft = EditorDraft::default();
        let mut harness = Harness::new_ui(|ui| {
            super::profile_card(ui, &mut draft);
            super::apply_footer(ui);
        });
        harness.run();

        let name = harness.get_by_role_and_label(Role::TextInput, "Profile name");
        assert_eq!(name.accesskit_node().value(), Some("My Profile".to_owned()));
        assert!(!name.accesskit_node().is_disabled());

        let apply = harness.get_by_role_and_label(Role::Button, "Apply changes");
        assert!(apply.accesskit_node().is_disabled());
    }

    #[test]
    fn preview_offers_explicit_editable_intent_choices() {
        let draft = EditorDraft::default();
        assert!(POLLING_OPTIONS.contains(&draft.polling_hz));
        assert!(MAPPING_OPTIONS.contains(&"Media action (intent)"));
        assert!(!EditorDraft::can_apply());
    }
}

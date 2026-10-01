#![cfg_attr(windows, windows_subsystem = "windows")]

//! Optional hardware-free editor for persisted, local profile intent drafts.
//! Saving only writes the separate draft library. Apply is permanently disabled.

use std::path::PathBuf;

use eframe::egui::{self, Color32, RichText, Stroke, Vec2, Visuals};
use viper_v4_utility::draft_editor::DraftEditor;
use viper_v4_utility::profile_intent::{
    ButtonActionIntent, DpiTarget, KeyboardKey, PROFILE_INTENT_SCHEMA_VERSION, ProfileIntentV1,
};
use viper_v4_utility::storage::StoragePaths;

const ROSE: Color32 = Color32::from_rgb(255, 91, 151);
const ROSE_DARK: Color32 = Color32::from_rgb(105, 39, 67);
const CANVAS: Color32 = Color32::from_rgb(19, 20, 25);
const PANEL: Color32 = Color32::from_rgb(28, 30, 37);
const PANEL_RAISED: Color32 = Color32::from_rgb(38, 40, 49);
const MUTED: Color32 = Color32::from_rgb(156, 160, 174);
const POLLING_PRESETS: [u16; 7] = [125, 250, 500, 1_000, 2_000, 4_000, 8_000];

#[derive(Clone, Debug, PartialEq, Eq)]
enum DeferredAction {
    Select(String),
    New,
    Duplicate,
    Delete(String),
    Reload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionKind {
    PassThrough,
    Unassigned,
    KeyboardShortcut,
    Unmodeled,
}

struct EguiPreview {
    paths: Result<StoragePaths, String>,
    editor: Option<DraftEditor>,
    selected_id: Option<String>,
    buffer: ProfileIntentV1,
    baseline: ProfileIntentV1,
    duplicate_name: String,
    error: Option<String>,
    status: String,
    deferred: Option<DeferredAction>,
    show_dirty_prompt: bool,
    show_delete_prompt: Option<String>,
    show_close_prompt: bool,
    allow_close: bool,
}

impl EguiPreview {
    fn new(context: &eframe::CreationContext<'_>, paths: Result<StoragePaths, String>) -> Self {
        configure_theme(&context.egui_ctx);
        Self::from_paths(paths)
    }

    fn from_paths(paths: Result<StoragePaths, String>) -> Self {
        let mut app = Self {
            paths,
            editor: None,
            selected_id: None,
            buffer: new_intent(),
            baseline: new_intent(),
            duplicate_name: String::new(),
            error: None,
            status: "Opening local draft library…".to_owned(),
            deferred: None,
            show_dirty_prompt: false,
            show_delete_prompt: None,
            show_close_prompt: false,
            allow_close: false,
        };
        app.retry_open();
        app
    }

    fn retry_open(&mut self) {
        let paths = match &self.paths {
            Ok(paths) => paths.clone(),
            Err(error) => {
                self.error = Some(error.clone());
                "Draft storage is unavailable.".clone_into(&mut self.status);
                return;
            }
        };
        match DraftEditor::open(paths) {
            Ok(editor) => {
                self.editor = Some(editor);
                self.error = None;
                "Local profile drafts are ready.".clone_into(&mut self.status);
                self.select_initial_draft();
            }
            Err(error) => {
                self.editor = None;
                self.selected_id = None;
                self.error = Some(error);
                "Draft library could not be loaded. Retry after fixing the file."
                    .clone_into(&mut self.status);
            }
        }
    }

    fn select_initial_draft(&mut self) {
        let first = self
            .editor
            .as_ref()
            .and_then(|editor| editor.library().entries().first())
            .map(|draft| draft.id().to_owned());
        if let Some(id) = first {
            self.load_selected(&id);
        } else {
            self.start_new();
        }
    }

    fn start_new(&mut self) {
        self.selected_id = None;
        self.buffer = new_intent();
        self.baseline = self.buffer.clone();
        "New draft — edits are local until saved.".clone_into(&mut self.status);
    }

    fn load_selected(&mut self, id: &str) {
        let Some(editor) = self.editor.as_ref() else {
            return;
        };
        match editor.library().get(id) {
            Ok(draft) => {
                self.selected_id = Some(id.to_owned());
                self.buffer = draft.intent().clone();
                self.baseline = self.buffer.clone();
                self.status = format!("Opened draft {}.", draft.intent().name);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn is_dirty(&self) -> bool {
        self.buffer != self.baseline
    }

    fn request_action(&mut self, action: DeferredAction) {
        if self.is_dirty() {
            self.deferred = Some(action);
            self.show_dirty_prompt = true;
        } else {
            self.perform_action(action);
        }
    }

    fn resolve_dirty(&mut self, save: bool) {
        if save && !self.save_draft() {
            return;
        }
        self.show_dirty_prompt = false;
        let Some(action) = self.deferred.take() else {
            return;
        };
        self.perform_action(action);
    }

    fn discard_and_continue(&mut self) {
        self.buffer = self.baseline.clone();
        self.show_dirty_prompt = false;
        if let Some(action) = self.deferred.take() {
            self.perform_action(action);
        }
    }

    fn discard_and_close(&mut self, context: &egui::Context) {
        self.buffer = self.baseline.clone();
        self.allow_close = true;
        self.show_close_prompt = false;
        context.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn perform_action(&mut self, action: DeferredAction) {
        match action {
            DeferredAction::Select(id) => self.load_selected(&id),
            DeferredAction::New => self.start_new(),
            DeferredAction::Duplicate => {
                if let Some(selected) = self.selected_id.clone() {
                    self.error = None;
                    self.duplicate_name = format!("{} copy", self.buffer.name);
                    self.deferred = Some(DeferredAction::Duplicate);
                    self.status = format!("Choose a name for the copy of {selected}.");
                } else {
                    "Save a new draft before duplicating it.".clone_into(&mut self.status);
                }
            }
            DeferredAction::Delete(id) => {
                self.error = None;
                self.show_delete_prompt = Some(id);
            }
            DeferredAction::Reload => self.retry_open(),
        }
    }

    fn save_draft(&mut self) -> bool {
        let Some(editor) = self.editor.as_mut() else {
            return false;
        };
        if let Err(error) = self.buffer.validate() {
            self.error = Some(error);
            return false;
        }
        let result = if let Some(id) = self.selected_id.clone() {
            let existing = match editor.library().get(&id) {
                Ok(draft) => draft.intent().clone(),
                Err(error) => {
                    self.error = Some(error);
                    return false;
                }
            };
            if existing == self.buffer {
                Ok(())
            } else if same_settings_except_name(&existing, &self.buffer) {
                editor.rename(&id, &self.buffer.name)
            } else {
                editor.edit(&id, self.buffer.clone())
            }
        } else {
            editor.create(self.buffer.clone()).map(|id| {
                self.selected_id = Some(id);
            })
        };
        match result {
            Ok(()) => {
                self.baseline = self.buffer.clone();
                self.error = None;
                "Draft saved locally. Apply remains unavailable.".clone_into(&mut self.status);
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    fn duplicate_draft(&mut self) {
        let (Some(editor), Some(source_id)) = (self.editor.as_mut(), self.selected_id.clone())
        else {
            return;
        };
        match editor.duplicate(&source_id, &self.duplicate_name) {
            Ok(id) => {
                self.show_delete_prompt = None;
                self.deferred = None;
                self.load_selected(&id);
                "Draft duplicated and saved locally.".clone_into(&mut self.status);
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn delete_draft(&mut self, id: &str) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        match editor.delete(id) {
            Ok(()) => {
                self.show_delete_prompt = None;
                self.error = None;
                "Draft deleted from local storage.".clone_into(&mut self.status);
                self.select_initial_draft();
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn view(&mut self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        if context.input(|input| input.viewport().close_requested())
            && self.is_dirty()
            && !self.allow_close
        {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.show_close_prompt = true;
        }
        egui::Panel::top("top_bar").show(ui, |ui| top_bar(ui, &self.status));
        egui::Panel::left("draft_navigation")
            .exact_size(245.0)
            .show(ui, |ui| self.navigation(ui));
        egui::CentralPanel::default().show(ui, |ui| self.editor_panel(ui));
        self.dialogs(&context);
    }

    fn navigation(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(
            RichText::new("LOCAL PROFILE DRAFTS")
                .strong()
                .size(10.0)
                .color(MUTED),
        );
        ui.add_space(8.0);
        let rows: Vec<(String, String)> = self.editor.as_ref().map_or_else(Vec::new, |editor| {
            editor
                .library()
                .entries()
                .iter()
                .map(|draft| (draft.id().to_owned(), draft.intent().name.clone()))
                .collect()
        });
        egui::ScrollArea::vertical()
            .max_height(250.0)
            .show(ui, |ui| {
                for (id, name) in rows {
                    let selected = self.selected_id.as_deref() == Some(id.as_str());
                    ui.push_id(&id, |ui| {
                        if ui.selectable_label(selected, name).clicked() && !selected {
                            self.request_action(DeferredAction::Select(id.clone()));
                        }
                    });
                }
            });
        if self.selected_id.is_none() && self.editor.is_some() {
            ui.label(RichText::new("New unsaved draft").color(ROSE));
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.editor.is_some(), egui::Button::new("New draft"))
                .clicked()
            {
                self.request_action(DeferredAction::New);
            }
            if ui
                .add_enabled(
                    self.editor.is_some() && self.selected_id.is_some(),
                    egui::Button::new("Duplicate draft"),
                )
                .clicked()
            {
                self.request_action(DeferredAction::Duplicate);
            }
        });
        if ui
            .add_enabled(
                self.editor.is_some() && self.selected_id.is_some(),
                egui::Button::new("Delete draft…"),
            )
            .clicked()
        {
            self.request_action(DeferredAction::Delete(
                self.selected_id.clone().unwrap_or_default(),
            ));
        }
        if self.editor.is_some()
            && self
                .error
                .as_deref()
                .is_some_and(|error| error.contains("changed outside this editor session"))
            && ui.button("Reload saved drafts…").clicked()
        {
            self.request_action(DeferredAction::Reload);
        }
        ui.add_space(18.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(
            RichText::new("UNVERIFIED SETTINGS")
                .strong()
                .size(10.0)
                .color(ROSE),
        );
        ui.label(RichText::new("Drafts are saved only on this PC. DPI changes, arbitrary polling values, keyboard actions and action combinations need device evidence.").size(11.0).color(MUTED));
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "DPI stages, other buttons, lift-off and sleep are not editable or saved here.",
            )
            .size(10.0)
            .color(MUTED),
        );
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(
                RichText::new("No hardware connection · Apply disabled")
                    .size(10.0)
                    .color(MUTED),
            );
        });
    }

    fn editor_panel(&mut self, ui: &mut egui::Ui) {
        egui::Frame::new().fill(CANVAS).inner_margin(egui::Margin::same(24)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Profile drafts").strong().size(26.0));
                    ui.label(RichText::new("Edit local intent and save it for later review.").size(12.0).color(MUTED));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let state = if self.is_dirty() {
                        "UNSAVED CHANGES"
                    } else if self.selected_id.is_some() {
                        "SAVED DRAFT"
                    } else {
                        "NEW DRAFT"
                    };
                    ui.label(RichText::new(state).strong().size(10.0).color(ROSE));
                });
            });
            ui.add_space(16.0);
            if let Some(error) = &self.error {
                error_banner(ui, error);
                if self.editor.is_none() && ui.button("Retry opening draft library").clicked() {
                    self.retry_open();
                }
                if self.editor.is_none() { return; }
            }
            egui::ScrollArea::vertical()
                .max_height((ui.available_height() - 104.0).max(160.0))
                .auto_shrink([false, false]).show(ui, |ui| {
                profile_fields(ui, &mut self.buffer);
                ui.add_space(14.0);
                action_fields(ui, "Mouse4", &mut self.buffer.mouse4);
                ui.add_space(12.0);
                action_fields(ui, "Mouse5", &mut self.buffer.mouse5);
            });
            ui.add_space(10.0);
            if let Err(error) = self.buffer.validate() {
                ui.label(RichText::new(error).size(10.0).color(ROSE));
            }
            egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::same(12)).corner_radius(9).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Apply changes").strong().size(12.0));
                        ui.label(RichText::new("No connected device or complete draft has been verified for Apply.").size(10.0).color(MUTED));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled(false, egui::Button::new("Apply changes"));
                        let valid = self.buffer.validate().is_ok() && self.editor.is_some();
                        if ui.add_enabled(valid, egui::Button::new("Save draft").fill(ROSE_DARK)).clicked() { self.save_draft(); }
                    });
                });
            });
        });
    }

    fn dialogs(&mut self, context: &egui::Context) {
        if self.show_dirty_prompt {
            egui::Modal::new(egui::Id::new("dirty_draft_modal")).show(context, |ui| {
                ui.set_min_width(340.0);
                ui.heading("Unsaved draft changes");
                ui.label("Save or discard your edits before continuing?");
                modal_error(ui, self.error.as_deref());
                ui.horizontal(|ui| {
                    if ui.button("Save and continue").clicked() {
                        self.resolve_dirty(true);
                    }
                    if ui.button("Discard and continue").clicked() {
                        self.discard_and_continue();
                    }
                    if ui.button("Cancel").clicked() {
                        self.deferred = None;
                        self.show_dirty_prompt = false;
                    }
                });
            });
        } else if let Some(id) = self.show_delete_prompt.clone() {
            egui::Modal::new(egui::Id::new("delete_draft_modal")).show(context, |ui| {
                ui.set_min_width(340.0);
                ui.heading("Delete profile draft?");
                ui.label("This removes the saved local draft from disk.");
                modal_error(ui, self.error.as_deref());
                ui.horizontal(|ui| {
                    if ui.button("Delete draft").clicked() {
                        self.delete_draft(&id);
                    }
                    if ui.button("Cancel").clicked() {
                        self.show_delete_prompt = None;
                    }
                });
            });
        } else if self.deferred == Some(DeferredAction::Duplicate) && !self.show_dirty_prompt {
            egui::Modal::new(egui::Id::new("duplicate_draft_modal")).show(context, |ui| {
                ui.set_min_width(340.0);
                ui.heading("Duplicate profile draft");
                ui.label("Name for the new local draft");
                modal_error(ui, self.error.as_deref());
                ui.text_edit_singleline(&mut self.duplicate_name);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.duplicate_name.trim().is_empty(),
                            egui::Button::new("Create copy"),
                        )
                        .clicked()
                    {
                        self.duplicate_draft();
                    }
                    if ui.button("Cancel").clicked() {
                        self.deferred = None;
                    }
                });
            });
        } else if self.show_close_prompt {
            egui::Modal::new(egui::Id::new("close_draft_modal")).show(context, |ui| {
                ui.set_min_width(340.0);
                ui.heading("Unsaved draft changes");
                ui.label("Save or discard your edits before closing?");
                modal_error(ui, self.error.as_deref());
                ui.horizontal(|ui| {
                    if ui.button("Save and close").clicked() && self.save_draft() {
                        self.allow_close = true;
                        self.show_close_prompt = false;
                        context.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Discard and close").clicked() {
                        self.discard_and_close(context);
                    }
                    if ui.button("Keep editing").clicked() {
                        self.show_close_prompt = false;
                    }
                });
            });
        }
    }
}

impl eframe::App for EguiPreview {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.view(ui);
    }
}

fn new_intent() -> ProfileIntentV1 {
    ProfileIntentV1 {
        schema_version: PROFILE_INTENT_SCHEMA_VERSION,
        name: "My Profile".to_owned(),
        dpi: DpiTarget { x: 800, y: 800 },
        polling_hz: 1_000,
        mouse4: ButtonActionIntent::PassThrough,
        mouse5: ButtonActionIntent::PassThrough,
    }
}

fn same_settings_except_name(left: &ProfileIntentV1, right: &ProfileIntentV1) -> bool {
    left.schema_version == right.schema_version
        && left.dpi == right.dpi
        && left.polling_hz == right.polling_hz
        && left.mouse4 == right.mouse4
        && left.mouse5 == right.mouse5
}

fn top_bar(ui: &mut egui::Ui, status: &str) {
    egui::Frame::new()
        .fill(PANEL)
        .inner_margin(egui::Margin::symmetric(22, 14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("V").strong().size(23.0).color(ROSE));
                ui.vertical(|ui| {
                    ui.label(RichText::new("ViperPilot").strong().size(16.0));
                    ui.label(
                        RichText::new("PROFILE DRAFT EDITOR PREVIEW")
                            .size(10.0)
                            .color(MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("PREVIEW · NO DEVICE WRITES")
                            .strong()
                            .size(10.0)
                            .color(ROSE),
                    );
                    ui.separator();
                    ui.label(RichText::new(status).size(10.0).color(MUTED));
                });
            });
        });
}

fn profile_fields(ui: &mut egui::Ui, intent: &mut ProfileIntentV1) {
    card(ui, |ui| {
        section_header(ui, "Profile", "Names stay local and can change");
        ui.add_space(10.0);
        let name_label = ui.label(
            RichText::new("PROFILE NAME")
                .strong()
                .size(10.0)
                .color(MUTED),
        );
        let name_input = ui.add(
            egui::TextEdit::singleline(&mut intent.name)
                .desired_width(420.0)
                .hint_text("Name this draft"),
        );
        name_input.clone().labelled_by(name_label.id);
        ui.ctx().accesskit_node_builder(name_input.id, |node| {
            node.set_label("Profile name");
        });
        ui.add_space(10.0);
        section_header(
            ui,
            "Sensitivity",
            "DPI X/Y and polling are local draft values",
        );
        ui.horizontal(|ui| {
            let label = ui.label("DPI X");
            let input = ui.add(
                egui::DragValue::new(&mut intent.dpi.x)
                    .range(1..=65_535)
                    .suffix(" DPI"),
            );
            input.clone().labelled_by(label.id);
            ui.ctx()
                .accesskit_node_builder(input.id, |node| node.set_label("DPI X"));
            let label = ui.label("DPI Y");
            let input = ui.add(
                egui::DragValue::new(&mut intent.dpi.y)
                    .range(1..=65_535)
                    .suffix(" DPI"),
            );
            input.clone().labelled_by(label.id);
            ui.ctx()
                .accesskit_node_builder(input.id, |node| node.set_label("DPI Y"));
        });
        ui.horizontal(|ui| {
            let label = ui.label("Polling rate");
            egui::ComboBox::from_id_salt("draft_polling")
                .selected_text(format!("{} Hz", intent.polling_hz))
                .show_ui(ui, |ui| {
                    for value in POLLING_PRESETS {
                        ui.selectable_value(&mut intent.polling_hz, value, format!("{value} Hz"));
                    }
                })
                .response
                .labelled_by(label.id);
        });
        ui.label(
            RichText::new("Current non-preset values remain unchanged until you choose a preset.")
                .size(10.0)
                .color(MUTED),
        );
    });
}

fn action_fields(ui: &mut egui::Ui, button: &str, action: &mut ButtonActionIntent) {
    card(ui, |ui| {
        section_header(
            ui,
            button,
            "Action intent only · combinations need separate evidence",
        );
        ui.add_space(8.0);
        let old_kind = match action {
            ButtonActionIntent::PassThrough => ActionKind::PassThrough,
            ButtonActionIntent::Unassigned => ActionKind::Unassigned,
            ButtonActionIntent::KeyboardShortcut { .. } => ActionKind::KeyboardShortcut,
            ButtonActionIntent::Unmodeled { .. } => ActionKind::Unmodeled,
        };
        let mut kind = old_kind;
        egui::ComboBox::from_id_salt(format!("action_kind_{button}"))
            .selected_text(action_label(action))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut kind, ActionKind::PassThrough, "Pass through (native)");
                ui.selectable_value(&mut kind, ActionKind::Unassigned, "Unassigned");
                ui.selectable_value(&mut kind, ActionKind::KeyboardShortcut, "Keyboard shortcut");
                ui.selectable_value(&mut kind, ActionKind::Unmodeled, "Unmodeled description");
            });
        change_action_kind(action, old_kind, kind, button);
        let mut next = action.clone();
        if let ButtonActionIntent::KeyboardShortcut {
            control,
            alt,
            shift,
            windows,
            key,
        } = &mut next
        {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(control, "Ctrl");
                ui.checkbox(alt, "Alt");
                ui.checkbox(shift, "Shift");
                ui.checkbox(windows, "Windows");
                egui::ComboBox::from_id_salt(format!("key_{button}"))
                    .selected_text(format!("{key:?}"))
                    .show_ui(ui, |ui| {
                        for &option in keyboard_keys() {
                            ui.selectable_value(key, option, format!("{option:?}"));
                        }
                    });
            });
            ui.label(RichText::new("Every named key and modifier combination is preserved as intent; support is unverified.").size(10.0).color(MUTED));
        }
        if let ButtonActionIntent::Unmodeled { description } = &mut next {
            ui.label(
                RichText::new("Research-only text; never mapped to a device action.")
                    .size(10.0)
                    .color(MUTED),
            );
            ui.add(
                egui::TextEdit::singleline(description)
                    .desired_width(f32::INFINITY)
                    .hint_text("Describe the unmodeled idea"),
            );
        }
        *action = next;
    });
}

fn default_shortcut(button: &str) -> ButtonActionIntent {
    ButtonActionIntent::KeyboardShortcut {
        control: false,
        alt: false,
        shift: false,
        windows: false,
        key: if button == "Mouse4" {
            KeyboardKey::F10
        } else {
            KeyboardKey::F11
        },
    }
}

fn change_action_kind(
    action: &mut ButtonActionIntent,
    old_kind: ActionKind,
    new_kind: ActionKind,
    button: &str,
) {
    if old_kind == new_kind {
        return;
    }
    *action = match new_kind {
        ActionKind::PassThrough => ButtonActionIntent::PassThrough,
        ActionKind::Unassigned => ButtonActionIntent::Unassigned,
        ActionKind::KeyboardShortcut => default_shortcut(button),
        ActionKind::Unmodeled => ButtonActionIntent::Unmodeled {
            description: String::new(),
        },
    };
}

fn action_label(action: &ButtonActionIntent) -> String {
    match action {
        ButtonActionIntent::PassThrough => "Pass through (native)".to_owned(),
        ButtonActionIntent::Unassigned => "Unassigned".to_owned(),
        ButtonActionIntent::KeyboardShortcut {
            control,
            alt,
            shift,
            windows,
            key,
        } => {
            let mut parts = Vec::new();
            if *control {
                parts.push("Ctrl");
            }
            if *alt {
                parts.push("Alt");
            }
            if *shift {
                parts.push("Shift");
            }
            if *windows {
                parts.push("Windows");
            }
            if parts.is_empty() {
                format!("Keyboard shortcut · {key:?}")
            } else {
                format!("Keyboard shortcut · {}+{key:?}", parts.join("+"))
            }
        }
        ButtonActionIntent::Unmodeled { description } => {
            if description.is_empty() {
                "Unmodeled description".to_owned()
            } else {
                format!("Unmodeled · {description}")
            }
        }
    }
}

fn keyboard_keys() -> &'static [KeyboardKey] {
    use KeyboardKey::{
        A, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, B, Backspace, C, D, Delete, Digit0, Digit1,
        Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, E, End, Enter, Escape, F,
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20,
        F21, F22, F23, F24, G, H, Home, I, Insert, J, K, L, M, N, O, P, PageDown, PageUp, Q, R, S,
        Space, T, Tab, U, V, W, X, Y, Z,
    };
    &[
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, Digit0,
        Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, F1, F2, F3, F4, F5,
        F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F20, F21, F22, F23, F24,
        Enter, Escape, Space, Tab, Backspace, Delete, Insert, Home, End, PageUp, PageDown, ArrowUp,
        ArrowDown, ArrowLeft, ArrowRight,
    ]
}

fn card(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, Color32::from_rgb(53, 55, 64)))
        .inner_margin(egui::Margin::same(16))
        .corner_radius(12)
        .show(ui, content);
}

fn section_header(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.label(RichText::new(title).strong().size(14.0));
    ui.label(RichText::new(detail).size(10.0).color(MUTED));
}

fn error_banner(ui: &mut egui::Ui, error: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(58, 31, 38))
        .stroke(Stroke::new(1.0, ROSE_DARK))
        .inner_margin(egui::Margin::same(12))
        .corner_radius(8)
        .show(ui, |ui| {
            ui.label(RichText::new("Draft storage error").strong().color(ROSE));
            ui.label(RichText::new(error).color(Color32::WHITE));
        });
}

fn modal_error(ui: &mut egui::Ui, error: Option<&str>) {
    if let Some(error) = error {
        ui.label(
            RichText::new(format!("Could not continue: {error}"))
                .size(11.0)
                .color(ROSE),
        );
    }
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
    context.global_style_mut(|style| style.spacing.item_spacing = Vec2::new(10.0, 9.0));
}

fn storage_paths_from_args(args: impl IntoIterator<Item = String>) -> Result<StoragePaths, String> {
    let mut args = args.into_iter();
    let _program = args.next();
    let mut draft_root: Option<PathBuf> = None;
    while let Some(argument) = args.next() {
        if argument == "--draft-root" {
            let path = args.next().ok_or("--draft-root requires a path")?;
            draft_root = Some(PathBuf::from(path));
        } else {
            return Err(format!("unknown argument {argument:?}"));
        }
    }
    if let Some(root) = draft_root {
        Ok(StoragePaths::under(root))
    } else {
        StoragePaths::discover().map_err(|error| error.to_string())
    }
}

fn main() -> eframe::Result {
    let paths = storage_paths_from_args(std::env::args());
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
        Box::new(move |context| Ok(Box::new(EguiPreview::new(context, paths)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::accesskit::Role;
    use egui_kittest::{
        Harness,
        kittest::{NodeT as _, Queryable as _},
    };
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_paths(label: &str) -> StoragePaths {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        StoragePaths::under(std::env::temp_dir().join(format!(
            "viperpilot-egui-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }

    #[test]
    fn full_editor_save_reopen_and_apply_accessibility() {
        let paths = temp_paths("save");
        let root = paths.root.clone();
        let app = EguiPreview::from_paths(Ok(paths.clone()));
        let mut harness = Harness::builder()
            .with_size(Vec2::new(1080.0, 790.0))
            .build_ui_state(|ui, app| app.view(ui), app);
        harness.run();

        let name = harness.get_by_role_and_label(Role::TextInput, "Profile name");
        assert_eq!(name.value(), Some("My Profile".to_owned()));
        name.click();
        harness.run();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        harness.run();
        harness
            .get_by_role_and_label(Role::TextInput, "Profile name")
            .type_text("Saved from UI");
        harness.run();
        let save = harness.get_by_role_and_label(Role::Button, "Save draft");
        assert!(!save.accesskit_node().is_disabled());
        save.click();
        harness.run();

        let apply = harness.get_by_role_and_label(Role::Button, "Apply changes");
        assert!(apply.accesskit_node().is_disabled());
        let selected_id = harness
            .state()
            .selected_id
            .clone()
            .expect("saved draft has stable ID");
        let reopened = DraftEditor::open(paths).unwrap();
        let intent = reopened.library().get(&selected_id).unwrap().intent();
        assert_eq!(intent.name, "Saved from UI");
        assert_eq!(intent.dpi, DpiTarget { x: 800, y: 800 });
        assert_eq!(intent.polling_hz, 1_000);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn every_keyboard_key_is_offered_and_arbitrary_loaded_values_stay_visible() {
        assert_eq!(keyboard_keys().len(), 75);
        assert!(keyboard_keys().contains(&KeyboardKey::ArrowRight));
        assert!(keyboard_keys().contains(&KeyboardKey::F24));
        let paths = temp_paths("arbitrary");
        let mut editor = DraftEditor::open(paths.clone()).unwrap();
        let mut intent = new_intent();
        intent.name = "Custom values".to_owned();
        intent.dpi = DpiTarget { x: 1_234, y: 4_321 };
        intent.polling_hz = 1_333;
        editor.create(intent.clone()).unwrap();
        let app = EguiPreview::from_paths(Ok(paths.clone()));
        let mut harness = Harness::builder().build_ui_state(|ui, app| app.view(ui), app);
        harness.run();
        assert_eq!(harness.state().buffer, intent);
        fs::remove_dir_all(paths.root).unwrap();
    }

    #[test]
    fn modal_delete_blocks_background_navigation_and_shortcut_kind_is_preserved() {
        let paths = temp_paths("modal");
        let root = paths.root.clone();
        let mut seed = DraftEditor::open(paths.clone()).unwrap();
        let mut intent = new_intent();
        intent.name = "Saved profile".to_owned();
        intent.mouse4 = ButtonActionIntent::KeyboardShortcut {
            control: false,
            alt: true,
            shift: true,
            windows: false,
            key: KeyboardKey::Z,
        };
        let id = seed.create(intent.clone()).unwrap();
        let mut app = EguiPreview::from_paths(Ok(paths));
        app.show_delete_prompt = Some(id.clone());
        let mut harness = Harness::builder().build_ui_state(|ui, app| app.view(ui), app);
        harness.run();
        harness
            .get_by_role_and_label(Role::Button, "New draft")
            .click();
        harness.run();
        assert_eq!(harness.state().selected_id.as_deref(), Some(id.as_str()));
        assert_eq!(harness.state().buffer, intent);
        assert!(harness.state().show_delete_prompt.is_some());

        let mut unchanged = intent.mouse4.clone();
        let expected = unchanged.clone();
        change_action_kind(
            &mut unchanged,
            ActionKind::KeyboardShortcut,
            ActionKind::KeyboardShortcut,
            "Mouse4",
        );
        assert_eq!(unchanged, expected);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reload_requires_dirty_resolution_and_stale_save_keeps_buffer() {
        let paths = temp_paths("reload");
        let root = paths.root.clone();
        let mut seed = DraftEditor::open(paths.clone()).unwrap();
        let mut intent = new_intent();
        intent.name = "Original".to_owned();
        let id = seed.create(intent).unwrap();
        let mut app = EguiPreview::from_paths(Ok(paths.clone()));

        let mut external = DraftEditor::open(paths).unwrap();
        external.rename(&id, "External update").unwrap();
        app.buffer.name = "Local edit".to_owned();
        app.request_action(DeferredAction::Reload);
        assert!(app.show_dirty_prompt);
        assert_eq!(app.deferred, Some(DeferredAction::Reload));
        app.resolve_dirty(true);
        assert!(app.show_dirty_prompt);
        assert_eq!(app.buffer.name, "Local edit");
        assert!(
            app.error
                .as_deref()
                .is_some_and(|error| error.contains("changed outside this editor session"))
        );
        app.discard_and_continue();
        assert_eq!(app.buffer.name, "External update");
        assert!(!app.is_dirty());

        app.buffer.name = "Close test edit".to_owned();
        let context = egui::Context::default();
        app.discard_and_close(&context);
        assert!(!app.is_dirty());
        assert!(app.allow_close);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn draft_root_argument_selects_isolated_storage_root() {
        let paths = storage_paths_from_args([
            "preview".to_owned(),
            "--draft-root".to_owned(),
            "isolated".to_owned(),
        ])
        .unwrap();
        assert_eq!(paths.root, PathBuf::from("isolated"));
        assert!(
            storage_paths_from_args(["preview".to_owned(), "--draft-root".to_owned()]).is_err()
        );
    }

    #[test]
    fn dirty_navigation_save_discard_cancel_and_failed_save_keep_buffer() {
        let paths = temp_paths("dirty");
        let root = paths.root.clone();
        let mut app = EguiPreview::from_paths(Ok(paths.clone()));
        app.buffer.name = "Uncommitted edit".to_owned();
        app.request_action(DeferredAction::New);
        assert!(app.show_dirty_prompt);
        app.resolve_dirty(false);
        assert_eq!(app.buffer.name, "My Profile");

        app.buffer.name.clear();
        app.request_action(DeferredAction::New);
        app.deferred = None;
        app.show_dirty_prompt = false;
        assert!(app.is_dirty());
        assert!(!app.save_draft());
        assert!(app.buffer.name.is_empty());
        assert!(
            app.error
                .as_deref()
                .is_some_and(|error| error.contains("profile names"))
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn failed_modal_save_exposes_actionable_error_accessibly() {
        let paths = temp_paths("modal-error");
        let root = paths.root.clone();
        let mut app = EguiPreview::from_paths(Ok(paths));
        app.buffer.name.clear();
        app.request_action(DeferredAction::New);
        app.resolve_dirty(true);
        let expected = format!("Could not continue: {}", app.error.as_deref().unwrap());
        let mut harness = Harness::builder().build_ui_state(|ui, app| app.view(ui), app);
        harness.run();
        assert!(harness.query_by_label(&expected).is_some());
        fs::remove_dir_all(root).ok();
    }
}

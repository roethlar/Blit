//! Thin eframe view: render [`blit_gui::Session`]'s model and
//! dispatch the same [`blit_console_core::Msg`] values a TUI face
//! will use. No listing, discovery, or merge logic lives here.

use blit_console_core::{Endpoint, Msg};
use blit_gui::{enter_path, interactive_listing, parent_path, Session};
use eframe::egui::{self, Color32};
use std::sync::Arc;
use std::time::Duration;

/// Dracula red — same hue the CLI failure block uses (clp-3).
const ERROR_COLOR: Color32 = Color32::from_rgb(255, 85, 85);

pub struct ConsoleApp {
    session: Session,
}

impl ConsoleApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut session = Session::new();
        let ctx = cc.egui_ctx.clone();
        session.set_wake(Arc::new(move || ctx.request_repaint()));
        session.bootstrap();
        Self { session }
    }
}

impl eframe::App for ConsoleApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.session.poll();
        if self.session.model().is_loading() || self.session.model().is_discovering() {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }

        egui::Panel::left("fleet")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                ui.heading("Fleet");
                if self.session.model().is_discovering() {
                    ui.label("Scanning for daemons…");
                }
                if ui.button("Refresh fleet").clicked() {
                    self.session.dispatch(Msg::RefreshDiscovery);
                }
                ui.separator();

                let selected = self.session.model().selected();
                let rows: Vec<_> = self
                    .session
                    .model()
                    .endpoints()
                    .iter()
                    .map(|(id, endpoint)| {
                        (
                            *id,
                            endpoint.display_name().to_string(),
                            matches!(endpoint, Endpoint::Local),
                        )
                    })
                    .collect();
                for (id, name, _is_local) in rows {
                    let is_sel = selected == Some(id);
                    if ui.selectable_label(is_sel, name).clicked() && !is_sel {
                        self.session.dispatch(Msg::SelectEndpoint(id));
                    }
                }
            });

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Browse");
            ui.label(format!(
                "Path: {}",
                self.session.model().current_path().display()
            ));
            if let Some(parent) = parent_path(self.session.model().current_path()) {
                if ui.button("Up").clicked() {
                    self.session.dispatch(Msg::NavigateTo(parent));
                }
            }
            if self.session.model().is_loading() {
                ui.label("Loading…");
            }
            if let Some(err) = self.session.model().last_error() {
                ui.colored_label(ERROR_COLOR, err);
            }
            ui.separator();

            let entries: Vec<_> = interactive_listing(self.session.model())
                .iter()
                .map(|entry| (entry.name.clone(), entry.is_dir, entry.size))
                .collect();
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (name, is_dir, size) in entries {
                    if is_dir {
                        if ui.button(format!("{name}/")).clicked() {
                            let next = enter_path(self.session.model().current_path(), &name);
                            self.session.dispatch(Msg::NavigateTo(next));
                        }
                    } else {
                        ui.label(format!("{name}    {size}"));
                    }
                }
            });
        });
    }
}

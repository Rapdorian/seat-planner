pub mod constraint;
pub mod guest;
pub mod solver;
pub mod table;
pub mod ui;

use serde::{Deserialize, Serialize};

use crate::constraint::{Constraint, ConstraintGraph};
use crate::guest::GuestList;
use crate::table::{SeatingArrangement, TableConfig};
use crate::ui::AppUi;

#[derive(Serialize, Deserialize)]
struct SaveData {
    guests: GuestList,
    constraints: Vec<Constraint>,
    tables: Vec<TableConfig>,
}

pub struct SeatPlannerApp {
    guests: GuestList,
    constraints: ConstraintGraph,
    tables: Vec<TableConfig>,
    arrangement: Option<SeatingArrangement>,
    ui: AppUi,
}

impl Default for SeatPlannerApp {
    fn default() -> Self {
        #[allow(unused_mut)]
        let mut app = Self {
            guests: GuestList::new(),
            constraints: ConstraintGraph::new(),
            tables: Vec::new(),
            arrangement: None,
            ui: AppUi::new(),
        };
        #[cfg(target_arch = "wasm32")]
        app.load_from_storage();
        app
    }
}

impl SeatPlannerApp {
    fn save_to_storage(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let data = SaveData {
                guests: self.guests.clone(),
                constraints: self.constraints.all_constraints().to_vec(),
                tables: self.tables.clone(),
            };
            if let Ok(json) = serde_json::to_string(&data) {
                if let Some(window) = web_sys::window() {
                    if let Ok(Some(storage)) = window.local_storage() {
                        let _ = storage.set_item("seat_planner_save", &json);
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn load_from_storage(&mut self) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(json)) = storage.get_item("seat_planner_save") {
                    if let Ok(data) = serde_json::from_str::<SaveData>(&json) {
                        self.guests = data.guests;
                        self.tables = data.tables;
                        for c in &data.constraints {
                            self.constraints.add(c.a, c.b, c.kind);
                        }
                        self.ui.needs_solve = true;
                    }
                }
            }
        }
    }

    pub fn serialize_state(&self) -> Result<String, serde_json::Error> {
        let data = SaveData {
            guests: self.guests.clone(),
            constraints: self.constraints.all_constraints().to_vec(),
            tables: self.tables.clone(),
        };
        serde_json::to_string_pretty(&data)
    }

    pub fn deserialize_state(&mut self, json: &str) -> Result<(), serde_json::Error> {
        let data: SaveData = serde_json::from_str(json)?;
        self.guests = data.guests;
        self.constraints = ConstraintGraph::new();
        for c in &data.constraints {
            self.constraints.add(c.a, c.b, c.kind);
        }
        self.tables = data.tables;
        self.ui.needs_solve = true;
        Ok(())
    }
}

impl eframe::App for SeatPlannerApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        apply_wedding_theme(ctx);
        self.ui.render(
            ctx,
            &mut self.guests,
            &mut self.constraints,
            &mut self.tables,
            &mut self.arrangement,
        );

        if self.ui.export_triggered {
            self.ui.export_json = self.serialize_state().unwrap_or_default();
            self.ui.export_triggered = false;
            self.ui.show_export = true;
        }

        if self.ui.import_triggered {
            let import_text = self.ui.import_text.clone();
            match self.deserialize_state(&import_text) {
                Ok(()) => {
                    self.ui.show_import = false;
                    self.ui.import_text.clear();
                    self.ui.import_error = None;
                }
                Err(e) => {
                    self.ui.import_error = Some(e.to_string());
                }
            }
            self.ui.import_triggered = false;
        }

        self.save_to_storage();
    }
}

fn apply_wedding_theme(ctx: &eframe::egui::Context) {
    use eframe::egui::Color32;

    let mut visuals = eframe::egui::Visuals::light();

    visuals.hyperlink_color = Color32::from_rgb(180, 100, 130);

    visuals.panel_fill = Color32::from_rgb(255, 248, 245);
    visuals.window_fill = Color32::from_rgb(255, 248, 245);

    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(255, 252, 250);
    visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(220, 200, 200);
    visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(70, 50, 55);

    visuals.widgets.inactive.bg_fill = Color32::from_rgb(255, 242, 240);
    visuals.widgets.inactive.bg_stroke.color = Color32::from_rgb(212, 134, 156);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(70, 50, 55);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(240, 210, 218);
    visuals.widgets.hovered.bg_stroke.color = Color32::from_rgb(212, 134, 156);
    visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(70, 50, 55);

    visuals.widgets.active.bg_fill = Color32::from_rgb(212, 134, 156);
    visuals.widgets.active.bg_stroke.color = Color32::from_rgb(180, 100, 130);
    visuals.widgets.active.fg_stroke.color = Color32::WHITE;

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(60, 38, 44, 80);
    visuals.selection.stroke.color = Color32::from_rgb(212, 134, 156);

    ctx.set_visuals(visuals);
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    fn get_canvas_element() -> web_sys::HtmlCanvasElement {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("seat-planner-canvas"))
            .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("canvas element not found")
    }

    #[wasm_bindgen]
    pub fn main() {
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();

        let web_options = eframe::WebOptions::default();
        let canvas = get_canvas_element();

        wasm_bindgen_futures::spawn_local(async {
            eframe::WebRunner::new()
                .start(canvas, web_options, Box::new(|_cc| Ok(Box::new(super::SeatPlannerApp::default()))))
                .await
                .expect("failed to start eframe");
        });
    }
}

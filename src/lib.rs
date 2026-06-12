pub mod constraint;
pub mod guest;
pub mod solver;
pub mod table;
pub mod ui;

use crate::constraint::ConstraintGraph;
use crate::guest::GuestList;
use crate::table::{SeatingArrangement, TableConfig};
use crate::ui::AppUi;

pub struct SeatPlannerApp {
    guests: GuestList,
    constraints: ConstraintGraph,
    tables: Vec<TableConfig>,
    arrangement: Option<SeatingArrangement>,
    ui: AppUi,
}

impl Default for SeatPlannerApp {
    fn default() -> Self {
        Self {
            guests: GuestList::new(),
            constraints: ConstraintGraph::new(),
            tables: Vec::new(),
            arrangement: None,
            ui: AppUi::new(),
        }
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

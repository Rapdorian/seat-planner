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

#[cfg(target_arch = "wasm32")]
thread_local! {
    static IMPORT_QUEUE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
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
            if let Ok(json) = self.serialize_state() {
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
                    self.deserialize_state(&json);
                    self.ui.needs_solve = true;
                }
            }
        }
    }

    #[allow(dead_code)]
    fn serialize_state(&self) -> Result<String, serde_json::Error> {
        let data = SaveData {
            guests: self.guests.clone(),
            constraints: self.constraints.all_constraints().to_vec(),
            tables: self.tables.clone(),
        };
        serde_json::to_string_pretty(&data)
    }

    #[allow(dead_code)]
    fn deserialize_state(&mut self, json: &str) {
        if let Ok(data) = serde_json::from_str::<SaveData>(json) {
            self.guests = data.guests;
            self.constraints = ConstraintGraph::new();
            for c in &data.constraints {
                self.constraints.add(c.a, c.b, c.kind);
            }
            self.tables = data.tables;
            self.ui.needs_solve = true;
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn download_state(&self) {
        if let Ok(json) = self.serialize_state() {
            use wasm_bindgen::JsCast;
            use web_sys::{Blob, HtmlAnchorElement, Url};

            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();

            let blob = Blob::new_with_str_sequence(
                &js_sys::Array::of1(&wasm_bindgen::JsValue::from(json)),
            )
            .unwrap();
            let url = Url::create_object_url_with_blob(&blob).unwrap();

            let a = document
                .create_element("a")
                .unwrap()
                .dyn_into::<HtmlAnchorElement>()
                .unwrap();
            a.set_href(&url);
            a.set_download("seat-planner.seatplan");
            document.body().unwrap().append_child(&a).unwrap();
            a.click();
            a.remove();
            Url::revoke_object_url(&url).unwrap();
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn trigger_import_picker() {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let input = document
            .create_element("input")
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        input.set_type("file");
        input.set_accept(".seatplan,.json");
        let _ = input.style().set_property("display", "none");
        document.body().unwrap().append_child(&input).unwrap();

        let input_clone = input.clone();
        let onchange = Closure::<dyn Fn()>::new(move || {
            if let Some(file) = input_clone.files().and_then(|f| f.item(0)) {
                let reader = web_sys::FileReader::new().unwrap();
                let reader_c = reader.clone();
                let onload = Closure::<dyn Fn()>::new(move || {
                    if let Ok(text) = reader_c.result() {
                        if let Some(s) = text.as_string() {
                            IMPORT_QUEUE.with(|q| *q.borrow_mut() = Some(s));
                        }
                    }
                });
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                reader.read_as_text(&file).unwrap();
            }
            let _ = input_clone.remove();
        });
        input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
        onchange.forget();
        input.click();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn download_state(&self) {
        // native fallback: no automatic download, but state is accessible
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

        if self.ui.export_requested {
            self.ui.export_requested = false;
            self.download_state();
        }

        if self.ui.import_requested {
            self.ui.import_requested = false;
            #[cfg(target_arch = "wasm32")]
            Self::trigger_import_picker();
        }

        #[cfg(target_arch = "wasm32")]
        IMPORT_QUEUE.with(|q| {
            if let Some(json) = q.borrow_mut().take() {
                self.deserialize_state(&json);
            }
        });

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

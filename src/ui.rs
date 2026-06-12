use eframe::egui;
use eframe::egui::{CentralPanel, ComboBox, ScrollArea, SidePanel, TextEdit, Widget};

use crate::constraint::{ConstraintGraph, LinkType};
use crate::guest::{GuestId, GuestList};
use crate::solver;
use crate::table::{SeatingArrangement, TableConfig};

pub struct AppUi {
    pub new_guest_name: String,
    pub selected_guest: Option<GuestId>,
    pub constraint_search_must: String,
    pub constraint_search_should: String,
    pub constraint_search_could: String,
    pub constraint_search_wont: String,
    pub needs_solve: bool,
    pub solve_error: Option<String>,
    pub new_table_capacity: usize,
    pub new_table_count: usize,
    pub export_requested: bool,
    pub import_requested: bool,
}

impl AppUi {
    pub fn new() -> Self {
        Self {
            new_guest_name: String::new(),
            selected_guest: None,
            constraint_search_must: String::new(),
            constraint_search_should: String::new(),
            constraint_search_could: String::new(),
            constraint_search_wont: String::new(),
            needs_solve: true,
            solve_error: None,
            new_table_capacity: 8,
            new_table_count: 1,
            export_requested: false,
            import_requested: false,
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        guests: &mut GuestList,
        constraints: &mut ConstraintGraph,
        tables: &mut Vec<TableConfig>,
        arrangement: &mut Option<SeatingArrangement>,
    ) {
        if self.needs_solve && guests.len() > 0 && !tables.is_empty() {
            self.run_solver(guests, constraints, tables, arrangement);
            self.needs_solve = false;
        }

        // --- LEFT PANEL: Tables ---
        SidePanel::left("table_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Tables");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label("Capacity:");
                    ui.add(
                        egui::DragValue::new(&mut self.new_table_capacity)
                            .range(1..=100)
                            .speed(1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Count:");
                    ui.add(
                        egui::DragValue::new(&mut self.new_table_count)
                            .range(1..=100)
                            .speed(1),
                    );
                });
                if ui.button("Add Table(s)").clicked() {
                    tables.push(TableConfig::new(self.new_table_capacity, self.new_table_count));
                    self.needs_solve = true;
                }

                ui.separator();

                let mut remove_idx: Option<usize> = None;
                for (i, table) in tables.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Table {}: {} x {} seats",
                            i + 1,
                            table.count,
                            table.capacity
                        ));
                        if ui.small_button("✕").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }
                if let Some(idx) = remove_idx {
                    tables.remove(idx);
                    self.needs_solve = true;
                }

                if tables.is_empty() {
                    ui.label("No tables");
                } else {
                    let total: usize = tables.iter().map(|t| t.total_capacity()).sum();
                    ui.label(format!("Total: {} seats ({} guests)", total, guests.len()));
                }
            });

        // --- CENTER PANEL: Add guests + Seating ---
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Seat Planner");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Import").clicked() {
                        self.import_requested = true;
                    }
                    if ui.button("Export").clicked() {
                        self.export_requested = true;
                    }
                });
            });
            ui.separator();

            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.horizontal(|ui| {
                    let response = TextEdit::singleline(&mut self.new_guest_name)
                        .hint_text("New guest name...")
                        .desired_width(250.0)
                        .ui(ui);
                    let input_id = response.id;
                    let pressed_enter =
                        response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if pressed_enter || ui.button("Add").clicked() {
                        let name = self.new_guest_name.trim().to_string();
                        if !name.is_empty() {
                            guests.add(name);
                            self.new_guest_name.clear();
                            self.needs_solve = true;
                        }
                        ctx.memory_mut(|mem| mem.request_focus(input_id));
                    }
                });
            });

            ui.separator();

            if let Some(err) = &self.solve_error {
                ui.colored_label(egui::Color32::from_rgb(200, 80, 100), err);
            }

            if guests.is_empty() {
                ui.label("Add guests to get started.");
            } else if tables.is_empty() {
                ui.label("Configure tables in the left panel.");
            } else if let Some(arr) = arrangement {
                self.render_seating(ui, guests, constraints, arr);
            } else {
                ui.label("Solving...");
            }
        });

        // --- RIGHT PANEL: Constraint Editor ---
        if let Some(selected) = self.selected_guest {
            if guests.get(selected).is_some() {
                SidePanel::right("constraint_panel")
                    .resizable(true)
                    .default_width(240.0)
                    .show(ctx, |ui| {
                        let name = guests.get(selected).map(|g| g.name.as_str()).unwrap_or("?");
                        ui.horizontal(|ui| {
                            ui.heading(format!("Constraints for {}", name));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("✕").clicked() {
                                    self.selected_guest = None;
                                }
                            });
                        });
                        ui.separator();

                        Self::render_constraint_section(
                            ui,
                            selected,
                            guests,
                            constraints,
                            LinkType::Must,
                            "Must sit with",
                            &mut self.constraint_search_must,
                            &mut self.needs_solve,
                        );
                        ui.separator();
                        Self::render_constraint_section(
                            ui,
                            selected,
                            guests,
                            constraints,
                            LinkType::Should,
                            "Should sit with",
                            &mut self.constraint_search_should,
                            &mut self.needs_solve,
                        );
                        ui.separator();
                        Self::render_constraint_section(
                            ui,
                            selected,
                            guests,
                            constraints,
                            LinkType::Could,
                            "Could sit with",
                            &mut self.constraint_search_could,
                            &mut self.needs_solve,
                        );
                        ui.separator();
                        Self::render_constraint_section(
                            ui,
                            selected,
                            guests,
                            constraints,
                            LinkType::Wont,
                            "Wont sit with",
                            &mut self.constraint_search_wont,
                            &mut self.needs_solve,
                        );
                    });
            } else {
                self.selected_guest = None;
            }
        }
    }

    fn render_constraint_section(
        ui: &mut egui::Ui,
        selected: GuestId,
        guests: &GuestList,
        constraints: &mut ConstraintGraph,
        kind: LinkType,
        label: &str,
        search: &mut String,
        needs_solve: &mut bool,
    ) {
        let current: Vec<GuestId> = constraints
            .neighbors(selected)
            .into_iter()
            .filter(|(_, k)| *k == kind)
            .map(|(id, _)| id)
            .collect();

        ui.label(format!("{} ({})", label, current.len()));

        for &cid in &current {
            let cname = guests.get(cid).map(|g| g.name.as_str()).unwrap_or("?");
            ui.horizontal(|ui| {
                ui.label(cname);
                if ui.small_button("✕").clicked() {
                    constraints.remove(selected, cid);
                    *needs_solve = true;
                }
            });
        }

        let candidates: Vec<(GuestId, String)> = guests
            .guests
            .iter()
            .filter(|g| g.id != selected && !current.contains(&g.id))
            .map(|g| (g.id, g.name.clone()))
            .collect();

        if !candidates.is_empty() {
            ui.horizontal(|ui| {
                let current_text = search.clone();
                ComboBox::from_id_salt(format!("combo_{:?}_{}", kind, selected))
                    .selected_text(if current_text.is_empty() {
                        "Add guest..."
                    } else {
                        &current_text
                    })
                    .show_ui(ui, |ui| {
                        for (cid, cname) in &candidates {
                            let lower_search = search.to_lowercase();
                            if lower_search.is_empty()
                                || cname.to_lowercase().contains(&lower_search)
                            {
                                if ui.selectable_label(false, cname.as_str()).clicked() {
                                    constraints.add(selected, *cid, kind);
                                    *needs_solve = true;
                                    search.clear();
                                    ui.close_menu();
                                }
                            }
                        }
                    });

                let response = TextEdit::singleline(search)
                    .hint_text("Search...")
                    .desired_width(80.0)
                    .ui(ui);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let lower = search.to_lowercase();
                    if let Some((cid, _)) = candidates
                        .iter()
                        .find(|(_, n)| n.to_lowercase() == lower)
                    {
                        constraints.add(selected, *cid, kind);
                        *needs_solve = true;
                        search.clear();
                    }
                }
            });
        }
    }

    fn render_seating(
        &mut self,
        ui: &mut egui::Ui,
        guests: &GuestList,
        constraints: &ConstraintGraph,
        arrangement: &SeatingArrangement,
    ) {
        if !arrangement.is_feasible() {
            ui.colored_label(
                egui::Color32::from_rgb(200, 160, 80),
                format!(
                    "{} guest(s) could not be seated",
                    arrangement.unseated.len()
                ),
            );
            ui.label("Unseated:");
            for &id in &arrangement.unseated {
                let name = guests.get(id).map(|g| g.name.as_str()).unwrap_or("?");
                ui.label(name);
            }
            ui.separator();
        }

        ScrollArea::vertical()
            .id_salt("seating_scroll")
            .max_height(ui.available_height() - 10.0)
            .show(ui, |ui| {
                for table in &arrangement.tables {
                    if table.num_occupied() == 0 {
                        continue;
                    }

                    ui.group(|ui| {
                        ui.label(format!(
                            "Table {} ({} seats)",
                            table.id + 1,
                            table.capacity
                        ));

                        ui.horizontal_wrapped(|ui| {
                            for (si, seat) in table.seats.iter().enumerate() {
                                if let Some(guest_id) = seat {
                                    let name = guests
                                        .get(*guest_id)
                                        .map(|g| g.name.as_str())
                                        .unwrap_or("?");

                                    let neighbors = table.neighbors(si);
                                    let mut strong = 0;
                                    for n in &neighbors {
                                        if let Some(kind) = constraints.get(*guest_id, *n) {
                                            if kind == LinkType::Must
                                                || kind == LinkType::Should
                                            {
                                                strong += 1;
                                            }
                                        }
                                    }

                                    let color = if strong >= 2 {
                                        egui::Color32::from_rgb(140, 185, 155)
                                    } else if strong == 1 {
                                        egui::Color32::from_rgb(220, 185, 110)
                                    } else {
                                        egui::Color32::from_rgb(210, 140, 165)
                                    };

                                    let is_selected = self.selected_guest == Some(*guest_id);
                                    let frame = egui::Frame::NONE
                                        .fill(if is_selected {
                                            egui::Color32::from_rgb(245, 225, 232)
                                        } else {
                                            egui::Color32::from_rgb(255, 252, 250)
                                        })
                                        .stroke(egui::Stroke::new(2.0, color));

                                    let inner = frame
                                        .show(ui, |ui| {
                                            let resp = ui.selectable_label(is_selected, name);
                                            if resp.clicked() {
                                                self.selected_guest = Some(*guest_id);
                                            }
                                            resp
                                        })
                                        .inner;
                                    if inner.clicked() {
                                        self.selected_guest = Some(*guest_id);
                                    }
                                    ui.add_space(4.0);
                                }
                            }
                        });
                    });
                    ui.add_space(8.0);
                }
            });
    }

    fn run_solver(
        &mut self,
        guests: &GuestList,
        constraints: &ConstraintGraph,
        tables: &[TableConfig],
        arrangement: &mut Option<SeatingArrangement>,
    ) {
        if tables.is_empty() {
            self.solve_error = Some("Add at least one table first".to_string());
            return;
        }
        let guest_ids: Vec<GuestId> = guests.guests.iter().map(|g| g.id).collect();
        if guest_ids.is_empty() {
            self.solve_error = None;
            *arrangement = None;
            return;
        }

        let capacities: Vec<usize> = tables
            .iter()
            .flat_map(|t| std::iter::repeat(t.capacity).take(t.count))
            .collect();

        match solver::solve(&guest_ids, constraints, &capacities) {
            Ok(result) => {
                *arrangement = Some(result);
                self.solve_error = None;
            }
            Err(e) => {
                self.solve_error = Some(e.to_string());
                *arrangement = None;
            }
        }
    }
}

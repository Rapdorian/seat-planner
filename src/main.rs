fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Seat Planner",
        options,
        Box::new(|_cc| Ok(Box::new(seat_planner::SeatPlannerApp::default()))),
    )
}

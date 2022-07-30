#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// hide console window on Windows in release

mod app;

fn main() {
    let options = eframe::NativeOptions::default();
    // options.icon_data;
    eframe::run_native(
        "存款计算器",
        options,
        Box::new(|cc| Box::new(app::App::new(cc))),
    );
}

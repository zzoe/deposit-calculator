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

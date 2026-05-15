mod app;
mod audio_engine;
mod ffi;

use app::MemDeckGuiApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("MemDeck GUI Foundation"),
        ..Default::default()
    };

    eframe::run_native(
        "MemDeck GUI Foundation",
        native_options,
        Box::new(|cc| {
            MemDeckGuiApp::configure_visuals(&cc.egui_ctx);
            Ok(Box::new(MemDeckGuiApp::default()))
        }),
    )
}

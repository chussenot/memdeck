mod app;
mod audio_engine;
mod editor;
mod ffi;
mod playback;

use app::MemDeckGuiApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([960.0, 640.0])
            .with_title("MEMDECK SOUND MACHINE"),
        ..Default::default()
    };

    eframe::run_native(
        "MEMDECK SOUND MACHINE",
        native_options,
        Box::new(|cc| {
            MemDeckGuiApp::configure_visuals(&cc.egui_ctx);
            Ok(Box::new(MemDeckGuiApp::default()))
        }),
    )
}

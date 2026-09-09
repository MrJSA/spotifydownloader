use eframe::egui;
use spotifydownloader::ui::SpotifyDownloaderApp;

fn main() -> eframe::Result<()> {
    // Start multithreaded Tokio runtime for async tasks
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime");

    let _guard = rt.enter();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Spotify Downloader")
            .with_inner_size([1060.0, 720.0])
            .with_min_inner_size([800.0, 540.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Spotify Downloader",
        options,
        Box::new(|cc| {
            spotifydownloader::theme::install_system_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(SpotifyDownloaderApp::new(cc)))
        }),
    )
}

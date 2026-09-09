mod app;
mod theme;
mod views;

use anyhow::Result;

fn main() -> Result<()> {
    // Wszystko żyje obok pliku wykonywalnego. To jest cała przenośność:
    // skasowanie folderu usuwa launcher, paczkę, Javę i dane gry.
    let katalog = std::env::current_exe()?
        .parent()
        .expect("plik wykonywalny ma katalog")
        .to_path_buf();

    let opcje = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 560.0])
            .with_min_inner_size([720.0, 480.0])
            .with_decorations(false)
            .with_title("Chmurkowy Launcher"),
        ..Default::default()
    };

    eframe::run_native(
        "Chmurkowy Launcher",
        opcje,
        Box::new(move |cc| {
            theme::zastosuj_motyw(&cc.egui_ctx);
            Ok(Box::new(app::App::nowa(katalog)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("nie udało się otworzyć okna: {e}"))
}

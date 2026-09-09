use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        super::pasek_tytulu(ui, ctx, "Ustawienia");
        ui.add_space(20.0);

        ui.label(egui::RichText::new("Pamięć dla gry").color(theme::TEKST));
        ui.add(
            egui::Slider::new(&mut app.pamiec_mb, 2048..=16384)
                .suffix(" MB")
                .step_by(512.0),
        );
        ui.label(
            egui::RichText::new("Paczka z 249 modami potrzebuje co najmniej 4 GB.")
                .size(11.0)
                .color(theme::TEKST_PRZYGASZONY),
        );

        ui.add_space(20.0);
        if ui.button("Otwórz folder gry").clicked() {
            let _ = open::that(app.data().join("instance"));
        }

        ui.add_space(10.0);
        let log = app.data().join("logs").join("game.log");
        if ui
            .add_enabled(log.is_file(), egui::Button::new("Otwórz log gry"))
            .clicked()
        {
            let _ = open::that(&log);
        }

        ui.add_space(10.0);
        if ui.button("Napraw instalację").clicked() {
            // Kasujemy tylko to, co da się odtworzyć z sieci.
            // Katalog instance ze światami gracza zostaje nietknięty.
            let _ = std::fs::remove_dir_all(app.data().join("mc"));
            app.log.push(
                "Usunięto pliki gry — zostaną pobrane ponownie przy następnym starcie.".into(),
            );
        }
        ui.label(
            egui::RichText::new("Pobiera grę od nowa. Światy i ustawienia zostają.")
                .size(11.0)
                .color(theme::TEKST_PRZYGASZONY),
        );

        if app.konto.is_some() {
            ui.add_space(10.0);
            if ui.button("Wyloguj").clicked() {
                chmurka_core::auth::store::clear(&app.data().join("auth.json"));
                app.konto = None;
            }
        }

        ui.add_space(22.0);
        ui.label(
            egui::RichText::new(format!("Chmurkowy Launcher {}", env!("CARGO_PKG_VERSION")))
                .size(11.0)
                .color(theme::TEKST_PRZYGASZONY),
        );
        if let Some(m) = &app.manifest {
            if m.launcher.latest_version != env!("CARGO_PKG_VERSION") {
                ui.label(
                    egui::RichText::new(format!(
                        "Dostępna nowsza wersja: {}",
                        m.launcher.latest_version
                    ))
                    .color(theme::AKCENT),
                );
            }
        }

        ui.add_space(18.0);
        if ui.small_button("Wróć").clicked() {
            app.widok = Widok::Glowny;
        }
    });
}

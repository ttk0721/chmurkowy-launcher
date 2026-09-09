use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("gora-ust")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Ustawienia");
        });

    egui::TopBottomPanel::bottom("dol-ust")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            if let Some(k) = &app.komunikat {
                let kolor = if k.starts_with("Nie udało") {
                    theme::BLAD
                } else {
                    theme::SUKCES
                };
                ui.label(egui::RichText::new(k).size(12.0).color(kolor));
                ui.add_space(theme::S1);
            }
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("Wróć").min_size(egui::vec2(120.0, 44.0)))
                    .clicked()
                {
                    app.komunikat = None;
                    app.widok = Widok::Glowny;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(theme::drobny(&format!(
                        "wersja {}",
                        env!("CARGO_PKG_VERSION")
                    )));
                });
            });
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let instancja = app.data().join("instance");
                let log = app.data().join("logs").join("game.log");
                let mc = app.data().join("mc");

                theme::naglowek_sekcji(ui, "PAMIĘĆ");
                ui.add(
                    egui::Slider::new(&mut app.pamiec_mb, 2048..=16384)
                        .suffix(" MB")
                        .step_by(512.0),
                );
                ui.add_space(4.0);
                ui.label(theme::drobny(
                    "Paczka z 249 modami potrzebuje co najmniej 4 GB.",
                ));

                ui.add_space(theme::S4);
                theme::naglowek_sekcji(ui, "PLIKI");
                ui.horizontal(|ui| {
                    // Folder tworzymy przed otwarciem — inaczej na świeżym
                    // launcherze przycisk cicho nic nie robił.
                    if ui.add(theme::przycisk_zwykly("Otwórz folder gry")).clicked() {
                        let _ = std::fs::create_dir_all(&instancja);
                        match open::that(&instancja) {
                            Ok(()) => app.komunikat = None,
                            Err(e) => {
                                app.komunikat =
                                    Some(format!("Nie udało się otworzyć folderu: {e}"))
                            }
                        }
                    }
                    if super::przycisk_warunkowy(
                        ui,
                        "Otwórz log gry",
                        log.is_file(),
                        "Log powstanie po pierwszym uruchomieniu gry.",
                    )
                    .clicked()
                    {
                        if let Err(e) = open::that(&log) {
                            app.komunikat = Some(format!("Nie udało się otworzyć logu: {e}"));
                        }
                    }
                });

                ui.add_space(theme::S4);
                theme::naglowek_sekcji(ui, "NAPRAWA");
                if super::przycisk_warunkowy(
                    ui,
                    "Napraw instalację",
                    mc.exists(),
                    "Gra nie jest jeszcze zainstalowana — nie ma czego naprawiać.",
                )
                .clicked()
                {
                    match std::fs::remove_dir_all(&mc) {
                        Ok(()) => {
                            app.komunikat = Some(
                                "Pliki gry usunięte. Pobiorą się ponownie przy następnym starcie."
                                    .into(),
                            );
                            app.log
                                .push("Usunięto pliki gry na żądanie użytkownika.".into());
                        }
                        Err(e) => {
                            app.komunikat = Some(format!("Nie udało się usunąć plików gry: {e}"))
                        }
                    }
                }
                ui.add_space(4.0);
                ui.label(theme::drobny(
                    "Pobiera grę od nowa. Światy i ustawienia zostają.",
                ));

                if app.konto.is_some() {
                    ui.add_space(theme::S4);
                    theme::naglowek_sekcji(ui, "KONTO");
                    if ui.add(theme::przycisk_zwykly("Wyloguj")).clicked() {
                        chmurka_core::auth::store::clear(&app.data().join("auth.json"));
                        app.konto = None;
                        app.komunikat = Some("Wylogowano.".into());
                    }
                }

                if let Some(m) = &app.manifest {
                    if m.launcher.latest_version != env!("CARGO_PKG_VERSION") {
                        ui.add_space(theme::S4);
                        theme::naglowek_sekcji(ui, "AKTUALIZACJA");
                        ui.label(
                            egui::RichText::new(format!(
                                "Dostępna nowsza wersja: {}",
                                m.launcher.latest_version
                            ))
                            .size(13.0)
                            .color(theme::AKCENT),
                        );
                        ui.add_space(theme::S1);
                        let klucz = if cfg!(target_os = "windows") {
                            "windows-x64"
                        } else {
                            "linux-x64"
                        };
                        if let Some(adres) = m.launcher.urls.get(klucz).cloned() {
                            if ui.add(theme::przycisk_zwykly("Pobierz nową wersję")).clicked() {
                                if let Err(e) = open::that(&adres) {
                                    app.komunikat =
                                        Some(format!("Nie udało się otworzyć przeglądarki: {e}"));
                                }
                            }
                        }
                        ui.add_space(4.0);
                        ui.label(theme::drobny(
                            "Podmień plik launchera. Folder data zostaje nietknięty.",
                        ));
                    }
                }

                ui.add_space(theme::S3);
            });
        });
}

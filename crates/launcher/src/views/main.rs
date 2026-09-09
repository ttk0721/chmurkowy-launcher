use crate::app::{App, Widok};
use crate::theme::{self, Ikona};

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("gora")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            let tytul = app
                .manifest
                .as_ref()
                .map(|m| m.pack.name.clone())
                .unwrap_or_else(|| "Chmurkowy Launcher".to_string());
            super::pasek_tytulu(ui, ctx, &tytul);

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(egui::RichText::new("Ustawienia").size(13.0)))
                        .clicked()
                    {
                        app.widok = Widok::Ustawienia;
                    }
                    if ui
                        .add(egui::Button::new(egui::RichText::new("Paczki").size(13.0)))
                        .on_hover_text("Paczki zasobów i shadery")
                        .clicked()
                    {
                        // Czytamy stan z plików gry — mógł się zmienić w samej grze.
                        app.odswiez_paczki();
                        app.widok = Widok::Paczki;
                    }
                    let (etykieta, podpowiedz) = match &app.konto {
                        Some(k) => (k.name.clone(), "Zmień konto"),
                        None => ("Nie zalogowano".to_string(), "Kliknij, żeby się zalogować"),
                    };
                    if ui
                        .add(egui::Button::new(egui::RichText::new(etykieta).size(13.0)))
                        .on_hover_text(podpowiedz)
                        .clicked()
                    {
                        app.widok = Widok::Logowanie;
                    }
                });
            });
        });

    // Dolny pasek trzyma stan i log przy krawędzi okna, zamiast zostawiać
    // pustą dolną połowę ekranu.
    egui::TopBottomPanel::bottom("dol")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            if let Some(p) = &app.postep {
                match p.ulamek() {
                    Some(ulamek) => {
                        ui.label(theme::drobny(&format!(
                            "{} — {}",
                            p.stage.opis(),
                            p.licznik()
                        )));
                        ui.add_space(4.0);
                        ui.add(
                            egui::ProgressBar::new(ulamek)
                                .show_percentage()
                                .fill(theme::AKCENT_CIEMNY)
                                .corner_radius(egui::CornerRadius::same(6)),
                        );
                    }
                    // Etap bez mierzalnego postępu — kręciołek zamiast paska
                    // stojącego uparcie na zerze.
                    None => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new().size(14.0).color(theme::AKCENT));
                            ui.label(theme::drobny(&p.label));
                        });
                    }
                }
                ui.add_space(theme::S1);
            }

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (ikona, opis) = if app.pokaz_szczegoly {
                        (Ikona::StrzalkaGora, "Ukryj szczegóły")
                    } else {
                        (Ikona::StrzalkaDol, "Pokaż szczegóły")
                    };
                    if theme::przycisk_ikona(ui, ikona, opis).clicked() {
                        app.pokaz_szczegoly = !app.pokaz_szczegoly;
                    }
                    ui.label(theme::drobny("szczegóły"));
                });
            });

            if app.pokaz_szczegoly {
                ui.add_space(theme::S1);
                egui::Frame::NONE
                    .fill(theme::PANEL)
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(120.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                if app.log.is_empty() {
                                    ui.label(theme::drobny("Nic tu jeszcze nie ma."));
                                }
                                for linia in &app.log {
                                    ui.label(
                                        egui::RichText::new(linia)
                                            .size(11.0)
                                            .color(theme::TEKST_PRZYGASZONY),
                                    );
                                }
                            });
                    });
            }
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            // Wyśrodkowanie pionowe tego, co zostało między paskami.
            let wolne = ui.available_height();
            ui.add_space(((wolne - 200.0) / 2.0).max(0.0));

            ui.vertical_centered(|ui| {
                match &app.manifest {
                    Some(m) => {
                        ui.label(theme::naglowek(&m.pack.edition, 32.0));
                        ui.add_space(6.0);
                        let mody = m
                            .files
                            .iter()
                            .filter(|f| f.path.starts_with("mods/"))
                            .count();
                        ui.label(theme::drobny(&format!(
                            "Minecraft {} · NeoForge {} · {} modów",
                            m.pack.minecraft, m.pack.loader.version, mody
                        )));
                    }
                    None => {
                        ui.add(egui::Spinner::new().size(24.0).color(theme::AKCENT));
                        ui.add_space(theme::S1);
                        ui.label(theme::drobny("Sprawdzam paczkę…"));
                    }
                }

                ui.add_space(theme::S4);

                let gotowy = app.manifest.is_some() && !app.zajety;
                let napis = if app.konto.is_some() {
                    "GRAJ"
                } else {
                    "ZALOGUJ SIĘ"
                };
                if ui
                    .add_enabled(gotowy, theme::przycisk_glowny(napis))
                    .clicked()
                {
                    if app.konto.is_some() {
                        crate::app::uruchom(app);
                    } else {
                        app.widok = Widok::Logowanie;
                    }
                }
                if app.zajety {
                    ui.add_space(theme::S1);
                    ui.label(theme::drobny("Pracuję…"));
                }
            });
        });
}

use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let tytul = app
            .manifest
            .as_ref()
            .map(|m| format!("☁ {}", m.pack.name))
            .unwrap_or_else(|| "☁ Chmurkowy Launcher".to_string());
        super::pasek_tytulu(ui, ctx, &tytul);

        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙").on_hover_text("Ustawienia").clicked() {
                    app.widok = Widok::Ustawienia;
                }
                let etykieta = match &app.konto {
                    Some(k) => k.name.clone(),
                    None => "Nie zalogowano".to_string(),
                };
                if ui.button(etykieta).clicked() {
                    app.widok = Widok::Logowanie;
                }
            });
        });

        ui.add_space(36.0);
        ui.vertical_centered(|ui| {
            match &app.manifest {
                Some(m) => {
                    ui.label(
                        egui::RichText::new(&m.pack.edition)
                            .size(20.0)
                            .color(theme::TEKST),
                    );
                    let mody = m.files.iter().filter(|f| f.path.starts_with("mods/")).count();
                    ui.label(
                        egui::RichText::new(format!(
                            "Minecraft {} · NeoForge {} · {} modów",
                            m.pack.minecraft, m.pack.loader.version, mody
                        ))
                        .color(theme::TEKST_PRZYGASZONY),
                    );
                }
                None => {
                    ui.label(
                        egui::RichText::new("Sprawdzam paczkę…").color(theme::TEKST_PRZYGASZONY),
                    );
                }
            }

            ui.add_space(30.0);

            let gotowy = app.manifest.is_some() && !app.zajety;
            let napis = if app.konto.is_some() {
                "GRAJ"
            } else {
                "ZALOGUJ SIĘ"
            };
            let przycisk = egui::Button::new(egui::RichText::new(napis).size(22.0).strong())
                .min_size(egui::vec2(240.0, 56.0));

            if ui.add_enabled(gotowy, przycisk).clicked() {
                if app.konto.is_some() {
                    crate::app::uruchom(app);
                } else {
                    app.widok = Widok::Logowanie;
                }
            }
        });

        ui.add_space(24.0);

        if let Some(p) = &app.postep {
            let ulamek = if p.total > 0 {
                p.done as f32 / p.total as f32
            } else {
                0.0
            };
            ui.label(
                egui::RichText::new(format!("{} — {}/{}", p.stage.opis(), p.done, p.total))
                    .color(theme::TEKST_PRZYGASZONY),
            );
            ui.add(egui::ProgressBar::new(ulamek).show_percentage());
        }

        if let Some(e) = &app.blad {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(e).color(theme::BLAD));
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let napis = if app.pokaz_szczegoly {
                    "szczegóły ▴"
                } else {
                    "szczegóły ▾"
                };
                if ui.small_button(napis).clicked() {
                    app.pokaz_szczegoly = !app.pokaz_szczegoly;
                }
            });
        });

        if app.pokaz_szczegoly {
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for linia in &app.log {
                        ui.label(
                            egui::RichText::new(linia)
                                .size(11.0)
                                .color(theme::TEKST_PRZYGASZONY),
                        );
                    }
                });
        }
    });
}

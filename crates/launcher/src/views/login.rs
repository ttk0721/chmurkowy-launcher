use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        super::pasek_tytulu(ui, ctx, "Zaloguj się");
        ui.add_space(26.0);

        ui.vertical_centered(|ui| {
            match &app.kod {
                Some(kod) => {
                    let kod_tekst = kod.user_code.clone();
                    let adres = kod.verification_uri.clone();
                    ui.label(
                        egui::RichText::new("Wpisz ten kod na stronie Microsoftu:")
                            .color(theme::TEKST_PRZYGASZONY),
                    );
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(&kod_tekst)
                            .size(34.0)
                            .strong()
                            .color(theme::AKCENT),
                    );
                    ui.add_space(14.0);
                    if ui.button("Kopiuj kod i otwórz przeglądarkę").clicked() {
                        if let Ok(mut schowek) = arboard::Clipboard::new() {
                            let _ = schowek.set_text(kod_tekst);
                        }
                        let _ = open::that(&adres);
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(&adres)
                            .size(11.0)
                            .color(theme::TEKST_PRZYGASZONY),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Czekam na potwierdzenie…")
                            .color(theme::TEKST_PRZYGASZONY),
                    );
                }
                None => {
                    let przycisk =
                        egui::Button::new(egui::RichText::new("Zaloguj przez Microsoft").size(16.0))
                            .min_size(egui::vec2(280.0, 46.0));
                    if ui.add_enabled(!app.zajety, przycisk).clicked() {
                        crate::app::zaloguj_microsoft(app);
                    }
                }
            }

            ui.add_space(24.0);
            ui.label(egui::RichText::new("—— albo ——").color(theme::TEKST_PRZYGASZONY));
            ui.add_space(16.0);

            ui.label(egui::RichText::new("Tryb offline (do testów)").color(theme::TEKST));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // Wyśrodkowanie pary pole+przycisk wewnątrz vertical_centered.
                ui.add_space((ui.available_width() - 300.0).max(0.0) / 2.0);
                ui.add(
                    egui::TextEdit::singleline(&mut app.nick_offline)
                        .hint_text("nick")
                        .desired_width(190.0),
                );
                let mozna = !app.nick_offline.trim().is_empty();
                if ui.add_enabled(mozna, egui::Button::new("Graj")).clicked() {
                    let nick = app.nick_offline.trim().to_string();
                    app.konto = Some(chmurka_core::auth::offline::offline_account(&nick));
                    app.blad = None;
                    app.widok = Widok::Glowny;
                }
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Działa tylko na serwerze z online-mode=false")
                    .size(11.0)
                    .color(theme::TEKST_PRZYGASZONY),
            );

            ui.add_space(20.0);
            if ui.small_button("Wróć").clicked() {
                app.widok = Widok::Glowny;
            }

            if let Some(e) = &app.blad {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(e).color(theme::BLAD));
            }
        });
    });
}

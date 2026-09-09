use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("gora-log")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Zaloguj się");
        });

    egui::TopBottomPanel::bottom("dol-log")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            if ui
                .add(egui::Button::new("Wróć").min_size(egui::vec2(120.0, 44.0)))
                .clicked()
            {
                app.widok = Widok::Glowny;
            }
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            let wolne = ui.available_height();
            ui.add_space(((wolne - 270.0) / 2.0).max(0.0));

            ui.vertical_centered(|ui| {
                match &app.kod {
                    Some(kod) => {
                        let kod_tekst = kod.user_code.clone();
                        let adres = kod.verification_uri.clone();
                        ui.label(theme::drobny("Wpisz ten kod na stronie Microsoftu:"));
                        ui.add_space(theme::S2);
                        egui::Frame::NONE
                            .fill(theme::PANEL)
                            .stroke(egui::Stroke::new(1.0_f32, theme::OBRYS))
                            .corner_radius(egui::CornerRadius::same(12))
                            .inner_margin(egui::Margin::symmetric(28, 14))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&kod_tekst)
                                        .size(34.0)
                                        .family(theme::polgruba())
                                        .color(theme::AKCENT),
                                );
                            });
                        ui.add_space(theme::S2);
                        if ui
                            .add(theme::przycisk_zwykly("Kopiuj kod i otwórz przeglądarkę"))
                            .clicked()
                        {
                            if let Ok(mut schowek) = arboard::Clipboard::new() {
                                let _ = schowek.set_text(kod_tekst);
                            }
                            let _ = open::that(&adres);
                        }
                        ui.add_space(theme::S1);
                        ui.label(theme::drobny(&adres));
                        ui.add_space(theme::S2);
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() - 210.0).max(0.0) / 2.0);
                            ui.add(egui::Spinner::new().size(14.0).color(theme::AKCENT));
                            ui.label(theme::drobny("Czekam na potwierdzenie…"));
                        });
                    }
                    None => {
                        if ui
                            .add_enabled(!app.zajety, theme::przycisk_glowny("Zaloguj przez Microsoft"))
                            .clicked()
                        {
                            crate::app::zaloguj_microsoft(app);
                        }
                    }
                }

                ui.add_space(theme::S4);
                ui.label(theme::drobny("albo"));
                ui.add_space(theme::S3);

                ui.label(
                    egui::RichText::new("Tryb offline (do testów)")
                        .size(13.0)
                        .family(theme::polgruba())
                        .color(theme::TEKST),
                );
                ui.add_space(theme::S1);
                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 300.0).max(0.0) / 2.0);
                    // Wysokość pola bierze się z marginesu wewnętrznego, nie z
                    // add_sized — tamto rozciągało ramkę, a tekst zostawał
                    // przyklejony do lewego górnego rogu.
                    ui.add(
                        egui::TextEdit::singleline(&mut app.ustawienia.nick_offline)
                            .hint_text("nick")
                            .desired_width(200.0)
                            .margin(egui::Margin::symmetric(12, 11)),
                    );
                    let mozna = !app.ustawienia.nick_offline.trim().is_empty();
                    if ui
                        .add_enabled(
                            mozna,
                            egui::Button::new("Graj").min_size(egui::vec2(88.0, 40.0)),
                        )
                        .on_disabled_hover_text("Najpierw wpisz nick.")
                        .clicked()
                    {
                        let nick = app.ustawienia.nick_offline.trim().to_string();
                        app.konto = Some(chmurka_core::auth::offline::offline_account(&nick));
                        app.zapisz_ustawienia();
                        app.widok = Widok::Glowny;
                    }
                });
                ui.add_space(theme::S1);
                ui.label(theme::drobny(
                    "Działa tylko na serwerze z online-mode=false",
                ));
            });
        });
}

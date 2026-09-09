use crate::app::{App, Widok};
use crate::theme;
use chmurka_core::bledy::OSTATNIA_DESKA;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    let Some(b) = app.blad_z_kodem.clone() else {
        app.widok = Widok::Glowny;
        return;
    };

    egui::TopBottomPanel::top("gora-blad")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Coś poszło nie tak");
        });

    egui::TopBottomPanel::bottom("dol-blad")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            // Zdanie o administracji jest na samym końcu i celowo uprzedza,
            // że odpowiedź może nie przyjść od razu.
            ui.label(
                egui::RichText::new(OSTATNIA_DESKA)
                    .size(11.0)
                    .color(theme::TEKST_PRZYGASZONY),
            );
            ui.add_space(theme::S2);

            ui.horizontal(|ui| {
                if ui
                    .add(theme::przycisk_glowny("Spróbuj ponownie").min_size(egui::vec2(200.0, 44.0)))
                    .clicked()
                {
                    app.blad_z_kodem = None;
                    app.widok = Widok::Glowny;
                }
                if ui
                    .add(theme::przycisk_zwykly("Kopiuj szczegóły dla administracji"))
                    .on_hover_text("Wklej to w wiadomości do administracji serwera.")
                    .clicked()
                {
                    if let Ok(mut schowek) = arboard::Clipboard::new() {
                        let _ = schowek.set_text(b.do_schowka());
                        app.komunikat = Some("Skopiowano.".into());
                    }
                }
                if app.komunikat.is_some() {
                    ui.label(
                        egui::RichText::new("skopiowane")
                            .size(11.0)
                            .color(theme::SUKCES),
                    );
                }
            });
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Kod błędu jako pierwszy i wyraźny — to jego gracz podyktuje
                // przez telefon albo wklei w wiadomości.
                egui::Frame::NONE
                    .fill(theme::PANEL)
                    .stroke(egui::Stroke::new(1.0_f32, theme::BLAD))
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&b.kod)
                                    .size(15.0)
                                    .family(theme::polgruba())
                                    .color(theme::BLAD),
                            );
                            ui.add_space(theme::S2);
                            ui.label(theme::naglowek(&b.tytul, 17.0));
                        });
                    });

                ui.add_space(theme::S3);
                ui.label(
                    egui::RichText::new(&b.co_sie_stalo)
                        .size(14.0)
                        .color(theme::TEKST),
                );

                ui.add_space(theme::S4);
                ui.label(theme::naglowek("Co zrobić", 14.0));
                ui.add_space(theme::S1);

                for (i, krok) in b.co_zrobic.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{}.", i + 1))
                                .size(13.0)
                                .family(theme::polgruba())
                                .color(theme::AKCENT),
                        );
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(krok).size(13.0).color(theme::TEKST));
                    });
                    ui.add_space(6.0);
                }

                ui.add_space(theme::S3);
                // Szczegoly techniczne domyslnie schowane — dla gracza to szum,
                // a dla administracji i tak jest przycisk kopiowania.
                ui.collapsing(
                    egui::RichText::new("Szczegóły techniczne")
                        .size(11.0)
                        .color(theme::TEKST_PRZYGASZONY),
                    |ui| {
                        egui::Frame::NONE
                            .fill(theme::PANEL)
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&b.szczegoly)
                                        .size(10.0)
                                        .color(theme::TEKST_PRZYGASZONY),
                                );
                            });
                    },
                );
                ui.add_space(theme::S2);
            });
        });
}

use crate::app::{App, Widok};
use crate::theme;

/// Okno z tym, co gra wypisuje w trakcie ładowania.
///
/// Powstało z prostego pytania testera: „nie wiem, czy program od dziesięciu
/// minut coś ładuje, czy po prostu umarł, i nie mam jak tego sprawdzić".
/// Najważniejsze jest tu jedno zdanie u góry — sam log to materiał dla
/// administracji, nie dla gracza.
pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    app.konsola.odswiez();

    egui::TopBottomPanel::top("gora-konsola")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Co się dzieje");
        });

    egui::TopBottomPanel::bottom("dol-konsola")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.add(theme::przycisk_zwykly("Wróć")).clicked() {
                    app.widok = Widok::Glowny;
                }
                if ui
                    .add(theme::przycisk_zwykly("Kopiuj log"))
                    .on_hover_text("Wklej to w wiadomości do administracji serwera.")
                    .clicked()
                {
                    app.komunikat = Some(crate::schowek::komunikat(
                        crate::schowek::kopiuj(&app.konsola.do_schowka()),
                        "Użyj przycisku „Otwórz folder z logiem” i wyślij plik game.log.",
                    ));
                }
                if super::przycisk_warunkowy(
                    ui,
                    "Otwórz folder z logiem",
                    app.konsola.sciezka().parent().is_some_and(|p| p.is_dir()),
                    "Log powstanie dopiero przy pierwszym uruchomieniu gry.",
                )
                .clicked()
                {
                    if let Some(katalog) = app.konsola.sciezka().parent() {
                        let _ = open::that_detached(katalog);
                    }
                }
                // Wcześniej stało tu na sztywno „skopiowane" — także wtedy,
                // gdy schowek odmówił i nic się nie skopiowało.
                if let Some(k) = &app.komunikat {
                    let kolor = if k.starts_with("Skopiowano") {
                        theme::SUKCES
                    } else {
                        theme::BLAD
                    };
                    ui.label(egui::RichText::new(k).size(11.0).color(kolor));
                }
            });
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            // Zdanie o tym, czy czekać, czy działać — jedyna rzecz, którą
            // gracz naprawdę musi tu przeczytać. Stąd nad logiem i większą
            // czcionką niż on.
            let opis = app.konsola.opis_stanu(app.gra_dziala);
            let kolor = match app.konsola.stan() {
                chmurka_core::konsola::Stan::Cisza { .. } => theme::AKCENT,
                _ => theme::TEKST_PRZYGASZONY,
            };
            ui.label(egui::RichText::new(opis).size(12.0).color(kolor));
            ui.add_space(theme::S2);

            let linie = app.konsola.linie();
            if linie.is_empty() {
                ui.label(theme::drobny(
                    "Log jest pusty. Uruchom grę przyciskiem GRAJ — zapiski pojawią się tutaj.",
                ));
                return;
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                // Trzymamy się końca, bo tam dzieje się to, co świeże.
                // Bez tego okno stałoby na pierwszej linii sprzed pół godziny.
                .stick_to_bottom(true)
                .show_rows(ui, 14.0, linie.len(), |ui, zakres| {
                    for i in zakres {
                        ui.label(
                            egui::RichText::new(&linie[i])
                                .monospace()
                                .size(11.0)
                                .color(kolor_linii(&linie[i])),
                        );
                    }
                });
        });
}

/// Błędy i ostrzeżenia mają się rzucać w oczy — reszta to szum.
fn kolor_linii(linia: &str) -> egui::Color32 {
    if linia.contains("ERROR") || linia.contains("FATAL") || linia.contains("Exception") {
        theme::BLAD
    } else if linia.contains("WARN") {
        theme::AKCENT
    } else {
        theme::TEKST_PRZYGASZONY
    }
}

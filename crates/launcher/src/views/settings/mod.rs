//! Ekran Ustawień: pasek zakładek i to, co jest wspólne dla wszystkich.
//!
//! Do wersji 0.4.20 wszystko siedziało na jednym przewijanym ekranie i było
//! tego pięć przełączników. Po dołożeniu okna gry, Javy i własnych komend
//! ta lista miałaby kilkanaście pozycji, a droga do pola „ścieżka do Javy"
//! wiodłaby przez pół ekranu przewijania.
//!
//! Dwie zakładki — Java i Zaawansowane — potrafią sprawić, że gra przestanie
//! się uruchamiać. Nie chowamy ich za przełącznikiem, bo ktoś, kto ich
//! potrzebuje, ma je znaleźć od razu. Zamiast tego mówimy wprost: raz w oknie
//! przy pierwszym wejściu i **za każdym razem** paskiem na górze zakładki.

mod gra;
mod java;
mod ogolne;
mod zaawansowane;

use crate::app::{App, Widok};
use crate::theme;

/// Zakładki ekranu Ustawień, w kolejności od najbezpieczniejszej.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zakladka {
    Ogolne,
    Gra,
    Java,
    Zaawansowane,
}

impl Zakladka {
    pub const WSZYSTKIE: [Zakladka; 4] = [
        Zakladka::Ogolne,
        Zakladka::Gra,
        Zakladka::Java,
        Zakladka::Zaawansowane,
    ];

    pub fn napis(&self) -> &'static str {
        match self {
            Zakladka::Ogolne => "Ogólne",
            Zakladka::Gra => "Gra",
            Zakladka::Java => "Java",
            Zakladka::Zaawansowane => "Zaawansowane",
        }
    }

    /// Nazwa, pod jaką zakładkę da się wskazać z wiersza poleceń
    /// (`CHMURKA_ZAKLADKA`). Tylko do pracy nad wyglądem.
    pub fn z_nazwy(nazwa: &str) -> Option<Zakladka> {
        Zakladka::WSZYSTKIE
            .into_iter()
            .find(|z| z.klucz() == nazwa.to_lowercase())
    }

    fn klucz(&self) -> &'static str {
        match self {
            Zakladka::Ogolne => "ogolne",
            Zakladka::Gra => "gra",
            Zakladka::Java => "java",
            Zakladka::Zaawansowane => "zaawansowane",
        }
    }

    /// Czy wejście tutaj wymaga ostrzeżenia.
    pub fn tylko_dla_zaawansowanych(&self) -> bool {
        matches!(self, Zakladka::Java | Zakladka::Zaawansowane)
    }

    /// Czy gracz przyjął już ostrzeżenie o tej zakładce.
    fn ostrzezenie_przyjete(&self, app: &App) -> bool {
        match self {
            Zakladka::Java => app.ustawienia.java.przyjeto_ostrzezenie,
            Zakladka::Zaawansowane => app.ustawienia.zaawansowane.przyjeto_ostrzezenie,
            _ => true,
        }
    }

    /// Czym grozi nieumiejętna zmiana akurat tutaj. Zdanie trafia i do okna
    /// ostrzeżenia, i na stały pasek — mają mówić to samo.
    fn czym_grozi(&self) -> &'static str {
        match self {
            Zakladka::Java => {
                "Zmiana tych wartości bez wiedzy, co robią, może sprawić, że gra \
                 przestanie się uruchamiać."
            }
            Zakladka::Zaawansowane => {
                "Wpisane tu komendy uruchamiają się na Twoim komputerze przy każdym \
                 starcie gry. Błąd w nich może sprawić, że gra przestanie się uruchamiać."
            }
            _ => "",
        }
    }
}

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    if app.pyta_o_odinstalowanie {
        okno_odinstalowania(app, ctx);
    }
    if let Some(z) = app.ostrzezenie_zakladki {
        okno_ostrzezenia(app, ctx, z);
    }

    egui::TopBottomPanel::top("gora-ust")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Ustawienia");
            pasek_zakladek(app, ui);
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
                    .add(theme::przycisk_zwykly("Wróć").min_size(egui::vec2(120.0, 44.0)))
                    .clicked()
                {
                    app.zapisz_ustawienia();
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
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let zmienione = match app.zakladka {
                        Zakladka::Ogolne => ogolne::rysuj(app, ui),
                        Zakladka::Gra => gra::rysuj(app, ui),
                        Zakladka::Java => java::rysuj(app, ui),
                        Zakladka::Zaawansowane => zaawansowane::rysuj(app, ui),
                    };
                    ui.add_space(theme::S3);

                    // Zapis po każdej zmianie — ustawienia nie mogą znikać
                    // po zamknięciu launchera, jak działo się wcześniej.
                    if zmienione {
                        app.zapisz_ustawienia();
                    }
                });
        });
}

fn pasek_zakladek(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = theme::S2;
        for z in Zakladka::WSZYSTKIE {
            if przycisk_zakladki(ui, z.napis(), app.zakladka == z).clicked() {
                otworz_zakladke(app, z);
            }
        }
    });
    ui.add_space(theme::S1);
}

/// Przechodzi na zakładkę — albo najpierw pyta, czy gracz wie, gdzie wchodzi.
pub(crate) fn otworz_zakladke(app: &mut App, z: Zakladka) {
    if z.tylko_dla_zaawansowanych() && !z.ostrzezenie_przyjete(app) {
        app.ostrzezenie_zakladki = Some(z);
        return;
    }
    app.zakladka = z;
    app.komunikat = None;
}

fn przycisk_zakladki(ui: &mut egui::Ui, napis: &str, aktywna: bool) -> egui::Response {
    let kolor = if aktywna {
        theme::TEKST
    } else {
        theme::TEKST_PRZYGASZONY
    };
    let odp = ui
        .add(
            egui::Button::new(egui::RichText::new(napis).size(13.0).color(kolor))
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE)
                .corner_radius(egui::CornerRadius::ZERO)
                .min_size(egui::vec2(0.0, 30.0)),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    // Podkreślenie zamiast wypełnienia: na ciemnym tle wypełniony prostokąt
    // wygląda jak wciśnięty przycisk, a nie jak otwarta zakładka.
    if aktywna {
        ui.painter().hline(
            odp.rect.x_range(),
            odp.rect.bottom(),
            egui::Stroke::new(2.0_f32, theme::AKCENT),
        );
    }
    odp
}

/// Stały pasek na górze zakładki dla zaawansowanych.
///
/// Jest przy każdym wejściu, nie tylko przy pierwszym. Okno z ostrzeżeniem
/// klika się raz i zapomina; pasek przypomina, gdzie się jest, także wtedy,
/// gdy ktoś wróci tu za pół roku.
pub fn pasek_ostrzezenia(ui: &mut egui::Ui, z: Zakladka) {
    egui::Frame::NONE
        .fill(theme::PANEL)
        .stroke(egui::Stroke::new(
            1.0_f32,
            theme::OSTRZEZENIE.gamma_multiply(0.5),
        ))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new("TYLKO DLA ZAAWANSOWANYCH")
                    .size(11.0)
                    .family(theme::polgruba())
                    .color(theme::OSTRZEZENIE),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(z.czym_grozi())
                    .size(12.0)
                    .color(theme::TEKST_PRZYGASZONY),
            );
        });
    ui.add_space(theme::S3);
}

/// Przycisk przywracający całą zakładkę do stanu fabrycznego.
///
/// Najważniejszy element obu zaawansowanych zakładek: bez drogi powrotnej
/// każde pole tutaj byłoby pułapką bez wyjścia.
pub fn przycisk_domyslnych(ui: &mut egui::Ui, cokolwiek_zmienione: bool) -> bool {
    ui.add_space(theme::S4);
    let odp = super::przycisk_warunkowy(
        ui,
        "Przywróć domyślne",
        cokolwiek_zmienione,
        "Wszystko w tej zakładce jest już domyślne.",
    );
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Cofa wszystkie ustawienia z tej zakładki do stanu, w jakim launcher \
         przyszedł. Nic poza tą zakładką się nie zmieni.",
    ));
    odp.clicked()
}

/// Okno pokazywane przy pierwszym wejściu w zakładkę dla zaawansowanych.
fn okno_ostrzezenia(app: &mut App, ctx: &egui::Context, z: Zakladka) {
    let mut otwarte = true;
    egui::Window::new(format!("Zakładka „{}\u{201d}", z.napis()))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .open(&mut otwarte)
        .frame(
            egui::Frame::NONE
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0_f32, theme::OSTRZEZENIE))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ctx, |ui| {
            ui.set_max_width(430.0);
            ui.label(
                egui::RichText::new("Ta zakładka jest dla zaawansowanych użytkowników.")
                    .size(13.0)
                    .color(theme::OSTRZEZENIE),
            );
            ui.add_space(theme::S1);
            ui.label(
                egui::RichText::new(z.czym_grozi())
                    .size(12.0)
                    .color(theme::TEKST),
            );
            ui.add_space(theme::S2);
            ui.label(theme::drobny(
                "Wszystko da się cofnąć przyciskiem „Przywróć domyślne” na dole zakładki. \
                 Światy i pliki gry nie mają z tym nic wspólnego i nic im nie grozi.",
            ));
            ui.add_space(theme::S3);

            if ui
                .add(theme::przycisk_glowny("Rozumiem, pokaż").min_size(egui::vec2(400.0, 44.0)))
                .clicked()
            {
                match z {
                    Zakladka::Java => app.ustawienia.java.przyjeto_ostrzezenie = true,
                    Zakladka::Zaawansowane => {
                        app.ustawienia.zaawansowane.przyjeto_ostrzezenie = true
                    }
                    _ => {}
                }
                app.zapisz_ustawienia();
                app.ostrzezenie_zakladki = None;
                app.zakladka = z;
                app.komunikat = None;
            }
            ui.add_space(theme::S1);
            if ui
                .add(theme::przycisk_zwykly("Wolę nie").min_size(egui::vec2(400.0, 40.0)))
                .clicked()
            {
                app.ostrzezenie_zakladki = None;
            }
        });

    // Krzyżyk w rogu okna znaczy „rozmyśliłem się".
    if !otwarte {
        app.ostrzezenie_zakladki = None;
    }
}

/// Okno potwierdzenia odinstalowania.
///
/// Pytanie o dane jest tu osobnym, świadomym wyborem, a nie polem wyboru
/// obok przycisku — skasowania światów z singleplayera nie da się cofnąć.
/// Łagodniejsze wyjście zostawia dane; wariant kasujący wszystko jest
/// czerwony i mówi wprost, co zniknie.
fn okno_odinstalowania(app: &mut App, ctx: &egui::Context) {
    use chmurka_core::odinstaluj::Zakres;

    let mut otwarte = true;
    egui::Window::new("Odinstalować Chmurkowy Launcher?")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .open(&mut otwarte)
        .frame(
            egui::Frame::NONE
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0_f32, theme::BLAD))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ctx, |ui| {
            ui.set_max_width(430.0);
            ui.label(theme::drobny(
                "Sam launcher zniknie w obu przypadkach. Pytanie dotyczy tego, \
                 co zostanie po nim na dysku:",
            ));
            ui.add_space(theme::S2);

            ui.label(
                egui::RichText::new("Twoje światy z singleplayera, ustawienia gry i paczka modów")
                    .size(12.0)
                    .color(theme::TEKST),
            );
            ui.add_space(theme::S1);
            ui.label(theme::drobny(&format!("Leżą w: {}", app.data().display())));
            ui.add_space(theme::S3);

            if ui
                .add(theme::przycisk_zwykly("Zostaw moje światy").min_size(egui::vec2(400.0, 44.0)))
                .on_hover_text("Usuwa tylko program. Wszystko inne zostaje na dysku.")
                .clicked()
            {
                app.odinstaluj(Zakres::TylkoProgram);
            }
            ui.add_space(theme::S1);

            if ui
                .add(
                    theme::przycisk_zwykly("Usuń wszystko, razem ze światami")
                        .min_size(egui::vec2(400.0, 44.0))
                        .fill(theme::BLAD),
                )
                .on_hover_text("Tego nie da się cofnąć.")
                .clicked()
            {
                app.odinstaluj(Zakres::Wszystko);
            }
            ui.add_space(theme::S2);

            if ui
                .add(theme::przycisk_zwykly("Nie odinstalowuj").min_size(egui::vec2(400.0, 40.0)))
                .clicked()
            {
                app.pyta_o_odinstalowanie = false;
            }
        });

    // Krzyżyk w rogu okna znaczy „rozmyśliłem się".
    if !otwarte {
        app.pyta_o_odinstalowanie = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Zakladki bezpieczne nie moga niczego blokowac, a obie zaawansowane
    /// musza miec zdanie o tym, czym grozi ich uzycie — pasek i okno bierza
    /// tekst stad, wiec puste zostawiloby pusty pasek.
    #[test]
    fn tylko_java_i_zaawansowane_ostrzegaja() {
        for z in Zakladka::WSZYSTKIE {
            if z.tylko_dla_zaawansowanych() {
                assert!(
                    !z.czym_grozi().is_empty(),
                    "{} ostrzega, wiec musi miec czym",
                    z.napis()
                );
            } else {
                assert!(z.czym_grozi().is_empty());
            }
        }
        assert!(!Zakladka::Ogolne.tylko_dla_zaawansowanych());
        assert!(!Zakladka::Gra.tylko_dla_zaawansowanych());
        assert!(Zakladka::Java.tylko_dla_zaawansowanych());
        assert!(Zakladka::Zaawansowane.tylko_dla_zaawansowanych());
    }

    #[test]
    fn nazwy_z_wiersza_polecen_trafiaja_we_wlasciwe_zakladki() {
        for z in Zakladka::WSZYSTKIE {
            assert_eq!(Zakladka::z_nazwy(z.klucz()), Some(z));
            assert_eq!(Zakladka::z_nazwy(&z.klucz().to_uppercase()), Some(z));
        }
        assert_eq!(Zakladka::z_nazwy("nie-ma-takiej"), None);
    }

    #[test]
    fn kazda_zakladka_ma_nazwe() {
        for z in Zakladka::WSZYSTKIE {
            assert!(!z.napis().is_empty());
        }
    }
}

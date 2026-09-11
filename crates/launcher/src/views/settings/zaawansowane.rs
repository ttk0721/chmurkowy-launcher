//! Zakładka „Zaawansowane": własne komendy i zmienne środowiskowe.
//!
//! Wszystko tutaj wykonuje się na komputerze gracza przy każdym starcie gry.
//! Nazwy zmiennych są takie same jak w Prism Launcherze — gotowe skrypty
//! przenoszą się między launcherami bez przeróbek, a wymyślanie własnych
//! nazw nie dałoby nikomu niczego.

use super::Zakladka;
use crate::app::App;
use crate::theme;
use chmurka_core::ustawienia::{Zaawansowane, Zmienna};

/// Zmienne, które launcher wstawia komendom. Wypisane wprost, bo bez tej
/// listy trzy puste pola tekstowe nie mówią nic o tym, co da się w nich zrobić.
const ZMIENNE_OPIS: [(&str, &str); 6] = [
    ("$INST_NAME", "nazwa paczki"),
    ("$INST_ID", "krótki identyfikator instancji"),
    ("$INST_DIR", "folder z modami, światami i konfiguracją"),
    ("$INST_MC_DIR", "folder z plikami samego Minecrafta"),
    ("$INST_JAVA", "plik wykonywalny Javy, którym ruszy gra"),
    (
        "$INST_JAVA_ARGS",
        "parametry pamięci i inne, które dostanie Java",
    ),
];

pub fn rysuj(app: &mut App, ui: &mut egui::Ui) -> bool {
    super::pasek_ostrzezenia(ui, Zakladka::Zaawansowane);
    let mut zmienione = false;

    // --- KOMENDY ---
    theme::naglowek_sekcji(ui, "WŁASNE KOMENDY");
    zmienione |= komenda(
        ui,
        &mut app.ustawienia.zaawansowane.komenda_przed,
        "Przed uruchomieniem gry",
        "np. cp -r \"$INST_DIR/saves\" ~/kopie",
        "Wykona się przed startem gry. Jeśli zakończy się błędem, launcher \
         NIE uruchomi gry — bo skoro kopia świata się nie udała, granie na nim \
         jest dokładnie tym, przed czym ta komenda miała chronić.",
    );

    ui.add_space(theme::S2);
    zmienione |= komenda(
        ui,
        &mut app.ustawienia.zaawansowane.komenda_wrapper,
        "Komenda opakowująca",
        "np. gamemoderun",
        "Wstawi się przed ścieżką do Javy, więc gra uruchomi się „wewnątrz” tego \
         programu. Na Linuksie tak włącza się gamemoderun albo prime-run. \
         Zostaw puste, jeśli nie wiesz, po co to.",
    );

    ui.add_space(theme::S2);
    zmienione |= komenda(
        ui,
        &mut app.ustawienia.zaawansowane.komenda_po,
        "Po zakończeniu gry",
        "np. notify-send \"Koniec grania\"",
        "Wykona się po wyjściu z gry. Błąd tutaj niczego nie przerywa — gra już \
         się skończyła — a launcher tylko o nim wspomni.",
    );

    ui.add_space(theme::S2);
    ui.label(theme::drobny(
        "Komendy uruchamiają się w folderze gry, przez powłokę systemu, \
         z limitem dwóch minut. Dostają te zmienne:",
    ));
    ui.add_space(theme::S1);
    for (nazwa, opis) in ZMIENNE_OPIS {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(nazwa)
                    .size(11.0)
                    .family(egui::FontFamily::Monospace)
                    .color(theme::AKCENT),
            );
            ui.label(theme::drobny(&format!("— {opis}")));
        });
    }

    // --- ZMIENNE ŚRODOWISKOWE ---
    ui.add_space(theme::S4);
    theme::naglowek_sekcji(ui, "ZMIENNE ŚRODOWISKOWE");
    ui.label(theme::drobny(
        "Dokładane do procesu gry i do własnych komend powyżej. Na Linuksie to \
         jedyna droga do rzeczy w rodzaju MESA_GL_VERSION_OVERRIDE czy DRI_PRIME.",
    ));
    ui.add_space(theme::S2);
    zmienione |= tabela_zmiennych(app, ui);

    // --- POWRÓT DO FABRYCZNYCH ---
    let domyslne = Zaawansowane {
        przyjeto_ostrzezenie: app.ustawienia.zaawansowane.przyjeto_ostrzezenie,
        ..Zaawansowane::default()
    };
    if super::przycisk_domyslnych(ui, app.ustawienia.zaawansowane != domyslne) {
        app.ustawienia.zaawansowane = domyslne;
        app.komunikat = Some("Wyczyszczono komendy i zmienne środowiskowe.".into());
        zmienione = true;
    }

    zmienione
}

fn komenda(
    ui: &mut egui::Ui,
    wartosc: &mut String,
    etykieta: &str,
    podpowiedz: &str,
    opis: &str,
) -> bool {
    ui.label(egui::RichText::new(etykieta).size(12.0).color(theme::TEKST));
    ui.add_space(4.0);
    let zmiana = ui
        .add(
            egui::TextEdit::singleline(wartosc)
                .hint_text(podpowiedz)
                .desired_width(ui.available_width().min(560.0))
                .margin(egui::Margin::symmetric(12, 11)),
        )
        .changed();
    ui.add_space(4.0);
    ui.label(theme::drobny(opis));
    zmiana
}

fn tabela_zmiennych(app: &mut App, ui: &mut egui::Ui) -> bool {
    let mut zmienione = false;
    let mut do_usuniecia: Option<usize> = None;
    let szerokosc = ui.available_width().min(560.0);

    for (i, z) in app.ustawienia.zaawansowane.zmienne.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            zmienione |= ui
                .add(
                    egui::TextEdit::singleline(&mut z.nazwa)
                        .hint_text("NAZWA")
                        .desired_width(szerokosc * 0.38)
                        .margin(egui::Margin::symmetric(10, 9)),
                )
                .changed();
            zmienione |= ui
                .add(
                    egui::TextEdit::singleline(&mut z.wartosc)
                        .hint_text("wartość")
                        .desired_width(szerokosc * 0.45)
                        .margin(egui::Margin::symmetric(10, 9)),
                )
                .changed();
            if ui
                .add(
                    egui::Button::new(egui::RichText::new("Usuń").size(11.0))
                        .min_size(egui::vec2(56.0, 0.0)),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                do_usuniecia = Some(i);
            }
        });

        // Zepsuty wpis musi powiedzieć o sobie od razu. Bez tego zmienna
        // z literówką po prostu nie dochodzi do gry i nikt nie wie dlaczego.
        if let Some(powod) = z.powod_odrzucenia() {
            ui.label(
                egui::RichText::new(format!("Ten wpis zostanie pominięty: {powod}"))
                    .size(11.0)
                    .color(theme::OSTRZEZENIE),
            );
        }
        ui.add_space(4.0);
    }

    if let Some(i) = do_usuniecia {
        app.ustawienia.zaawansowane.zmienne.remove(i);
        zmienione = true;
    }

    ui.add_space(theme::S1);
    if ui
        .add(theme::przycisk_zwykly("Dodaj zmienną"))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        app.ustawienia.zaawansowane.zmienne.push(Zmienna::nowa());
        zmienione = true;
    }

    let dziala = app.ustawienia.zaawansowane.srodowisko().len();
    if dziala > 0 {
        ui.add_space(theme::S1);
        ui.label(theme::drobny(&format!(
            "Do gry trafi {dziala} zmienn(ych)."
        )));
    }

    zmienione
}

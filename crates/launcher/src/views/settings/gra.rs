//! Zakładka „Gra": okno Minecrafta i co launcher robi wokół niego.
//!
//! Nie ma tu nic, co mogłoby zepsuć start gry, więc zakładka jest bez
//! ostrzeżenia. Najgorsze, co można sobie zrobić, to okno 320×240 — i widać
//! to od razu, a naprawia jednym kliknięciem.

use crate::app::App;
use crate::theme;
use chmurka_core::ustawienia as ust;

/// Rozmiary, o które ludzie proszą najczęściej. Wpisywanie ich ręcznie
/// w dwa pola to cztery ruchy myszy i dwie okazje do pomyłki.
const GOTOWE: [(u32, u32, &str); 3] = [
    (854, 480, "Domyślny"),
    (1280, 720, "HD"),
    (1920, 1080, "Full HD"),
];

pub fn rysuj(app: &mut App, ui: &mut egui::Ui) -> bool {
    let mut zmienione = false;

    // --- OKNO GRY ---
    theme::naglowek_sekcji(ui, "OKNO GRY");
    zmienione |= ui
        .checkbox(
            &mut app.ustawienia.gra.wlasny_rozmiar_okna,
            "Ustaw własny rozmiar okna gry",
        )
        .changed();
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Wyłączone znaczy, że o rozmiarze decyduje sama gra — pamięta ten, \
         w jakim zamknąłeś ją ostatnio.",
    ));

    let wlasny = app.ustawienia.gra.wlasny_rozmiar_okna;
    ui.add_space(theme::S2);
    ui.add_enabled_ui(wlasny, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Szerokość").size(12.0));
            zmienione |= ui
                .add(
                    egui::DragValue::new(&mut app.ustawienia.gra.szerokosc)
                        .range(ust::NAJMNIEJSZA_SZEROKOSC..=ust::NAJWIEKSZA_SZEROKOSC)
                        .speed(4.0)
                        .suffix(" px"),
                )
                .changed();
            ui.add_space(theme::S1);
            ui.label(
                egui::RichText::new("×")
                    .size(12.0)
                    .color(theme::TEKST_PRZYGASZONY),
            );
            ui.add_space(theme::S1);
            ui.label(egui::RichText::new("Wysokość").size(12.0));
            zmienione |= ui
                .add(
                    egui::DragValue::new(&mut app.ustawienia.gra.wysokosc)
                        .range(ust::NAJMNIEJSZA_WYSOKOSC..=ust::NAJWIEKSZA_WYSOKOSC)
                        .speed(4.0)
                        .suffix(" px"),
                )
                .changed();
        });

        ui.add_space(theme::S1);
        ui.horizontal(|ui| {
            for (szer, wys, nazwa) in GOTOWE {
                let wybrany =
                    app.ustawienia.gra.szerokosc == szer && app.ustawienia.gra.wysokosc == wys;
                let napis = format!("{nazwa} · {szer}×{wys}");
                let tekst = egui::RichText::new(napis).size(11.0).color(if wybrany {
                    theme::AKCENT
                } else {
                    theme::TEKST_PRZYGASZONY
                });
                if ui
                    .add(egui::Button::new(tekst))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    app.ustawienia.gra.szerokosc = szer;
                    app.ustawienia.gra.wysokosc = wys;
                    zmienione = true;
                }
            }
        });
    });

    ui.add_space(theme::S2);
    zmienione |= ui
        .checkbox(
            &mut app.ustawienia.gra.pelny_ekran,
            "Uruchamiaj grę na pełnym ekranie",
        )
        .changed();
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Gra zajmie cały ekran od razu po starcie. Rozmiar okna powyżej nadal ma \
         znaczenie — to on obowiązuje po wyjściu z pełnego ekranu klawiszem F11.",
    ));

    // --- ZACHOWANIE LAUNCHERA ---
    ui.add_space(theme::S4);
    theme::naglowek_sekcji(ui, "ZACHOWANIE LAUNCHERA");
    zmienione |= ui
        .checkbox(
            &mut app.ustawienia.ukryj_po_starcie,
            "Schowaj launcher do zasobnika po uruchomieniu gry",
        )
        .changed();
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Launcher znika z ekranu i przestaje zabierać zasoby, ale nadal czuwa — \
         wróci sam, gdyby gra się zamknęła z błędem.",
    ));

    ui.add_space(theme::S2);
    zmienione |= ui
        .checkbox(
            &mut app.ustawienia.gra.zamknij_po_grze,
            "Zamknij launcher po zakończeniu gry",
        )
        .changed();
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Dotyczy tylko normalnego wyjścia z gry. Kiedy gra padnie, launcher zostaje \
         i pokazuje, co się stało — bez tego zniknąłby razem z jedynym wyjaśnieniem.",
    ));

    zmienione
}

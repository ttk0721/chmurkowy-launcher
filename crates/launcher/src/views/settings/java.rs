//! Zakładka „Java": którą Javą uruchamiać grę i ile dać jej pamięci.
//!
//! Domyślnie nie ma tu nic do roboty — launcher pobiera własną Javę i to
//! działa na każdym komputerze, na którym w ogóle da się grać. Zakładka
//! istnieje dla przypadków, w których to zawodzi: polityka firmowa blokująca
//! pliki w katalogu użytkownika, dystrybucja z nietypową biblioteką systemową,
//! ktoś, kto po prostu ma swoją Javę i chce jej użyć.
//!
//! Sprawdzanie jest tu ważniejsze od wybierania. Wskazanie złego pliku kończy
//! się grą, która nie startuje, a komunikat JVM nie mówi nic — więc mówimy
//! o tym tutaj, zanim ktokolwiek kliknie GRAJ.

use super::Zakladka;
use crate::app::App;
use crate::theme;
use chmurka_core::ustawienia as ust;

pub fn rysuj(app: &mut App, ui: &mut egui::Ui) -> bool {
    super::pasek_ostrzezenia(ui, Zakladka::Java);
    let mut zmienione = false;
    let wymagany = app.manifest.as_ref().map(|m| m.java.major);

    // --- KTÓRA JAVA ---
    theme::naglowek_sekcji(ui, "PLIK WYKONYWALNY JAVY");
    zmienione |= pole_sciezki(app, ui);
    ui.add_space(theme::S1);
    przyciski(app, ui);
    ui.add_space(theme::S2);
    wynik_sprawdzenia(app, ui, wymagany);
    lista_kandydatow(app, ui, &mut zmienione);

    ui.add_space(theme::S2);
    zmienione |= ui
        .checkbox(
            &mut app.ustawienia.java.pomin_sprawdzanie,
            "Pomiń sprawdzanie zgodności wersji Javy",
        )
        .changed();
    ui.add_space(4.0);
    ui.label(theme::drobny(&match wymagany {
        Some(w) => format!(
            "Zwykle launcher odmawia startu, gdy wskazana Java nie jest w wersji {w}. \
             Zaznacz, jeśli wiesz, że Twoja i tak zadziała."
        ),
        None => "Zwykle launcher odmawia startu, gdy wersja Javy nie pasuje do paczki. \
                 Zaznacz, jeśli wiesz, że Twoja i tak zadziała."
            .to_string(),
    }));

    // --- PAMIĘĆ STARTOWA ---
    ui.add_space(theme::S4);
    theme::naglowek_sekcji(ui, "PAMIĘĆ STARTOWA");
    ui.horizontal(|ui| {
        zmienione |= ui
            .add(
                egui::DragValue::new(&mut app.ustawienia.java.pamiec_min_mb)
                    .range(ust::NAJMNIEJSZE_MINIMUM_MB..=16384)
                    .speed(64.0)
                    .suffix(" MB"),
            )
            .changed();
        ui.label(
            egui::RichText::new("(-Xms)")
                .size(11.0)
                .color(theme::TEKST_PRZYGASZONY),
        );
    });
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Ile pamięci Java bierze od razu na starcie. Maksimum ustawia suwak \
         w zakładce Ogólne. Podniesienie tej wartości skraca zacinanie się gry \
         w pierwszych minutach, ale zajmuje pamięć od pierwszej sekundy.",
    ));
    if app.ustawienia.minimum_przekracza_maksimum() {
        ui.add_space(theme::S1);
        ui.label(
            egui::RichText::new(format!(
                "To więcej niż maksimum ustawione w zakładce Ogólne ({} MB). \
                 Launcher użyje {} MB — przy odwrotnych wartościach Java odmawia startu.",
                app.ustawienia.pamiec_mb,
                app.ustawienia.minimum_sterty_mb()
            ))
            .size(11.0)
            .color(theme::OSTRZEZENIE),
        );
    }

    // --- PARAMETRY ---
    ui.add_space(theme::S4);
    theme::naglowek_sekcji(ui, "DODATKOWE PARAMETRY JAVY");
    zmienione |= ui
        .add(
            egui::TextEdit::multiline(&mut app.ustawienia.dodatkowe_argumenty)
                .hint_text("np. -XX:+UseG1GC")
                .desired_width(ui.available_width().min(560.0))
                .desired_rows(4)
                .margin(egui::Margin::symmetric(12, 11)),
        )
        .changed();
    ui.add_space(4.0);
    let liczba = ust::podziel_argumenty(&app.ustawienia.dodatkowe_argumenty).len();
    if liczba > 0 {
        ui.label(
            egui::RichText::new(format!(
                "Rozpoznano {liczba} parametr(ów). Jeśli gra przestanie się uruchamiać, \
                 wyczyść to pole."
            ))
            .size(11.0)
            .color(theme::TEKST_PRZYGASZONY),
        );
    } else {
        ui.label(theme::drobny(
            "Zostaw puste, jeśli nie wiesz, do czego to służy. Puste jest bezpieczne. \
             Parametry można pisać po jednym w wierszu.",
        ));
    }
    if app.ustawienia.wlasny_rozmiar_sterty() {
        ui.add_space(theme::S1);
        ui.label(
            egui::RichText::new(
                "Podałeś tu własne -Xmx, więc suwak pamięci w zakładce Ogólne jest wyłączony.",
            )
            .size(11.0)
            .color(theme::AKCENT),
        );
    }

    // --- POWRÓT DO FABRYCZNYCH ---
    let domyslne = ust::UstawieniaJavy {
        // Ostrzeżenie raz przyjęte zostaje przyjęte — przywracanie domyślnych
        // nie jest powodem, żeby pokazywać je od nowa.
        przyjeto_ostrzezenie: app.ustawienia.java.przyjeto_ostrzezenie,
        ..ust::UstawieniaJavy::default()
    };
    let cos_zmienione =
        app.ustawienia.java != domyslne || !app.ustawienia.dodatkowe_argumenty.is_empty();
    if super::przycisk_domyslnych(ui, cos_zmienione) {
        app.ustawienia.java = domyslne;
        app.ustawienia.dodatkowe_argumenty.clear();
        app.wynik_javy = None;
        app.kandydaci_javy = None;
        app.komunikat = Some("Przywrócono domyślne ustawienia Javy.".into());
        zmienione = true;
    }

    zmienione
}

fn pole_sciezki(app: &mut App, ui: &mut egui::Ui) -> bool {
    let zmiana = ui
        .add(
            egui::TextEdit::singleline(&mut app.ustawienia.java.sciezka)
                .hint_text("puste = Java pobrana przez launcher")
                .desired_width(ui.available_width().min(560.0))
                .margin(egui::Margin::symmetric(12, 11)),
        )
        .changed();
    if zmiana {
        // Stary wynik dotyczyłby poprzedniej ścieżki, a zielony napis
        // „wszystko w porządku" pod zupełnie inną Javą byłby kłamstwem.
        app.wynik_javy = None;
    }
    ui.add_space(4.0);

    match app.ustawienia.java.wlasna() {
        None => {
            let opis = match (&app.manifest, znajdz_pobrana(app)) {
                (_, Some(sc)) => format!("Launcher używa własnej Javy: {}", sc.display()),
                (Some(m), None) => format!(
                    "Launcher pobierze i użyje Javy {} — nic nie musisz tu wpisywać.",
                    m.java.major
                ),
                (None, None) => {
                    "Launcher pobierze i użyje własnej Javy — nic nie musisz tu wpisywać."
                        .to_string()
                }
            };
            ui.label(theme::drobny(&opis));
        }
        Some(_) => {
            ui.label(theme::drobny(
                "Launcher użyje tej Javy zamiast własnej. Wyczyść pole, żeby wrócić \
                 do pobranej — to zawsze bezpieczny wybór.",
            ));
        }
    }
    zmiana
}

fn przyciski(app: &mut App, ui: &mut egui::Ui) {
    let wolny = !app.zajety;
    ui.horizontal(|ui| {
        if super::super::przycisk_warunkowy(
            ui,
            "Wykryj",
            wolny,
            "Poczekaj, aż launcher skończy to, co teraz robi.",
        )
        .on_hover_text("Przeszuka typowe miejsca, w których instaluje się Java.")
        .clicked()
        {
            app.wykryj_javy();
        }
        if super::super::przycisk_warunkowy(
            ui,
            "Przeglądaj",
            wolny,
            "Poczekaj, aż launcher skończy to, co teraz robi.",
        )
        .clicked()
        {
            app.wybierz_jave();
        }

        let do_sprawdzenia = app.ustawienia.java.wlasna().or_else(|| znajdz_pobrana(app));
        if super::super::przycisk_warunkowy(
            ui,
            "Sprawdź",
            wolny && do_sprawdzenia.is_some(),
            if wolny {
                "Nie ma jeszcze czego sprawdzać — wpisz ścieżkę albo kliknij „Wykryj”."
            } else {
                "Poczekaj, aż launcher skończy to, co teraz robi."
            },
        )
        .on_hover_text("Zapyta tę Javę o wersję i powie, czy pasuje do paczki.")
        .clicked()
        {
            if let Some(sc) = do_sprawdzenia {
                app.sprawdz_jave(sc);
            }
        }
    });
}

fn wynik_sprawdzenia(app: &App, ui: &mut egui::Ui, wymagany: Option<u32>) {
    match &app.wynik_javy {
        None if app.zajety => {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(12.0).color(theme::AKCENT));
                ui.label(theme::drobny("Pytam Javę o wersję…"));
            });
        }
        None => {}
        Some(Err(e)) => {
            ui.label(
                egui::RichText::new(format!("Nie udało się użyć tej Javy: {e}"))
                    .size(12.0)
                    .color(theme::BLAD),
            );
        }
        Some(Ok(info)) => {
            let pasuje = wymagany.is_none_or(|w| w == info.major);
            let kolor = if pasuje && info.bity64 {
                theme::SUKCES
            } else {
                theme::OSTRZEZENIE
            };
            ui.label(
                egui::RichText::new(format!(
                    "Java {} ({}), {}",
                    info.major,
                    info.wersja,
                    if info.bity64 {
                        "64-bitowa"
                    } else {
                        "32-bitowa"
                    }
                ))
                .size(13.0)
                .color(kolor),
            );
            ui.add_space(4.0);
            ui.label(theme::drobny(&info.opis));

            if let Some(w) = wymagany {
                if w != info.major {
                    ui.add_space(theme::S1);
                    ui.label(
                        egui::RichText::new(format!(
                            "Paczka potrzebuje Javy {w}. Launcher nie uruchomi gry tą Javą, \
                             dopóki nie zaznaczysz „Pomiń sprawdzanie zgodności wersji”."
                        ))
                        .size(11.0)
                        .color(theme::OSTRZEZENIE),
                    );
                }
            }
            if !info.bity64 {
                ui.add_space(theme::S1);
                ui.label(
                    egui::RichText::new(
                        "To Java 32-bitowa — nie obejmie tyle pamięci, ile potrzebuje paczka. \
                         Gra padłaby przy starcie na braku miejsca na stertę.",
                    )
                    .size(11.0)
                    .color(theme::BLAD),
                );
            }
        }
    }
}

fn lista_kandydatow(app: &mut App, ui: &mut egui::Ui, zmienione: &mut bool) {
    let Some(lista) = app.kandydaci_javy.clone() else {
        return;
    };
    ui.add_space(theme::S2);
    if lista.is_empty() {
        ui.label(theme::drobny(
            "Nie znaleziono żadnej Javy w typowych miejscach. Zostaw pole puste — \
             launcher pobierze własną i to zadziała.",
        ));
        return;
    }

    ui.label(
        egui::RichText::new(format!("Znalezione na tym komputerze ({})", lista.len()))
            .size(11.0)
            .family(theme::polgruba())
            .color(theme::TEKST_PRZYGASZONY),
    );
    ui.add_space(theme::S1);
    for sc in lista {
        let napis = sc.display().to_string();
        let wybrana = app.ustawienia.java.sciezka == napis;
        let tekst = egui::RichText::new(&napis).size(11.0).color(if wybrana {
            theme::AKCENT
        } else {
            theme::TEKST
        });
        if ui
            .add(
                egui::Button::new(tekst)
                    .fill(theme::PANEL)
                    .min_size(egui::vec2(ui.available_width().min(560.0), 0.0)),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Użyj tej Javy")
            .clicked()
        {
            app.ustawienia.java.sciezka = napis;
            *zmienione = true;
            app.sprawdz_jave(sc);
        }
    }
}

/// Ścieżka do Javy, którą launcher pobrał sam — o ile już ją ma.
fn znajdz_pobrana(app: &App) -> Option<std::path::PathBuf> {
    let major = app.manifest.as_ref()?.java.major;
    chmurka_core::java::znajdz_binarke(
        &app.data().join("java").join(major.to_string()),
        chmurka_core::java::biezacy_os(),
    )
}

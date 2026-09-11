//! Zakładka „Ogólne": to, czego dotyka każdy.
//!
//! Suwak pamięci stoi tu, a nie w zakładce Java, choć technicznie jest
//! parametrem Javy. To jedyne ustawienie, które rodzic dziesięciolatka
//! naprawdę czasem musi zmienić — schowanie go za ostrzeżeniem „tylko dla
//! zaawansowanych" byłoby wrogie.

use crate::app::App;
use crate::theme;

pub fn rysuj(app: &mut App, ui: &mut egui::Ui) -> bool {
    let instancja = app.data().join("instance");
    let log = app.data().join("logs").join("game.log");
    let mc = app.data().join("mc");
    let mut zmienione = false;

    // --- PAMIĘĆ ---
    theme::naglowek_sekcji(ui, "PAMIĘĆ");
    let wlasna_sterta = app.ustawienia.wlasny_rozmiar_sterty();
    let suwak = ui.add_enabled(
        !wlasna_sterta,
        egui::Slider::new(&mut app.ustawienia.pamiec_mb, 2048..=16384)
            .suffix(" MB")
            .step_by(512.0),
    );
    zmienione |= suwak.changed();
    ui.add_space(4.0);
    if wlasna_sterta {
        ui.label(
            egui::RichText::new(
                "Suwak jest wyłączony, bo w zakładce Java, w parametrach Javy, \
                 ustawiłeś własne -Xmx.",
            )
            .size(11.0)
            .color(theme::AKCENT),
        );
    } else {
        // Liczba modow pochodzi z manifestu — wpisana w kod
        // rozjezdzalaby sie z paczka przy kazdej jej zmianie.
        let opis_paczki = match &app.manifest {
            Some(m) => {
                let mody = m
                    .files
                    .iter()
                    .filter(|f| f.path.starts_with("mods/"))
                    .count();
                format!("Paczka z {mody} modami potrzebuje co najmniej 4 GB.")
            }
            None => "Ta paczka potrzebuje co najmniej 4 GB.".to_string(),
        };
        ui.label(theme::drobny(&opis_paczki));

        if let Some(calkowita) = chmurka_core::pamiec::calkowita_mb() {
            use chmurka_core::pamiec;
            let zalecana = pamiec::zalecana_mb(calkowita);

            // Suwak pokazuje sterte, ale gra bierze wiecej: metaspace,
            // cache kodu i bufory sterownika grafiki leza poza nia.
            // Bez tego zdania gracz podnosil suwak „bo ma 8 GB"
            // i system ubijal mu gre w polowie rozgrywki.
            ui.add_space(theme::S1);
            ui.label(theme::drobny(&format!(
                "Suwak ustawia pamięć samej gry. Z dodatkami zajmie ona około {} MB \
                 z {} MB, jakie ma Twój komputer.",
                pamiec::szacowany_proces_mb(app.ustawienia.pamiec_mb),
                calkowita
            )));

            if pamiec::grozi_brakiem_pamieci(app.ustawienia.pamiec_mb, calkowita) {
                ui.add_space(theme::S1);
                ui.label(
                    egui::RichText::new(
                        "To ustawienie jest za wysokie dla tego komputera — \
                         system może zamknąć grę w trakcie zabawy.",
                    )
                    .size(11.0)
                    .color(theme::BLAD),
                );
            }

            ui.add_space(theme::S1);
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("Dobierz automatycznie").size(11.0),
                    ))
                    .on_hover_text(format!(
                        "Ustawi {zalecana} MB — tyle bezpiecznie mieści się \
                         w {} MB tego komputera.",
                        calkowita
                    ))
                    .clicked()
                {
                    app.ustawienia.pamiec_mb = zalecana;
                    zmienione = true;
                }
                if pamiec::warto_zaproponowac(app.ustawienia.pamiec_mb, zalecana) {
                    ui.label(
                        egui::RichText::new(format!("proponujemy {zalecana} MB"))
                            .size(11.0)
                            .color(theme::AKCENT),
                    );
                }
            });
        }
    }

    // Minimum sterty mieszka w zakladce Java, wiec sprzeczne ustawienie
    // widac dopiero po przejsciu miedzy zakladkami. Mowimy o tym w obu.
    if app.ustawienia.minimum_przekracza_maksimum() {
        ui.add_space(theme::S1);
        ui.label(
            egui::RichText::new(format!(
                "W zakładce Java ustawiłeś minimum {} MB, czyli więcej niż ten suwak. \
                 Launcher użyje {} MB — inaczej Java odmówiłaby startu.",
                app.ustawienia.java.pamiec_min_mb,
                app.ustawienia.minimum_sterty_mb()
            ))
            .size(11.0)
            .color(theme::OSTRZEZENIE),
        );
    }

    // --- DODATKI ---
    ui.add_space(theme::S4);
    theme::naglowek_sekcji(ui, "DODATKI");
    zmienione |= ui
        .checkbox(
            &mut app.ustawienia.biblioteki_dzwieku,
            "Przygotuj biblioteki dźwięku przed pierwszym uruchomieniem",
        )
        .changed();
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Mod Create: Harmonics potrzebuje programów yt-dlp i ffmpeg. Launcher pobierze \
         je z ich oficjalnych źródeł (około 160 MB), żeby mod nie pytał o to w trakcie \
         gry. Wyłącz, jeśli masz wolne łącze — mod poradzi sobie sam.",
    ));

    // --- PLIKI ---
    ui.add_space(theme::S4);
    theme::naglowek_sekcji(ui, "PLIKI");
    ui.horizontal(|ui| {
        // Folder tworzymy przed otwarciem — inaczej na świeżym
        // launcherze przycisk cicho nic nie robił.
        if ui
            .add(theme::przycisk_zwykly("Otwórz folder gry"))
            .clicked()
        {
            let _ = std::fs::create_dir_all(&instancja);
            match open::that_detached(&instancja) {
                Ok(()) => app.komunikat = None,
                Err(e) => app.komunikat = Some(format!("Nie udało się otworzyć folderu: {e}")),
            }
        }
        if super::super::przycisk_warunkowy(
            ui,
            "Otwórz log gry",
            log.is_file(),
            "Log powstanie po pierwszym uruchomieniu gry.",
        )
        .clicked()
        {
            if let Err(e) = open::that_detached(&log) {
                app.komunikat = Some(format!("Nie udało się otworzyć logu: {e}"));
            }
        }
    });

    // --- NAPRAWA ---
    ui.add_space(theme::S4);
    theme::naglowek_sekcji(ui, "NAPRAWA");
    if super::super::przycisk_warunkowy(
        ui,
        "Napraw instalację",
        mc.exists(),
        "Gra nie jest jeszcze zainstalowana — nie ma czego naprawiać.",
    )
    .clicked()
    {
        match std::fs::remove_dir_all(&mc) {
            Ok(()) => {
                app.komunikat =
                    Some("Pliki gry usunięte. Pobiorą się ponownie przy następnym starcie.".into());
                app.log
                    .push("Usunięto pliki gry na żądanie użytkownika.".into());
            }
            Err(e) => app.komunikat = Some(format!("Nie udało się usunąć plików gry: {e}")),
        }
    }
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Pobiera grę od nowa. Światy i ustawienia zostają.",
    ));

    // --- KONTO ---
    if app.konto.is_some() {
        ui.add_space(theme::S4);
        theme::naglowek_sekcji(ui, "KONTO");
        // Pojedynczym kontem zarzadza sie na ekranie Konta —
        // tu zostaje tylko wyczyszczenie calej listy naraz.
        if ui
            .add(theme::przycisk_zwykly("Wyloguj wszystkie konta"))
            .on_hover_text(
                "Usuwa z launchera wszystkie zapamiętane konta. \
                 Światy i pliki gry zostają.",
            )
            .clicked()
        {
            chmurka_core::auth::store::clear(&app.data().join("auth.json"));
            app.konta = chmurka_core::auth::store::Konta::default();
            app.konto = None;
            app.komunikat = Some("Wylogowano ze wszystkich kont.".into());
        }
    }

    // --- AKTUALIZACJA ---
    //
    // Sekcja pokazuje się ZAWSZE. Wcześniej całość siedziała pod
    // warunkiem „jest nowsza wersja", więc po zaktualizowaniu
    // znikała bez śladu — razem z przyciskiem i z informacją,
    // jaką wersję się w ogóle ma. Stan aktualizacji musi być
    // widoczny niezależnie od tego, czy akurat jest co pobierać.
    ui.add_space(theme::S4);
    theme::naglowek_sekcji(ui, "AKTUALIZACJA");

    match &app.manifest {
        None => {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(12.0).color(theme::AKCENT));
                ui.label(theme::drobny("Sprawdzam, czy jest nowsza wersja…"));
            });
        }
        Some(m) => {
            // Porównanie wersji, a NIE „różne od". Przy zwykłym !=
            // manifest ogłoszony chwilowo na starszą wersję wyglądał
            // na aktualizację: launcher 0.4.17 pisał „dostępna nowsza:
            // 0.4.14" i proponował cofnięcie się wstecz.
            let jest_nowsza = chmurka_core::aktualizacja::nowsza(
                env!("CARGO_PKG_VERSION"),
                &m.launcher.latest_version,
            );

            if jest_nowsza {
                ui.label(
                    egui::RichText::new(format!(
                        "Dostępna nowsza wersja: {}",
                        m.launcher.latest_version
                    ))
                    .size(13.0)
                    .color(theme::AKCENT),
                );
            } else {
                ui.label(
                    egui::RichText::new(format!(
                        "Masz najnowszą wersję ({}).",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .size(13.0)
                    .color(theme::SUKCES),
                );
            }
            ui.add_space(theme::S1);

            // Wcześniej stał tu przycisk otwierający przeglądarkę
            // z linkiem do GitHuba — zaszłość sprzed samoaktualizacji.
            // Gracz miał wtedy sam pobrać plik i podmienić go ręcznie,
            // co przy dziesięciolatku nie ma prawa się udać.
            let ma_plik = m
                .launcher
                .urls
                .contains_key(chmurka_core::aktualizacja::klucz_systemu());
            let napis = if jest_nowsza {
                "Zaktualizuj teraz"
            } else {
                "Sprawdź ponownie"
            };
            let aktywny = !app.zajety && (ma_plik || !jest_nowsza);

            if super::super::przycisk_warunkowy(
                ui,
                napis,
                aktywny,
                if app.zajety {
                    "Poczekaj, aż launcher skończy to, co teraz robi."
                } else {
                    "Dla tego systemu nie ma jeszcze gotowego pliku."
                },
            )
            .clicked()
            {
                if jest_nowsza {
                    app.zaktualizuj_recznie();
                } else {
                    // Pyta serwer tylko wtedy, gdy od ostatniego
                    // sprawdzenia minęło dość czasu — inaczej
                    // odpowiada tym, co już wie.
                    app.sprawdz_aktualizacje();
                }
            }
            ui.add_space(4.0);
            ui.label(theme::drobny(if jest_nowsza {
                "Launcher pobierze nową wersję, podmieni się i uruchomi ponownie. \
                 Nic nie musisz robić, a folder data zostaje nietknięty."
            } else {
                "Launcher sprawdza to sam przy każdym uruchomieniu. Ten przycisk \
                 jest na wypadek, gdyby poprawka wyszła w trakcie grania."
            }));
        }
    }

    // --- STREFA ZAGROŻENIA ---
    //
    // Osobno, na samym dole i na czerwono, bo to jedyne miejsce
    // w launcherze, po którym nie ma odwrotu.
    ui.add_space(theme::S4);
    ui.label(
        egui::RichText::new("STREFA ZAGROŻENIA")
            .size(11.0)
            .family(theme::polgruba())
            .color(theme::BLAD),
    );
    ui.add_space(theme::S1);

    if ui
        .add(theme::przycisk_zwykly("Odinstaluj launcher"))
        .clicked()
    {
        app.pyta_o_odinstalowanie = true;
    }
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Usuwa launcher z tego komputera. Zapyta, czy skasować też \
         światy i paczkę modów.",
    ));

    zmienione
}

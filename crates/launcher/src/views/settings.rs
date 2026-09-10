use crate::app::{App, Widok};
use crate::theme;
use chmurka_core::ustawienia::podziel_argumenty;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    if app.pyta_o_odinstalowanie {
        okno_odinstalowania(app, ctx);
    }

    egui::TopBottomPanel::top("gora-ust")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Ustawienia");
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
            egui::ScrollArea::vertical().show(ui, |ui| {
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
                            "Suwak jest wyłączony, bo w parametrach Javy ustawiłeś własne -Xmx.",
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

                // --- GRA ---
                ui.add_space(theme::S4);
                theme::naglowek_sekcji(ui, "GRA");
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

                // --- PARAMETRY JAVY ---
                ui.add_space(theme::S4);
                theme::naglowek_sekcji(ui, "DODATKOWE PARAMETRY JAVY");
                zmienione |= ui
                    .add(
                        egui::TextEdit::singleline(&mut app.ustawienia.dodatkowe_argumenty)
                            .hint_text("np. -XX:+UseG1GC")
                            .desired_width(ui.available_width().min(520.0))
                            .margin(egui::Margin::symmetric(12, 11)),
                    )
                    .changed();
                ui.add_space(4.0);
                let liczba = podziel_argumenty(&app.ustawienia.dodatkowe_argumenty).len();
                if liczba > 0 {
                    ui.label(
                        egui::RichText::new(format!(
                            "Rozpoznano {liczba} parametr(ów). Jeśli gra przestanie się uruchamiać, wyczyść to pole."
                        ))
                        .size(11.0)
                        .color(theme::TEKST_PRZYGASZONY),
                    );
                } else {
                    ui.label(theme::drobny(
                        "Zostaw puste, jeśli nie wiesz, do czego to służy. Puste jest bezpieczne.",
                    ));
                }

                // --- PLIKI ---
                ui.add_space(theme::S4);
                theme::naglowek_sekcji(ui, "PLIKI");
                ui.horizontal(|ui| {
                    // Folder tworzymy przed otwarciem — inaczej na świeżym
                    // launcherze przycisk cicho nic nie robił.
                    if ui.add(theme::przycisk_zwykly("Otwórz folder gry")).clicked() {
                        let _ = std::fs::create_dir_all(&instancja);
                        match open::that_detached(&instancja) {
                            Ok(()) => app.komunikat = None,
                            Err(e) => {
                                app.komunikat =
                                    Some(format!("Nie udało się otworzyć folderu: {e}"))
                            }
                        }
                    }
                    if super::przycisk_warunkowy(
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
                if super::przycisk_warunkowy(
                    ui,
                    "Napraw instalację",
                    mc.exists(),
                    "Gra nie jest jeszcze zainstalowana — nie ma czego naprawiać.",
                )
                .clicked()
                {
                    match std::fs::remove_dir_all(&mc) {
                        Ok(()) => {
                            app.komunikat = Some(
                                "Pliki gry usunięte. Pobiorą się ponownie przy następnym starcie."
                                    .into(),
                            );
                            app.log
                                .push("Usunięto pliki gry na żądanie użytkownika.".into());
                        }
                        Err(e) => {
                            app.komunikat = Some(format!("Nie udało się usunąć plików gry: {e}"))
                        }
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
                if let Some(m) = &app.manifest {
                    if m.launcher.latest_version != env!("CARGO_PKG_VERSION") {
                        ui.add_space(theme::S4);
                        theme::naglowek_sekcji(ui, "AKTUALIZACJA");
                        ui.label(
                            egui::RichText::new(format!(
                                "Dostępna nowsza wersja: {}",
                                m.launcher.latest_version
                            ))
                            .size(13.0)
                            .color(theme::AKCENT),
                        );
                        ui.add_space(theme::S1);
                        // Wcześniej stał tu przycisk otwierający przeglądarkę
                        // z linkiem do GitHuba — zaszłość sprzed samoaktualizacji.
                        // Gracz miał wtedy sam pobrać plik i podmienić go ręcznie,
                        // co przy dziesięciolatku nie ma prawa się udać.
                        let ma_plik = m
                            .launcher
                            .urls
                            .contains_key(chmurka_core::aktualizacja::klucz_systemu());
                        if super::przycisk_warunkowy(
                            ui,
                            "Zaktualizuj teraz",
                            ma_plik && !app.zajety,
                            if ma_plik {
                                "Poczekaj, aż launcher skończy to, co teraz robi."
                            } else {
                                "Dla tego systemu nie ma jeszcze gotowego pliku."
                            },
                        )
                        .clicked()
                        {
                            app.zaktualizuj_recznie();
                        }
                        ui.add_space(4.0);
                        ui.label(theme::drobny(
                            "Launcher pobierze nową wersję, podmieni się i uruchomi ponownie. \
                             Nic nie musisz robić, a folder data zostaje nietknięty.",
                        ));
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

                ui.add_space(theme::S3);

                // Zapis po każdej zmianie — ustawienia nie mogą znikać
                // po zamknięciu launchera, jak działo się wcześniej.
                if zmienione {
                    app.zapisz_ustawienia();
                }
            });
        });
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

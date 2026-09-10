use crate::app::{App, Widok};
use crate::theme;
use chmurka_core::ustawienia::podziel_argumenty;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
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
                        match open::that(&instancja) {
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
                        if let Err(e) = open::that(&log) {
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
                    if ui.add(theme::przycisk_zwykly("Wyloguj")).clicked() {
                        chmurka_core::auth::store::clear(&app.data().join("auth.json"));
                        app.konto = None;
                        app.komunikat = Some("Wylogowano.".into());
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
                        let klucz = if cfg!(target_os = "windows") {
                            "windows-x64"
                        } else {
                            "linux-x64"
                        };
                        if let Some(adres) = m.launcher.urls.get(klucz).cloned() {
                            if ui.add(theme::przycisk_zwykly("Pobierz nową wersję")).clicked() {
                                if let Err(e) = open::that(&adres) {
                                    app.komunikat =
                                        Some(format!("Nie udało się otworzyć przeglądarki: {e}"));
                                }
                            }
                        }
                        ui.add_space(4.0);
                        ui.label(theme::drobny(
                            "Podmień plik launchera. Folder data zostaje nietknięty.",
                        ));
                    }
                }

                ui.add_space(theme::S3);

                // Zapis po każdej zmianie — ustawienia nie mogą znikać
                // po zamknięciu launchera, jak działo się wcześniej.
                if zmienione {
                    app.zapisz_ustawienia();
                }
            });
        });
}

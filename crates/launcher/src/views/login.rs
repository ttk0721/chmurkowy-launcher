use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("gora-log")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Dodaj konto");
        });

    egui::TopBottomPanel::bottom("dol-log")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            if ui
                .add(theme::przycisk_zwykly("Wróć").min_size(egui::vec2(120.0, 44.0)))
                .clicked()
            {
                // Wracamy do menedżera, gdy jest już czym zarządzać.
                app.widok = if app.konta.konta.is_empty() {
                    Widok::Glowny
                } else {
                    Widok::Konta
                };
            }
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                match &app.kod {
                    Some(kod) => {
                        let kod_tekst = kod.user_code.clone();
                        // Adres z kodem w środku — przycisk niżej otwiera gracza
                        // od razu przy wyborze konta. Goły adres pokazujemy
                        // osobno, bo przepisuje się go na telefon.
                        let adres_kliku = kod.adres_do_otwarcia();
                        let adres = kod.verification_uri.clone();
                        ui.label(theme::drobny(
                            "Kliknij przycisk niżej — otworzy stronę Microsoftu z tym kodem:",
                        ));
                        ui.add_space(theme::S2);
                        egui::Frame::NONE
                            .fill(theme::PANEL)
                            .stroke(egui::Stroke::new(1.0_f32, theme::OBRYS))
                            .corner_radius(egui::CornerRadius::same(12))
                            .inner_margin(egui::Margin::symmetric(28, 14))
                            .show(ui, |ui| {
                                // Kod musi dać się zaznaczyć myszą. Gdy schowek
                                // odmówi — a na Windowsie potrafi — to jedyna
                                // droga do skopiowania go bez przepisywania.
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&kod_tekst)
                                            .size(34.0)
                                            .family(theme::polgruba())
                                            .color(theme::AKCENT),
                                    )
                                    .selectable(true),
                                );
                            });
                        ui.add_space(theme::S2);
                        // Dwa wejścia zamiast jednego, bo to dwie różne sytuacje.
                        // Zwykłe logowanie ma iść przez przeglądarkę gracza —
                        // z jej zapamiętanymi hasłami i zalogowaną sesją.
                        // „Inne konto" musi zaczynać od zera, bo przy istniejącej
                        // sesji Microsoft w ogóle nie pyta, kim jesteś, tylko
                        // wchodzi na konto, które akurat zastał.
                        let profil = app.data().join("przegladarka");
                        let otworz = |wlasny_profil: bool, app: &mut App| {
                            app.komunikat = Some(crate::schowek::komunikat(
                                crate::schowek::kopiuj(&kod_tekst),
                                "Przepisz kod ręcznie — jest widoczny powyżej.",
                            ));
                            if wlasny_profil {
                                chmurka_core::przegladarka::wyczysc_profil(&profil);
                            }
                            let gdzie = wlasny_profil.then_some(profil.as_path());
                            // Osobne okienko tylko ze stroną Microsoftu. Gdy nie
                            // ma czym go otworzyć — zwykła karta, bo logowanie
                            // musi zadziałać zawsze.
                            if !chmurka_core::przegladarka::otworz_w_okienku(&adres_kliku, gdzie) {
                                let _ = open::that_detached(&adres_kliku);
                            }
                        };

                        if ui
                            .add(theme::przycisk_glowny("Otwórz stronę logowania"))
                            .clicked()
                        {
                            otworz(false, app);
                        }
                        ui.add_space(theme::S1);
                        if ui
                            .add(theme::przycisk_zwykly("Zaloguj na inne konto"))
                            .on_hover_text(
                                "Otwiera czyste okno — Microsoft zapyta, na które konto \
                                 się logujesz, zamiast wejść na zapamiętane.",
                            )
                            .clicked()
                        {
                            otworz(true, app);
                        }
                        if let Some(k) = &app.komunikat {
                            ui.add_space(theme::S1);
                            let kolor = if k.starts_with("Skopiowano") {
                                theme::SUKCES
                            } else {
                                theme::BLAD
                            };
                            ui.label(egui::RichText::new(k).size(11.0).color(kolor));
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
                            .add_enabled(
                                !app.zajety,
                                theme::przycisk_glowny("Zaloguj przez Microsoft"),
                            )
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
                            theme::przycisk_zwykly("Graj").min_size(egui::vec2(88.0, 40.0)),
                        )
                        .on_disabled_hover_text("Najpierw wpisz nick.")
                        .clicked()
                    {
                        let nick = app.ustawienia.nick_offline.trim().to_string();
                        let konto = chmurka_core::auth::offline::offline_account(&nick);
                        // Konto offline też trafia na listę — gracz może mieć
                        // ich kilka i przełączać się bez wpisywania nicku od nowa.
                        app.zapamietaj_konto(konto, None);
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

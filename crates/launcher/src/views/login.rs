use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("gora-log")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Konta");
        });

    egui::TopBottomPanel::bottom("dol-log")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            if ui
                .add(theme::przycisk_zwykly("Wróć").min_size(egui::vec2(120.0, 44.0)))
                .clicked()
            {
                app.widok = Widok::Glowny;
            }
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                lista_kont(app, ui);

                match &app.kod {
                    Some(kod) => {
                        let kod_tekst = kod.user_code.clone();
                        let adres = kod.verification_uri.clone();
                        ui.label(theme::drobny("Wpisz ten kod na stronie Microsoftu:"));
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
                        if ui
                            .add(theme::przycisk_zwykly("Kopiuj kod i otwórz przeglądarkę"))
                            .clicked()
                        {
                            app.komunikat = Some(crate::schowek::komunikat(
                                crate::schowek::kopiuj(&kod_tekst),
                                "Przepisz kod ręcznie — jest widoczny powyżej.",
                            ));
                            let _ = open::that_detached(&adres);
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
                            .add_enabled(!app.zajety, theme::przycisk_glowny("Zaloguj przez Microsoft"))
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

/// Lista zapamiętanych kont z przełączaniem.
///
/// Przy jednym komputerze siedzi rodzeństwo, a jedna osoba miewa konto do
/// gry i drugie do testów. Bez listy każde przelogowanie oznaczało wpisywanie
/// kodu z Microsoftu od nowa.
fn lista_kont(app: &mut App, ui: &mut egui::Ui) {
    if app.konta.konta.is_empty() {
        return;
    }

    theme::naglowek_sekcji(ui, "TWOJE KONTA");

    let wybrane = app.konta.wybrane.clone();
    // Kopia, bo w pętli wołamy metody `app`, które listę zmieniają.
    let konta: Vec<_> = app.konta.konta.clone();
    let mut przelacz = None;
    let mut zapomnij = None;

    for konto in &konta {
        let klucz = konto.klucz();
        let aktywne = wybrane.as_deref() == Some(klucz.as_str());
        let (opis, kolor) = match &konto.rodzaj {
            chmurka_core::auth::store::Rodzaj::Microsoft { .. } => ("Microsoft", theme::AKCENT),
            chmurka_core::auth::store::Rodzaj::Offline => ("offline", theme::TEKST_PRZYGASZONY),
        };

        ui.horizontal(|ui| {
            ui.add_space((ui.available_width() - 420.0).max(0.0) / 2.0);

            let etykieta = if aktywne {
                format!("● {}", konto.nick)
            } else {
                format!("   {}", konto.nick)
            };
            let przycisk = ui.add_enabled(
                !aktywne && !app.zajety,
                theme::przycisk_zwykly(&etykieta).min_size(egui::vec2(300.0, 40.0)),
            );
            if przycisk.clicked() {
                przelacz = Some(klucz.clone());
            }

            ui.label(egui::RichText::new(opis).size(11.0).color(kolor));

            if ui
                .add(theme::przycisk_zwykly("Zapomnij").min_size(egui::vec2(90.0, 40.0)))
                .on_hover_text("Usuwa konto z listy. Światy i pliki gry zostają.")
                .clicked()
            {
                zapomnij = Some(klucz.clone());
            }
        });
        ui.add_space(4.0);
    }

    if let Some(k) = przelacz {
        app.przelacz_konto(&k);
    }
    if let Some(k) = zapomnij {
        app.zapomnij_konto(&k);
    }

    ui.add_space(theme::S3);
    theme::naglowek_sekcji(ui, "DODAJ KOLEJNE KONTO");
    ui.add_space(theme::S1);
}

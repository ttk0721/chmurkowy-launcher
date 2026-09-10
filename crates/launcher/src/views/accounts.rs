use crate::app::{App, Widok};
use crate::theme;
use chmurka_core::auth::store::Rodzaj;

/// Menedżer kont: lista, przełączanie, dodawanie i usuwanie.
///
/// Wcześniej wszystko siedziało na ekranie logowania, przez co zarządzanie
/// kontami wyglądało jak skutek uboczny logowania. Ekran logowania służy
/// teraz wyłącznie dodaniu konta i pokazuje się sam tylko wtedy, gdy nie ma
/// jeszcze żadnego — wtedy nie ma czym zarządzać.
pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("gora-konta")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Konta");
        });

    egui::TopBottomPanel::bottom("dol-konta")
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
            if ui
                .add(theme::przycisk_zwykly("Wróć").min_size(egui::vec2(120.0, 44.0)))
                .clicked()
            {
                app.komunikat = None;
                app.widok = Widok::Glowny;
            }
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                lista(app, ui);
                ui.add_space(theme::S4);
                dodawanie(app, ui);
            });
        });
}

fn lista(app: &mut App, ui: &mut egui::Ui) {
    theme::naglowek_sekcji(ui, "ZAPAMIĘTANE KONTA");

    if app.konta.konta.is_empty() {
        ui.label(theme::drobny(
            "Nie ma jeszcze żadnego konta. Dodaj je poniżej.",
        ));
        return;
    }

    let wybrane = app.konta.wybrane.clone();
    // Kopia listy, bo w pętli wołamy metody `app`, które ją zmieniają.
    let konta = app.konta.konta.clone();
    let mut przelacz = None;
    let mut zapomnij = None;

    for konto in &konta {
        let klucz = konto.klucz();
        let aktywne = wybrane.as_deref() == Some(klucz.as_str());

        egui::Frame::NONE
            .fill(if aktywne { theme::PANEL_JASNY } else { theme::PANEL })
            .stroke(egui::Stroke::new(
                1.0_f32,
                if aktywne { theme::AKCENT } else { theme::OBRYS },
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(14, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&konto.nick)
                                .size(14.0)
                                .family(theme::polgruba())
                                .color(theme::TEKST),
                        );
                        let (opis, kolor) = match &konto.rodzaj {
                            Rodzaj::Microsoft { .. } => ("konto Microsoft", theme::AKCENT),
                            Rodzaj::Offline => ("konto offline", theme::TEKST_PRZYGASZONY),
                        };
                        ui.label(egui::RichText::new(opis).size(11.0).color(kolor));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(theme::przycisk_zwykly("Zapomnij").min_size(egui::vec2(100.0, 36.0)))
                            .on_hover_text("Usuwa konto z launchera. Światy i pliki gry zostają.")
                            .clicked()
                        {
                            zapomnij = Some(klucz.clone());
                        }
                        if aktywne {
                            ui.label(
                                egui::RichText::new("● używane teraz")
                                    .size(11.0)
                                    .color(theme::SUKCES),
                            );
                        } else if ui
                            .add_enabled(
                                !app.zajety,
                                theme::przycisk_zwykly("Przełącz")
                                    .min_size(egui::vec2(110.0, 36.0)),
                            )
                            .clicked()
                        {
                            przelacz = Some(klucz.clone());
                        }
                    });
                });
            });
        ui.add_space(theme::S1);
    }

    if let Some(k) = przelacz {
        app.przelacz_konto(&k);
    }
    if let Some(k) = zapomnij {
        app.zapomnij_konto(&k);
    }
}

fn dodawanie(app: &mut App, ui: &mut egui::Ui) {
    theme::naglowek_sekcji(ui, "DODAJ KONTO");

    if ui
        .add_enabled(
            !app.zajety,
            theme::przycisk_zwykly("Dodaj konto Microsoft").min_size(egui::vec2(280.0, 44.0)),
        )
        .clicked()
    {
        // Kod urządzenia pokazuje ekran logowania — tu tylko go zaczynamy.
        app.widok = Widok::Logowanie;
        crate::app::zaloguj_microsoft(app);
    }
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Wpiszesz kod na stronie Microsoftu. Konto zostanie zapamiętane, \
         więc następnym razem wystarczy je wybrać z listy.",
    ));

    ui.add_space(theme::S3);
    ui.label(
        egui::RichText::new("Konto offline (do testów)")
            .size(13.0)
            .family(theme::polgruba())
            .color(theme::TEKST),
    );
    ui.add_space(theme::S1);

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.ustawienia.nick_offline)
                .hint_text("nick")
                .desired_width(220.0)
                .margin(egui::Margin::symmetric(12, 11)),
        );
        let nick = app.ustawienia.nick_offline.trim().to_string();
        // Konto offline o tym samym nicku juz na liscie — dodawanie drugi raz
        // niczego by nie zmienilo, wiec zamiast tego mowimy o tym wprost.
        let juz_jest = app
            .konta
            .konta
            .iter()
            .any(|k| matches!(k.rodzaj, Rodzaj::Offline) && k.nick.eq_ignore_ascii_case(&nick));
        if ui
            .add_enabled(
                !nick.is_empty() && !juz_jest,
                theme::przycisk_zwykly("Dodaj").min_size(egui::vec2(110.0, 40.0)),
            )
            .on_disabled_hover_text(if juz_jest {
                "To konto offline jest już na liście."
            } else {
                "Najpierw wpisz nick."
            })
            .clicked()
        {
            let konto = chmurka_core::auth::offline::offline_account(&nick);
            app.zapamietaj_konto(konto, None);
            app.zapisz_ustawienia();
        }
    });
    ui.add_space(4.0);
    ui.label(theme::drobny(
        "Działa tylko na serwerze z online-mode=false. Kont offline możesz \
         mieć kilka — po jednym dla każdego, kto gra na tym komputerze.",
    ));
}

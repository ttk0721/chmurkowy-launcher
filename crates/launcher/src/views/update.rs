use crate::app::App;
use crate::theme;

/// Ekran pokazywany, gdy launcher podmienia sam siebie.
///
/// Bez niego cała aktualizacja wyglądała tak: okno znika i po chwili wraca.
/// Dla dorosłego to zagadka, dla dziecka — powód, żeby zawołać rodzica.
/// Jeden ekran z jednym zdaniem załatwia sprawę: wiadomo, co się dzieje
/// i że nic nie trzeba klikać.
pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("gora-akt")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Aktualizacja");
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            let wolne = ui.available_height();
            ui.add_space(((wolne - 200.0) / 2.0).max(0.0));

            ui.vertical_centered(|ui| {
                ui.label(theme::naglowek("Aktualizuję launcher", 22.0));
                ui.add_space(theme::S2);
                ui.add(egui::Spinner::new().size(28.0).color(theme::AKCENT));
                ui.add_space(theme::S2);

                // Ten sam pasek, co przy pobieraniu paczki — gracz zna go
                // z ekranu głównego, więc nie musi się go uczyć od nowa.
                if let Some(p) = &app.postep {
                    if let Some(u) = p.ulamek() {
                        ui.add(
                            egui::ProgressBar::new(u)
                                .desired_width(320.0)
                                .fill(theme::AKCENT_CIEMNY),
                        );
                        ui.add_space(theme::S1);
                    }
                    ui.label(theme::drobny(&p.label));
                }

                ui.add_space(theme::S3);
                ui.label(theme::drobny(
                    "Za chwilę launcher zamknie się i otworzy sam w nowej wersji. \
                     Nic nie musisz robić.",
                ));
            });
        });
}

/// Okienko z propozycją aktualizacji.
///
/// Pokazuje się tylko po samodzielnym sprawdzeniu w trakcie pracy launchera —
/// przy starcie launcher podmienia się sam i nikogo o nic nie pyta, bo przy
/// starcie nikt nie jest w połowie niczego.
///
/// Tu jest inaczej: ktoś może wpisywać nick, przeglądać paczki albo czekać
/// na koniec pobierania. Zamknięcie mu okna pod rękami, bez uprzedzenia, jest
/// gorsze niż jedno kliknięcie.
pub fn okno_propozycji(app: &mut App, ctx: &egui::Context) {
    let Some(wersja) = app.proponowana_wersja.clone() else {
        return;
    };

    let mut otwarte = true;
    egui::Window::new("Jest nowsza wersja")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .open(&mut otwarte)
        .frame(
            egui::Frame::NONE
                .fill(theme::PANEL)
                .stroke(egui::Stroke::new(1.0_f32, theme::AKCENT))
                .corner_radius(egui::CornerRadius::same(12))
                .inner_margin(egui::Margin::same(20)),
        )
        .show(ctx, |ui| {
            ui.set_max_width(430.0);
            ui.label(
                egui::RichText::new(format!("Chmurkowy Launcher {wersja} jest gotowy."))
                    .size(13.0)
                    .color(theme::AKCENT),
            );
            ui.add_space(theme::S1);
            ui.label(theme::drobny(&format!(
                "Masz wersję {}.",
                env!("CARGO_PKG_VERSION")
            )));
            ui.add_space(theme::S2);
            ui.label(theme::drobny(
                "Aktualizacja trwa kilkanaście sekund: launcher zamknie się i otworzy \
                 sam w nowej wersji. Twoje światy, paczka modów i ustawienia zostają \
                 nietknięte.",
            ));
            ui.add_space(theme::S3);

            if ui
                .add(theme::przycisk_glowny("Zaktualizuj teraz").min_size(egui::vec2(400.0, 44.0)))
                .clicked()
            {
                app.przyjmij_propozycje();
            }
            ui.add_space(theme::S1);
            if ui
                .add(theme::przycisk_zwykly("Nie teraz").min_size(egui::vec2(400.0, 40.0)))
                .clicked()
            {
                app.odloz_propozycje();
            }
            ui.add_space(4.0);
            ui.label(theme::drobny(
                "Przypomnę przy następnym uruchomieniu launchera.",
            ));
        });

    // Krzyżyk w rogu znaczy to samo, co „Nie teraz". Inaczej okienko wracałoby
    // za dziesięć minut do kogoś, kto właśnie je zamknął.
    if !otwarte {
        app.odloz_propozycje();
    }
}

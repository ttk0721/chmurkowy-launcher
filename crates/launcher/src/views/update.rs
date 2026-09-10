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

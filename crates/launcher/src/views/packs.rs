use crate::app::{App, Widok};
use crate::theme;
use chmurka_core::paczki;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    obsluz_upuszczone(app, ctx);

    egui::TopBottomPanel::top("gora-pak")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            super::pasek_tytulu(ui, ctx, "Paczki i shadery");
        });

    egui::TopBottomPanel::bottom("dol-pak")
        .frame(theme::ramka())
        .show(ctx, |ui| {
            if let Some(k) = &app.komunikat {
                let kolor = if k.starts_with("Nie ") {
                    theme::BLAD
                } else {
                    theme::SUKCES
                };
                ui.label(egui::RichText::new(k).size(12.0).color(kolor));
                ui.add_space(theme::S1);
            }
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("Wróć").min_size(egui::vec2(120.0, 44.0)))
                    .clicked()
                {
                    app.komunikat = None;
                    app.widok = Widok::Glowny;
                }
                ui.label(theme::drobny(
                    "Zmiany zadziałają przy najbliższym uruchomieniu gry.",
                ));
            });
        });

    egui::CentralPanel::default()
        .frame(theme::ramka())
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                strefa_upuszczania(ui, ctx);

                ui.add_space(theme::S4);
                sekcja_zasobow(app, ui);

                ui.add_space(theme::S4);
                sekcja_shaderow(app, ui);

                ui.add_space(theme::S3);
            });
        });
}

/// Kopiuje pliki upuszczone na okno do właściwego katalogu instancji.
fn obsluz_upuszczone(app: &mut App, ctx: &egui::Context) {
    let upuszczone: Vec<std::path::PathBuf> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .filter_map(|p| p.path.clone())
            .collect()
    });
    if upuszczone.is_empty() {
        return;
    }

    let instancja = app.data().join("instance");
    let mut dodane = Vec::new();
    let mut odrzucone = Vec::new();

    for sciezka in upuszczone {
        match paczki::dodaj_paczke(&instancja, &sciezka) {
            Ok((rodzaj, nazwa)) => {
                let co = match rodzaj {
                    paczki::RodzajPaczki::Shader => "shader",
                    _ => "paczkę zasobów",
                };
                dodane.push(format!("{nazwa} jako {co}"));
            }
            Err(_) => odrzucone.push(
                sciezka
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
            ),
        }
    }

    app.odswiez_paczki();
    app.komunikat = Some(match (dodane.is_empty(), odrzucone.is_empty()) {
        (false, true) => format!("Dodano: {}", dodane.join(", ")),
        (false, false) => format!(
            "Dodano: {}. Pominięto: {} — to nie wygląda na paczkę ani shader.",
            dodane.join(", "),
            odrzucone.join(", ")
        ),
        _ => format!(
            "Nie rozpoznano: {}. Przeciągnij plik .zip z paczką zasobów albo shaderem.",
            odrzucone.join(", ")
        ),
    });
}

fn strefa_upuszczania(ui: &mut egui::Ui, ctx: &egui::Context) {
    let nad_oknem = ctx.input(|i| !i.raw.hovered_files.is_empty());
    let (obrys, tlo, tekst) = if nad_oknem {
        (theme::AKCENT, theme::PANEL_JASNY, "Upuść — dodam we właściwe miejsce")
    } else {
        (
            theme::OBRYS,
            theme::PANEL,
            "Przeciągnij tutaj plik .zip z paczką zasobów albo shaderem",
        )
    };

    egui::Frame::NONE
        .fill(tlo)
        .stroke(egui::Stroke::new(1.0_f32, obrys))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(16, 22))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(tekst).size(13.0).color(theme::TEKST));
                ui.add_space(4.0);
                ui.label(theme::drobny(
                    "Launcher sam rozpozna, czy to paczka zasobów, czy shader.",
                ));
            });
        });
}

fn sekcja_zasobow(app: &mut App, ui: &mut egui::Ui) {
    theme::naglowek_sekcji(ui, "PACZKI ZASOBÓW");

    if app.zasoby.paczki.is_empty() {
        ui.label(theme::drobny("Nie masz jeszcze żadnych paczek zasobów."));
        return;
    }

    ui.label(theme::drobny(
        "Można włączyć kilka naraz. Wyżej na liście znaczy: nakłada się na te poniżej.",
    ));
    ui.add_space(theme::S1);

    let mut zmienione = false;
    let mut do_usuniecia: Option<String> = None;
    let ile = app.zasoby.paczki.len();

    for i in 0..ile {
        let nazwa = app.zasoby.paczki[i].plik.clone();
        ui.horizontal(|ui| {
            let mut wlaczona = app.zasoby.paczki[i].wlaczona;
            if ui.checkbox(&mut wlaczona, "").changed() {
                app.zasoby.paczki[i].wlaczona = wlaczona;
                zmienione = true;
            }
            ui.label(egui::RichText::new(&nazwa).size(13.0).color(theme::TEKST));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("Usuń")
                    .on_hover_text("Kasuje plik paczki z dysku.")
                    .clicked()
                {
                    do_usuniecia = Some(nazwa.clone());
                }
                // Kolejność ma znaczenie tylko wśród włączonych paczek.
                if wlaczona {
                    if ui
                        .add_enabled(i > 0, egui::Button::new(egui::RichText::new("wyżej").size(11.0)))
                        .on_hover_text("Wcześniej na liście ładowania")
                        .clicked()
                    {
                        app.zasoby.paczki.swap(i, i - 1);
                        zmienione = true;
                    }
                    if ui
                        .add_enabled(i + 1 < ile, egui::Button::new(egui::RichText::new("niżej").size(11.0)))
                        .on_hover_text("Później na liście ładowania")
                        .clicked()
                    {
                        app.zasoby.paczki.swap(i, i + 1);
                        zmienione = true;
                    }
                }
            });
        });
    }

    if let Some(nazwa) = do_usuniecia {
        let instancja = app.data().join("instance");
        match paczki::usun_paczke(&instancja, "resourcepacks", &nazwa) {
            Ok(()) => {
                app.komunikat = Some(format!("Usunięto {nazwa}."));
                app.odswiez_paczki();
                app.zapisz_paczki();
            }
            Err(e) => app.komunikat = Some(format!("Nie udało się usunąć {nazwa}: {e}")),
        }
    } else if zmienione {
        app.zapisz_paczki();
    }
}

fn sekcja_shaderow(app: &mut App, ui: &mut egui::Ui) {
    theme::naglowek_sekcji(ui, "SHADERY");

    if app.shadery.dostepne.is_empty() {
        ui.label(theme::drobny(
            "Nie masz jeszcze żadnych shaderów. Przeciągnij plik .zip powyżej.",
        ));
        return;
    }

    let mut zmienione = false;
    let mut wlaczone = app.shadery.wlaczone;
    if ui
        .checkbox(&mut wlaczone, "Włącz shadery")
        .on_hover_text("Shadery znacznie obciążają kartę graficzną.")
        .changed()
    {
        app.shadery.wlaczone = wlaczone;
        zmienione = true;
    }
    ui.add_space(theme::S1);

    // Shader może być aktywny tylko jeden, więc lista działa jak wybór pojedynczy.
    let mut do_usuniecia: Option<String> = None;
    let dostepne = app.shadery.dostepne.clone();
    for nazwa in &dostepne {
        ui.horizontal(|ui| {
            let wybrany = app.shadery.wybrany.as_deref() == Some(nazwa.as_str());
            if ui
                .add_enabled(wlaczone, egui::RadioButton::new(wybrany, ""))
                .clicked()
            {
                app.shadery.wybrany = Some(nazwa.clone());
                zmienione = true;
            }
            let kolor = if wlaczone {
                theme::TEKST
            } else {
                theme::TEKST_PRZYGASZONY
            };
            ui.label(egui::RichText::new(nazwa).size(13.0).color(kolor));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Usuń").clicked() {
                    do_usuniecia = Some(nazwa.clone());
                }
            });
        });
    }

    if !wlaczone {
        ui.add_space(4.0);
        ui.label(theme::drobny(
            "Zaznacz „Włącz shadery”, żeby wybrać któryś z listy.",
        ));
    }

    if let Some(nazwa) = do_usuniecia {
        let instancja = app.data().join("instance");
        // Usunięcie aktywnego shadera musi też wyczyścić wybór, inaczej gra
        // szukałaby pliku, którego już nie ma.
        if app.shadery.wybrany.as_deref() == Some(nazwa.as_str()) {
            app.shadery.wybrany = None;
            app.shadery.wlaczone = false;
        }
        match paczki::usun_paczke(&instancja, "shaderpacks", &nazwa) {
            Ok(()) => {
                app.komunikat = Some(format!("Usunięto {nazwa}."));
                app.zapisz_paczki();
                app.odswiez_paczki();
            }
            Err(e) => app.komunikat = Some(format!("Nie udało się usunąć {nazwa}: {e}")),
        }
    } else if zmienione {
        app.zapisz_paczki();
    }
}

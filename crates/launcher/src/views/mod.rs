pub mod accounts;
pub mod console;
pub mod error;
pub mod login;
pub mod main;
pub mod packs;
pub mod settings;
pub mod update;

use crate::theme::{self, Ikona};

/// Szerokość strefy przycisków okna. Obszar przeciągania musi się o tyle
/// skrócić, inaczej przechwytuje kliknięcia w „zamknij" i „zminimalizuj".
const STREFA_PRZYCISKOW: f32 = 100.0;

/// Pasek tytułu zastępujący ramkę systemową.
pub fn pasek_tytulu(ui: &mut egui::Ui, ctx: &egui::Context, tytul: &str) {
    let wiersz = ui
        .horizontal(|ui| {
            ui.label(theme::naglowek(tytul, 15.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if theme::przycisk_ikona(ui, Ikona::Zamknij, "Zamknij").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if theme::przycisk_ikona(ui, Ikona::Minimalizuj, "Zminimalizuj").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            });
        })
        .response
        .rect;

    // Przeciąganie tylko po lewej części paska i tylko przy faktycznym ruchu.
    // Wcześniej `click_and_drag` na całej szerokości zjadał kliknięcia przycisków,
    // a `is_pointer_button_down_on` uruchamiał przeciąganie już na samo wciśnięcie.
    let mut uchwyt = wiersz;
    uchwyt.max.x = (uchwyt.max.x - STREFA_PRZYCISKOW).max(uchwyt.min.x);
    let odp = ui.interact(
        uchwyt,
        egui::Id::new("uchwyt-okna"),
        egui::Sense::click_and_drag(),
    );
    if odp.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }

    // Bez wlasnej linii — TopBottomPanel rysuje juz separator na swojej krawedzi,
    // a dwie kreski obok siebie wygladaly na blad.
    ui.add_space(theme::S1);
}

/// Przycisk, który po wyszarzeniu tłumaczy, dlaczego nie działa.
/// Bez tego wygląda po prostu na zepsuty.
pub fn przycisk_warunkowy(
    ui: &mut egui::Ui,
    napis: &str,
    aktywny: bool,
    powod: &str,
) -> egui::Response {
    let odp = ui.add_enabled(aktywny, theme::przycisk_zwykly(napis));
    if aktywny {
        odp
    } else {
        odp.on_disabled_hover_text(powod)
    }
}

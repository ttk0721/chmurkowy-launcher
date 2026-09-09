pub mod login;
pub mod main;
pub mod settings;

use crate::theme;

/// Pasek tytułu zastępujący ramkę systemową: przeciąganie okna i zamykanie.
pub fn pasek_tytulu(ui: &mut egui::Ui, ctx: &egui::Context, tytul: &str) {
    let obszar = ui
        .horizontal(|ui| {
            ui.label(egui::RichText::new(tytul).size(16.0).color(theme::TEKST));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕").on_hover_text("Zamknij").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if ui.button("—").on_hover_text("Zminimalizuj").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            });
        })
        .response
        .rect;

    // Pasek jest uchwytem do przeciągania okna — bez ramki systemowej
    // nie ma innego sposobu, żeby przesunąć okno.
    let odp = ui.interact(
        obszar,
        egui::Id::new("pasek-tytulu"),
        egui::Sense::click_and_drag(),
    );
    if odp.is_pointer_button_down_on() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    ui.separator();
}

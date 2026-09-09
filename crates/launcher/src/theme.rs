use egui::{Color32, CornerRadius, Stroke};

pub const TLO: Color32 = Color32::from_rgb(18, 20, 26);
pub const PANEL: Color32 = Color32::from_rgb(26, 29, 38);
pub const AKCENT: Color32 = Color32::from_rgb(96, 165, 250);
pub const AKCENT_CIEMNY: Color32 = Color32::from_rgb(59, 130, 246);
pub const TEKST: Color32 = Color32::from_rgb(226, 232, 240);
pub const TEKST_PRZYGASZONY: Color32 = Color32::from_rgb(148, 163, 184);
pub const BLAD: Color32 = Color32::from_rgb(248, 113, 113);

pub fn zastosuj_motyw(ctx: &egui::Context) {
    let mut styl = (*ctx.style()).clone();

    styl.visuals.dark_mode = true;
    styl.visuals.panel_fill = TLO;
    styl.visuals.window_fill = TLO;
    styl.visuals.override_text_color = Some(TEKST);

    let w = &mut styl.visuals.widgets;
    for stan in [&mut w.inactive, &mut w.hovered, &mut w.active] {
        stan.corner_radius = CornerRadius::same(8);
        stan.bg_fill = PANEL;
        stan.weak_bg_fill = PANEL;
    }
    w.hovered.weak_bg_fill = AKCENT_CIEMNY;
    w.active.weak_bg_fill = AKCENT;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(40, 44, 56));

    styl.spacing.item_spacing = egui::vec2(10.0, 10.0);
    styl.spacing.button_padding = egui::vec2(16.0, 10.0);
    ctx.set_style(styl);
}

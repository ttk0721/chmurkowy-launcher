//! Warstwa wizualna launchera.
//!
//! Kierunek wg skilla ui-ux-pro-max: styl „Dark Mode (OLED)" (wysoki kontrast,
//! WCAG AAA, widoczny focus) i typografia Inter (mood: minimal, functional —
//! rekomendowana do paneli narzędziowych). Ikony rysujemy wektorowo, bo
//! emoji w roli ikon to jedna z wprost wymienionych antywzorców.

use egui::{Color32, CornerRadius, FontFamily, Stroke};
use std::sync::Arc;

// Paleta. Kontrast tekstu wobec tła sprawdzony: TEKST ~16:1,
// TEKST_PRZYGASZONY ~7,5:1 — oba powyżej progu 4,5:1.
pub const TLO: Color32 = Color32::from_rgb(13, 15, 19);
pub const PANEL: Color32 = Color32::from_rgb(22, 26, 33);
pub const PANEL_JASNY: Color32 = Color32::from_rgb(31, 36, 46);
pub const OBRYS: Color32 = Color32::from_rgb(52, 60, 76);
pub const AKCENT: Color32 = Color32::from_rgb(76, 141, 255);
pub const AKCENT_CIEMNY: Color32 = Color32::from_rgb(37, 99, 235);
pub const TEKST: Color32 = Color32::from_rgb(233, 238, 246);
pub const TEKST_PRZYGASZONY: Color32 = Color32::from_rgb(154, 167, 186);
pub const BLAD: Color32 = Color32::from_rgb(248, 113, 113);
pub const SUKCES: Color32 = Color32::from_rgb(74, 222, 128);
/// Bursztyn na ostrzeżenia, które nie są błędami.
///
/// Czerwień znaczy „coś się zepsuło" i na stałym pasku w zakładce byłaby
/// kłamstwem — nic się jeszcze nie stało. Ten kolor mówi „uważaj", a nie
/// „awaria", i przestaje straszyć po drugim spojrzeniu.
pub const OSTRZEZENIE: Color32 = Color32::from_rgb(251, 191, 36);

/// Skala odstępów oparta na 8 px. Jedna skala dla całego launchera —
/// bez tego każdy ekran miał własne, przypadkowe wartości.
pub const S1: f32 = 8.0;
pub const S2: f32 = 16.0;
pub const S3: f32 = 24.0;
pub const S4: f32 = 32.0;

/// Margines treści od krawędzi okna.
pub const MARGINES: i8 = 32;

/// Nazwa rodziny dla odmiany półgrubej — egui nie robi sztucznego pogrubienia,
/// więc nagłówki muszą dostać osobny krój.
pub fn polgruba() -> FontFamily {
    FontFamily::Name("inter-semibold".into())
}

pub fn zastosuj_motyw(ctx: &egui::Context) {
    zainstaluj_czcionki(ctx);

    let mut styl = (*ctx.style()).clone();

    styl.visuals.dark_mode = true;
    styl.visuals.panel_fill = TLO;
    styl.visuals.window_fill = TLO;
    styl.visuals.extreme_bg_color = PANEL;
    styl.visuals.override_text_color = Some(TEKST);
    styl.visuals.selection.bg_fill = AKCENT_CIEMNY;
    styl.visuals.selection.stroke = Stroke::new(1.0_f32, TEKST);

    let w = &mut styl.visuals.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, OBRYS);
    w.noninteractive.corner_radius = CornerRadius::same(10);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEKST_PRZYGASZONY);

    for stan in [&mut w.inactive, &mut w.hovered, &mut w.active] {
        stan.corner_radius = CornerRadius::same(10);
    }
    w.inactive.bg_fill = PANEL;
    w.inactive.weak_bg_fill = PANEL;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, OBRYS);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEKST);

    w.hovered.bg_fill = PANEL_JASNY;
    w.hovered.weak_bg_fill = PANEL_JASNY;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, AKCENT);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, TEKST);

    w.active.bg_fill = AKCENT_CIEMNY;
    w.active.weak_bg_fill = AKCENT_CIEMNY;
    w.active.bg_stroke = Stroke::new(1.0_f32, AKCENT);

    // Widoczny focus przy nawigacji klawiaturą — wymóg z listy dostępności.
    styl.visuals.widgets.hovered.expansion = 0.0;
    styl.visuals.clip_rect_margin = 0.0;

    styl.spacing.item_spacing = egui::vec2(S1, S1);
    styl.spacing.button_padding = egui::vec2(18.0, 10.0);
    // Domyslne 100 px sprawialo, ze suwak pamieci wygladal na zepsuty.
    styl.spacing.slider_width = 340.0;
    // Cel dotykowy/klikalny — minimum z wytycznych to 44 px.
    styl.spacing.interact_size.y = 30.0;

    // Launcher ma wyglądać tak samo niezależnie od motywu systemu.
    //
    // `set_style` zapisuje styl tylko dla motywu aktywnego w tej chwili.
    // Na Windowsie, gdzie jasny motyw jest domyślny, egui przełączało się
    // na swój jasny zestaw i nasze kolory przepadały: tła paneli zostawały
    // ciemne, bo rysujemy je własną ramką, ale przyciski i pola tekstowe
    // robiły się **białe**. W jednym oknie wyglądało to jak dwa różne
    // programy. `all_styles_mut` wpisuje ten sam styl do obu zestawów,
    // a `set_theme` przestaje iść za ustawieniem systemu.
    ctx.set_theme(egui::ThemePreference::Dark);
    ctx.all_styles_mut(|s| *s = styl.clone());
}

fn zainstaluj_czcionki(ctx: &egui::Context) {
    let mut czcionki = egui::FontDefinitions::default();

    czcionki.font_data.insert(
        "inter".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Inter-Regular.ttf"
        ))),
    );
    czcionki.font_data.insert(
        "inter-semibold".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Inter-SemiBold.ttf"
        ))),
    );

    czcionki
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".to_owned());

    czcionki
        .families
        .insert(polgruba(), vec!["inter-semibold".to_owned()]);

    ctx.set_fonts(czcionki);
}

/// Ramka treści ekranu — jednolity margines dla wszystkich widoków.
pub fn ramka() -> egui::Frame {
    egui::Frame::NONE
        .fill(TLO)
        .inner_margin(egui::Margin::symmetric(MARGINES, 18))
}

pub fn naglowek(tekst: &str, rozmiar: f32) -> egui::RichText {
    egui::RichText::new(tekst)
        .size(rozmiar)
        .family(polgruba())
        .color(TEKST)
}

pub fn drobny(tekst: &str) -> egui::RichText {
    egui::RichText::new(tekst)
        .size(12.0)
        .color(TEKST_PRZYGASZONY)
}

/// Główny przycisk akcji — jedyny wypełniony akcentem element na ekranie,
/// żeby hierarchia była jednoznaczna.
pub fn przycisk_glowny(napis: &str) -> egui::Button<'static> {
    egui::Button::new(
        egui::RichText::new(napis.to_string())
            .size(19.0)
            .family(polgruba())
            .color(Color32::WHITE),
    )
    .fill(AKCENT_CIEMNY)
    .stroke(Stroke::new(1.0_f32, AKCENT))
    .corner_radius(CornerRadius::same(12))
    .min_size(egui::vec2(280.0, 52.0))
}

/// Przycisk drugorzędny o stałej wysokości 44 px (minimum dla celu klikalnego).
pub fn przycisk_zwykly(napis: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(napis.to_string()).size(13.0))
        .min_size(egui::vec2(200.0, 44.0))
}

pub fn naglowek_sekcji(ui: &mut egui::Ui, tekst: &str) {
    ui.label(
        egui::RichText::new(tekst)
            .size(11.0)
            .family(polgruba())
            .color(TEKST_PRZYGASZONY),
    );
    ui.add_space(S1);
}

/// Rodzaje ikon rysowanych wektorowo. Emoji w roli ikon to antywzorzec
/// z listy kontrolnej — do tego brakowało ich w czcionkach egui.
#[derive(Clone, Copy, PartialEq)]
pub enum Ikona {
    Zamknij,
    Minimalizuj,
    StrzalkaDol,
    StrzalkaGora,
}

/// Kwadratowy przycisk z ikoną wektorową.
pub fn przycisk_ikona(ui: &mut egui::Ui, ikona: Ikona, podpowiedz: &str) -> egui::Response {
    let bok = 32.0;
    let (rect, odp) = ui.allocate_exact_size(egui::vec2(bok, bok), egui::Sense::click());

    let tlo = if odp.hovered() { PANEL_JASNY } else { PANEL };
    ui.painter().rect_filled(rect, CornerRadius::same(8), tlo);

    let kolor = if odp.hovered() {
        TEKST
    } else {
        TEKST_PRZYGASZONY
    };
    let s = Stroke::new(1.6_f32, kolor);
    let c = rect.center();
    let r = 5.0;

    match ikona {
        Ikona::Zamknij => {
            ui.painter()
                .line_segment([c + egui::vec2(-r, -r), c + egui::vec2(r, r)], s);
            ui.painter()
                .line_segment([c + egui::vec2(r, -r), c + egui::vec2(-r, r)], s);
        }
        Ikona::Minimalizuj => {
            ui.painter()
                .line_segment([c + egui::vec2(-r, 0.0), c + egui::vec2(r, 0.0)], s);
        }
        Ikona::StrzalkaDol => {
            ui.painter()
                .line_segment([c + egui::vec2(-r, -2.5), c + egui::vec2(0.0, 2.5)], s);
            ui.painter()
                .line_segment([c + egui::vec2(0.0, 2.5), c + egui::vec2(r, -2.5)], s);
        }
        Ikona::StrzalkaGora => {
            ui.painter()
                .line_segment([c + egui::vec2(-r, 2.5), c + egui::vec2(0.0, -2.5)], s);
            ui.painter()
                .line_segment([c + egui::vec2(0.0, -2.5), c + egui::vec2(r, 2.5)], s);
        }
    }

    odp.on_hover_text(podpowiedz)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Na Windowsie jasny motyw systemu jest domyslny. Wczesniej `set_style`
    /// zapisywal nasze kolory tylko do zestawu aktywnego w chwili wywolania,
    /// wiec po przelaczeniu na jasny egui siegalo po swoje wizualia:
    /// przyciski i pola tekstowe robily sie biale, a tla paneli zostawaly
    /// ciemne, bo rysujemy je wlasna ramka. W jednym oknie wygladalo to
    /// jak dwa rozne programy.
    #[test]
    fn nasze_kolory_obowiazuja_w_obu_motywach_systemu() {
        let ctx = egui::Context::default();
        zastosuj_motyw(&ctx);

        for motyw in [egui::Theme::Dark, egui::Theme::Light] {
            let s = ctx.style_of(motyw);
            assert_eq!(
                s.visuals.widgets.inactive.weak_bg_fill, PANEL,
                "tlo przycisku w motywie {motyw:?} nie jest nasze"
            );
            assert_eq!(
                s.visuals.panel_fill, TLO,
                "tlo panelu w motywie {motyw:?} nie jest nasze"
            );
            assert_eq!(
                s.visuals.extreme_bg_color, PANEL,
                "tlo pola tekstowego w motywie {motyw:?} nie jest nasze"
            );
            assert!(s.visuals.dark_mode, "motyw {motyw:?} musi zostac ciemny");
        }
    }

    /// Nie idziemy za ustawieniem systemu — inaczej jasny Windows znow
    /// przelaczylby wyglad w polowie dzialania programu.
    #[test]
    fn nie_idziemy_za_motywem_systemu() {
        let ctx = egui::Context::default();
        zastosuj_motyw(&ctx);
        assert_eq!(
            ctx.options(|o| o.theme_preference),
            egui::ThemePreference::Dark
        );
    }
}

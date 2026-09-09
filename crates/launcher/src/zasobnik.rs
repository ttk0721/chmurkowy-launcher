//! Ikona w zasobniku systemowym.
//!
//! Po starcie gry okno launchera znika, a launcher zostaje w zasobniku —
//! nie zabiera zasobów, ale nadal czeka na zakończenie gry, żeby zebrać log
//! i pokazać błąd, gdyby coś poszło nie tak.
//!
//! Na Linuksie używamy `ksni` (czysty Rust po D-Bus), a nie `tray-icon`,
//! bo ten drugi wciągnąłby GTK i libxdo do binarki, która dziś ma tylko
//! trzy zależności systemowe.

use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZdarzenieZasobnika {
    Pokaz,
    Zakoncz,
}

/// Uchwyt utrzymujący ikonę przy życiu. Porzucenie go usuwa ikonę.
pub struct Zasobnik {
    #[allow(dead_code)]
    trzymaj: Box<dyn std::any::Any + Send>,
}

// ---------------------------------------------------------------- Linux

#[cfg(target_os = "linux")]
pub async fn utworz(nadawca: Sender<ZdarzenieZasobnika>) -> Option<Zasobnik> {
    use ksni::TrayMethods;

    struct Ikona {
        nadawca: Sender<ZdarzenieZasobnika>,
    }

    impl ksni::Tray for Ikona {
        fn id(&self) -> String {
            "chmurkowy-launcher".into()
        }
        fn title(&self) -> String {
            "Chmurkowy Launcher".into()
        }
        // Nazwa z motywu ikon — dzięki temu nie musimy dołączać własnego pliku
        // i ikona wygląda spójnie z resztą systemu.
        fn icon_name(&self) -> String {
            "applications-games".into()
        }
        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            use ksni::menu::{MenuItem, StandardItem};
            vec![
                StandardItem {
                    label: "Pokaż launcher".into(),
                    activate: Box::new(|i: &mut Ikona| {
                        let _ = i.nadawca.send(ZdarzenieZasobnika::Pokaz);
                    }),
                    ..Default::default()
                }
                .into(),
                MenuItem::Separator,
                StandardItem {
                    label: "Zakończ".into(),
                    activate: Box::new(|i: &mut Ikona| {
                        let _ = i.nadawca.send(ZdarzenieZasobnika::Zakoncz);
                    }),
                    ..Default::default()
                }
                .into(),
            ]
        }
    }

    // Brak hosta zasobnika (np. GNOME bez rozszerzenia) konczy sie bledem —
    // wtedy zwracamy None, a launcher zamiast chowac okno tylko je minimalizuje.
    let uchwyt = Ikona { nadawca }.spawn().await.ok()?;
    Some(Zasobnik {
        trzymaj: Box::new(uchwyt),
    })
}

// -------------------------------------------------------------- Windows

#[cfg(target_os = "windows")]
pub async fn utworz(nadawca: Sender<ZdarzenieZasobnika>) -> Option<Zasobnik> {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{Icon, TrayIconBuilder};

    let menu = Menu::new();
    let pokaz = MenuItem::new("Pokaż launcher", true, None);
    let zakoncz = MenuItem::new("Zakończ", true, None);
    let id_pokaz = pokaz.id().clone();
    let id_zakoncz = zakoncz.id().clone();
    menu.append(&pokaz).ok()?;
    menu.append(&tray_icon::menu::PredefinedMenuItem::separator())
        .ok()?;
    menu.append(&zakoncz).ok()?;

    let ikona = Icon::from_rgba(ikona_rgba(), 32, 32).ok()?;
    let tray = TrayIconBuilder::new()
        .with_tooltip("Chmurkowy Launcher")
        .with_menu(Box::new(menu))
        .with_icon(ikona)
        .build()
        .ok()?;

    // Zdarzenia menu przychodzą własnym kanałem biblioteki; tłumaczymy je
    // na nasz kanał, żeby reszta launchera nie znała szczegółów platformy.
    std::thread::spawn(move || {
        let odbiorca = MenuEvent::receiver();
        while let Ok(zdarzenie) = odbiorca.recv() {
            let co = if zdarzenie.id == id_pokaz {
                ZdarzenieZasobnika::Pokaz
            } else if zdarzenie.id == id_zakoncz {
                ZdarzenieZasobnika::Zakoncz
            } else {
                continue;
            };
            if nadawca.send(co).is_err() {
                break;
            }
        }
    });

    Some(Zasobnik {
        trzymaj: Box::new(tray),
    })
}

/// Prosta ikona rysowana w kodzie — zaokrąglony kwadrat w kolorze akcentu.
/// Lepsze niż dołączanie pliku .ico, którego i tak nie da się tu podejrzeć.
#[cfg(target_os = "windows")]
fn ikona_rgba() -> Vec<u8> {
    const BOK: i32 = 32;
    const PROMIEN: f32 = 7.0;
    let mut piksele = Vec::with_capacity((BOK * BOK * 4) as usize);
    for y in 0..BOK {
        for x in 0..BOK {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let dx = (PROMIEN - fx).max(fx - (BOK as f32 - PROMIEN)).max(0.0);
            let dy = (PROMIEN - fy).max(fy - (BOK as f32 - PROMIEN)).max(0.0);
            let w_srodku = dx * dx + dy * dy <= PROMIEN * PROMIEN;
            if w_srodku {
                piksele.extend_from_slice(&[37, 99, 235, 255]);
            } else {
                piksele.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    piksele
}

// ------------------------------------------------- pozostałe systemy

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub async fn utworz(_nadawca: Sender<ZdarzenieZasobnika>) -> Option<Zasobnik> {
    None
}

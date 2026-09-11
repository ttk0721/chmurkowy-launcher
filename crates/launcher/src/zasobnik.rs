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
///
/// Celowo bez wymogu `Send`: na Windowsie `tray_icon::TrayIcon` trzyma w środku
/// `Rc<RefCell<..>>` i `Send` nie spełnia. Uchwyt i tak nigdy nie opuszcza
/// wątku interfejsu, a eframe nie wymaga `Send` od aplikacji.
pub struct Zasobnik {
    #[allow(dead_code)]
    trzymaj: Box<dyn std::any::Any>,
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

    let (piksele, szer, wys) = ikona_rgba()?;
    let ikona = Icon::from_rgba(piksele, szer, wys).ok()?;
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

/// Ikona zasobnika: prawdziwa chmurka z `assets/ikona.png`.
///
/// Wcześniej był tu zaokrąglony niebieski kwadrat rysowany w kodzie, z
/// komentarzem, że to „lepsze niż dołączanie pliku .ico". Gracz na Windowsie
/// zgłosił to jako „nie renderuje się chmurka, tylko niebieski kwadrat" — i
/// miał rację co do objawu. Nic się nie psuło; tam po prostu nigdy nie było
/// chmurki.
///
/// Plik i tak siedzi w binarce (rejestruje się nim skrót w menu), więc jedyne,
/// czego brakowało, to rozpakowanie go do pikseli. 256 dzieli się przez 8 bez
/// reszty, więc zejście do 32×32 jest zwykłym uśrednieniem bloków.
#[cfg(target_os = "windows")]
fn ikona_rgba() -> Option<(Vec<u8>, u32, u32)> {
    const IKONA: &[u8] = include_bytes!("../assets/ikona.png");
    let duza = crate::ikona::dekoduj(IKONA)?;
    let mala = crate::ikona::zmniejsz(&duza, 8).unwrap_or(duza);
    Some((mala.piksele, mala.szerokosc, mala.wysokosc))
}

// ------------------------------------------------- pozostałe systemy

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub async fn utworz(_nadawca: Sender<ZdarzenieZasobnika>) -> Option<Zasobnik> {
    None
}

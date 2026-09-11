// Bez tego na Windowsie obok okna launchera otwiera sie druga, czarna konsola.
// W buildach debug zostawiamy ja celowo — tam wyjscie na stderr jest przydatne.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ikona;
mod schowek;
mod theme;
mod views;
mod zasobnik;

use anyhow::Result;

/// Ikona launchera — ta sama, którą instalator kładzie w menu aplikacji.
const IKONA: &[u8] = include_bytes!("../assets/ikona.png");

fn main() -> Result<()> {
    // Wszystko żyje obok pliku wykonywalnego. To jest cała przenośność:
    // skasowanie folderu usuwa launcher, paczkę, Javę i dane gry.
    let katalog = std::env::current_exe()?
        .parent()
        .expect("plik wykonywalny ma katalog")
        .to_path_buf();

    // Zadomowienie. Gracz pobiera z listy wydań gołą binarkę, bo to ona
    // wygląda jak „ten plik do kliknięcia". Działa, ale zostaje jednym
    // plikiem w Pobranych: nie ma jej w menu, nie da się jej znaleźć
    // wyszukiwarką, a każda aktualizacja dokładała kolejną kopię z numerkiem.
    //
    // Więc przy pierwszym uruchomieniu launcher przenosi się tam, gdzie jego
    // miejsce, dopisuje do menu i startuje już stamtąd. Raz, po cichu.
    if let Ok(exe) = std::env::current_exe() {
        match chmurka_core::zadomowienie::zadomow(&exe) {
            chmurka_core::zadomowienie::Wynik::Zainstalowany(nowy) => {
                let _ = chmurka_core::skrot::zarejestruj(&nowy, IKONA);
                // Startujemy z docelowego miejsca i schodzimy z drogi. Dzięki
                // temu samoaktualizacja podmieni właściwy plik, a nie ten
                // leżący w Pobranych.
                if chmurka_core::aktualizacja::uruchom_ponownie(&nowy).is_ok() {
                    return Ok(());
                }
            }
            // Wpis w menu sprawdzamy przy każdym starcie: gracz mógł go
            // skasować, a system — zgubić przy aktualizacji środowiska.
            // Gdy wszystko jest na miejscu, ta funkcja nic nie robi.
            _ => {
                let _ = chmurka_core::skrot::zarejestruj(&exe, IKONA);
            }
        }
    }

    // Launcher chowa się do zasobnika, więc rodzic, który go tam nie zauważy,
    // klika skrót jeszcze raz — i do teraz dostawał drugi egzemplarz.
    // Zgłoszenie od gracza mówiło o trzech ikonach naraz.
    //
    // Samo ciche zamknięcie byłoby gorsze niż te trzy ikony: kliknięcie skrótu
    // nie robiłoby widocznie nic. Dlatego zostawiamy znacznik, a działający
    // egzemplarz pokazuje się na jego widok.
    //
    // Blokadę trzymamy do końca działania programu — `_blokada` musi żyć aż do
    // wyjścia z `main`, bo jej porzucenie wpuściłoby kolejne egzemplarze.
    let Some(_blokada) = chmurka_core::jedna_instancja::zajmij() else {
        chmurka_core::jedna_instancja::popros_o_pokazanie();
        return Ok(());
    };

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([900.0, 560.0])
        .with_min_inner_size([720.0, 480.0])
        .with_decorations(false)
        .with_title("Chmurkowy Launcher");
    // Bez tego okno i pasek zadan pokazuja zastepcza grafike systemu.
    // Ikona jest ozdoba: gdy sie nie rozpakuje, launcher startuje bez niej.
    let viewport = match ikona::dekoduj(IKONA) {
        Some(o) => viewport.with_icon(egui::IconData {
            rgba: o.piksele,
            width: o.szerokosc,
            height: o.wysokosc,
        }),
        None => viewport,
    };

    let opcje = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Chmurkowy Launcher",
        opcje,
        Box::new(move |cc| {
            theme::zastosuj_motyw(&cc.egui_ctx);
            Ok(Box::new(app::App::nowa(katalog)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("nie udało się otworzyć okna: {e}"))
}

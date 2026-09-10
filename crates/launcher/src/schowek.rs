//! Kopiowanie do schowka, które mówi, czy się udało.
//!
//! Wcześniej każde miejsce robiło `if let Ok(mut s) = Clipboard::new()` i po
//! cichu odpuszczało. Na Windowsie schowek bywa przez ułamek sekundy zajęty
//! przez inny program i `Clipboard::new()` zwraca wtedy błąd — kliknięcie
//! „Kopiuj kod” nie robiło zupełnie nic, a gracz nie wiedział, czy przycisk
//! jest zepsuty, czy on sam czegoś nie zrozumiał.

use std::time::Duration;

/// Ile razy próbujemy. Zajęty schowek zwalnia się zwykle po kilkudziesięciu
/// milisekundach, więc dwie dokładki wystarczają.
const PROBY: u32 = 3;

/// Kopiuje tekst do schowka.
///
/// Zwraca `Err` z treścią błędu, gdy się nie udało — wołający ma to pokazać
/// graczowi, a nie połknąć.
pub fn kopiuj(tekst: &str) -> Result<(), String> {
    let mut ostatni = String::from("brak prób");
    for proba in 0..PROBY {
        match sprobuj(tekst) {
            Ok(()) => return Ok(()),
            Err(e) => ostatni = e,
        }
        if proba + 1 < PROBY {
            // Krótko, bo dzieje się to na wątku rysującym — łącznie
            // najwyżej ćwierć sekundy, czyli poniżej progu zauważalności.
            std::thread::sleep(Duration::from_millis(60));
        }
    }
    Err(ostatni)
}

fn sprobuj(tekst: &str) -> Result<(), String> {
    let mut schowek = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    schowek.set_text(tekst.to_string()).map_err(|e| e.to_string())
}

/// Zdanie dla gracza po próbie kopiowania. Przy niepowodzeniu mówi, co robić
/// dalej, zamiast zostawiać go z samym komunikatem błędu.
pub fn komunikat(wynik: Result<(), String>, co_dalej: &str) -> String {
    match wynik {
        Ok(()) => "Skopiowano.".to_string(),
        Err(e) => format!("Nie udało się skopiować ({e}). {co_dalej}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udane_kopiowanie_daje_krotkie_potwierdzenie() {
        assert_eq!(komunikat(Ok(()), "cokolwiek"), "Skopiowano.");
    }

    /// Przy niepowodzeniu gracz musi uslyszec, co ma zrobic zamiast tego —
    /// sam komunikat bledu zostawialby go w tym samym miejscu co wczesniej.
    #[test]
    fn nieudane_kopiowanie_mowi_co_robic_dalej() {
        let m = komunikat(Err("schowek zajęty".into()), "Przepisz kod ręcznie.");
        assert!(m.contains("schowek zajęty"), "{m}");
        assert!(m.contains("Przepisz kod ręcznie."), "{m}");
    }
}

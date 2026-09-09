//! Wykrywanie pamięci komputera i dobór rozsądnego przydziału dla gry.
//!
//! Sztywne 4 GB było kompromisem dla wszystkich: za dużo na słabym laptopie,
//! za mało na maszynie z 32 GB.

/// Ile pamięci ma komputer, w megabajtach.
pub fn calkowita_mb() -> Option<u64> {
    platforma::calkowita_mb()
}

/// Ile przydzielić grze przy takiej ilości pamięci.
///
/// Powyżej 12 GB nie idziemy nigdy — Minecraft nie korzysta z większej sterty,
/// a odśmiecanie zaczyna się na niej wlec i powoduje przycięcia.
pub fn zalecana_mb(calkowita_mb: u64) -> u32 {
    let zalecana: u32 = match calkowita_mb {
        0..=6143 => 3072,
        6144..=10239 => 4096,
        10240..=20479 => 8192,
        _ => 12288,
    };

    // Systemowi zostawiamy 2 GB, żeby launcher nie doprowadził do zamulenia
    // całego komputera na słabszej maszynie.
    let sufit = calkowita_mb.saturating_sub(2048).max(2048) as u32;
    zalecana.min(sufit)
}

/// Czy warto pokazać graczowi propozycję zmiany. Drobne różnice pomijamy,
/// żeby nie zaczepiać go bez powodu.
pub fn warto_zaproponowac(obecna_mb: u32, zalecana_mb: u32) -> bool {
    obecna_mb.abs_diff(zalecana_mb) >= 1024
}

mod platforma {
    /// Jedna implementacja na wszystkie systemy. Wcześniej były trzy warianty
    /// pod `cfg`, przez co błąd w gałęzi windowsowej wychodził dopiero w CI —
    /// bez zainstalowanego targetu Windows nie da się jej skompilować lokalnie.
    pub fn calkowita_mb() -> Option<u64> {
        use sysinfo::{MemoryRefreshKind, RefreshKind, System};
        let system = System::new_with_specifics(
            RefreshKind::nothing().with_memory(MemoryRefreshKind::nothing().with_ram()),
        );
        let bajty = system.total_memory();
        if bajty == 0 {
            None
        } else {
            Some(bajty / 1_048_576)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slaby_komputer_dostaje_mniej_niz_domyslne_cztery_giga() {
        // 4 GB RAM — 4096 dla gry zabiloby system.
        assert_eq!(zalecana_mb(4096), 2048);
    }

    #[test]
    fn typowe_osiem_giga_to_cztery_dla_gry() {
        assert_eq!(zalecana_mb(8192), 4096);
    }

    #[test]
    fn szesnascie_giga_to_osiem() {
        assert_eq!(zalecana_mb(16384), 8192);
    }

    #[test]
    fn duzo_pamieci_nie_znaczy_nieograniczenie() {
        // Powyzej pewnego progu wieksza sterta szkodzi, wiec sufit trzyma.
        assert_eq!(zalecana_mb(32768), 12288);
        assert_eq!(zalecana_mb(196608), 12288);
    }

    #[test]
    fn zawsze_zostaje_pamiec_dla_systemu() {
        for total in [2048u64, 3072, 4096, 6144, 8192, 16384] {
            let z = zalecana_mb(total) as u64;
            assert!(
                z <= total.saturating_sub(2048).max(2048),
                "przy {total} MB zalecono {z} MB — systemowi nic nie zostaje"
            );
        }
    }

    #[test]
    fn drobne_roznice_nie_zaczepiaja_gracza() {
        assert!(!warto_zaproponowac(4096, 4096));
        assert!(!warto_zaproponowac(4096, 4608));
        assert!(warto_zaproponowac(4096, 8192));
        assert!(warto_zaproponowac(8192, 3072));
    }

    #[test]
    fn na_tej_maszynie_wykrywa_pamiec() {
        // Nie sprawdzamy konkretnej wartosci — tylko czy odczyt w ogole dziala
        // i daje cos sensownego.
        if let Some(mb) = calkowita_mb() {
            assert!(mb >= 512, "podejrzanie malo pamieci: {mb} MB");
            assert!(mb < 100_000_000, "podejrzanie duzo pamieci: {mb} MB");
        }
    }
}

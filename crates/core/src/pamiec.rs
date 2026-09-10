//! Wykrywanie pamięci komputera i dobór rozsądnego przydziału dla gry.
//!
//! Sztywne 4 GB było kompromisem dla wszystkich: za dużo na słabym laptopie,
//! za mało na maszynie z 32 GB.

/// Ile pamięci ma komputer, w megabajtach.
pub fn calkowita_mb() -> Option<u64> {
    platforma::calkowita_mb()
}

/// Ile pamięci proces gry bierze **ponad** stertę.
///
/// `-Xmx` ogranicza samą stertę Javy i nic więcej. Do tego dochodzi metaspace
/// (przy paczce z 260 modami to kilkaset megabajtów), cache skompilowanego
/// kodu, struktury odśmiecacza, stosy wątków oraz — zwykle najwięcej —
/// bufory i tekstury sterownika grafiki, które leżą poza stertą.
///
/// Zmierzone na tej paczce: przy 4 GB sterty proces sięgał blisko 6 GB.
/// Stała część to metaspace i cache kodu, część rosnąca idzie za stertą,
/// bo odśmiecacz i bufory skalują się razem z nią.
pub fn narzut_jvm_mb(sterta_mb: u32) -> u32 {
    768 + sterta_mb / 4
}

/// Ile zostawiamy systemowi: pulpit, przeglądarka i wszystko poza grą.
///
/// Na maszynie z 8 GB sam system w spoczynku potrafi zająć 2,5 GB, więc
/// 2 GB rezerwy było za mało — komputer wchodził w wymianę stron i jądro
/// ubijało grę. Bierzemy ćwiartkę pamięci, ale nigdy mniej niż 3 GB.
pub fn rezerwa_systemu_mb(calkowita_mb: u64) -> u64 {
    (calkowita_mb / 4).max(3072)
}

/// Ile pamięci zajmie cały proces gry przy takiej stercie.
pub fn szacowany_proces_mb(sterta_mb: u32) -> u32 {
    sterta_mb.saturating_add(narzut_jvm_mb(sterta_mb))
}

/// Czy taki przydział grozi tym, że system ubije grę?
///
/// Dokładnie to spotkało testera: sterta 4 GB na maszynie z 8 GB wyglądała
/// niewinnie, ale cały proces urósł do prawie 6 GB i razem z systemem
/// przekroczył pamięć komputera.
pub fn grozi_brakiem_pamieci(sterta_mb: u32, calkowita_mb: u64) -> bool {
    szacowany_proces_mb(sterta_mb) as u64 + rezerwa_systemu_mb(calkowita_mb) > calkowita_mb
}

/// Największa sterta, jaką ma sens proponować. Powyżej Minecraft i tak
/// z niej nie korzysta, a odśmiecanie zaczyna się wlec i powoduje przycięcia.
const NAJWIEKSZA_ZALECANA_MB: u64 = 8192;

/// Ile przydzielić grze przy takiej ilości pamięci.
///
/// Liczymy budżet dla **całego procesu**, nie dla samej sterty: od pamięci
/// komputera odejmujemy rezerwę systemu, a z reszty wykrawamy tyle sterty,
/// żeby zmieściła się razem ze swoim narzutem.
pub fn zalecana_mb(calkowita_mb: u64) -> u32 {
    let budzet = calkowita_mb.saturating_sub(rezerwa_systemu_mb(calkowita_mb));

    // sterta + 768 + sterta/4 <= budzet  ⇒  sterta <= (budzet − 768) · 4/5
    let sterta = budzet.saturating_sub(768) * 4 / 5;

    // Zaokrąglamy w dół do pełnych 512 MB — okrągłe liczby czyta się łatwiej,
    // a zaokrąglenie w dół nigdy nie zjada rezerwy.
    let sterta = (sterta / 512) * 512;

    sterta.clamp(1024, NAJWIEKSZA_ZALECANA_MB) as u32
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

    /// Dokladnie przypadek testera: 8 GB pamieci, sterta 4 GB. Wygladalo
    /// niewinnie, ale caly proces urosl do prawie 6 GB i jadro ubilo gre.
    #[test]
    fn cztery_giga_sterty_na_osmiogigowej_maszynie_to_za_duzo() {
        assert!(
            grozi_brakiem_pamieci(4096, 8192),
            "to ustawienie zabilo gre u testera, musi byc rozpoznane jako grozne"
        );
        assert!(!grozi_brakiem_pamieci(zalecana_mb(8192), 8192));
    }

    /// Na kazdej maszynie, na ktorej paczka ma szanse ruszyc, zalecenie musi
    /// byc bezpieczne — inaczej sami wpychalibysmy gracza w ubicie przez jadro.
    #[test]
    fn zalecenie_nigdy_nie_grozi_ubiciem_gry() {
        for total in [6144u64, 8192, 12288, 16384, 32768, 65536] {
            let z = zalecana_mb(total);
            assert!(
                !grozi_brakiem_pamieci(z, total),
                "przy {total} MB zalecono {z} MB, a to nie miesci sie w pamieci"
            );
        }
    }

    /// Ponizej 6 GB nie ma bezpiecznego ustawienia: sama rezerwa systemu
    /// i narzut JVM zjadaja cala pamiec. Zalecamy wtedy minimum, ale
    /// `grozi_brakiem_pamieci` ma o tym uczciwie mowic — to jedyny sposob,
    /// zeby gracz uslyszal, ze problem jest w komputerze, a nie w suwaku.
    #[test]
    fn na_czterech_gigach_nie_ma_bezpiecznego_ustawienia() {
        assert!(grozi_brakiem_pamieci(zalecana_mb(4096), 4096));
    }

    #[test]
    fn narzut_rosnie_razem_ze_sterta() {
        assert!(narzut_jvm_mb(8192) > narzut_jvm_mb(2048));
        // Przy 4 GB sterty proces ma miec okolo 6 GB — tyle zmierzylismy.
        let p = szacowany_proces_mb(4096);
        assert!((5500..=6500).contains(&p), "oszacowanie procesu: {p} MB");
    }

    #[test]
    fn osiem_giga_dostaje_trzy_a_nie_cztery() {
        assert_eq!(zalecana_mb(8192), 3072);
    }

    #[test]
    fn duzo_pamieci_nie_znaczy_nieograniczenie() {
        // Powyzej pewnego progu wieksza sterta szkodzi, wiec sufit trzyma.
        assert_eq!(zalecana_mb(32768), 8192);
        assert_eq!(zalecana_mb(196608), 8192);
    }

    #[test]
    fn wiecej_pamieci_nigdy_nie_znaczy_mniejsza_sterta() {
        let mut poprzednia = 0;
        for total in [4096u64, 6144, 8192, 12288, 16384, 32768] {
            let z = zalecana_mb(total);
            assert!(z >= poprzednia, "przy {total} MB zalecenie spadlo do {z}");
            poprzednia = z;
        }
    }

    #[test]
    fn slaby_komputer_dostaje_minimum_a_nie_zero() {
        // Paczka i tak nie pojdzie na 4 GB, ale zalecenie musi byc liczba
        // sensowna, a nie zerem albo wartoscia ujemna po odjeciu rezerwy.
        assert_eq!(zalecana_mb(4096), 1024);
        assert_eq!(zalecana_mb(2048), 1024);
    }

    #[test]
    fn systemowi_zawsze_zostaja_co_najmniej_trzy_giga() {
        for total in [4096u64, 8192, 16384] {
            assert!(rezerwa_systemu_mb(total) >= 3072);
        }
        // Na duzej maszynie rezerwa rosnie razem z pamiecia.
        assert_eq!(rezerwa_systemu_mb(32768), 8192);
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

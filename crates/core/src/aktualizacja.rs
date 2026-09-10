//! Samoaktualizacja launchera.
//!
//! Poprawki wychodzą czasem po kilka na godzinę, a gracz ma usłyszeć tylko
//! „zamknij i odpal ponownie". Launcher sprawdza więc przy każdym starcie,
//! czy manifest podaje nowszą wersję, i jeśli tak — pobiera ją, podmienia
//! sam siebie i uruchamia się od nowa. Nikt nie klika w żadne linki.
//!
//! Podmiana działa tak samo na obu systemach: pliku, który właśnie się
//! wykonuje, nie da się nadpisać, ale wolno go przemianować. Odsuwamy więc
//! stary na bok, wstawiamy nowy pod tę samą nazwę i startujemy go.

use crate::net::{DownloadSpec, Downloader, Expect, NetError};
use crate::progress::{Progress, Stage};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum BladAktualizacji {
    #[error(transparent)]
    Siec(#[from] NetError),
    #[error("operacja na pliku {0}: {1}")]
    Plik(String, std::io::Error),
    #[error("pobrany plik nie wygląda na program dla tego systemu: {0}")]
    NiePlikWykonywalny(String),
}

/// Co zrobić z wersją, którą podaje manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decyzja {
    /// Nie ma czego pobierać.
    Aktualna,
    Nowsza { wersja: String, url: String },
    /// Manifest nie podaje pliku dla tego systemu — nie ma jak się zaktualizować.
    BrakAdresu,
    /// Ta wersja już raz nie wskoczyła. Druga próba skończyłaby się tak samo,
    /// a launcher podmieniałby się w kółko zamiast wpuścić gracza do gry.
    JuzProbowano(String),
}

/// Klucz, pod którym manifest trzyma plik dla tego systemu.
pub fn klucz_systemu() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows-x64"
    } else {
        "linux-x64"
    }
}

/// Rozbija „0.4.5" na liczby. Wersje spoza tego kształtu zwracają `None`.
fn numery(w: &str) -> Option<(u32, u32, u32)> {
    let mut cz = w.trim().split('.');
    let a = cz.next()?.parse().ok()?;
    let b = cz.next()?.parse().ok()?;
    let c = cz.next()?.parse().ok()?;
    if cz.next().is_some() {
        return None;
    }
    Some((a, b, c))
}

/// Czy `zdalna` jest nowsza od `biezacej`?
///
/// Porównujemy liczbowo, żeby 0.4.10 wygrało z 0.4.9 — tekstowo byłoby
/// odwrotnie. Wersje o nieznanym kształcie porównujemy tekstowo: różnica
/// znaczy „coś się zmieniło", a jedna aktualizacja za dużo szkodzi mniej
/// niż przegapiona poprawka.
pub fn nowsza(biezaca: &str, zdalna: &str) -> bool {
    match (numery(biezaca), numery(zdalna)) {
        (Some(a), Some(b)) => b > a,
        _ => biezaca.trim() != zdalna.trim(),
    }
}

fn plik_znacznika(data: &Path) -> PathBuf {
    data.join("aktualizacja.txt")
}

/// Po tym czasie znacznik nieudanej próby przestaje obowiązywać.
///
/// Znacznik ma chronić przed kręceniem się w kółko, a nie zamykać drogę do
/// poprawki na zawsze. Nieudana próba bierze się zwykle stąd, że manifest
/// ogłosił wersję, której wydanie jeszcze się buduje — kwadrans później plik
/// zwykle już jest i warto spróbować ponownie.
const WAZNOSC_ZNACZNIKA_S: u64 = 30 * 60;

fn teraz_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Zwraca wersję z ważnego jeszcze znacznika. Przeterminowany traktujemy
/// tak, jakby go nie było.
fn wersja_ze_znacznika(data: &Path) -> Option<String> {
    let tresc = std::fs::read_to_string(plik_znacznika(data)).ok()?;
    let mut cz = tresc.split_whitespace();
    let wersja = cz.next()?.to_string();
    // Znacznik bez czasu pochodzi ze starszej wersji launchera — nie ma jak
    // stwierdzic, czy jest swiezy, wiec nie blokujemy nim niczego.
    let kiedy: u64 = cz.next()?.parse().ok()?;
    if teraz_s().saturating_sub(kiedy) > WAZNOSC_ZNACZNIKA_S {
        return None;
    }
    Some(wersja)
}

/// Decyduje, co zrobić — bez sięgania do sieci.
pub fn zdecyduj(biezaca: &str, najnowsza: &str, url: Option<&str>, data: &Path) -> Decyzja {
    if !nowsza(biezaca, najnowsza) {
        // Wersja z manifestu wskoczyła (albo wyprzedziliśmy manifest) —
        // znacznik nieudanej próby nie jest już do niczego potrzebny.
        let _ = std::fs::remove_file(plik_znacznika(data));
        return Decyzja::Aktualna;
    }

    if wersja_ze_znacznika(data).as_deref() == Some(najnowsza.trim()) {
        return Decyzja::JuzProbowano(najnowsza.to_string());
    }

    match url {
        Some(u) => Decyzja::Nowsza {
            wersja: najnowsza.to_string(),
            url: u.to_string(),
        },
        None => Decyzja::BrakAdresu,
    }
}

/// Binarka launchera waży kilkanaście megabajtów. Cokolwiek mniejszego to
/// strona błędu albo urwane pobieranie, a nie program.
const NAJMNIEJSZY_SENSOWNY: u64 = 2 * 1024 * 1024;

/// Czy to na pewno program dla tego systemu?
///
/// Manifest nie podaje hasha binarki — powstaje ona w CI dopiero po
/// zbudowaniu manifestu, więc nie ma go skąd wziąć. Zaufanie opiera się na
/// HTTPS do GitHuba, a tutaj sprawdzamy tylko, czy nie podmieniamy launchera
/// na stronę błędu albo połowę pliku.
fn sprawdz_plik_wykonywalny(p: &Path) -> Result<(), BladAktualizacji> {
    let rozmiar = std::fs::metadata(p)
        .map_err(|e| BladAktualizacji::Plik(p.display().to_string(), e))?
        .len();
    if rozmiar < NAJMNIEJSZY_SENSOWNY {
        return Err(BladAktualizacji::NiePlikWykonywalny(format!(
            "ma tylko {rozmiar} bajtów"
        )));
    }

    let dane = std::fs::read(p).map_err(|e| BladAktualizacji::Plik(p.display().to_string(), e))?;
    let naglowek = dane.get(0..4).unwrap_or(&[]);
    let pasuje = if cfg!(target_os = "windows") {
        naglowek.starts_with(b"MZ")
    } else {
        naglowek == b"\x7fELF"
    };
    if !pasuje {
        return Err(BladAktualizacji::NiePlikWykonywalny(format!(
            "początek pliku to {naglowek:?}"
        )));
    }
    Ok(())
}

/// Bez prawa uruchamiania nowy plik byłby na Linuksie bezużyteczny —
/// pobrany plik nie dziedziczy go po niczym.
fn nadaj_prawo_uruchamiania(p: &Path) -> Result<(), BladAktualizacji> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut u = std::fs::metadata(p)
            .map_err(|e| BladAktualizacji::Plik(p.display().to_string(), e))?
            .permissions();
        u.set_mode(0o755);
        std::fs::set_permissions(p, u)
            .map_err(|e| BladAktualizacji::Plik(p.display().to_string(), e))?;
    }
    #[cfg(not(unix))]
    let _ = p;
    Ok(())
}

fn nazwy_robocze(exe: &Path) -> (PathBuf, PathBuf) {
    let nazwa = exe
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ChmurkowyLauncher".to_string());
    (
        exe.with_file_name(format!("{nazwa}.nowy")),
        exe.with_file_name(format!("{nazwa}.stary")),
    )
}

/// Pobiera nową wersję i wstawia ją pod nazwę, spod której launcher wystartował.
///
/// Zwraca ścieżkę do gotowego pliku — uruchomienie go i zakończenie bieżącego
/// procesu zostaje po stronie interfejsu, bo tylko on wie, czy właśnie nie
/// dzieje się coś, czego nie wolno przerwać.
pub async fn pobierz_i_podmien(
    url: &str,
    wersja: &str,
    data: &Path,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<PathBuf, BladAktualizacji> {
    let exe = std::env::current_exe()
        .map_err(|e| BladAktualizacji::Plik("ścieżka launchera".to_string(), e))?;
    let (nowy, stary) = nazwy_robocze(&exe);

    // `Expect::Any` przepuściłby plik leżący po poprzedniej, przerwanej próbie.
    let _ = std::fs::remove_file(&nowy);

    // Znacznik stawiamy PRZED podmianą. Gdyby nowa wersja okazała się nie
    // działać, kolejny start zobaczy, że raz już nie wskoczyła, i nie będzie
    // podmieniał się w kółko — gracz wejdzie do gry na starej.
    zapisz_znacznik(data, wersja);

    dl.fetch_one_obserwowane(
        &DownloadSpec {
            urls: vec![url.to_string()],
            dest: nowy.clone(),
            expect: Expect::Any,
        },
        Some((
            Stage::Loader,
            format!("Pobieram launcher {wersja}"),
            on.clone(),
        )),
    )
    .await?;

    sprawdz_plik_wykonywalny(&nowy)?;
    nadaj_prawo_uruchamiania(&nowy)?;

    on(Progress::trwa(Stage::Loader, "Podmieniam launcher…"));

    let _ = std::fs::remove_file(&stary);
    std::fs::rename(&exe, &stary)
        .map_err(|e| BladAktualizacji::Plik(exe.display().to_string(), e))?;
    if let Err(e) = std::fs::rename(&nowy, &exe) {
        // Wracamy do stanu sprzed próby — stara wersja jest lepsza niż żadna.
        let _ = std::fs::rename(&stary, &exe);
        return Err(BladAktualizacji::Plik(exe.display().to_string(), e));
    }
    Ok(exe)
}

fn zapisz_znacznik(data: &Path, wersja: &str) {
    let _ = std::fs::create_dir_all(data);
    let _ = std::fs::write(plik_znacznika(data), format!("{wersja} {}", teraz_s()));
}

/// Sprząta po poprzedniej podmianie. Wołane przy starcie: dopiero wtedy
/// stary plik na pewno nie jest już uruchomiony.
pub fn posprzataj_po_podmianie() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let (nowy, stary) = nazwy_robocze(&exe);
    let _ = std::fs::remove_file(stary);
    let _ = std::fs::remove_file(nowy);
}

/// Startuje podmieniony launcher. Zakończenie bieżącego procesu należy
/// do wołającego — inaczej przez chwilę biegłyby dwa okna.
pub fn uruchom_ponownie(exe: &Path) -> Result<(), BladAktualizacji> {
    let mut cmd = std::process::Command::new(exe);
    if let Some(katalog) = exe.parent() {
        // Launcher jest przenośny: „data" leży obok pliku, więc nowy proces
        // musi wystartować z tego samego katalogu, co stary.
        cmd.current_dir(katalog);
    }
    cmd.spawn()
        .map_err(|e| BladAktualizacji::Plik(exe.display().to_string(), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nowsza_wersja_wygrywa() {
        assert!(nowsza("0.4.5", "0.4.6"));
        assert!(nowsza("0.4.5", "0.5.0"));
        assert!(nowsza("0.9.9", "1.0.0"));
    }

    #[test]
    fn ta_sama_albo_starsza_nie_wywoluje_aktualizacji() {
        assert!(!nowsza("0.4.5", "0.4.5"));
        assert!(!nowsza("0.4.6", "0.4.5"));
        assert!(!nowsza("1.0.0", "0.9.9"));
    }

    /// Porownanie tekstowe uznaloby 0.4.9 za nowsze od 0.4.10 i gracz
    /// utknalby na przedostatniej poprawce.
    #[test]
    fn dwucyfrowe_numery_porownujemy_liczbowo() {
        assert!(nowsza("0.4.9", "0.4.10"));
        assert!(!nowsza("0.4.10", "0.4.9"));
    }

    #[test]
    fn wersje_o_nieznanym_ksztalcie_porownujemy_tekstowo() {
        assert!(nowsza("0.4.5", "0.4.6-rc1"));
        assert!(!nowsza("0.4.6-rc1", "0.4.6-rc1"));
    }

    #[test]
    fn ta_sama_wersja_to_brak_roboty() {
        let kat = tempfile::tempdir().unwrap();
        assert_eq!(
            zdecyduj("0.4.5", "0.4.5", Some("https://x/y"), kat.path()),
            Decyzja::Aktualna
        );
    }

    #[test]
    fn nowsza_wersja_daje_adres_do_pobrania() {
        let kat = tempfile::tempdir().unwrap();
        assert_eq!(
            zdecyduj("0.4.5", "0.4.6", Some("https://x/y"), kat.path()),
            Decyzja::Nowsza {
                wersja: "0.4.6".into(),
                url: "https://x/y".into()
            }
        );
    }

    #[test]
    fn bez_adresu_dla_tego_systemu_nie_ma_co_pobierac() {
        let kat = tempfile::tempdir().unwrap();
        assert_eq!(
            zdecyduj("0.4.5", "0.4.6", None, kat.path()),
            Decyzja::BrakAdresu
        );
    }

    /// Gdyby nowa wersja nie wstawala, launcher bez tego pobieralby ja
    /// przy kazdym starcie i gracz nigdy nie doszedlby do przycisku GRAJ.
    #[test]
    fn wersji_ktora_raz_nie_wskoczyla_nie_probujemy_w_kolko() {
        let kat = tempfile::tempdir().unwrap();
        zapisz_znacznik(kat.path(), "0.4.6");
        assert_eq!(
            zdecyduj("0.4.5", "0.4.6", Some("https://x/y"), kat.path()),
            Decyzja::JuzProbowano("0.4.6".into())
        );
    }

    /// Znacznik dotyczy jednej wersji. Kolejna poprawka ma isc normalnie.
    #[test]
    fn znacznik_nie_blokuje_nastepnej_wersji() {
        let kat = tempfile::tempdir().unwrap();
        zapisz_znacznik(kat.path(), "0.4.6");
        assert!(matches!(
            zdecyduj("0.4.5", "0.4.7", Some("https://x/y"), kat.path()),
            Decyzja::Nowsza { .. }
        ));
    }

    /// Gdy aktualizacja w koncu wskoczyla, znacznik ma zniknac — inaczej
    /// zostalby na dysku i zablokowal powrot do tej wersji po naprawie.
    #[test]
    fn udana_aktualizacja_kasuje_znacznik() {
        let kat = tempfile::tempdir().unwrap();
        zapisz_znacznik(kat.path(), "0.4.6");
        assert_eq!(
            zdecyduj("0.4.6", "0.4.6", Some("https://x/y"), kat.path()),
            Decyzja::Aktualna
        );
        assert!(!plik_znacznika(kat.path()).is_file());
    }

    /// Najczestsza przyczyna nieudanej proby to manifest, ktory ogloszil
    /// wersje przed konskiem budowania wydania. Kwadrans pozniej plik juz
    /// jest — znacznik nie moze zamykac drogi do poprawki na zawsze.
    #[test]
    fn przeterminowany_znacznik_przestaje_blokowac() {
        let kat = tempfile::tempdir().unwrap();
        let dawno = teraz_s() - WAZNOSC_ZNACZNIKA_S - 60;
        std::fs::write(plik_znacznika(kat.path()), format!("0.4.6 {dawno}")).unwrap();
        assert!(matches!(
            zdecyduj("0.4.5", "0.4.6", Some("https://x/y"), kat.path()),
            Decyzja::Nowsza { .. }
        ));
    }

    #[test]
    fn swiezy_znacznik_wciaz_blokuje() {
        let kat = tempfile::tempdir().unwrap();
        std::fs::write(
            plik_znacznika(kat.path()),
            format!("0.4.6 {}", teraz_s() - 60),
        )
        .unwrap();
        assert_eq!(
            zdecyduj("0.4.5", "0.4.6", Some("https://x/y"), kat.path()),
            Decyzja::JuzProbowano("0.4.6".into())
        );
    }

    /// Znacznik zapisany przez starsza wersje launchera nie ma czasu.
    /// Nie da sie stwierdzic, czy jest swiezy, wiec nie blokuje niczego.
    #[test]
    fn znacznik_bez_czasu_nie_blokuje() {
        let kat = tempfile::tempdir().unwrap();
        std::fs::write(plik_znacznika(kat.path()), "0.4.6").unwrap();
        assert!(matches!(
            zdecyduj("0.4.5", "0.4.6", Some("https://x/y"), kat.path()),
            Decyzja::Nowsza { .. }
        ));
    }

    #[test]
    fn strona_bledu_zamiast_programu_nie_przechodzi() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("niby-launcher");
        std::fs::write(&p, b"<html>404</html>").unwrap();
        assert!(matches!(
            sprawdz_plik_wykonywalny(&p),
            Err(BladAktualizacji::NiePlikWykonywalny(_))
        ));
    }

    /// Plik odpowiednio duzy, ale o zlym naglowku, tez musi odpasc —
    /// inaczej podmienilibysmy launcher na cokolwiek.
    #[test]
    fn duzy_plik_o_zlym_naglowku_tez_odpada() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("niby-launcher");
        std::fs::write(&p, vec![b'x'; (NAJMNIEJSZY_SENSOWNY + 1) as usize]).unwrap();
        assert!(matches!(
            sprawdz_plik_wykonywalny(&p),
            Err(BladAktualizacji::NiePlikWykonywalny(_))
        ));
    }

    #[test]
    fn pliki_robocze_leza_obok_launchera() {
        let (nowy, stary) = nazwy_robocze(Path::new("/dom/gracz/ChmurkowyLauncher"));
        assert_eq!(nowy, Path::new("/dom/gracz/ChmurkowyLauncher.nowy"));
        assert_eq!(stary, Path::new("/dom/gracz/ChmurkowyLauncher.stary"));
    }
}

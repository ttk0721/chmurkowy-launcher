//! Paczki zasobów i shadery — odczyt i zapis tego, co gra ma włączone.
//!
//! Stan trzymają pliki gry, nie launcher: paczki zasobów siedzą w linii
//! `resourcePacks` w `options.txt`, a shader w `config/iris.properties`.
//! Dzięki temu zmiana zrobiona w grze jest widoczna w launcherze i odwrotnie —
//! nie ma dwóch źródeł prawdy, które mogłyby się rozjechać.

use std::path::{Path, PathBuf};

const KLUCZ_ZASOBOW: &str = "resourcePacks:";
const PLIK_IRIS: &str = "config/iris.properties";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaczkaZasobow {
    /// Nazwa pliku albo katalogu w `resourcepacks/`.
    pub plik: String,
    pub wlaczona: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StanZasobow {
    /// Wpisy niebędące plikami, np. `vanilla` albo `fabric`. Launcher ich nie
    /// dotyka — nie odpowiadają żadnemu plikowi i skasowanie ich psuje grę.
    pub wbudowane: Vec<String>,
    /// Włączone najpierw, w kolejności ładowania, potem reszta dostępnych.
    pub paczki: Vec<PaczkaZasobow>,
}

impl StanZasobow {
    pub fn wlaczone(&self) -> Vec<&str> {
        self.paczki
            .iter()
            .filter(|p| p.wlaczona)
            .map(|p| p.plik.as_str())
            .collect()
    }
}

/// Czyta stan paczek zasobów: co leży w katalogu i co gra ma włączone.
pub fn wczytaj_zasoby(instancja: &Path) -> StanZasobow {
    let wpisy = czytaj_liste_zasobow(&instancja.join("options.txt"));

    let mut wbudowane = Vec::new();
    let mut wlaczone_pliki = Vec::new();
    for w in wpisy {
        match w.strip_prefix("file/") {
            Some(nazwa) => wlaczone_pliki.push(nazwa.to_string()),
            None => wbudowane.push(w),
        }
    }

    let dostepne = pliki_w_katalogu(&instancja.join("resourcepacks"));

    // Najpierw włączone w kolejności z gry, potem reszta. Wpisy wskazujące
    // na nieistniejące pliki znikają same — gra i tak by je zignorowała.
    let mut paczki: Vec<PaczkaZasobow> = wlaczone_pliki
        .iter()
        .filter(|n| dostepne.contains(n))
        .map(|n| PaczkaZasobow {
            plik: n.clone(),
            wlaczona: true,
        })
        .collect();
    for d in dostepne {
        if !wlaczone_pliki.contains(&d) {
            paczki.push(PaczkaZasobow {
                plik: d,
                wlaczona: false,
            });
        }
    }

    StanZasobow { wbudowane, paczki }
}

/// Zapisuje włączone paczki z powrotem do `options.txt`.
pub fn zapisz_zasoby(instancja: &Path, stan: &StanZasobow) -> std::io::Result<()> {
    let mut lista: Vec<String> = stan.wbudowane.clone();
    for p in stan.paczki.iter().filter(|p| p.wlaczona) {
        lista.push(format!("file/{}", p.plik));
    }
    let wartosc = serde_json::to_string(&lista)?;
    podmien_linie(
        &instancja.join("options.txt"),
        KLUCZ_ZASOBOW,
        &format!("{KLUCZ_ZASOBOW}{wartosc}"),
    )
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StanShaderow {
    pub wlaczone: bool,
    pub wybrany: Option<String>,
    pub dostepne: Vec<String>,
}

pub fn wczytaj_shadery(instancja: &Path) -> StanShaderow {
    let wlasciwosci = czytaj_wlasciwosci(&instancja.join(PLIK_IRIS));
    let wybrany = wlasciwosci
        .iter()
        .find(|(k, _)| k == "shaderPack")
        .map(|(_, v)| v.clone())
        .filter(|v| !v.is_empty());
    let wlaczone = wlasciwosci
        .iter()
        .any(|(k, v)| k == "enableShaders" && v == "true");

    StanShaderow {
        wlaczone,
        wybrany,
        dostepne: pliki_w_katalogu(&instancja.join("shaderpacks")),
    }
}

pub fn zapisz_shadery(
    instancja: &Path,
    wlaczone: bool,
    wybrany: Option<&str>,
) -> std::io::Result<()> {
    let sciezka = instancja.join(PLIK_IRIS);
    let mut wlasciwosci = czytaj_wlasciwosci(&sciezka);

    ustaw(&mut wlasciwosci, "enableShaders", &wlaczone.to_string());
    ustaw(&mut wlasciwosci, "shaderPack", wybrany.unwrap_or(""));

    if let Some(rodzic) = sciezka.parent() {
        std::fs::create_dir_all(rodzic)?;
    }
    let tresc: String = wlasciwosci
        .iter()
        .map(|(k, v)| format!("{k}={v}\n"))
        .collect();
    std::fs::write(&sciezka, tresc)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RodzajPaczki {
    Zasoby,
    Shader,
    Nieznana,
}

impl RodzajPaczki {
    pub fn katalog(&self) -> Option<&'static str> {
        match self {
            RodzajPaczki::Zasoby => Some("resourcepacks"),
            RodzajPaczki::Shader => Some("shaderpacks"),
            RodzajPaczki::Nieznana => None,
        }
    }
}

/// Rozpoznaje po zawartości, czy to paczka zasobów, czy shader.
///
/// Oba to zwykłe pliki zip, więc po samej nazwie nie da się ich odróżnić —
/// a wrzucenie shadera do `resourcepacks/` po prostu nic nie da.
pub fn rozpoznaj(sciezka: &Path) -> RodzajPaczki {
    if sciezka.is_dir() {
        if sciezka.join("shaders").is_dir() {
            return RodzajPaczki::Shader;
        }
        if sciezka.join("pack.mcmeta").is_file() {
            return RodzajPaczki::Zasoby;
        }
        return RodzajPaczki::Nieznana;
    }

    let Ok(plik) = std::fs::File::open(sciezka) else {
        return RodzajPaczki::Nieznana;
    };
    let Ok(mut zip) = zip::ZipArchive::new(plik) else {
        return RodzajPaczki::Nieznana;
    };

    let mut ma_shadery = false;
    let mut ma_mcmeta = false;
    for i in 0..zip.len() {
        let Ok(wpis) = zip.by_index(i) else { continue };
        let nazwa = wpis.name();
        if nazwa.starts_with("shaders/") {
            ma_shadery = true;
        }
        if nazwa == "pack.mcmeta" {
            ma_mcmeta = true;
        }
    }

    // Shader sprawdzamy pierwszy: niektóre paczki shaderów dokładają pack.mcmeta,
    // ale paczka zasobów nigdy nie ma katalogu shaders/.
    if ma_shadery {
        RodzajPaczki::Shader
    } else if ma_mcmeta {
        RodzajPaczki::Zasoby
    } else {
        RodzajPaczki::Nieznana
    }
}

/// Kopiuje upuszczony plik do właściwego katalogu instancji.
pub fn dodaj_paczke(instancja: &Path, zrodlo: &Path) -> std::io::Result<(RodzajPaczki, String)> {
    let rodzaj = rozpoznaj(zrodlo);
    let Some(katalog) = rodzaj.katalog() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "to nie wygląda ani na paczkę zasobów, ani na shader",
        ));
    };
    let nazwa = zrodlo
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "paczka.zip".into());

    let cel_katalog = instancja.join(katalog);
    std::fs::create_dir_all(&cel_katalog)?;
    let cel = cel_katalog.join(&nazwa);
    if zrodlo.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "przeciągnij plik .zip, nie katalog",
        ));
    }
    std::fs::copy(zrodlo, &cel)?;
    Ok((rodzaj, nazwa))
}

pub fn usun_paczke(instancja: &Path, katalog: &str, nazwa: &str) -> std::io::Result<()> {
    let sciezka = instancja.join(katalog).join(nazwa);
    if sciezka.is_dir() {
        std::fs::remove_dir_all(sciezka)
    } else {
        std::fs::remove_file(sciezka)
    }
}

// --- pomocnicze ---

fn pliki_w_katalogu(katalog: &Path) -> Vec<String> {
    let Ok(wpisy) = std::fs::read_dir(katalog) else {
        return Vec::new();
    };
    let mut out: Vec<String> = wpisy
        .flatten()
        .map(|w| w.file_name().to_string_lossy().to_string())
        .filter(|n| !n.starts_with('.'))
        .collect();
    out.sort_by_key(|n| n.to_lowercase());
    out
}

fn czytaj_liste_zasobow(options: &Path) -> Vec<String> {
    let Ok(tekst) = std::fs::read_to_string(options) else {
        return Vec::new();
    };
    for linia in tekst.lines() {
        if let Some(reszta) = linia.strip_prefix(KLUCZ_ZASOBOW) {
            return serde_json::from_str(reszta.trim()).unwrap_or_default();
        }
    }
    Vec::new()
}

/// Podmienia jedną linię w pliku, zachowując resztę bez zmian.
/// Gdy linii nie ma, dokleja ją na końcu.
fn podmien_linie(sciezka: &Path, klucz: &str, nowa: &str) -> std::io::Result<()> {
    let tekst = std::fs::read_to_string(sciezka).unwrap_or_default();
    let mut linie: Vec<String> = tekst.lines().map(|s| s.to_string()).collect();

    match linie.iter().position(|l| l.starts_with(klucz)) {
        Some(i) => linie[i] = nowa.to_string(),
        None => linie.push(nowa.to_string()),
    }

    if let Some(rodzic) = sciezka.parent() {
        std::fs::create_dir_all(rodzic)?;
    }
    std::fs::write(sciezka, linie.join("\n") + "\n")
}

/// Czyta plik properties Javy, zachowując kolejność i pomijając komentarze.
fn czytaj_wlasciwosci(sciezka: &Path) -> Vec<(String, String)> {
    let Ok(tekst) = std::fs::read_to_string(sciezka) else {
        return Vec::new();
    };
    tekst
        .lines()
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            l.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

fn ustaw(wlasciwosci: &mut Vec<(String, String)>, klucz: &str, wartosc: &str) {
    match wlasciwosci.iter_mut().find(|(k, _)| k == klucz) {
        Some((_, v)) => *v = wartosc.to_string(),
        None => wlasciwosci.push((klucz.to_string(), wartosc.to_string())),
    }
}

/// Ścieżka do katalogu paczek danego rodzaju.
pub fn katalog_paczek(instancja: &Path, rodzaj: RodzajPaczki) -> Option<PathBuf> {
    rodzaj.katalog().map(|k| instancja.join(k))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn instancja(nazwa: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("chmurka-paczki-{nazwa}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("resourcepacks")).unwrap();
        std::fs::create_dir_all(d.join("shaderpacks")).unwrap();
        d
    }

    #[test]
    fn wbudowane_wpisy_przezywaja_zapis() {
        // "fabric" nie jest plikiem — skasowanie go psuje gre.
        let d = instancja("wbudowane");
        std::fs::write(
            d.join("options.txt"),
            "fov:70\nresourcePacks:[\"vanilla\",\"fabric\"]\nfullscreen:false\n",
        )
        .unwrap();

        let stan = wczytaj_zasoby(&d);
        assert_eq!(stan.wbudowane, vec!["vanilla", "fabric"]);
        assert!(stan.paczki.is_empty());

        zapisz_zasoby(&d, &stan).unwrap();
        let po = std::fs::read_to_string(d.join("options.txt")).unwrap();
        assert!(po.contains(r#"resourcePacks:["vanilla","fabric"]"#));
        // Reszta pliku nietknieta.
        assert!(po.contains("fov:70"));
        assert!(po.contains("fullscreen:false"));
    }

    #[test]
    fn wlaczanie_i_wylaczanie_paczki() {
        let d = instancja("wlaczanie");
        std::fs::write(d.join("options.txt"), "resourcePacks:[\"vanilla\"]\n").unwrap();
        std::fs::write(d.join("resourcepacks").join("ladne.zip"), b"x").unwrap();

        let mut stan = wczytaj_zasoby(&d);
        assert_eq!(stan.paczki.len(), 1);
        assert!(!stan.paczki[0].wlaczona);

        stan.paczki[0].wlaczona = true;
        zapisz_zasoby(&d, &stan).unwrap();

        let po = wczytaj_zasoby(&d);
        assert!(po.paczki[0].wlaczona);
        assert_eq!(po.wlaczone(), vec!["ladne.zip"]);
    }

    #[test]
    fn zmiana_zrobiona_w_grze_jest_widoczna_w_launcherze() {
        // Gracz wlaczyl paczke w menu gry — launcher czyta stan z options.txt,
        // wiec musi to zobaczyc bez zadnej wlasnej ewidencji.
        let d = instancja("dwustronnie");
        std::fs::write(d.join("resourcepacks").join("a.zip"), b"x").unwrap();
        std::fs::write(d.join("resourcepacks").join("b.zip"), b"x").unwrap();
        std::fs::write(
            d.join("options.txt"),
            "resourcePacks:[\"vanilla\",\"file/b.zip\"]\n",
        )
        .unwrap();

        let stan = wczytaj_zasoby(&d);
        assert_eq!(stan.wlaczone(), vec!["b.zip"]);
        // Wlaczona idzie pierwsza, reszta po niej.
        assert_eq!(stan.paczki[0].plik, "b.zip");
        assert_eq!(stan.paczki[1].plik, "a.zip");
    }

    #[test]
    fn kilka_paczek_naraz_zachowuje_kolejnosc() {
        let d = instancja("kolejnosc");
        for n in ["a.zip", "b.zip", "c.zip"] {
            std::fs::write(d.join("resourcepacks").join(n), b"x").unwrap();
        }
        std::fs::write(
            d.join("options.txt"),
            "resourcePacks:[\"vanilla\",\"file/c.zip\",\"file/a.zip\"]\n",
        )
        .unwrap();

        let stan = wczytaj_zasoby(&d);
        assert_eq!(stan.wlaczone(), vec!["c.zip", "a.zip"]);
        zapisz_zasoby(&d, &stan).unwrap();
        let po = std::fs::read_to_string(d.join("options.txt")).unwrap();
        assert!(po.contains(r#"["vanilla","file/c.zip","file/a.zip"]"#));
    }

    #[test]
    fn wpis_bez_pliku_znika() {
        // Gracz skasowal plik recznie — martwy wpis nie moze zostac.
        let d = instancja("martwy");
        std::fs::write(
            d.join("options.txt"),
            "resourcePacks:[\"vanilla\",\"file/nie-ma.zip\"]\n",
        )
        .unwrap();
        let stan = wczytaj_zasoby(&d);
        assert!(stan.paczki.is_empty());
        assert_eq!(stan.wbudowane, vec!["vanilla"]);
    }

    #[test]
    fn shadery_czytane_i_zapisywane() {
        let d = instancja("shadery");
        std::fs::create_dir_all(d.join("config")).unwrap();
        std::fs::write(
            d.join(PLIK_IRIS),
            "#komentarz\nenableShaders=false\nshaderPack=\nmaxShadowRenderDistance=4\n",
        )
        .unwrap();
        std::fs::write(d.join("shaderpacks").join("BSL.zip"), b"x").unwrap();

        let stan = wczytaj_shadery(&d);
        assert!(!stan.wlaczone);
        assert_eq!(stan.wybrany, None);
        assert_eq!(stan.dostepne, vec!["BSL.zip"]);

        zapisz_shadery(&d, true, Some("BSL.zip")).unwrap();
        let po = wczytaj_shadery(&d);
        assert!(po.wlaczone);
        assert_eq!(po.wybrany.as_deref(), Some("BSL.zip"));

        // Pozostale ustawienia Irisa przetrwaly zapis.
        let tresc = std::fs::read_to_string(d.join(PLIK_IRIS)).unwrap();
        assert!(tresc.contains("maxShadowRenderDistance=4"));
    }

    #[test]
    fn wylaczenie_shadera_czysci_wybor() {
        let d = instancja("shader-off");
        std::fs::create_dir_all(d.join("config")).unwrap();
        zapisz_shadery(&d, true, Some("BSL.zip")).unwrap();
        zapisz_shadery(&d, false, None).unwrap();
        let stan = wczytaj_shadery(&d);
        assert!(!stan.wlaczone);
        assert_eq!(stan.wybrany, None);
    }

    fn zip_z(nazwy: &[&str]) -> PathBuf {
        // Nazwy wpisow zawieraja ukosniki, wiec nie moga trafic wprost
        // do nazwy pliku tymczasowego.
        let etykieta: String = nazwy.join("_").replace(['/', '.'], "-");
        let p = std::env::temp_dir().join(format!("chmurka-zip-{etykieta}.zip"));
        let plik = std::fs::File::create(&p).unwrap();
        let mut zip = zip::ZipWriter::new(plik);
        for n in nazwy {
            zip.start_file::<_, ()>(*n, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap();
        p
    }

    #[test]
    fn rozpoznaje_paczke_zasobow_po_zawartosci() {
        let p = zip_z(&["pack.mcmeta", "assets/minecraft/textures/x.png"]);
        assert_eq!(rozpoznaj(&p), RodzajPaczki::Zasoby);
    }

    #[test]
    fn rozpoznaje_shader_po_katalogu_shaders() {
        let p = zip_z(&["shaders/gbuffers_basic.vsh"]);
        assert_eq!(rozpoznaj(&p), RodzajPaczki::Shader);
    }

    #[test]
    fn shader_z_mcmeta_to_nadal_shader() {
        // Niektore shadery dokladaja pack.mcmeta — katalog shaders/ rozstrzyga.
        let p = zip_z(&["pack.mcmeta", "shaders/gbuffers_basic.vsh"]);
        assert_eq!(rozpoznaj(&p), RodzajPaczki::Shader);
    }

    #[test]
    fn losowy_zip_nie_jest_paczka() {
        let p = zip_z(&["dokument.txt"]);
        assert_eq!(rozpoznaj(&p), RodzajPaczki::Nieznana);
    }

    #[test]
    fn dodanie_paczki_trafia_do_wlasciwego_katalogu() {
        let d = instancja("dodawanie");
        let zrodlo = zip_z(&["shaders/x.vsh"]);
        let (rodzaj, nazwa) = dodaj_paczke(&d, &zrodlo).unwrap();
        assert_eq!(rodzaj, RodzajPaczki::Shader);
        assert!(d.join("shaderpacks").join(&nazwa).is_file());
        assert!(!d.join("resourcepacks").join(&nazwa).exists());
    }

    #[test]
    fn nierozpoznana_paczka_nie_jest_kopiowana() {
        let d = instancja("odrzucenie");
        let zrodlo = zip_z(&["cokolwiek.bin"]);
        assert!(dodaj_paczke(&d, &zrodlo).is_err());
        assert_eq!(pliki_w_katalogu(&d.join("resourcepacks")).len(), 0);
        assert_eq!(pliki_w_katalogu(&d.join("shaderpacks")).len(), 0);
    }
}

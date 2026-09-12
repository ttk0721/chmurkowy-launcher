//! Wymuszanie wybranych ustawień gry — w praktyce przypisań klawiszy.
//!
//! # Dlaczego to musi istnieć osobno
//!
//! `options.txt` jest w paczce od początku, ale z zasadą `seed`: zapisujemy go
//! raz, przy pierwszej instalacji, i nigdy więcej nie dotykamy. Skutek jest
//! taki, że zmiana klawiszy przez utrzymującego **nie dociera do nikogo**, kto
//! ma już launcher — a to była prawdziwa skarga.
//!
//! Zasada `smart` też by nie pomogła. Pomija pliki zmienione przez gracza,
//! a `options.txt` Minecraft przepisuje przy **każdym** wyjściu z gry: głośność,
//! zasięg widzenia, ostatni serwer. Po pierwszym uruchomieniu plik zawsze
//! wygląda na zmieniony.
//!
//! Wysłanie całego pliku odpada z drugiej strony: skasowałoby graczowi
//! głośność, czułość myszy, ustawienia grafiki i język. To jego rzeczy.
//!
//! Zostaje scalanie: paczka niesie **wyłącznie te wpisy, na których jej
//! zależy**, a launcher podmienia tylko je i zostawia resztę pliku nietkniętą.
//!
//! # Dlaczego wpis po wpisie
//!
//! Gdyby paczka narzucała swoje klawisze przy każdym uruchomieniu, gracz, który
//! przestawił sobie klawisz, dostawałby go z powrotem po każdym starcie i nie
//! miałby jak tego obejść. Stosujemy więc tylko te wpisy, które **zmieniły się
//! od ostatniego razu**.
//!
//! Porównujemy każdy wpis osobno, a nie odcisk całego zestawu. Paczka niesie
//! komplet ponad dwustu przypisań, więc przy odcisku całości zmiana jednego
//! klawisza przez administrację kasowałaby graczowi wszystkie jego własne
//! przestawienia — a zmienić miał się tylko ten jeden.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Ustawienia gry, które paczka narzuca.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OpcjeGry {
    /// Odcisk całego zestawu. Do wglądu przy diagnozowaniu — decyzję,
    /// co zastosować, podejmujemy porównując wpisy pojedynczo.
    #[serde(default)]
    pub odcisk: String,
    /// Klucz z `options.txt` (bez dwukropka) i wartość, np.
    /// `key_key.attack` → `key.mouse.left`.
    #[serde(default)]
    pub wymuszone: BTreeMap<String, String>,
}

impl OpcjeGry {
    pub fn pusty(&self) -> bool {
        self.wymuszone.is_empty()
    }
}

/// Podmienia w treści `options.txt` wskazane wpisy, resztę zostawiając bez
/// zmiany. Zwraca `None`, gdy nic nie trzeba było poprawiać.
///
/// Wpis, którego w pliku nie ma, jest **dopisywany** — inaczej klawisz do nowego
/// moda nigdy by nie trafił do gracza, który ma starszy `options.txt`.
///
/// Kolejność i formatowanie pozostałych wierszy zostają nienaruszone, bo
/// Minecraft czyta ten plik wiersz po wierszu i nie lubi niespodzianek.
pub fn scal(tresc: &str, wymuszone: &BTreeMap<String, String>) -> Option<String> {
    if wymuszone.is_empty() {
        return None;
    }

    let konczy_sie_enterem = tresc.ends_with('\n');
    let mut zmieniono = false;
    let mut zostalo: BTreeMap<&String, &String> = wymuszone.iter().collect();

    let mut wiersze: Vec<String> = Vec::new();
    for wiersz in tresc.lines() {
        let klucz = wiersz.split_once(':').map(|(k, _)| k);
        match klucz.and_then(|k| wymuszone.get(k).map(|w| (k, w))) {
            Some((k, chciana)) => {
                zostalo.remove(&k.to_string());
                let nowy = format!("{k}:{chciana}");
                if nowy != wiersz {
                    zmieniono = true;
                }
                wiersze.push(nowy);
            }
            None => wiersze.push(wiersz.to_string()),
        }
    }

    // Czego w pliku nie było — dopisujemy na końcu.
    for (k, v) in zostalo {
        wiersze.push(format!("{k}:{v}"));
        zmieniono = true;
    }

    if !zmieniono {
        return None;
    }
    let mut wynik = wiersze.join("\n");
    if konczy_sie_enterem {
        wynik.push('\n');
    }
    Some(wynik)
}

/// Odcisk całego zestawu — do wglądu, nie do podejmowania decyzji.
///
/// Launcher porównuje wpisy pojedynczo, więc odcisk niczego nie steruje. Jest
/// w manifeście po to, żeby dało się jednym spojrzeniem stwierdzić, czy dwie
/// paczki niosą ten sam zestaw klawiszy — przy dwustu wpisach porównywanie
/// ich wzrokiem odpada.
pub fn odcisk(wymuszone: &BTreeMap<String, String>) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    // `BTreeMap` daje stałą kolejność, więc ten sam zestaw zawsze da ten sam
    // odcisk. Bajt zerowy rozdziela pola, żeby para („ab", „c") nie dała tego
    // samego skrótu co („a", „bc").
    for (k, v) in wymuszone {
        h.update(k.as_bytes());
        h.update([0]);
        h.update(v.as_bytes());
        h.update([0]);
    }
    h.finalize()
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Stosuje wymuszone ustawienia w instancji gracza, o ile jest co stosować.
///
/// Zwraca notatkę dla gracza albo `None`, gdy nic nie zrobiono. Sam fakt, że
/// launcher przestawił klawisze, trzeba pokazać — inaczej gracz zobaczy, że
/// jego klawisz „sam się zmienił", i uzna to za usterkę.
///
/// Stan zapamiętujemy dopiero po udanym zapisie pliku. Po błędzie dysku
/// launcher spróbuje ponownie przy następnym uruchomieniu, zamiast uznać
/// zestaw za wgrany i zostawić gracza ze starymi klawiszami.
pub fn zastosuj(
    instancja: &std::path::Path,
    opcje: &OpcjeGry,
    stan: &mut crate::state::State,
) -> std::io::Result<Option<String>> {
    // Tylko to, co administracja zmieniła od ostatniego razu. Wpis, który
    // gracz przestawił sobie sam, a którego paczka nie ruszała, zostaje jego.
    let swieze: BTreeMap<String, String> = opcje
        .wymuszone
        .iter()
        .filter(|(k, v)| stan.opcje_zastosowane.get(*k) != Some(*v))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if swieze.is_empty() {
        return Ok(None);
    }

    let plik = instancja.join("options.txt");
    // Brak pliku nie jest błędem: gra jeszcze nie wystartowała ani razu,
    // a `options.txt` z paczki i tak wejdzie zaraz jako plik zasiewany.
    let Ok(tresc) = std::fs::read_to_string(&plik) else {
        stan.opcje_zastosowane = opcje.wymuszone.clone();
        return Ok(None);
    };

    let zmieniono = scal(&tresc, &swieze);
    if let Some(nowa) = &zmieniono {
        std::fs::write(&plik, nowa)?;
    }
    // Zapamiętujemy cały zestaw, nie tylko świeże wpisy: reszta jest już
    // u gracza właściwa, a bez tego liczylibyśmy ją jako zmianę co uruchomienie.
    stan.opcje_zastosowane = opcje.wymuszone.clone();

    Ok(zmieniono.map(|_| {
        format!(
            "Paczka ustawiła {} ustawień gry (m.in. klawisze). Możesz je zmienić w grze — \
             launcher nie ruszy ich ponownie, dopóki administracja ich nie zmieni.",
            swieze.len()
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapa(pary: &[(&str, &str)]) -> BTreeMap<String, String> {
        pary.iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// Sedno: podmieniamy wskazany klawisz i NIE ruszamy niczego innego.
    /// Gracz ma zachowac swoja glosnosc, czulosc myszy i zasieg widzenia.
    #[test]
    fn podmienia_tylko_wskazane_wpisy() {
        let przed = "version:3955\nkey_key.attack:key.mouse.left\nsoundCategory_master:0.35\nkey_key.jump:key.keyboard.space\nfov:0.7\n";
        let po = scal(przed, &mapa(&[("key_key.jump", "key.keyboard.j")])).expect("byla zmiana");

        assert!(po.contains("key_key.jump:key.keyboard.j"));
        // Wszystko poza tym jednym wierszem bez zmian.
        assert!(po.contains("soundCategory_master:0.35"));
        assert!(po.contains("fov:0.7"));
        assert!(po.contains("key_key.attack:key.mouse.left"));
        assert!(po.contains("version:3955"));
        assert_eq!(przed.lines().count(), po.lines().count());
    }

    /// Klawisz do nowego moda moze nie istniec w pliku gracza — trzeba go
    /// dopisac, inaczej nigdy by do niego nie trafil.
    #[test]
    fn dopisuje_brakujacy_wpis() {
        let po = scal(
            "fov:0.7\n",
            &mapa(&[("key_key.nowy_mod.cos", "key.keyboard.k")]),
        )
        .expect("byla zmiana");
        assert!(po.contains("fov:0.7"));
        assert!(po.contains("key_key.nowy_mod.cos:key.keyboard.k"));
    }

    /// Gdy plik juz ma zadane wartosci, nie zapisujemy go na nowo. Bez tego
    /// launcher dotykalby pliku przy kazdym uruchomieniu bez powodu.
    #[test]
    fn brak_zmian_nie_powoduje_zapisu() {
        let tresc = "key_key.jump:key.keyboard.space\nfov:0.7\n";
        assert!(scal(tresc, &mapa(&[("key_key.jump", "key.keyboard.space")])).is_none());
        assert!(scal(tresc, &BTreeMap::new()).is_none());
    }

    /// Wartosc moze zawierac dwukropek (np. `key.mouse.left`), wiec dzielimy
    /// tylko na pierwszym. Podzial na ostatnim urwalby klucz.
    #[test]
    fn dzieli_na_pierwszym_dwukropku() {
        let po = scal(
            "key_key.cos:key.keyboard.a\n",
            &mapa(&[("key_key.cos", "key.mouse.5")]),
        )
        .expect("byla zmiana");
        assert_eq!(po, "key_key.cos:key.mouse.5\n");
    }

    /// Minecraft czyta ten plik wiersz po wierszu; brak konca linii na koncu
    /// pliku ma zostac brakiem, a obecny — obecnym.
    #[test]
    fn zachowuje_koniec_pliku() {
        let z_enterem = scal("fov:0.7\n", &mapa(&[("a", "b")])).unwrap();
        assert!(z_enterem.ends_with('\n'));
        let bez = scal("fov:0.7", &mapa(&[("a", "b")])).unwrap();
        assert!(!bez.ends_with('\n'));
    }

    fn opcje(odcisk: &str, pary: &[(&str, &str)]) -> OpcjeGry {
        OpcjeGry {
            odcisk: odcisk.to_string(),
            wymuszone: mapa(pary),
        }
    }

    /// Sedno calego pomyslu: paczka narzuca klawisze RAZ, przy nowym zestawie.
    /// Gracz, ktory potem przestawi klawisz, ma go zachowac — inaczej nie
    /// mialby jak uzywac wlasnego ustawienia.
    #[test]
    fn narzuca_raz_i_szanuje_pozniejsza_zmiane_gracza() {
        let kat = tempfile::tempdir().unwrap();
        let plik = kat.path().join("options.txt");
        std::fs::write(&plik, "key_key.jump:key.keyboard.space\nfov:0.7\n").unwrap();
        let mut stan = crate::state::State::new();

        let o = opcje("aaa", &[("key_key.jump", "key.keyboard.j")]);
        assert!(zastosuj(kat.path(), &o, &mut stan).unwrap().is_some());
        assert!(std::fs::read_to_string(&plik)
            .unwrap()
            .contains("key_key.jump:key.keyboard.j"));

        // Gracz przestawia klawisz po swojemu...
        std::fs::write(&plik, "key_key.jump:key.keyboard.z\nfov:0.7\n").unwrap();
        // ...i ten sam zestaw juz go nie rusza.
        assert!(zastosuj(kat.path(), &o, &mut stan).unwrap().is_none());
        assert!(std::fs::read_to_string(&plik)
            .unwrap()
            .contains("key_key.jump:key.keyboard.z"));

        // Dopiero ZMIANA tego wpisu przez administracje wygrywa.
        let o2 = opcje("bbb", &[("key_key.jump", "key.keyboard.k")]);
        assert!(zastosuj(kat.path(), &o2, &mut stan).unwrap().is_some());
        assert!(std::fs::read_to_string(&plik)
            .unwrap()
            .contains("key_key.jump:key.keyboard.k"));
    }

    /// Najwazniejsza wlasnosc calego mechanizmu. Paczka niesie komplet ponad
    /// dwustu przypisan. Gdy administracja zmieni JEDEN klawisz, gracz ma
    /// stracic tylko ten jeden — a nie wszystkie swoje przestawienia.
    #[test]
    fn zmiana_jednego_klawisza_nie_kasuje_pozostalych_przestawien() {
        let kat = tempfile::tempdir().unwrap();
        let plik = kat.path().join("options.txt");
        std::fs::write(
            &plik,
            "key_key.jump:key.keyboard.space\nkey_key.attack:key.mouse.left\n",
        )
        .unwrap();
        let mut stan = crate::state::State::new();

        let zestaw = &[
            ("key_key.jump", "key.keyboard.space"),
            ("key_key.attack", "key.mouse.left"),
        ];
        zastosuj(kat.path(), &opcje("aaa", zestaw), &mut stan).unwrap();

        // Gracz przestawia sobie skok na „z".
        std::fs::write(
            &plik,
            "key_key.jump:key.keyboard.z\nkey_key.attack:key.mouse.left\n",
        )
        .unwrap();

        // Administracja zmienia CO INNEGO — atak.
        let nowy = &[
            ("key_key.jump", "key.keyboard.space"),
            ("key_key.attack", "key.mouse.right"),
        ];
        assert!(zastosuj(kat.path(), &opcje("bbb", nowy), &mut stan)
            .unwrap()
            .is_some());

        let po = std::fs::read_to_string(&plik).unwrap();
        assert!(po.contains("key_key.attack:key.mouse.right"), "{po}");
        assert!(
            po.contains("key_key.jump:key.keyboard.z"),
            "skok gracza mial zostac nietkniety: {po}"
        );
    }

    /// Brak `options.txt` (gra nie ruszyla ani razu) nie moze byc bledem —
    /// plik z paczki i tak zaraz wejdzie jako zasiewany.
    #[test]
    fn brak_pliku_nie_jest_bledem() {
        let kat = tempfile::tempdir().unwrap();
        let mut stan = crate::state::State::new();
        let wynik = zastosuj(kat.path(), &opcje("aaa", &[("a", "b")]), &mut stan);
        assert!(wynik.unwrap().is_none());
        assert_eq!(
            stan.opcje_zastosowane.get("a").map(String::as_str),
            Some("b"),
            "zestaw uznany za wgrany"
        );
    }

    /// Odcisk musi zalezec od TRESCI, i to jednoznacznie. Gdyby dwa rozne
    /// zestawy dawaly ten sam odcisk, zmiana klawiszy nie doszlaby do graczy.
    #[test]
    fn odcisk_zalezy_od_tresci() {
        let a = odcisk(&mapa(&[("k", "v")]));
        assert_eq!(
            a,
            odcisk(&mapa(&[("k", "v")])),
            "ten sam zestaw, ten sam odcisk"
        );
        assert_ne!(a, odcisk(&mapa(&[("k", "w")])), "inna wartosc");
        assert_ne!(a, odcisk(&mapa(&[("l", "v")])), "inny klucz");
        // Rozdzielenie pol bajtem zerowym: („ab",„c") i („a",„bc") to rozne zestawy.
        assert_ne!(odcisk(&mapa(&[("ab", "c")])), odcisk(&mapa(&[("a", "bc")])));
        // Kolejnosc wpisania nie ma znaczenia — BTreeMap i tak je porzadkuje.
        assert_eq!(
            odcisk(&mapa(&[("a", "1"), ("b", "2")])),
            odcisk(&mapa(&[("b", "2"), ("a", "1")]))
        );
    }

    /// Pusty zestaw nie moze niczego dotykac.
    #[test]
    fn pusty_zestaw_nic_nie_robi() {
        let kat = tempfile::tempdir().unwrap();
        std::fs::write(kat.path().join("options.txt"), "fov:0.7\n").unwrap();
        let mut stan = crate::state::State::new();
        assert!(zastosuj(kat.path(), &opcje("ccc", &[]), &mut stan)
            .unwrap()
            .is_none());
        assert!(stan.opcje_zastosowane.is_empty());
    }
}

//! Zapamiętane konta gracza.
//!
//! Do wersji 0.4.17 launcher pamiętał jedno konto Microsoft i jeden nick
//! offline. W praktyce przy jednym komputerze siedzi rodzeństwo, a jedna
//! osoba miewa konto do gry i drugie do testów — przelogowywanie za każdym
//! razem oznaczało wpisywanie kodu z Microsoftu od nowa.
//!
//! Teraz trzymamy listę kont i to, które jest wybrane. Stary plik z jednym
//! kontem wczytuje się dalej: nikt nie wylogowuje się przez aktualizację.

use super::{Account, AccountKind};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Czym jest zapamiętane konto i co trzeba, żeby na nie wrócić.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rodzaj")]
pub enum Rodzaj {
    /// Konto Microsoft. Token odświeżania pozwala wrócić bez wpisywania kodu.
    Microsoft { refresh_token: String },
    /// Konto offline — sam nick, do grania na serwerze bez weryfikacji.
    Offline,
}

/// Token odświeżania zastąpiony znacznikiem — pozwala odtworzyć sesję gracza,
/// więc jest sekretem na równi z tokenem dostępu. `Debug` zostaje, bo bez
/// niego nie da się użyć `assert_eq!` w testach.
impl std::fmt::Debug for Rodzaj {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rodzaj::Microsoft { .. } => f
                .debug_struct("Microsoft")
                .field("refresh_token", &"<ukryty>")
                .finish(),
            Rodzaj::Offline => f.write_str("Offline"),
        }
    }
}

impl Rodzaj {
    fn znacznik(&self) -> &'static str {
        match self {
            Rodzaj::Microsoft { .. } => "msa",
            Rodzaj::Offline => "offline",
        }
    }

    pub fn to_kind(&self) -> AccountKind {
        match self {
            Rodzaj::Microsoft { .. } => AccountKind::Msa,
            Rodzaj::Offline => AccountKind::Offline,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZapisaneKonto {
    pub nick: String,
    /// Znane dopiero po zalogowaniu. Puste u kont przeniesionych ze starego
    /// pliku, który go nie zapisywał.
    #[serde(default)]
    pub uuid: String,
    #[serde(flatten)]
    pub rodzaj: Rodzaj,
}

impl ZapisaneKonto {
    /// Klucz, po którym rozpoznajemy konto na liście.
    ///
    /// Celowo nie UUID: przy koncie przeniesionym ze starego pliku jeszcze
    /// go nie znamy, a klucz musi działać od pierwszej chwili. Nick w obrębie
    /// jednego rodzaju wystarcza — nie da się mieć dwóch kont Microsoft
    /// o tej samej nazwie.
    pub fn klucz(&self) -> String {
        format!("{}:{}", self.rodzaj.znacznik(), self.nick.to_lowercase())
    }

    pub fn offline(nick: &str) -> Self {
        let konto = super::offline::offline_account(nick);
        Self {
            nick: konto.name,
            uuid: konto.uuid,
            rodzaj: Rodzaj::Offline,
        }
    }
}

/// Lista kont i to, które jest wybrane.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Konta {
    /// Klucz wybranego konta. `None`, gdy lista jest pusta.
    #[serde(default)]
    pub wybrane: Option<String>,
    #[serde(default)]
    pub konta: Vec<ZapisaneKonto>,
}

impl Konta {
    pub fn wybrane(&self) -> Option<&ZapisaneKonto> {
        let klucz = self.wybrane.as_ref()?;
        self.konta.iter().find(|k| &k.klucz() == klucz)
    }

    /// Dodaje konto albo odświeża już zapamiętane, i ustawia je jako wybrane.
    pub fn dodaj(&mut self, konto: ZapisaneKonto) {
        let klucz = konto.klucz();
        match self.konta.iter_mut().find(|k| k.klucz() == klucz) {
            Some(istniejace) => *istniejace = konto,
            None => self.konta.push(konto),
        }
        self.wybrane = Some(klucz);
    }

    pub fn wybierz(&mut self, klucz: &str) -> bool {
        if self.konta.iter().any(|k| k.klucz() == klucz) {
            self.wybrane = Some(klucz.to_string());
            true
        } else {
            false
        }
    }

    /// Usuwa konto z listy. Gdy było wybrane, wybór przechodzi na pierwsze
    /// z pozostałych — gracz nie powinien zostać z niczym po skasowaniu
    /// jednego z kilku kont.
    pub fn usun(&mut self, klucz: &str) {
        self.konta.retain(|k| k.klucz() != klucz);
        if self.wybrane.as_deref() == Some(klucz) {
            self.wybrane = self.konta.first().map(|k| k.klucz());
        }
    }
}

/// Konto offline daje się odtworzyć bez sieci — nick wystarczy.
pub fn na_konto_offline(z: &ZapisaneKonto) -> Account {
    super::offline::offline_account(&z.nick)
}

pub fn wczytaj(path: &Path) -> Konta {
    let Ok(tresc) = std::fs::read_to_string(path) else {
        return Konta::default();
    };
    let Ok(wartosc) = serde_json::from_str::<serde_json::Value>(&tresc) else {
        return Konta::default();
    };

    // Rozpoznajemy format wprost, po kluczu `refresh_token` na wierzchu.
    //
    // Nie da się tego zrobić „spróbuj nowy, potem stary": `Konta` ma same
    // pola opcjonalne, więc pochłonęłoby stary plik po cichu i zwróciło
    // pustą listę. Czyli aktualizacja launchera wylogowywałaby wszystkich,
    // nie zgłaszając żadnego błędu.
    if let Some(token) = wartosc.get("refresh_token").and_then(|v| v.as_str()) {
        let nick = wartosc
            .get("nick")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let mut k = Konta::default();
        k.dodaj(ZapisaneKonto {
            nick: nick.to_string(),
            uuid: String::new(),
            rodzaj: Rodzaj::Microsoft {
                refresh_token: token.to_string(),
            },
        });
        return k;
    }

    serde_json::from_value::<Konta>(wartosc).unwrap_or_default()
}

pub fn zapisz(path: &Path, k: &Konta) -> std::io::Result<()> {
    if let Some(rodzic) = path.parent() {
        std::fs::create_dir_all(rodzic)?;
    }
    let tresc = serde_json::to_vec_pretty(k)?;

    // Plik zawiera tokeny odświeżania, więc na Linuksie zawężamy uprawnienia.
    // Na Windowsie polegamy na uprawnieniach katalogu użytkownika.
    //
    // Prawa ustawiamy PRZY TWORZENIU, a nie po zapisaniu. Wcześniej plik
    // powstawał z prawami 0644 i dopiero potem był zawężany — między jednym
    // a drugim istniało okno, w którym token odświeżania mógł przeczytać
    // każdy użytkownik maszyny. Okno krótkie, ale otwierane przy każdym
    // logowaniu i przy każdym „Wyloguj wszystkie konta".
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut plik = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        plik.write_all(&tresc)?;

        // `mode` działa tylko przy tworzeniu pliku. Gdy plik już istniał —
        // choćby zapisany przez starszą wersję launchera — trzeba go zawęzić
        // osobno, inaczej zostałby przy swoich dawnych prawach.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    std::fs::write(path, &tresc)?;

    Ok(())
}

pub fn clear(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msa(nick: &str, token: &str) -> ZapisaneKonto {
        ZapisaneKonto {
            nick: nick.into(),
            uuid: String::new(),
            rodzaj: Rodzaj::Microsoft {
                refresh_token: token.into(),
            },
        }
    }

    #[test]
    fn dodane_konto_staje_sie_wybrane() {
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "t1"));
        assert_eq!(k.wybrane().map(|x| x.nick.as_str()), Some("Zosia"));
    }

    /// Sedno zgloszenia: dwa konta online i kilka offline naraz.
    #[test]
    fn trzymamy_wiele_kont_obu_rodzajow() {
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "t1"));
        k.dodaj(msa("Franek", "t2"));
        k.dodaj(ZapisaneKonto::offline("Test1"));
        k.dodaj(ZapisaneKonto::offline("Test2"));
        assert_eq!(k.konta.len(), 4);
        assert!(k.wybierz("msa:zosia"));
        assert_eq!(k.wybrane().map(|x| x.nick.as_str()), Some("Zosia"));
    }

    /// Ponowne zalogowanie na to samo konto ma ODSWIEZYC wpis, a nie
    /// dokladac drugi taki sam — inaczej lista rosnie w nieskonczonosc.
    #[test]
    fn ponowne_logowanie_nie_dubluje_konta() {
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "stary"));
        k.dodaj(msa("Zosia", "nowy"));
        assert_eq!(k.konta.len(), 1);
        match &k.konta[0].rodzaj {
            Rodzaj::Microsoft { refresh_token } => assert_eq!(refresh_token, "nowy"),
            _ => panic!("zly rodzaj"),
        }
    }

    /// Konto online i offline o tym samym nicku to dwa rozne konta —
    /// wlasnie tak testuje sie paczke przed wpuszczeniem graczy.
    #[test]
    fn ten_sam_nick_online_i_offline_to_dwa_konta() {
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "t"));
        k.dodaj(ZapisaneKonto::offline("Zosia"));
        assert_eq!(k.konta.len(), 2);
    }

    #[test]
    fn wielkosc_liter_w_nicku_nie_tworzy_drugiego_konta() {
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "t"));
        k.dodaj(msa("zosia", "t"));
        assert_eq!(k.konta.len(), 1);
    }

    /// Po skasowaniu wybranego konta gracz nie moze zostac z niczym,
    /// skoro ma jeszcze inne.
    #[test]
    fn usuniecie_wybranego_przenosi_wybor_na_inne() {
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "t"));
        k.dodaj(ZapisaneKonto::offline("Test1"));
        k.wybierz("msa:zosia");
        k.usun("msa:zosia");
        assert_eq!(k.konta.len(), 1);
        assert_eq!(k.wybrane().map(|x| x.nick.as_str()), Some("Test1"));
    }

    #[test]
    fn usuniecie_ostatniego_zostawia_pusto() {
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "t"));
        k.usun("msa:zosia");
        assert!(k.konta.is_empty());
        assert!(k.wybrane().is_none());
    }

    #[test]
    fn nieznany_klucz_nie_zmienia_wyboru() {
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "t"));
        assert!(!k.wybierz("msa:ktos-inny"));
        assert_eq!(k.wybrane().map(|x| x.nick.as_str()), Some("Zosia"));
    }

    /// Aktualizacja launchera nie moze nikogo wylogowac. Stary plik mial
    /// jedno konto Microsoft i zaden UUID.
    #[test]
    fn stary_plik_z_jednym_kontem_wczytuje_sie_dalej() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("auth.json");
        std::fs::write(&p, r#"{"refresh_token":"stary-token","nick":"Zosia"}"#).unwrap();

        let k = wczytaj(&p);
        assert_eq!(k.konta.len(), 1);
        let konto = k.wybrane().expect("konto ma byc wybrane");
        assert_eq!(konto.nick, "Zosia");
        match &konto.rodzaj {
            Rodzaj::Microsoft { refresh_token } => assert_eq!(refresh_token, "stary-token"),
            _ => panic!("stare konto bylo kontem Microsoft"),
        }
    }

    #[test]
    fn zapis_i_odczyt_sa_odwracalne() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("auth.json");
        let mut k = Konta::default();
        k.dodaj(msa("Zosia", "t1"));
        k.dodaj(ZapisaneKonto::offline("Test1"));
        zapisz(&p, &k).unwrap();
        assert_eq!(wczytaj(&p), k);
    }

    #[test]
    fn brak_pliku_to_pusta_lista() {
        let kat = tempfile::tempdir().unwrap();
        assert_eq!(wczytaj(&kat.path().join("nie-ma")), Konta::default());
    }

    /// Uszkodzony plik nie moze wywrocic launchera — gracz po prostu
    /// zaloguje sie ponownie.
    #[test]
    fn uszkodzony_plik_daje_pusta_liste() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("auth.json");
        std::fs::write(&p, "{to nie jest json").unwrap();
        assert_eq!(wczytaj(&p), Konta::default());
    }

    /// Konto offline odtwarzamy bez sieci, a jego UUID musi byc taki sam
    /// jak przy zwyklym logowaniu offline — inaczej gracz straci postepy.
    #[test]
    fn konto_offline_odtwarza_ten_sam_uuid() {
        let zapisane = ZapisaneKonto::offline("Test1");
        let konto = na_konto_offline(&zapisane);
        assert_eq!(
            konto.uuid,
            super::super::offline::offline_account("Test1").uuid
        );
        assert_eq!(konto.kind, AccountKind::Offline);
    }
}

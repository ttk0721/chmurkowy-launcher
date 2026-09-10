//! Podgląd logu gry na żywo.
//!
//! Log gry leci prosto do pliku, żeby launcher mógł schować się do zasobnika,
//! a zapis i tak powstawał w całości. Skutek uboczny był taki, że gracz nie
//! miał *żadnego* sposobu, żeby sprawdzić, co się dzieje: okno gry jeszcze nie
//! wstało, procesor stoi na jednym procencie i nie wiadomo, czy paczka wciąż
//! się ładuje, czy proces dawno umarł.
//!
//! Ten moduł czyta ogon pliku i odpowiada na to jednym zdaniem.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Ile ostatnich linii trzymamy. Ładowanie paczki z 261 modami potrafi
/// wypisać dziesiątki tysięcy linii, a w oknie i tak liczy się końcówka.
pub const ILE_LINII: usize = 800;

/// Po tylu sekundach ciszy w logu uznajemy, że coś jest nie tak.
///
/// Wczytywanie modów potrafi zamilknąć na kilkadziesiąt sekund przy
/// budowaniu atlasu tekstur, więc próg jest wysoki — fałszywy alarm byłby
/// gorszy od jego braku.
const CISZA_PODEJRZANA_S: u64 = 90;

/// Co widać w logu w tej chwili.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stan {
    /// Pliku jeszcze nie ma — gra nie została uruchomiona.
    BrakLogu,
    /// Log rośnie, coś się dzieje.
    Zyje,
    /// Log stoi od dłuższego czasu.
    Cisza { sekund: u64 },
}

pub struct Konsola {
    sciezka: PathBuf,
    linie: Vec<String>,
    /// Rozmiar pliku przy ostatnim odczycie — tanie sprawdzenie, czy coś doszło.
    rozmiar: u64,
    ostatnia_zmiana: Instant,
    czytany: bool,
}

impl Konsola {
    pub fn nowa(sciezka: PathBuf) -> Self {
        Self {
            sciezka,
            linie: Vec::new(),
            rozmiar: 0,
            ostatnia_zmiana: Instant::now(),
            czytany: false,
        }
    }

    pub fn linie(&self) -> &[String] {
        &self.linie
    }

    pub fn sciezka(&self) -> &Path {
        &self.sciezka
    }

    /// Dociąga nowe linie, jeśli plik urósł. Wołane co klatkę, więc w typowym
    /// przypadku kończy się na jednym `metadata` i niczym więcej.
    pub fn odswiez(&mut self) {
        let Ok(meta) = std::fs::metadata(&self.sciezka) else {
            return;
        };
        let rozmiar = meta.len();
        if self.czytany && rozmiar == self.rozmiar {
            return;
        }

        // Gra tworzy log od nowa przy każdym starcie, więc plik potrafi
        // zmaleć. Wtedy czytamy go w całości jeszcze raz.
        if rozmiar < self.rozmiar {
            self.linie.clear();
        }

        if let Ok(tresc) = std::fs::read_to_string(&self.sciezka) {
            let wszystkie: Vec<&str> = tresc.lines().collect();
            let od = wszystkie.len().saturating_sub(ILE_LINII);
            self.linie = wszystkie[od..].iter().map(|s| s.to_string()).collect();
        }

        self.rozmiar = rozmiar;
        self.ostatnia_zmiana = Instant::now();
        self.czytany = true;
    }

    pub fn stan(&self) -> Stan {
        if !self.czytany {
            return Stan::BrakLogu;
        }
        let cisza = self.ostatnia_zmiana.elapsed();
        if cisza > Duration::from_secs(CISZA_PODEJRZANA_S) {
            Stan::Cisza {
                sekund: cisza.as_secs(),
            }
        } else {
            Stan::Zyje
        }
    }

    /// Zdanie dla gracza. To jedyna rzecz, którą naprawdę trzeba przeczytać.
    pub fn opis_stanu(&self, gra_dziala: bool) -> String {
        match self.stan() {
            Stan::BrakLogu => "Gra jeszcze nie ruszyła — nie ma czego pokazać.".to_string(),
            Stan::Cisza { sekund } if gra_dziala => format!(
                "Gra działa, ale nic nie wypisała od {}. Przy dużej paczce to jeszcze bywa \
                 normalne; jeśli cisza się przeciąga, zamknij grę i uruchom ją ponownie.",
                opisz_czas(sekund)
            ),
            Stan::Cisza { sekund } => format!(
                "Ostatni wpis sprzed {}. Gra już nie działa.",
                opisz_czas(sekund)
            ),
            Stan::Zyje if gra_dziala => "Gra się ładuje — log rośnie.".to_string(),
            Stan::Zyje => "Gra już nie działa. Poniżej jej ostatnie zapiski.".to_string(),
        }
    }

    /// Cały widoczny log do wklejenia administracji.
    pub fn do_schowka(&self) -> String {
        self.linie.join("\n")
    }
}

fn opisz_czas(sekund: u64) -> String {
    if sekund < 120 {
        format!("{sekund} sekund")
    } else {
        format!("{} minut", sekund / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zapisz(p: &Path, tresc: &str) {
        std::fs::write(p, tresc).unwrap();
    }

    #[test]
    fn bez_pliku_nie_ma_czego_pokazywac() {
        let kat = tempfile::tempdir().unwrap();
        let k = Konsola::nowa(kat.path().join("brak.log"));
        assert_eq!(k.stan(), Stan::BrakLogu);
        assert!(k.linie().is_empty());
    }

    #[test]
    fn czyta_linie_z_pliku() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("game.log");
        zapisz(&p, "pierwsza\ndruga\ntrzecia\n");
        let mut k = Konsola::nowa(p);
        k.odswiez();
        assert_eq!(k.linie(), ["pierwsza", "druga", "trzecia"]);
        assert_eq!(k.stan(), Stan::Zyje);
    }

    /// Laduje sie 261 modow — log rosnie do dziesiatek tysiecy linii,
    /// a w oknie liczy sie tylko koncowka.
    #[test]
    fn trzymamy_tylko_ogon() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("game.log");
        let duzo: String = (0..ILE_LINII * 3)
            .map(|i| format!("linia {i}\n"))
            .collect();
        zapisz(&p, &duzo);
        let mut k = Konsola::nowa(p);
        k.odswiez();
        assert_eq!(k.linie().len(), ILE_LINII);
        assert_eq!(
            k.linie().last().unwrap(),
            &format!("linia {}", ILE_LINII * 3 - 1)
        );
    }

    #[test]
    fn dociaga_nowe_linie() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("game.log");
        zapisz(&p, "jedna\n");
        let mut k = Konsola::nowa(p.clone());
        k.odswiez();
        assert_eq!(k.linie().len(), 1);

        zapisz(&p, "jedna\ndruga\n");
        k.odswiez();
        assert_eq!(k.linie(), ["jedna", "druga"]);
    }

    /// Gra tworzy log od nowa przy kazdym starcie. Bez tego okno pokazywaloby
    /// wpisy z poprzedniej rozgrywki wymieszane z nowymi.
    #[test]
    fn krotszy_plik_czytamy_od_nowa() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("game.log");
        zapisz(&p, "stara jedna\nstara druga\nstara trzecia\n");
        let mut k = Konsola::nowa(p.clone());
        k.odswiez();
        assert_eq!(k.linie().len(), 3);

        zapisz(&p, "nowa\n");
        k.odswiez();
        assert_eq!(k.linie(), ["nowa"]);
    }

    /// To jest cale sedno tego okna: gracz ma sie dowiedziec, czy czekac,
    /// czy dzialac.
    #[test]
    fn opis_stanu_mowi_czy_czekac() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("game.log");
        zapisz(&p, "cokolwiek\n");
        let mut k = Konsola::nowa(p);
        k.odswiez();

        let dziala = k.opis_stanu(true);
        assert!(dziala.contains("ładuje"), "{dziala}");

        let nie_dziala = k.opis_stanu(false);
        assert!(nie_dziala.contains("nie działa"), "{nie_dziala}");
    }

    #[test]
    fn cisze_opisujemy_w_minutach_gdy_dluga() {
        assert_eq!(opisz_czas(45), "45 sekund");
        assert_eq!(opisz_czas(600), "10 minut");
    }

    #[test]
    fn schowek_oddaje_widoczne_linie() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("game.log");
        zapisz(&p, "a\nb\n");
        let mut k = Konsola::nowa(p);
        k.odswiez();
        assert_eq!(k.do_schowka(), "a\nb");
    }
}

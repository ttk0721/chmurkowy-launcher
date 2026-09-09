use std::collections::BTreeMap;
use std::path::Path;

/// Pamięć launchera o tym, co sam wgrał.
///
/// Bez tego nie da się odróżnić „gracz zmienił config" od „paczka się
/// zaktualizowała" — a od tego zależy, czy wolno nadpisać plik.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default)]
    pub written: BTreeMap<String, String>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// Brak pliku albo uszkodzony JSON dają pusty stan, nie błąd.
    /// Najgorsze, co się wtedy stanie, to jedno pominięcie aktualizacji configu.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(rodzic) = path.parent() {
            std::fs::create_dir_all(rodzic)?;
        }
        let tresc = serde_json::to_vec_pretty(self)?;
        std::fs::write(path, tresc)
    }
}

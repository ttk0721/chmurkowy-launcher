#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Manifest,
    Java,
    Loader,
    Libraries,
    Assets,
    Pack,
    Ready,
}

impl Stage {
    /// Tekst pokazywany użytkownikowi w linii stanu.
    pub fn opis(&self) -> &'static str {
        match self {
            Stage::Manifest => "Sprawdzam paczkę",
            Stage::Java => "Pobieram Javę",
            Stage::Loader => "Instaluję NeoForge",
            Stage::Libraries => "Pobieram biblioteki",
            Stage::Assets => "Pobieram zasoby gry",
            Stage::Pack => "Pobieram mody",
            Stage::Ready => "Gotowe",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub stage: Stage,
    pub done: u64,
    pub total: u64,
    pub bytes: u64,
    pub label: String,
}

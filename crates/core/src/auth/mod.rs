pub mod msa;
pub mod offline;
pub mod store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    Msa,
    Offline,
}

impl AccountKind {
    /// Wartość argumentu `--userType` oczekiwana przez klienta gry.
    pub fn user_type(&self) -> &'static str {
        match self {
            AccountKind::Msa => "msa",
            AccountKind::Offline => "legacy",
        }
    }
}

#[derive(Clone)]
pub struct Account {
    pub name: String,
    pub uuid: String,
    pub token: String,
    pub kind: AccountKind,
}

/// Token zastąpiony znacznikiem.
///
/// Nick i UUID są publiczne — widzi je każdy na serwerze — więc zostają.
/// Token daje pełną władzę nad kontem, więc nie ma prawa pojawić się
/// w żadnym `{:?}`, choćby dopisanym w pośpiechu przy szukaniu innej usterki.
impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account")
            .field("name", &self.name)
            .field("uuid", &self.uuid)
            .field("token", &"<ukryty>")
            .field("kind", &self.kind)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wyprowadzony `Debug` wypisywal token dostepu. Nick i UUID sa publiczne
    /// i zostaja — token nie ma prawa pojawic sie w zadnym `{:?}`.
    #[test]
    fn debug_konta_nie_wypisuje_tokenu() {
        let k = Account {
            name: "Tomek".into(),
            uuid: "61a50080-80aa-3842-8df5-cd674d3a57f2".into(),
            token: "TAJNY-TOKEN-DOSTEPU".into(),
            kind: AccountKind::Offline,
        };
        let s = format!("{k:?}");
        assert!(!s.contains("TAJNY-TOKEN-DOSTEPU"), "{s}");
        assert!(s.contains("Tomek"), "nick jest publiczny: {s}");
    }
}

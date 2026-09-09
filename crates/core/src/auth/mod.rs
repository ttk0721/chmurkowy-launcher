pub mod offline;

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

#[derive(Debug, Clone)]
pub struct Account {
    pub name: String,
    pub uuid: String,
    pub token: String,
    pub kind: AccountKind,
}

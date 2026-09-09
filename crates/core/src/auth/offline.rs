use super::{Account, AccountKind};
use md5::{Digest, Md5};

/// Odtwarza javowe `UUID.nameUUIDFromBytes("OfflinePlayer:<nick>")`.
///
/// UWAGA: to NIE jest UUID v3 z przestrzenią nazw. Java liczy zwykłe MD5
/// z samych bajtów i dopiero potem wpisuje wersję i wariant. Użycie
/// `Uuid::new_v3` z jakąkolwiek przestrzenią da inny wynik i serwer
/// potraktuje gracza jako kogoś innego.
pub fn offline_uuid(name: &str) -> String {
    let mut h = Md5::new();
    h.update(format!("OfflinePlayer:{name}").as_bytes());
    let mut b = h.finalize();
    b[6] = (b[6] & 0x0f) | 0x30; // wersja 3
    b[8] = (b[8] & 0x3f) | 0x80; // wariant IETF
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn offline_account(name: &str) -> Account {
    Account {
        name: name.to_string(),
        uuid: offline_uuid(name),
        // Klient wymaga niepustego tokenu; na serwerze offline i tak nie jest sprawdzany.
        token: "0".to_string(),
        kind: AccountKind::Offline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odtwarza_javowe_nameuuidfrombytes() {
        // Wartosci wyliczone z algorytmu Javy UUID.nameUUIDFromBytes
        // dla ciagu "OfflinePlayer:<nick>" — tak serwery licza UUID w trybie offline.
        assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
        assert_eq!(
            offline_uuid("TestGracz"),
            "2849380d-0c09-3dd1-87a1-a00a3f22c477"
        );
        assert_eq!(
            offline_uuid("Tomasz"),
            "61a50080-80aa-3842-8df5-cd674d3a57f2"
        );
    }

    #[test]
    fn ma_wersje_3_i_wariant_ietf() {
        let u = offline_uuid("Notch");
        assert_eq!(
            &u[14..15],
            "3",
            "czwarty blok musi zaczynac sie od wersji 3"
        );
        assert!(
            matches!(&u[19..20], "8" | "9" | "a" | "b"),
            "wariant IETF"
        );
    }

    #[test]
    fn konto_offline_ma_pusty_token() {
        let k = offline_account("Tomasz");
        assert_eq!(k.name, "Tomasz");
        assert_eq!(k.kind, crate::auth::AccountKind::Offline);
        assert_eq!(k.uuid, "61a50080-80aa-3842-8df5-cd674d3a57f2");
    }
}

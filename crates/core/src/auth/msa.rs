use super::{Account, AccountKind};
use serde::Deserialize;

const CONNECT: &str = "https://login.live.com/oauth20_connect.srf";
const TOKEN: &str = "https://login.live.com/oauth20_token.srf";
const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("błąd połączenia z Microsoft: {0}")]
    Siec(String),
    #[error("logowanie nie powiodło się: {0}")]
    Odmowa(String),
    #[error("kod wygasł — spróbuj zalogować się jeszcze raz")]
    Wygasl,
    #[error("Xbox Live odmówił: {0}")]
    Xbox(String),
    #[error("to konto Microsoft nie ma kupionego Minecrafta")]
    BrakGry,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    #[serde(rename = "interval")]
    pub interval_s: u64,
    #[serde(rename = "expires_in")]
    pub expires_in_s: u64,
}

#[derive(Debug, Clone)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug)]
pub enum PollResult {
    Czekamy,
    Zwolnij,
    Gotowe(Tokens),
}

fn klient() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("ChmurkowyLauncher/0.1")
        .build()
        .expect("klient HTTP")
}

/// Rozpoczyna logowanie. Zwraca kod, który użytkownik wpisuje na stronie Microsoftu.
pub async fn begin(client_id: &str) -> Result<DeviceCode, AuthError> {
    let odp = klient()
        .post(CONNECT)
        .form(&[
            ("client_id", client_id),
            ("scope", SCOPE),
            ("response_type", "device_code"),
        ])
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;
    let tekst = odp.text().await.map_err(|e| AuthError::Siec(e.to_string()))?;
    serde_json::from_str(&tekst).map_err(|_| AuthError::Odmowa(tekst))
}

pub fn zinterpretuj_blad(kod: &str) -> Option<PollResult> {
    match kod {
        "authorization_pending" => Some(PollResult::Czekamy),
        "slow_down" => Some(PollResult::Zwolnij),
        _ => None,
    }
}

pub async fn poll_once(client_id: &str, device_code: &str) -> Result<PollResult, AuthError> {
    #[derive(Deserialize)]
    struct Odp {
        access_token: Option<String>,
        refresh_token: Option<String>,
        error: Option<String>,
    }

    let tekst = klient()
        .post(TOKEN)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?
        .text()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;

    let o: Odp = serde_json::from_str(&tekst).map_err(|_| AuthError::Odmowa(tekst.clone()))?;

    if let (Some(a), Some(r)) = (o.access_token, o.refresh_token) {
        return Ok(PollResult::Gotowe(Tokens {
            access_token: a,
            refresh_token: r,
        }));
    }
    match o.error.as_deref() {
        Some(kod) => match zinterpretuj_blad(kod) {
            Some(stan) => Ok(stan),
            None if kod == "expired_token" => Err(AuthError::Wygasl),
            None => Err(AuthError::Odmowa(kod.to_string())),
        },
        None => Err(AuthError::Odmowa(tekst)),
    }
}

pub async fn refresh(client_id: &str, refresh_token: &str) -> Result<Tokens, AuthError> {
    #[derive(Deserialize)]
    struct Odp {
        access_token: String,
        refresh_token: String,
    }
    let tekst = klient()
        .post(TOKEN)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?
        .text()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;
    let o: Odp = serde_json::from_str(&tekst).map_err(|_| AuthError::Odmowa(tekst))?;
    Ok(Tokens {
        access_token: o.access_token,
        refresh_token: o.refresh_token,
    })
}

pub fn opis_xerr(kod: u64) -> String {
    match kod {
        2148916233 => "to konto Microsoft nie ma profilu Xbox — załóż go na xbox.com i spróbuj ponownie".into(),
        2148916238 => "to konto dziecka — musi zostać dodane do rodziny Microsoft, żeby móc grać".into(),
        inny => format!("Xbox Live odrzucił logowanie, kod {inny}"),
    }
}

/// Zamienia token Microsoftu na token Minecrafta. Cztery żądania pod rząd.
pub async fn zaloguj_minecraft(tokens: &Tokens) -> Result<Account, AuthError> {
    let c = klient();

    // 1. Xbox Live. Token z MBI_SSL jest juz biletem RPS, wiec idzie bez prefiksu "d=".
    #[derive(Deserialize)]
    struct XblOdp {
        #[serde(rename = "Token")]
        token: String,
        #[serde(rename = "DisplayClaims")]
        claims: Claims,
    }
    #[derive(Deserialize)]
    struct Claims {
        xui: Vec<Xui>,
    }
    #[derive(Deserialize)]
    struct Xui {
        uhs: String,
    }

    let xbl: XblOdp = c
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com",
                            "RpsTicket": tokens.access_token },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?
        .json()
        .await
        .map_err(|e| AuthError::Xbox(e.to_string()))?;

    let uhs = xbl
        .claims
        .xui
        .first()
        .map(|x| x.uhs.clone())
        .unwrap_or_default();

    // 2. XSTS.
    let odp = c
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl.token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;

    if odp.status() == reqwest::StatusCode::UNAUTHORIZED {
        #[derive(Deserialize)]
        struct Blad {
            #[serde(rename = "XErr")]
            xerr: u64,
        }
        let b: Blad = odp.json().await.map_err(|e| AuthError::Xbox(e.to_string()))?;
        return Err(AuthError::Xbox(opis_xerr(b.xerr)));
    }

    #[derive(Deserialize)]
    struct XstsOdp {
        #[serde(rename = "Token")]
        token: String,
    }
    let xsts: XstsOdp = odp.json().await.map_err(|e| AuthError::Xbox(e.to_string()))?;

    // 3. Token Minecrafta.
    #[derive(Deserialize)]
    struct McOdp {
        access_token: String,
    }
    let mc: McOdp = c
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({ "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token) }))
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?
        .json()
        .await
        .map_err(|e| AuthError::Odmowa(e.to_string()))?;

    // 4. Profil. 404 znaczy konto bez kupionej gry — to najczestszy blad u testerow.
    let odp = c
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc.access_token)
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;

    if odp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AuthError::BrakGry);
    }

    #[derive(Deserialize)]
    struct Profil {
        id: String,
        name: String,
    }
    let p: Profil = odp
        .json()
        .await
        .map_err(|e| AuthError::Odmowa(e.to_string()))?;

    // API zwraca UUID bez myslnikow, a gra oczekuje ich w argumencie.
    let u = &p.id;
    let uuid = if u.len() == 32 {
        format!(
            "{}-{}-{}-{}-{}",
            &u[0..8],
            &u[8..12],
            &u[12..16],
            &u[16..20],
            &u[20..32]
        )
    } else {
        u.clone()
    };

    Ok(Account {
        name: p.name,
        uuid,
        token: mc.access_token,
        kind: AccountKind::Msa,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn czyta_odpowiedz_device_code() {
        // Ksztalt sprawdzony empirycznie na login.live.com/oauth20_connect.srf
        let json = r#"{"user_code":"V3REVW36","device_code":"-Dk2jLDRyYPl32RFpA",
                       "verification_uri":"https://www.microsoft.com/link",
                       "interval":5,"expires_in":900}"#;
        let d: DeviceCode = serde_json::from_str(json).unwrap();
        assert_eq!(d.user_code, "V3REVW36");
        assert_eq!(d.verification_uri, "https://www.microsoft.com/link");
        assert_eq!(d.interval_s, 5);
        assert_eq!(d.expires_in_s, 900);
    }

    #[test]
    fn rozpoznaje_stany_odpytywania() {
        assert!(matches!(
            zinterpretuj_blad("authorization_pending"),
            Some(PollResult::Czekamy)
        ));
        assert!(matches!(
            zinterpretuj_blad("slow_down"),
            Some(PollResult::Zwolnij)
        ));
        assert!(zinterpretuj_blad("expired_token").is_none());
        assert!(zinterpretuj_blad("authorization_declined").is_none());
    }

    #[test]
    fn tlumaczy_kody_xsts_na_polski() {
        assert!(opis_xerr(2148916233).contains("konta Xbox") || opis_xerr(2148916233).contains("Xbox"));
        assert!(opis_xerr(2148916238).contains("dziecka"));
        assert!(opis_xerr(999).contains("999"));
    }
}

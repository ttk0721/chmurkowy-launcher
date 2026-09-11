use super::{Account, AccountKind};
use serde::Deserialize;

const CONNECT: &str = "https://login.live.com/oauth20_connect.srf";
const TOKEN: &str = "https://login.live.com/oauth20_token.srf";
const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("błąd połączenia z Microsoft: {0}")]
    Siec(String),
    #[error("logowanie nie powiodło się: {szczegoly}")]
    Odmowa {
        powod: PowodOdmowy,
        szczegoly: String,
    },
    #[error("kod wygasł — spróbuj zalogować się jeszcze raz")]
    Wygasl,
    #[error("Xbox Live odmówił: {szczegoly}")]
    Xbox { powod: PowodXbox, szczegoly: String },
    #[error("to konto Microsoft nie ma kupionego Minecrafta")]
    BrakGry,
}

impl AuthError {
    /// Odmowa, której nie potrafimy zaklasyfikować.
    ///
    /// Skrót dla miejsc, w których nie ma kodu błędu od Microsoftu — została
    /// sama treść odpowiedzi albo błąd odczytu.
    fn odmowa(szczegoly: impl Into<String>) -> Self {
        AuthError::Odmowa {
            powod: PowodOdmowy::Inny,
            szczegoly: szczegoly.into(),
        }
    }
}

/// Dlaczego Microsoft odmówił zalogowania.
///
/// Osobno od treści komunikatu — tak samo jak przy [`PowodXbox`] niżej.
/// Rozpoznawanie powodu po tekście błędu raz już się w tym projekcie zemściło
/// i nie ma po co powtarzać tej pomyłki.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowodOdmowy {
    /// Gracz kliknął „Nie" albo zamknął okno zgody, nie potwierdzając.
    Odrzucone,
    /// Kod urządzenia stracił ważność albo został już raz wymieniony.
    KodNiewazny,
    /// Kod był jeszcze ważny, a Microsoft i tak nie wydał dostępu.
    ///
    /// Zostaje wtedy, gdy konto wymaga czegoś, czego okienko z kodem nie
    /// potrafi pokazać: potwierdzenia tożsamości, zgody rodzica w rodzinie
    /// Microsoft albo akceptacji nowego regulaminu. Ponawianie nie pomoże ani
    /// za dziesiątym razem — dopóki nikt nie załatwi tego w przeglądarce,
    /// odpowiedź będzie identyczna.
    KontoWymagaDzialania,
    /// Zapamiętane logowanie przestało być przyjmowane przez Microsoft.
    ///
    /// Dotyczy odświeżania, nie kodu urządzenia — żadnego kodu wtedy na ekranie
    /// nie było, więc rady o przepisywaniu go są tu bez sensu. Zdarza się po
    /// zmianie hasła, po dłuższej przerwie i po wylogowaniu urządzeń
    /// w ustawieniach konta.
    ZapisaneLogowanieWygaslo,
    /// Coś, czego nie rozpoznajemy. Szczegóły niosą kod i opis od Microsoftu.
    Inny,
}

/// Ten sam kod błędu znaczy co innego przy odświeżaniu niż przy kodzie
/// urządzenia, bo inne jest pytanie, które zadaliśmy. `invalid_grant` przy
/// odświeżaniu nie mówi nic o żadnym kodzie — mówi, że zapisany token już nie
/// działa. Tak samo `access_denied`: przy odświeżaniu znaczy, że ktoś odebrał
/// launcherowi dostęp w ustawieniach konta, a nie że kliknął „Nie” w okienku.
pub fn powod_odmowy_odswiezania(kod: &str) -> PowodOdmowy {
    match kod {
        "invalid_grant" | "access_denied" => PowodOdmowy::ZapisaneLogowanieWygaslo,
        _ => PowodOdmowy::Inny,
    }
}

/// Tłumaczy kod błędu z odpowiedzi Microsoftu na powód, który da się objaśnić.
pub fn powod_odmowy(kod: &str) -> PowodOdmowy {
    match kod {
        "access_denied" => PowodOdmowy::Odrzucone,
        // Microsoft oddaje `invalid_grant`, gdy kod urządzenia stracił ważność
        // albo został już wymieniony na token — na przykład gdy ktoś otworzył
        // stronę z kodem dwa razy. RFC 8628 przewiduje na to osobne
        // `expired_token`, ale Microsoft bywa tu niekonsekwentny i oddaje
        // jedno albo drugie.
        "invalid_grant" => PowodOdmowy::KodNiewazny,
        _ => PowodOdmowy::Inny,
    }
}

/// Doprecyzowuje odmowę wiedzą, której samo `poll_once` nie ma: czy kod
/// urządzenia zdążył już wygasnąć.
///
/// Microsoft oddaje `invalid_grant` i przy kodzie przeterminowanym, i wtedy gdy
/// po prostu nie chce wydać dostępu temu kontu. Rozróżnienie po treści opisu
/// byłoby zgadywaniem z angielskiego zdania, które może się zmienić bez
/// uprzedzenia. Zegar zgadywaniem nie jest: kod urządzenia żyje kilkanaście
/// minut, więc odmowa, która przyszła przed terminem, na pewno nie jest
/// wygaśnięciem.
///
/// Ma to znaczenie praktyczne, bo rady się wykluczają. Przy wygaśnięciu trzeba
/// wziąć nowy kod i wpisać go szybciej. Przy odmowie dla konta nowy kod niczego
/// nie zmieni i gracz może ponawiać bez końca — trzeba odwiedzić konto
/// Microsoft w przeglądarce.
pub fn doprecyzuj_odmowe(blad: AuthError, kod_mogl_wygasnac: bool) -> AuthError {
    match blad {
        AuthError::Odmowa {
            powod: PowodOdmowy::KodNiewazny,
            szczegoly,
        } if !kod_mogl_wygasnac => AuthError::Odmowa {
            powod: PowodOdmowy::KontoWymagaDzialania,
            szczegoly,
        },
        inny => inny,
    }
}

/// Szczegóły techniczne odmowy: kod błędu wraz z opisem, o ile Microsoft go dał.
///
/// Opis (`error_description`) jest polem stworzonym po to, żeby je czytać —
/// zwykle niesie numer `AADSTS` i zdanie wyjaśnienia. Wcześniej launcher
/// wyrzucał go do kosza i pokazywał sam kod, więc gracz widział `invalid_grant`
/// i nic poza tym, a administracja nie miała czego szukać.
pub fn opis_odmowy(kod: &str, opis: Option<&str>) -> String {
    match opis {
        Some(o) if !o.trim().is_empty() => skrot(&format!("{kod}: {}", o.trim()), 300),
        _ => kod.to_string(),
    }
}

/// Dlaczego Xbox Live odmówił.
///
/// Wcześniej rozpoznawaliśmy to po treści komunikatu po polsku — wystarczyło,
/// żeby w opisie błędu padło słowo „Xbox", a gracz dostawał radę o zakładaniu
/// profilu Xbox przy zupełnie innym problemie. Zdanie dla gracza i decyzja,
/// co mu poradzić, muszą stać na osobnych nogach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowodXbox {
    BrakProfiluXbox,
    KontoDziecka,
    RegionNiedostepny,
    WymaganaWeryfikacja,
    KontoZablokowane,
    /// Cokolwiek innego — łącznie z odpowiedzią, której nie umiemy odczytać.
    Inny,
}

/// Kody, którymi Xbox Live tłumaczy odmowę.
pub fn powod_xerr(kod: u64) -> PowodXbox {
    match kod {
        2148916227 => PowodXbox::KontoZablokowane,
        2148916233 => PowodXbox::BrakProfiluXbox,
        2148916235 => PowodXbox::RegionNiedostepny,
        2148916236 | 2148916237 => PowodXbox::WymaganaWeryfikacja,
        2148916238 => PowodXbox::KontoDziecka,
        _ => PowodXbox::Inny,
    }
}

#[derive(Clone, Deserialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    /// Adres z kodem już wpisanym — RFC 8628 nazywa to `verification_uri_complete`.
    ///
    /// Nie każdy serwer go oddaje, stąd `Option`. Gdy jest, otwarcie go stawia
    /// gracza od razu przy wyborze konta, z pominięciem strony do przepisywania
    /// kodu.
    pub verification_uri_complete: Option<String>,
    #[serde(rename = "interval")]
    pub interval_s: u64,
    #[serde(rename = "expires_in")]
    pub expires_in_s: u64,
}

impl DeviceCode {
    /// Adres do otwarcia w przeglądarce — z kodem już wpisanym, jeśli się da.
    ///
    /// Przepisywanie kodu jest najwęższym gardłem całego logowania i źródłem
    /// całej rodziny błędów: kod przeterminowany, kod wpisany z literówką,
    /// strona otwarta dwa razy. Gracz, który dostaje adres z kodem w środku,
    /// żadnego z nich nie napotka — pierwsze, co zobaczy, to wybór konta.
    ///
    /// Gdy serwer nie oddał gotowego adresu, doklejamy kod sami. Microsoft
    /// przyjmuje go w parametrze `otc` (one-time code) na stronie
    /// `microsoft.com/link`. Gdyby kiedyś przestał, strona po prostu poprosi
    /// o kod jak dotąd — a kod nadal jest widoczny w oknie launchera.
    pub fn adres_do_otwarcia(&self) -> String {
        if let Some(gotowy) = &self.verification_uri_complete {
            if !gotowy.trim().is_empty() {
                return gotowy.clone();
            }
        }
        let rozdzielnik = if self.verification_uri.contains('?') {
            '&'
        } else {
            '?'
        };
        format!(
            "{}{}otc={}",
            self.verification_uri, rozdzielnik, self.user_code
        )
    }
}

/// `device_code` jest sekretem — kto go ma, ten odbierze token zamiast gracza.
/// `user_code` przeciwnie: gracz ma go przepisać, więc pokazujemy go wprost.
impl std::fmt::Debug for DeviceCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceCode")
            .field("user_code", &self.user_code)
            .field("device_code", &"<ukryty>")
            .field("verification_uri", &self.verification_uri)
            .field("interval_s", &self.interval_s)
            .field("expires_in_s", &self.expires_in_s)
            .finish()
    }
}

#[derive(Clone)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
}

/// Wypisanie tokenów zastąpione znacznikiem.
///
/// Wyprowadzony `Debug` wypisywał je w całości. Nikt tego dziś nie robi, ale
/// wystarczy jedno `{:?}` w komunikacie błędu, żeby żywy token gracza trafił
/// do `game.log` albo do raportu wklejanego na czacie — a takie linijki
/// dopisuje się w pośpiechu, przy szukaniu zupełnie innej usterki.
impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tokens")
            .field("access_token", &"<ukryty>")
            .field("refresh_token", &"<ukryty>")
            .finish()
    }
}

#[derive(Debug)]
pub enum PollResult {
    Czekamy,
    Zwolnij,
    Gotowe(Tokens),
}

/// Wszystkie zadania logowania to male JSON-y, wiec limit na cale zadanie
/// jest tu wlasciwy. Bez niego milczacy serwer Microsoftu albo filtrujacy
/// posrednik zostawial gracza z wygaszonym przyciskiem i kreciolkiem
/// „Czekam na potwierdzenie" bez konca.
fn klient() -> reqwest::Client {
    crate::limity::klient_maly()
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
    let tekst = odp
        .text()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;
    serde_json::from_str(&tekst).map_err(|_| AuthError::odmowa(bezpieczny_opis(&tekst)))
}

/// Zamienia odpowiedź serwera uwierzytelniania na opis, który wolno pokazać.
///
/// To jest zabezpieczenie, a nie kosmetyka. Treść błędu wędruje dalej jako
/// `AuthError::Odmowa`, stamtąd do pola `szczegoly`, a stamtąd pod przycisk
/// „Kopiuj szczegóły dla administracji" — czyli na czat, na oczy obcych.
/// Wkładanie tam surowej odpowiedzi z endpointu tokenów znaczyło, że przy
/// nieoczekiwanym kształcie odpowiedzi **żywy token gracza szedł w świat**.
///
/// Przepuszczamy więc wyłącznie pola `error` i `error_description` — te są po
/// to, żeby je czytać. Wszystko inne, łącznie z `access_token`, zostaje tutaj.
pub fn bezpieczny_opis(tekst: &str) -> String {
    const ILE_ZNAKOW: usize = 200;

    match serde_json::from_str::<serde_json::Value>(tekst) {
        Ok(v) => {
            let kod = v.get("error").and_then(|x| x.as_str());
            let opis = v.get("error_description").and_then(|x| x.as_str());
            match (kod, opis) {
                (None, None) => "odpowiedź w nieoczekiwanym kształcie".to_string(),
                (k, o) => skrot(
                    format!("{} {}", k.unwrap_or(""), o.unwrap_or("")).trim(),
                    ILE_ZNAKOW,
                ),
            }
        }
        // Odpowiedź, która nie jest JSON-em, to zwykle strona błędu serwera
        // pośredniczącego. Tokenu w niej nie ma, ale i tak ją tniemy — nie ma
        // powodu wklejać komuś na czat kilobajtów cudzego HTML-a.
        Err(_) => skrot(tekst.trim(), ILE_ZNAKOW),
    }
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
        // Bez tego pola opis błędu od Microsoftu ginął, zanim ktokolwiek go
        // zobaczył — a to w nim jest napisane, co właściwie poszło nie tak.
        error_description: Option<String>,
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

    let o: Odp =
        serde_json::from_str(&tekst).map_err(|_| AuthError::odmowa(bezpieczny_opis(&tekst)))?;

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
            None => Err(AuthError::Odmowa {
                powod: powod_odmowy(kod),
                szczegoly: opis_odmowy(kod, o.error_description.as_deref()),
            }),
        },
        None => Err(AuthError::odmowa(bezpieczny_opis(&tekst))),
    }
}

pub async fn refresh(client_id: &str, refresh_token: &str) -> Result<Tokens, AuthError> {
    // Pola są opcjonalne, bo ta sama odpowiedź niesie albo tokeny, albo błąd.
    // Wcześniej były wymagane, więc odmowa Microsoftu rozbijała się o parser
    // i szła dalej jako „nieoczekiwany kształt odpowiedzi" — czyli KONTO-05
    // z radą „uważnie przepisz kod", mimo że przy odświeżaniu żadnego kodu
    // gracz nie widział.
    #[derive(Deserialize)]
    struct Odp {
        access_token: Option<String>,
        refresh_token: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
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
    let o: Odp =
        serde_json::from_str(&tekst).map_err(|_| AuthError::odmowa(bezpieczny_opis(&tekst)))?;
    if let (Some(a), Some(r)) = (o.access_token, o.refresh_token) {
        return Ok(Tokens {
            access_token: a,
            refresh_token: r,
        });
    }
    match o.error.as_deref() {
        Some(kod) => Err(AuthError::Odmowa {
            powod: powod_odmowy_odswiezania(kod),
            szczegoly: opis_odmowy(kod, o.error_description.as_deref()),
        }),
        None => Err(AuthError::odmowa(bezpieczny_opis(&tekst))),
    }
}

pub fn opis_xerr(kod: u64) -> String {
    let nazwa = match powod_xerr(kod) {
        PowodXbox::KontoZablokowane => "konto zablokowane",
        PowodXbox::BrakProfiluXbox => "brak profilu Xbox",
        PowodXbox::RegionNiedostepny => "Xbox Live niedostępny w tym kraju",
        PowodXbox::WymaganaWeryfikacja => "konto wymaga potwierdzenia pełnoletności",
        PowodXbox::KontoDziecka => "konto dziecka spoza rodziny Microsoft",
        PowodXbox::Inny => "powód nieznany launcherowi",
    };
    format!("XErr {kod} ({nazwa})")
}

/// Skraca treść odpowiedzi do czegoś, co zmieści się w oknie błędu.
fn skrot(s: &str, ile: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= ile {
        return s.to_string();
    }
    let uciety: String = s.chars().take(ile).collect();
    format!("{uciety}…")
}

/// Czyta odpowiedź razem ze statusem HTTP i treścią.
///
/// Wcześniej każdy krok robił `.json()` od razu, więc gdy Microsoft oddał
/// cokolwiek nieoczekiwanego — stronę błędu, komunikat o przeciążeniu,
/// pustą treść — do okna gracza trafiało „error decoding response body".
/// Z takiego zdania ani gracz, ani administracja nie dowiadywali się niczego,
/// a każdy taki przypadek lądował pod jednym workiem KONTO-05.
async fn odczytaj(odp: reqwest::Response) -> Result<(reqwest::StatusCode, String), AuthError> {
    let status = odp.status();
    let tekst = odp
        .text()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;
    Ok((status, tekst))
}

/// Wyławia `XErr` z treści odpowiedzi. Xbox Live podaje go i przy 401,
/// i czasem przy innych statusach.
fn xerr_z_tresci(tekst: &str) -> Option<u64> {
    let v: serde_json::Value = serde_json::from_str(tekst).ok()?;
    match v.get("XErr")? {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Buduje błąd Xbox Live z tego, co naprawdę przyszło.
fn blad_xbox(etap: &str, status: reqwest::StatusCode, tekst: &str) -> AuthError {
    match xerr_z_tresci(tekst) {
        Some(kod) => AuthError::Xbox {
            powod: powod_xerr(kod),
            szczegoly: format!("{etap}: HTTP {status}, {}", opis_xerr(kod)),
        },
        None => AuthError::Xbox {
            powod: PowodXbox::Inny,
            szczegoly: format!("{etap}: HTTP {status}, treść: {}", skrot(tekst, 400)),
        },
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

    let odp = c
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com",
                            "RpsTicket": tokens.access_token },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;

    // Ten krok wcześniej w ogóle nie patrzył na status — a Xbox Live potrafi
    // odmówić już tutaj, razem z kodem XErr.
    let (status, tekst) = odczytaj(odp).await?;
    if !status.is_success() {
        return Err(blad_xbox("Xbox Live", status, &tekst));
    }
    let xbl: XblOdp = serde_json::from_str(&tekst).map_err(|e| AuthError::Xbox {
        powod: PowodXbox::Inny,
        szczegoly: format!(
            "Xbox Live: odpowiedź nie do odczytania ({e}), treść: {}",
            skrot(&tekst, 400)
        ),
    })?;

    let Some(uhs) = xbl.claims.xui.first().map(|x| x.uhs.clone()) else {
        // Bez identyfikatora użytkownika kolejny krok i tak by nie przeszedł,
        // a wcześniej szliśmy dalej z pustym ciągiem i błąd wychodził dopiero
        // przy Minecrafcie — w zupełnie innym miejscu, niż powstał.
        return Err(AuthError::Xbox {
            powod: PowodXbox::Inny,
            szczegoly: format!(
                "Xbox Live nie podał identyfikatora konta, treść: {}",
                skrot(&tekst, 400)
            ),
        });
    };

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

    let (status, tekst) = odczytaj(odp).await?;
    if !status.is_success() {
        return Err(blad_xbox("XSTS", status, &tekst));
    }

    #[derive(Deserialize)]
    struct XstsOdp {
        #[serde(rename = "Token")]
        token: String,
    }
    let xsts: XstsOdp = serde_json::from_str(&tekst).map_err(|e| AuthError::Xbox {
        powod: PowodXbox::Inny,
        szczegoly: format!(
            "XSTS: odpowiedź nie do odczytania ({e}), treść: {}",
            skrot(&tekst, 400)
        ),
    })?;

    // 3. Token Minecrafta.
    #[derive(Deserialize)]
    struct McOdp {
        access_token: String,
    }
    let odp = c
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({ "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token) }))
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;

    let (status, tekst) = odczytaj(odp).await?;
    if !status.is_success() {
        return Err(AuthError::odmowa(format!(
            "logowanie do Minecrafta: HTTP {status}, treść: {}",
            skrot(&tekst, 400)
        )));
    }
    let mc: McOdp = serde_json::from_str(&tekst).map_err(|e| {
        AuthError::odmowa(format!(
            "logowanie do Minecrafta: odpowiedź nie do odczytania ({e}), treść: {}",
            skrot(&tekst, 400)
        ))
    })?;

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
        .map_err(|e| AuthError::odmowa(e.to_string()))?;

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

    /// Zgloszenie od gracza: w szczegolach bledu widnialo samo `invalid_grant`
    /// i nic wiecej. Powod: struktura odpowiedzi nie odczytywala w ogole pola
    /// `error_description`, a sciezka nierozpoznanego kodu przekazywala dalej
    /// sam kod. Gracz dostawal rade „uwaznie przepisz kod", ktora przy tym
    /// bledzie jest nietrafiona.
    #[test]
    fn opis_bledu_od_microsoftu_dociera_do_szczegolow() {
        let s = opis_odmowy(
            "invalid_grant",
            Some("AADSTS70008: The provided authorization code has expired."),
        );
        assert!(s.contains("invalid_grant"), "{s}");
        assert!(
            s.contains("AADSTS70008"),
            "bez tego nie wiadomo, co sie stalo: {s}"
        );
        assert!(s.contains("expired"), "{s}");
    }

    /// Gdy Microsoft nie dal opisu, zostaje sam kod — i to tez jest w porzadku.
    #[test]
    fn bez_opisu_zostaje_sam_kod() {
        assert_eq!(opis_odmowy("invalid_grant", None), "invalid_grant");
        assert_eq!(opis_odmowy("invalid_grant", Some("   ")), "invalid_grant");
    }

    /// Powod odmowy rozpoznajemy po KODZIE, a nie po tresci komunikatu.
    /// Rozpoznawanie po tekscie raz juz sie w tym projekcie zemscilo.
    #[test]
    fn rozpoznajemy_powod_odmowy_po_kodzie() {
        assert_eq!(powod_odmowy("access_denied"), PowodOdmowy::Odrzucone);
        assert_eq!(powod_odmowy("invalid_grant"), PowodOdmowy::KodNiewazny);
        assert_eq!(powod_odmowy("cos_zupelnie_nowego"), PowodOdmowy::Inny);
    }

    /// Ten sam kod bledu, inne pytanie — inny wniosek. Przy odswiezaniu zadnego
    /// kodu urzadzenia nie bylo, wiec `invalid_grant` nie moze znaczyc
    /// „kod stracil waznosc", a `access_denied` nie moze znaczyc „gracz kliknal
    /// Nie w okienku" — okienka nie bylo.
    #[test]
    fn odswiezanie_tlumaczy_kody_inaczej_niz_kod_urzadzenia() {
        assert_eq!(powod_odmowy("invalid_grant"), PowodOdmowy::KodNiewazny);
        assert_eq!(
            powod_odmowy_odswiezania("invalid_grant"),
            PowodOdmowy::ZapisaneLogowanieWygaslo
        );

        assert_eq!(powod_odmowy("access_denied"), PowodOdmowy::Odrzucone);
        assert_eq!(
            powod_odmowy_odswiezania("access_denied"),
            PowodOdmowy::ZapisaneLogowanieWygaslo
        );

        assert_eq!(
            powod_odmowy_odswiezania("cos_zupelnie_nowego"),
            PowodOdmowy::Inny
        );
    }

    /// Tester dostawal „Kod stracil waznosc" przy odmowie, ktora przychodzila
    /// od razu po rozpoczeciu logowania, i ponawial dziesiec razy bez skutku.
    /// Kod urzadzenia zyje kilkanascie minut, wiec odmowa przed terminem nie
    /// moze byc wygasnieciem.
    #[test]
    fn odmowa_przed_terminem_to_nie_wygasniecie_kodu() {
        let powod = |b: &AuthError| match b {
            AuthError::Odmowa { powod, .. } => *powod,
            inny => panic!("spodziewano sie odmowy, jest {inny:?}"),
        };
        let odmowa = || AuthError::Odmowa {
            powod: PowodOdmowy::KodNiewazny,
            szczegoly: "invalid_grant: The user could not be authenticated".into(),
        };

        assert_eq!(
            powod(&doprecyzuj_odmowe(odmowa(), false)),
            PowodOdmowy::KontoWymagaDzialania,
            "kod byl jeszcze wazny — to nie wygasniecie"
        );
        assert_eq!(
            powod(&doprecyzuj_odmowe(odmowa(), true)),
            PowodOdmowy::KodNiewazny,
            "termin minal — wygasniecie jest tu wlasciwym wyjasnieniem"
        );
    }

    /// Doprecyzowanie dotyczy wylacznie `invalid_grant`. Odrzucenie zgody przez
    /// gracza znaczy to samo niezaleznie od tego, ile zostalo do terminu.
    #[test]
    fn doprecyzowanie_nie_rusza_pozostalych_odmow() {
        for powod in [PowodOdmowy::Odrzucone, PowodOdmowy::Inny] {
            let wynik = doprecyzuj_odmowe(
                AuthError::Odmowa {
                    powod,
                    szczegoly: "x".into(),
                },
                false,
            );
            match wynik {
                AuthError::Odmowa { powod: p, .. } => assert_eq!(p, powod),
                inny => panic!("spodziewano sie odmowy, jest {inny:?}"),
            }
        }

        // Wygasniecie kodu ma wlasny wariant bledu i nie jest odmowa —
        // doprecyzowanie musi je przepuscic nietkniete.
        assert!(matches!(
            doprecyzuj_odmowe(AuthError::Wygasl, false),
            AuthError::Wygasl
        ));
    }

    /// Dlugi opis od Microsoftu jest przycinany — potrafi miec kilka zdan
    /// i odsylacz do dokumentacji.
    #[test]
    fn dlugi_opis_odmowy_jest_przycinany() {
        let s = opis_odmowy("invalid_grant", Some(&"x".repeat(2000)));
        assert!(s.chars().count() <= 301, "{} znakow", s.chars().count());
    }

    /// Odpowiedz z endpointu tokenow, ktora nie ma ksztaltu, jakiego oczekujemy.
    /// Zawiera ZYWY token dostepu — dokladnie to, co wczesniej szlo w calosci
    /// do raportu wklejanego przez gracza na czat.
    const ODPOWIEDZ_Z_TOKENEM: &str = r#"{"token_type":"Bearer","expires_in":86400,
        "access_token":"EwAIA+pvBAAUKods6vX7RFshxTTnbfcpbCAAAZjAdkwIiJ",
        "scope":"XboxLive.signin"}"#;

    /// Najwazniejszy test w tym pliku.
    ///
    /// Cztery miejsca w module wkladaly surowa tresc odpowiedzi do
    /// `AuthError::Odmowa`, a ta wedruje do pola `szczegoly` i stamtad pod
    /// przycisk „Kopiuj szczegoly dla administracji" — czyli na czat.
    /// Przy nieoczekiwanym ksztalcie odpowiedzi szedl tam zywy token.
    #[test]
    fn tresc_odpowiedzi_nie_wynosi_tokenu() {
        let opis = bezpieczny_opis(ODPOWIEDZ_Z_TOKENEM);
        assert!(
            !opis.contains("EwAIA+pvBAAUKods6vX7RFshxTTnbfcpbCAAAZjAdkwIiJ"),
            "token wyszedl na zewnatrz: {opis}"
        );
        assert!(
            !opis.contains("access_token"),
            "nawet nazwa pola nie ma po co wychodzic: {opis}"
        );
    }

    /// Prawdziwy blad OAuth ma przejsc w calosci — po to sie go czyta.
    #[test]
    fn prawdziwy_blad_oauth_przechodzi() {
        let opis = bezpieczny_opis(
            r#"{"error":"invalid_grant","error_description":"The user has revoked access."}"#,
        );
        assert!(opis.contains("invalid_grant"), "{opis}");
        assert!(opis.contains("revoked"), "{opis}");
    }

    /// Odpowiedz, ktora nie jest JSON-em, to zwykle strona bledu posrednika.
    /// Tokenu w niej nie ma, ale nie ma tez powodu wklejac komus kilobajtow
    /// cudzego HTML-a.
    #[test]
    fn nie_json_jest_ucinany() {
        let dlugi = "<html>".to_string() + &"x".repeat(5000) + "</html>";
        let opis = bezpieczny_opis(&dlugi);
        assert!(
            opis.chars().count() <= 201,
            "za dlugie: {} znakow",
            opis.chars().count()
        );
    }

    /// Odpowiedz bez zadnego z pol bledu nie moze przepuscic reszty tresci.
    #[test]
    fn json_bez_pol_bledu_nie_wynosi_niczego() {
        let opis = bezpieczny_opis(r#"{"cos":"zupelnie innego","refresh_token":"M.C123_BAY"}"#);
        assert!(!opis.contains("M.C123_BAY"), "{opis}");
        assert!(!opis.contains("zupelnie innego"), "{opis}");
    }

    /// Wyprowadzony `Debug` wypisywal oba tokeny w calosci. Wystarczy jedno
    /// `{:?}` w komunikacie bledu, zeby trafily do logu albo do raportu.
    #[test]
    fn debug_nie_wypisuje_tokenow() {
        let t = Tokens {
            access_token: "TAJNY-DOSTEP".into(),
            refresh_token: "TAJNY-ODSWIEZ".into(),
        };
        let s = format!("{t:?}");
        assert!(!s.contains("TAJNY-DOSTEP"), "{s}");
        assert!(!s.contains("TAJNY-ODSWIEZ"), "{s}");
    }

    /// `device_code` jest sekretem: kto go ma, odbierze token zamiast gracza.
    /// `user_code` gracz ma przepisac, wiec zostaje widoczny.
    #[test]
    fn debug_ukrywa_device_code_ale_nie_user_code() {
        let k = DeviceCode {
            user_code: "V3REVW36".into(),
            device_code: "TAJNY-KOD-URZADZENIA".into(),
            verification_uri: "https://microsoft.com/link".into(),
            verification_uri_complete: None,
            interval_s: 5,
            expires_in_s: 900,
        };
        let s = format!("{k:?}");
        assert!(!s.contains("TAJNY-KOD-URZADZENIA"), "{s}");
        assert!(s.contains("V3REVW36"), "gracz ma go przepisac: {s}");
    }

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
        // Brak `verification_uri_complete` w odpowiedzi nie moze niczego zepsuc.
        assert!(d.verification_uri_complete.is_none());
    }

    fn kod_probny(uri: &str, gotowy: Option<&str>) -> DeviceCode {
        DeviceCode {
            user_code: "V3REVW36".into(),
            device_code: "tajny".into(),
            verification_uri: uri.into(),
            verification_uri_complete: gotowy.map(str::to_string),
            interval_s: 5,
            expires_in_s: 900,
        }
    }

    /// Przepisywanie kodu jest najwezszym gardlem logowania. Adres otwierany
    /// z launchera ma nosic kod w srodku, zeby gracz od razu trafial na wybor
    /// konta, a nie na strone z polem do wpisania.
    #[test]
    fn adres_do_otwarcia_niesie_kod() {
        assert_eq!(
            kod_probny("https://www.microsoft.com/link", None).adres_do_otwarcia(),
            "https://www.microsoft.com/link?otc=V3REVW36"
        );
        // Gdy adres ma juz parametry, doklejamy przez „&", nie przez drugie „?".
        assert_eq!(
            kod_probny("https://example.test/link?lang=pl", None).adres_do_otwarcia(),
            "https://example.test/link?lang=pl&otc=V3REVW36"
        );
    }

    /// Gdy Microsoft sam odda gotowy adres, uzywamy jego — nie sklejamy wlasnego.
    #[test]
    fn gotowy_adres_ma_pierwszenstwo() {
        let d = kod_probny(
            "https://www.microsoft.com/link",
            Some("https://www.microsoft.com/link?otc=INNY"),
        );
        assert_eq!(
            d.adres_do_otwarcia(),
            "https://www.microsoft.com/link?otc=INNY"
        );

        // Pusty napis to nie jest gotowy adres — wtedy skladamy sami.
        let pusty = kod_probny("https://www.microsoft.com/link", Some("   "));
        assert_eq!(
            pusty.adres_do_otwarcia(),
            "https://www.microsoft.com/link?otc=V3REVW36"
        );
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
        assert!(
            opis_xerr(2148916233).contains("konta Xbox") || opis_xerr(2148916233).contains("Xbox")
        );
        assert!(opis_xerr(2148916238).contains("dziecka"));
        assert!(opis_xerr(999).contains("999"));
    }

    /// Pieciu powodow odmowy Xbox Live rozpoznawalismy dwa. Reszta ladowala
    /// pod jednym workiem „Microsoft odmowil i nie podal powodu", choc kod
    /// odmowy przychodzil wprost w odpowiedzi.
    #[test]
    fn rozpoznajemy_wszystkie_znane_kody_odmowy() {
        assert_eq!(powod_xerr(2148916227), PowodXbox::KontoZablokowane);
        assert_eq!(powod_xerr(2148916233), PowodXbox::BrakProfiluXbox);
        assert_eq!(powod_xerr(2148916235), PowodXbox::RegionNiedostepny);
        assert_eq!(powod_xerr(2148916236), PowodXbox::WymaganaWeryfikacja);
        assert_eq!(powod_xerr(2148916237), PowodXbox::WymaganaWeryfikacja);
        assert_eq!(powod_xerr(2148916238), PowodXbox::KontoDziecka);
        assert_eq!(powod_xerr(1), PowodXbox::Inny);
    }

    /// Xbox Live podaje XErr raz jako liczbe, raz jako napis.
    #[test]
    fn xerr_czytamy_i_z_liczby_i_z_napisu() {
        assert_eq!(
            xerr_z_tresci(r#"{"XErr":2148916238,"Message":""}"#),
            Some(2148916238)
        );
        assert_eq!(xerr_z_tresci(r#"{"XErr":"2148916233"}"#), Some(2148916233));
        assert_eq!(xerr_z_tresci("<html>502 Bad Gateway</html>"), None);
        assert_eq!(xerr_z_tresci("{}"), None);
    }

    /// Odpowiedz bez XErr — na przyklad strona bledu posrednika — musi trafic
    /// do szczegolow w calosci na tyle, zeby dalo sie ja rozpoznac.
    #[test]
    fn nieznana_odpowiedz_zachowuje_status_i_tresc() {
        let e = blad_xbox(
            "XSTS",
            reqwest::StatusCode::BAD_GATEWAY,
            "<html>502 Bad Gateway</html>",
        );
        let AuthError::Xbox { powod, szczegoly } = e else {
            panic!("spodziewany blad Xbox");
        };
        assert_eq!(powod, PowodXbox::Inny);
        assert!(szczegoly.contains("502"), "{szczegoly}");
        assert!(szczegoly.contains("Bad Gateway"), "{szczegoly}");
        assert!(szczegoly.contains("XSTS"), "{szczegoly}");
    }

    #[test]
    fn znany_kod_odmowy_nie_gubi_sie_w_tresci() {
        let e = blad_xbox(
            "XSTS",
            reqwest::StatusCode::UNAUTHORIZED,
            r#"{"XErr":2148916235}"#,
        );
        let AuthError::Xbox { powod, .. } = e else {
            panic!("spodziewany blad Xbox");
        };
        assert_eq!(powod, PowodXbox::RegionNiedostepny);
    }

    /// Okno bledu ma swoja szerokosc — dluga odpowiedz trzeba przyciac,
    /// ale poczatek jest tym, co niesie informacje.
    #[test]
    fn dluga_tresc_jest_przycinana() {
        let dlugi = "x".repeat(1000);
        let s = skrot(&dlugi, 40);
        assert!(s.chars().count() <= 41, "{}", s.chars().count());
        assert!(s.ends_with('…'));
        assert_eq!(skrot("  krotki  ", 40), "krotki");
    }
}

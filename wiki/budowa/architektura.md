# Architektura

Projekt to jeden workspace Cargo z trzema crate'ami. `Cargo.toml` w korzeniu
wymienia je wprost:

```toml
[workspace]
members = ["crates/cli","crates/core", "crates/launcher"]
resolver = "2"

[workspace.package]
edition = "2021"
rust-version = "1.85"
```

Profil wydania jest w tym samym pliku: `opt-level = 3`, `lto = true`,
`codegen-units = 1`, `strip = true`. Wynikiem ma być jeden plik, który gracz
kopiuje i uruchamia, więc rozmiar i brak symboli mają znaczenie.

## Trzy crate'y

| Katalog | Pakiet | Co powstaje | Odpowiedzialność |
|---|---|---|---|
| `crates/core` | `chmurka-core` | biblioteka | Cała logika: manifest, pobieranie, Java, instalacja gry, synchronizacja paczki, konta, uruchamianie, samoaktualizacja |
| `crates/launcher` | `chmurkowy-launcher` | binarka `ChmurkowyLauncher` | Interfejs w egui — okno, ekrany, zasobnik systemowy. Cienka warstwa nad `core` |
| `crates/cli` | `chmurka-cli` | binarka `chmurka` | Narzędzie do budowania manifestu paczki i przejścia całego przebiegu bez okna |

Podział nie jest kosmetyczny. `crates/core/Cargo.toml` nie ma ani `egui`, ani
`eframe` — dzięki temu każdą decyzję logiki da się sprawdzić testem, który nie
otwiera okna, i tę samą logikę uruchomić z wiersza poleceń narzędziem
`chmurka`. Zależność idzie w jedną stronę: `launcher` i `cli` znają `core`,
`core` nie wie o istnieniu żadnego z nich.

Numer wersji launchera stoi w `crates/launcher/Cargo.toml` (dziś `0.4.24`).
To nie jest tylko metadana: kod czyta go przez `env!("CARGO_PKG_VERSION")`
i porównuje z wersją podaną w manifeście, żeby zdecydować o podmianie samego
siebie — patrz [Samoaktualizacja](samoaktualizacja.md).

## Moduły `crates/core`

`lib.rs` nie zawiera logiki, tylko listę modułów. Każdy moduł odpowiada za
jedną rzecz i większość z nich ma na początku pliku komentarz `//!`
tłumaczący, po co powstał.

| Moduł | Za co odpowiada |
|---|---|
| `aktualizacja` | Samoaktualizacja: sprawdzenie wersji z manifestu, pobranie nowego pliku, podmiana samego siebie i ponowne uruchomienie |
| `auth` | Konta gracza — wspólny typ `Account` i wartość `--userType` przekazywana grze (`msa` albo `legacy`) |
| `auth::msa` | Logowanie kontem Microsoft kodem urządzenia, przez Xbox Live, aż po token Minecrafta |
| `auth::offline` | Konto offline i odtworzenie javowego `UUID.nameUUIDFromBytes("OfflinePlayer:<nick>")` |
| `auth::store` | Lista zapamiętanych kont w `auth.json` i to, które z nich jest wybrane |
| `bledy` | Katalog błędów pisany dla rodzica: krótki kod do podyktowania przez telefon, zdanie bez żargonu i lista kroków |
| `game_install` | Instalacja gry: uruchomienie instalatora NeoForge, pobranie bibliotek i zasobów |
| `hash` | Liczenie SHA-512 i SHA-1 plików, kawałkami po 64 kB, żeby duży mod nie wjeżdżał w całości do pamięci |
| `java` | Pobranie i rozpakowanie własnej Javy z Adoptium dla bieżącego systemu |
| `javy` | Szukanie Javy już zainstalowanej na komputerze i sprawdzanie, czy wskazana ścieżka naprawdę nią jest |
| `komendy` | Własne komendy gracza: przed uruchomieniem, opakowująca i po zakończeniu gry |
| `konsola` | Podgląd ogona logu gry na żywo — jedyny sposób, żeby gracz sprawdził, czy paczka się jeszcze ładuje |
| `launch` | Złożenie wiersza poleceń, którym startuje gra: argumenty JVM, klasa główna, argumenty gry |
| `limity` | Górne granice czasu dla wszystkiego, co czeka na świat zewnętrzny, i gotowi klienci HTTP |
| `manifest` | Wczytanie i sprawdzenie `manifest.json`: wersja schematu, poprawność ścieżek, wymóg `https` |
| `miejsca` | Gdzie leżą dane launchera i jednorazowa przeprowadzka ze starego miejsca obok pliku programu |
| `narzedzia` | Zewnętrzne programy wymagane przez mody — `yt-dlp` i `ffmpeg` dla moda Create: Harmonics |
| `net` | Pobieranie plików: równoległość, ponawianie, adresy zapasowe, sprawdzenie skrótu każdego pliku |
| `odinstaluj` | Odinstalowanie launchera z poziomu samego launchera, z zasadą, że odmowa usunięcia danych nigdy nie kasuje świata |
| `pack_sync` | Plan synchronizacji paczki i jego wykonanie: co pobrać, co skasować, czego nie ruszać |
| `paczki` | Paczki zasobów i shadery — odczyt i zapis tego, co gra ma włączone, wprost w plikach gry |
| `pamiec` | Wykrywanie pamięci komputera i dobór rozsądnego przydziału dla gry |
| `paths` | Sprawdzanie ścieżek pochodzących z manifestu, żeby wpis z sieci nie nadpisał pliku poza katalogiem instancji |
| `progress` | Etapy pracy (`Stage`) i postęp pokazywany w interfejsie wraz z opisem po polsku |
| `skrot` | Dopisanie launchera do menu aplikacji systemu (na Windowsie robi to instalator, więc tam moduł nic nie robi) |
| `state` | Pamięć launchera o tym, które pliki sam wgrał — bez niej nie da się odróżnić zmiany gracza od zmiany w paczce |
| `ustawienia` | Ustawienia gracza zapisywane w `settings.json`, z układem pól zgodnym wstecz ze starszymi wersjami |
| `version` | Profile wersji Minecrafta: wczytanie, dziedziczenie, reguły systemu i podstawienia w argumentach |
| `zadomowienie` | Przeniesienie launchera przy pierwszym uruchomieniu tam, gdzie jego miejsce, zamiast zostawiania go w „Pobranych" |

Kilka z tych modułów ma nieoczywisty powód istnienia, wart zapamiętania przed
zmianą kodu:

- **`paths`** stoi między manifestem a dyskiem, bo manifest to dane z sieci.
  Bez `validate_rel` wpis `../../.bashrc` pozwoliłby nadpisać dowolny plik
  użytkownika.
- **`state`** istnieje wyłącznie po to, żeby odróżnić „gracz zmienił config"
  od „paczka się zaktualizowała". Od tego zależy, czy wolno nadpisać plik.
- **`limity`** zebrało wartości w jednym miejscu, bo `reqwest` domyślnie nie
  nakłada żadnego limitu czasu. Host, który przyjmował połączenie i milkł,
  zatrzymywał launcher na zawsze — a cała maszyneria odporności (trzy próby,
  adresy zapasowe) siedzi *za* tym oczekiwaniem, więc przy zawieszeniu nie
  uruchamiała się ani razu. Moduł daje dwóch klientów: `klient_maly` z limitem
  na całe żądanie (manifest, logowanie) i `klient_pobierania` bez limitu na
  całość, za to z limitem na ciszę — bo JRE waży 180 MB i na wolnym łączu
  uczciwie schodzi kilkanaście minut.

## Jak interfejs rozmawia z logiką

`egui` rysuje w trybie natychmiastowym: co klatkę cały ekran powstaje od nowa
z pól struktury `App`. Wątek rysujący nie może więc na nic czekać — sekunda
czekania to sekunda zamrożonego okna.

Stąd cały układ w `crates/launcher/src/app.rs`:

1. **Wątek rysujący.** `impl eframe::App for App` i jego metoda `update`.
   Wywołuje `self.odbierz()`, obsługuje polecenia okna, a potem rysuje jeden
   z ekranów.
2. **Zadania w tle.** `App` trzyma własny wielowątkowy runtime tokio.
   Zadania startuje metoda `w_tle`, która cicho odpuszcza, gdy launcher jest
   już w trakcie zamykania.
3. **Kanał wiadomości.** Zwykły `std::sync::mpsc`. Zadanie w tle dostaje
   sklonowany `Sender<Wiadomosc>`, a wątek rysujący czyta swój `Receiver`
   pętlą `try_recv`, dopóki coś w nim jest. Nigdy `recv` — to by zablokowało
   rysowanie.

Postęp instalacji idzie tą samą drogą: funkcje `core` przyjmują
`Arc<dyn Fn(Progress) + Send + Sync>`, a launcher podstawia pod to domknięcie,
które pakuje `Progress` w `Wiadomosc::Postep` i wysyła kanałem.

!!! uwaga

    `App` ma własną implementację `Drop`, która woła `shutdown_background`.
    Runtime tokio przy zwykłym porzuceniu czeka **bez limitu** na zadania
    blokujące, a jednym z nich jest pilnowanie procesu gry. Zamknięcie okna
    w trakcie rozgrywki zostawiało działający proces launchera: drugą ikonę
    w zasobniku i zajęty plik `.exe` na Windowsie, co miesza się z podmianą
    przy aktualizacji.

### Wiadomości

`enum Wiadomosc` to jedyna droga z tła do interfejsu. Wszystkie warianty
obsługuje `App::odbierz`.

| Wariant | Kto wysyła | Co robi interfejs |
|---|---|---|
| `Manifest` | pobranie manifestu przy starcie, z przycisku albo z pilnowania | Zapamiętuje manifest; przy starcie od razu aktualizuje launcher, przy pilnowaniu tylko proponuje |
| `Postep` | każdy etap instalacji i pobierania | Pokazuje pasek postępu z opisem etapu |
| `Notatka` | zadania w tle, gdy coś poszło nie tak, ale nie na tyle, żeby przerywać | Dopisuje wiersz do logu w oknie |
| `BladZKodem` | błąd, którego launcher nie naprawi sam | Przełącza na ekran błędu, odblokowuje przyciski i przywraca okno z zasobnika |
| `KodUrzadzenia` | logowanie Microsoft | Pokazuje kod do wpisania na stronie Microsoftu |
| `ZapamietajKonto` | udane logowanie albo odświeżenie tokenu | Dopisuje konto do listy, zapisuje `auth.json` i wraca na ekran główny |
| `GraWystartowala` | moment po `spawn` procesu gry | Przełącza na ekran konsoli, a gdy gracz tak ustawił — chowa okno |
| `GraZakonczona` | proces gry zakończył się normalnie | Odświeża stan paczek, wraca na ekran główny, ewentualnie zamyka launcher |
| `ZrestartujDo` | udana samoaktualizacja | Uruchamia nowy plik i kończy bieżący proces |
| `Wolny` | zadanie skończone bez wyniku do pokazania | Oddaje graczowi przyciski i zdejmuje pasek postępu |
| `JavaSprawdzona` | przycisk „Sprawdź" w zakładce Java | Pokazuje wersję Javy albo powód, dla którego ta ścieżka się nie nadaje |
| `JavyZnalezione` | przycisk „Wykryj" | Pokazuje listę Jav znalezionych na komputerze |
| `JavaWybrana` | systemowe okno wyboru pliku | Zapisuje ścieżkę i od razu ją sprawdza, żeby zły plik wyszedł na jaw tu, a nie po kliknięciu GRAJ |

Drugi, osobny kanał przychodzi z ikony w zasobniku i ma tylko dwa zdarzenia:
`Pokaz` i `Zakoncz`.

Rzeczy, które z pozoru mogłyby dziać się wprost, też idą w tło i wracają
wiadomością — bo każda z nich potrafi trwać:

- `java -version` na ścieżce wskazującej zawieszony zasób sieciowy,
- systemowe okno wyboru pliku (`rfd` w wariancie portalowym czeka na
  odpowiedź systemu po D-Bus),
- chodzenie po katalogach przy szukaniu Jav — to praca blokująca, więc idzie
  przez `spawn_blocking`, żeby nie zająć wątku, na którym stoi reszta zadań.

### Przerysowania

Bezczynne okno egui przestaje się przerysowywać, więc każdy odstęp trzeba
zamówić wprost przez `request_repaint_after`. Wartości dobrane pod to, co
akurat się dzieje:

| Sytuacja | Odstęp | Dlaczego tyle |
|---|---|---|
| Otwarty ekran konsoli | 400 ms | Przy sekundzie log doganiał okno z opóźnieniem i wyglądał na zamarły |
| Gra działa, konsola zamknięta | 1 s | Nic się nie zmienia aż do końca gry, a schowany launcher nie ma mielić procesora |
| Trwa robota, czeka kod logowania albo nie ma jeszcze manifestu | 120 ms | Pasek postępu i kręciołek mają się ruszać płynnie |
| Odliczanie do kolejnego sprawdzenia aktualizacji | do końca odstępu | Bez zamówionego przebudzenia nie byłoby komu zauważyć, że dziesięć minut minęło |

### Ekrany

`enum Widok` mówi, co jest na ekranie. `update` rozgałęzia się na nim jednym
`match`, a każdy wariant ma swój moduł w `crates/launcher/src/views/`.

| `Widok` | Moduł | Co pokazuje |
|---|---|---|
| `Glowny` | `views::main` | Przycisk GRAJ, wybrane konto, pasek postępu i log |
| `Logowanie` | `views::login` | Kod urządzenia do wpisania na stronie Microsoftu |
| `Konta` | `views::accounts` | Lista zapamiętanych kont, przełączanie i usuwanie |
| `Ustawienia` | `views::settings` | Zakładki Ogólne, Gra, Java, Zaawansowane |
| `Paczki` | `views::packs` | Paczki zasobów i shadery włączone w grze |
| `Blad` | `views::error` | Kod błędu, zdanie po polsku i kroki do wykonania |
| `Konsola` | `views::console` | Ogon logu gry na żywo |
| `Aktualizacja` | `views::update` | Przebieg podmiany launchera na nowszy |

Okno powstaje bez ramki systemowej (`with_decorations(false)`), więc pasek
tytułu rysuje sam launcher — `views::pasek_tytulu`. Obszar przeciągania jest
tam celowo skrócony o 100 px od prawej, inaczej przechwytywałby kliknięcia
w „zamknij" i „zminimalizuj".

### Zasobnik systemowy

Po starcie gry okno może zniknąć, a launcher zostaje w zasobniku — czeka na
zakończenie gry, żeby zebrać log i pokazać błąd, gdyby coś poszło nie tak.
Ikona powstaje w `App::nowa`, w kontekście runtime'u tokio i z limitem trzech
sekund: zamrożony host zasobnika (rozszerzenie GNOME, tray KDE) zatrzymywał
launcher **przed pierwszą klatką**, więc gracz klikał ikonę i nie dostawał
żadnego okna. Gdy ikony nie ma, launcher nie chowa okna, tylko je
minimalizuje — ukryte okno bez ikony byłoby nie do odzyskania.

### Samonaprawa przy uruchamianiu

Kliknięcie GRAJ nie wywołuje jednego przebiegu, tylko `odpal_z_samonaprawa`:
do trzech podejść, a między nimi launcher sam usuwa to, co zwykle bywa
przyczyną. `bledy::Samonaprawa` mówi, co zrobić — `Ponow` (chwila przerwy),
`JavaOdNowa` (kasuje katalog `java`), `GraOdNowa` (kasuje katalog `mc`) albo
`Nic`. Rada „wejdź w Ustawienia i kliknij Napraw instalację" jest poprawna,
ale nikt jej nie wykonuje.

!!! uwaga

    Od momentu, w którym proces gry ruszył, nie ponawiamy niczego. Pilnuje
    tego `AtomicBool` przekazywany do `przygotuj_i_odpal` — drugie okno
    Minecrafta byłoby gorsze od każdego błędu.

Szczegółowa kolejność kroków: [Od kliknięcia GRAJ do gry](przebieg-uruchomienia.md).

## Biblioteki zewnętrzne

| Biblioteka | Gdzie | Po co |
|---|---|---|
| `tokio` | core, launcher, cli | Runtime zadań w tle, procesy potomne, operacje na plikach, zegar |
| `reqwest` (rustls, bez domyślnych funkcji) | core, launcher, cli | HTTP; `stream` do pobierania plików kawałkami, `form` do logowania Microsoft |
| `serde`, `serde_json` | core, cli | Manifest, `settings.json`, `state.json`, `auth.json`, profile wersji Mojanga |
| `thiserror` | core, cli | Typy błędów z gotowym komunikatem (`NetError`, `ManifestError`, `BladLaunchera`) |
| `futures-util` | core | `StreamExt`: czytanie odpowiedzi kawałkami i `buffer_unordered` przy pobieraniu wielu plików naraz |
| `sha2`, `sha1` | core | Sprawdzanie skrótów: SHA-512 z manifestu paczki, SHA-1 z profili i bibliotek Mojanga |
| `md-5` | core | Wyłącznie UUID konta offline — Java liczy tam zwykłe MD5 z bajtów nicku |
| `zip`, `tar`, `flate2`, `lzma-rs` | core | Rozpakowywanie: Java (zip na Windowsie, `tar.gz` na Linuksie), narzędzia modów (`tar.xz`), zaglądanie do paczek zasobów |
| `sysinfo` (tylko `system`) | core | Odczyt ilości pamięci komputera pod dobór przydziału dla gry |
| `eframe`, `egui` | launcher | Okno i rysowanie interfejsu |
| `anyhow` | launcher, cli | Błędy w `main`, gdzie liczy się komunikat, a nie typ |
| `arboard` | launcher | Schowek — kopiowanie kodu logowania, treści błędu i logu gry |
| `open` | launcher | Otwarcie adresu albo katalogu programem systemowym (`that_detached`) |
| `rfd` (`xdg-portal`) | launcher | Okno wyboru pliku przy wskazywaniu własnej Javy |
| `ksni` | launcher, tylko Linux | Ikona w zasobniku po D-Bus |
| `tray-icon` | launcher, tylko Windows | Ikona w zasobniku |
| `clap` (`derive`) | cli | Rozbiór poleceń narzędzia `chmurka` |
| `tempfile` | core, testy | Katalogi tymczasowe, żeby testy nie dotykały prawdziwych danych |

Dobór bibliotek do zasobnika i okna wyboru pliku nie jest przypadkowy.
Na Linuksie `tray-icon` pociągnąłby GTK i libxdo, a `rfd` w domyślnym
wariancie dołożyłby gtk3 — do binarki, której cały sens polega na tym, że
jest jednym plikiem bez zależności. `ksni` i wariant `xdg-portal` gadają
wprost po D-Bus i nie dokładają ani jednej zależności C.

Dev-zależność `tokio` z funkcją `test-util` daje zegar, który da się
przewinąć. Bez niego test limitu czasu dla komendy gracza musiałby naprawdę
czekać dwie minuty.

## Budowanie i testy

Do zbudowania launchera na Linuksie potrzebne są biblioteki, z którymi
linkuje się `eframe`. `cargo test --workspace` buduje także launcher, więc bez
nich nie ruszą nawet testy. Lista jest ta sama, co w kontroli na GitHubie:

```bash
sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libasound2-dev
```

Codzienna praca:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p chmurkowy-launcher
cargo run -p chmurka-cli -- --help
```

Te same dwie komendy są wymaganymi kontrolami przed scaleniem pull requesta
— `-D warnings` znaczy, że każde ostrzeżenie przewraca kontrolę. Szczegóły:
[Kontrole i ochrona gałęzi](../projekt/kontrole.md).

Testy jednostkowe leżą w `mod tests` na końcu plików, których dotyczą, obok
kodu i jego komentarzy. Jedyny test integracyjny, `crates/core/tests/sync_integracja.rs`,
stawia własny serwer HTTP na `127.0.0.1` i przechodzi pełną synchronizację
paczki bez sieci.

!!! uwaga

    Adres manifestu jest **wkompilowany** w binarkę przez
    `crates/launcher/build.rs`, a nie czytany z pliku obok. Plik konfiguracyjny
    tester mógłby zepsuć albo zgubić, a launcher bez manifestu nie ma co robić.
    Budowanie bez podania zmiennej wstawia adres przykładowy — taka binarka
    uruchomi się, ale nie znajdzie żadnej paczki.

```bash
CHMURKA_MANIFEST_URL=https://twoj.adres/manifest.json cargo build --release -p chmurkowy-launcher
```

Przy wydaniu adres bierze się ze zmiennej repozytorium o tej samej nazwie —
patrz [Wydanie nowej wersji](../projekt/wydanie.md).

### Furtki do pracy nad wyglądem

Żeby obejrzeć ekran, do którego normalnie trzeba doklikać się przez logowanie
albo doczekać prawdziwego wydania, launcher czyta kilka zmiennych środowiskowych.
Działają tylko na starcie i nie zmieniają niczego w danych gracza.

| Zmienna | Efekt |
|---|---|
| `CHMURKA_WIDOK` | `logowanie`, `ustawienia`, `blad` albo `paczki` — ekran otwarty od razu po starcie (`blad` wypełnia się przykładowym błędem) |
| `CHMURKA_ZAKLADKA` | `gra`, `java` albo `zaawansowane` — otwiera zakładkę Ustawień tą samą drogą co kliknięcie, więc pokaże też okno ostrzeżenia |
| `CHMURKA_PROPOZYCJA` | Numer wersji, np. `0.9.9` — pokazuje okienko z propozycją aktualizacji, bez czekania dziesięciu minut na prawdziwe wydanie |
| `CHMURKA_ZRZUT` | Ścieżka do pliku `.ppm` — zapisuje zawartość okna po ośmiu klatkach i zamyka launcher |

`CHMURKA_ZRZUT` zapisuje **okno launchera**, a nie ekran: to jedyny sposób,
żeby obejrzeć układ bez wchodzenia komukolwiek na pulpit, i jedyny, który
działa tak samo pod Waylandem, pod X11 i na Windowsie. Format PPM wybrano
dlatego, że to dwie linijki nagłówka i surowe bajty — dokładanie biblioteki
do PNG-ów po to, żeby rozejrzeć się po własnym oknie, byłoby przesadą.

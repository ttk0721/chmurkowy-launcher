# Synchronizacja paczki

Synchronizacja doprowadza katalog paczki na dysku do stanu opisanego
w [manifeście](manifest.md): dociąga brakujące pliki, poprawia te, które się
rozjechały, i usuwa te, których w paczce być nie powinno. Całość siedzi
w `crates/core/src/pack_sync.rs`.

Robota dzieli się na dwa kroki, które nic o sobie nie wiedzą:

| Krok | Funkcja | Co robi |
| --- | --- | --- |
| Plan | `pack_sync::plan` | Porównuje manifest, dysk i `state.json`. Zwraca listę akcji. Niczego nie dotyka. |
| Zastosowanie | `pack_sync::apply` | Wykonuje listę akcji: kasuje, pobiera, zapisuje stan. |

Podział nie jest kosmetyczny. `plan` nie sięga do dysku bezpośrednio, tylko
przez cechę `FileProbe` z dwiema metodami — `hash_of` i `list`. Dzięki temu
testy podstawiają sondę trzymaną w pamięci (`Mapa` w testach modułu) i cała
logika decyzji daje się sprawdzić bez tworzenia jednego pliku. Na dysku
cechę realizuje `DiskProbe`.

## Gdzie to się dzieje

Ścieżki liczone są od katalogu danych launchera (patrz
[Gdzie leżą dane](dane.md)):

| Ścieżka | Zawartość |
| --- | --- |
| `instance/` | Katalog paczki: `mods`, `config`, `options.txt` i reszta |
| `state.json` | Pamięć launchera o tym, co sam wgrał |

Wywołanie w ścieżce przygotowania gry wygląda tak:

```rust
let sciezka_stanu = data.join("state.json");
let mut stan = state::State::load(&sciezka_stanu);
let akcje = pack_sync::plan(m, &stan, &pack_sync::DiskProbe::new(&instancja));
let notatki = pack_sync::apply(m, &instancja, &mut stan, &akcje, &dl, postep.clone()).await?;
stan.save(&sciezka_stanu).map_err(plik(&sciezka_stanu))?;
```

## Pamięć o tym, co launcher sam wgrał

`state.json` to jedna mapa: ścieżka względna → skrót SHA-512 pliku w chwili,
gdy launcher go zapisał (`crates/core/src/state.rs`).

Bez tej mapy nie da się odróżnić dwóch sytuacji, które na dysku wyglądają
identycznie: „gracz zmienił plik konfiguracyjny" i „paczka dostała nowszą
wersję tego pliku". W obu przypadkach skrót pliku różni się od skrótu
z manifestu. Dopiero porównanie z zapisanym skrótem mówi, czyja to zmiana.

Uszkodzony `state.json` albo jego brak nie jest błędem — `State::load`
zwraca wtedy pusty stan:

```rust
pub fn load(path: &Path) -> Self {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
```

Najgorsze, co się wtedy stanie, to jedno pominięcie aktualizacji pliku
konfiguracyjnego: przy pustym stanie plik niezgodny z manifestem trafia
w `SkipUnknown` i zostaje na dysku taki, jaki był. Przerwanie startu gry
z powodu nieczytelnego pliku pomocniczego byłoby kosztem znacznie większym.

## Polityki z manifestu

Każdy wpis w manifeście ma pole `policy`. To ono decyduje, jak mocno launcher
trzyma się swojej wersji pliku.

| Polityka | Zastosowanie | Kiedy pobiera |
| --- | --- | --- |
| `mirror` | Mody | Gdy pliku nie ma albo jego skrót nie zgadza się z manifestem |
| `smart` | Pliki konfiguracyjne | Gdy pliku nie ma albo jest, ale gracz go nie ruszał |
| `seed` | `options.txt` i podobne | Tylko gdy pliku nie ma |

`seed` nie patrzy nawet na skrót — sprawdza wyłącznie, czy plik istnieje.
Test `seed_wgrywa_tylko_raz` pilnuje właśnie tego: istniejący plik zostaje
nietknięty, nawet jeśli manifest ma inną treść. Tak ma być, bo `options.txt`
to ustawienia gracza, a nie treść paczki.

## Akcje

`plan` zwraca listę wariantów typu `Action`:

| Akcja | Znaczenie |
| --- | --- |
| `Download { index }` | Pobierz plik o tym numerze w `manifest.files` |
| `Delete { path }` | Usuń plik, którego nie ma w manifeście |
| `Record { path, sha512 }` | Plik jest już właściwy — tylko zapamiętaj go w `state.json` |
| `SkipModified { path }` | Gracz zmienił plik, zostawiamy jego wersję |
| `SkipUnknown { path }` | Plik istnieje, ale nie wiadomo, kto go wgrał — nie ruszamy |

### Tablica decyzji dla `smart`

Dla polityki `smart` decyzja zależy od trzech rzeczy naraz: czy plik jest na
dysku, czy jest w `state.json` i czy zgadza się z manifestem.

| Plik na dysku | Wpis w `state.json` | Skrót pliku a wpis | Akcja |
| --- | --- | --- | --- |
| brak | dowolnie | — | `Download` |
| jest | jest | zgodne, ale różne od manifestu | `Download` |
| jest | jest | zgodne i równe manifestowi | nic |
| jest | jest | różne | `SkipModified` |
| jest | brak | plik równy manifestowi | `Record` |
| jest | brak | plik różny od manifestu | `SkipUnknown` |

Sedno jest w wierszu „zgodne, ale różne od manifestu": skoro plik na dysku
wygląda dokładnie tak, jak launcher go zostawił, to znaczy, że gracz go nie
ruszał — i wolno go podmienić. Gdy plik różni się od tego, co launcher
zapisał (`SkipModified`), zmiana jest cudza i zostaje.

Dwa ostatnie wiersze dotyczą pliku bez wpisu w stanie, czyli takiego, który
wziął się z ręcznej instalacji albo z uszkodzonego `state.json`. `Record`
dopisuje go do pamięci, żeby przy następnej aktualizacji dało się już
normalnie policzyć, czy gracz go zmienił.

!!! wskazówka

    Zmiana w pliku objętym polityką `smart` przeżyje każdą aktualizację
    paczki. Jeśli chcesz wrócić do wersji z manifestu, wystarczy skasować
    swój plik — przy następnym starcie launcher wgra go od nowa.

## Co jest kasowane, a co zostaje

Kasowanie dotyczy wyłącznie katalogów wymienionych w manifeście jako
`mirror_dirs` (w praktyce `mods`):

```rust
for dir in &m.mirror_dirs {
    for sciezka in probe.list(dir) {
        if !znane.contains(&sciezka) {
            akcje.push(Action::Delete { path: sciezka });
        }
    }
}
```

`znane` to zbiór wszystkich ścieżek z manifestu, niezależnie od polityki.
Poza `mirror_dirs` launcher nie usuwa niczego — plik, którego nie ma
w manifeście, a leży w `config` albo w katalogu światów, po prostu zostaje.

!!! uwaga

    Mod dorzucony ręcznie do `mods` zniknie przy następnym uruchomieniu.
    Powód jest w komentarzu przy teście `mirror_kasuje_obcy_mod`: obcy mod
    w `mods` wywala całą paczkę przy starcie gry, a wtedy nie działa nic.
    Dlatego ten jeden katalog musi zgadzać się co do pliku.

Obchodzenie katalogu (funkcja `zbierz`) ma dwie osobliwości:

- Wpisy zaczynające się od kropki są pomijane. `.index` to metadane
  PrismLaunchera, nie nasza sprawa.
- Sprawdzenie, czy wpis jest katalogiem, idzie przez `wpis.file_type()`,
  które **nie** podąża za dowiązaniami. Bez tego dowiązanie wskazujące na
  swojego przodka zapętliłoby rekurencję aż do przepełnienia stosu, czyli
  do wywrócenia launchera.

## Zastosowanie planu

`apply` przechodzi listę akcji po kolei, ale nie wszystkie wykonuje na
miejscu:

- `Delete` — plik jest usuwany od razu, wpis znika ze stanu, do notatek
  trafia zdanie `usunięto obcy plik: {path}`.
- `Record` — wpis od razu ląduje w stanie.
- `SkipModified` i `SkipUnknown` — tylko notatka: `pomijam {path} — plik
  został zmieniony ręcznie` albo `pomijam {path} — nie wiadomo, skąd
  pochodzi`.
- `Download` — trafia na listę odłożoną na koniec.

Ponieważ `plan` dokłada kasowania na sam koniec listy, a `apply` wykonuje
kasowanie od razu i odkłada tylko pobieranie, w efekcie **wszystkie
usunięcia dzieją się przed pierwszym pobraniem**. Dopiero potem jedno
wywołanie `dl.fetch_many(...)` pobiera komplet plików.

Stan pobranych plików zapisuje się dopiero po powrocie z `fetch_many`:

```rust
// Stan zapisujemy dopiero po udanym pobraniu wszystkiego.
for (sciezka, hash, _) in do_pobrania {
    st.written.insert(sciezka, hash);
}
```

Gdyby wpisy powstawały przed pobraniem, przerwana synchronizacja zostawiłaby
w `state.json` deklarację „ten plik wgrałem ja" dla pliku, którego nigdy nie
było na dysku. Przy następnym uruchomieniu wyglądałoby to jak plik skasowany
przez gracza.

Notatki wracają do interfejsu jako `Wiadomosc::Notatka` i lądują w logu
launchera.

## Weryfikacja skrótów

Wszystko opiera się na SHA-512 liczonym z zawartości pliku
(`crates/core/src/hash.rs`). Plik czytany jest kawałkami po 64 kB, żeby
93-megabajtowy mod nie wjeżdżał w całości do pamięci.

Porównania skrótów idą przez `eq_ignore_ascii_case` — zapis szesnastkowy
w manifeście może być wielkimi literami i nie jest to błąd.

Skrót sprawdzany jest w trzech miejscach:

1. **Przed pobraniem**, w `plan` — decyduje, czy plik w ogóle wymaga roboty.
2. **Przed nawiązaniem połączenia**, w `Downloader::fetch_one`: funkcja
   `pasuje` sprawdza plik docelowy i przy zgodności zwraca `Ok(false)`, nie
   ruszając sieci. Test `pomija_plik_ktory_juz_jest_poprawny` podaje celowo
   nieosiągalny adres, żeby udowodnić, że launcher nawet nie zaczyna.
3. **Po pobraniu**, na pliku roboczym. Dopiero zgodny plik roboczy zostaje
   przemianowany na nazwę docelową.

Gdy pobrana treść nie zgadza się ze skrótem, próba kończy się komunikatem
`niezgodny hash pobranej treści z {url}`, plik roboczy jest kasowany i idzie
kolejna próba. Uszkodzony plik nigdy nie trafia pod nazwę docelową —
pilnuje tego test `zly_hash_konczy_sie_bledem_i_nie_zostawia_pliku`.

## Pobieranie: adresy zapasowe i ponowienia

Każdy wpis w manifeście ma listę adresów. Pętla w `fetch_one_obserwowane`
jest dwupoziomowa: każdy adres dostaje pełny komplet trzech prób (`PROBY`),
zanim launcher przejdzie do następnego.

Między próbami czeka `400 ms * 2^numer_próby`, czyli 400 ms i 800 ms; po
ostatniej próbie danego adresu przerwy już nie ma. Wyczerpanie wszystkich
adresów daje błąd `NetError::Wyczerpano` z nazwą pliku, listą adresów
i treścią ostatniego niepowodzenia.

Pobieranie idzie równolegle: `Downloader::new(8)` w ścieżce przygotowania
gry, a w `fetch_many` dodatkowo `buffer_unordered(16)`. Faktyczną
równoległość ogranicza semafor wewnątrz `fetch_one`.

### Plik roboczy

Pobranie leci najpierw do pliku roboczego obok celu. Nazwa powstaje przez
**dopisanie** `.part` do pełnej nazwy, a nie przez podmianę rozszerzenia:

```rust
fn plik_czesciowy(cel: &Path) -> PathBuf {
    let nazwa = cel
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pobieranie".to_string());
    cel.with_file_name(format!("{nazwa}.part"))
}
```

Kiedyś było tu `with_extension("part")`, które rozszerzenie podmienia. Dwa
pliki o tym samym trzonie dostawały przez to wspólny plik roboczy:
`chloride-client.toml_backup1` i `chloride-client.toml_backup2` walczyły oba
o `chloride-client.part`. Pobierają się równolegle, więc jedno zadanie
przenosiło plik na miejsce, a drugie chwilę później próbowało przenieść coś,
czego już nie było — gracz dostawał [błąd PLIK-03](../gracz/kody-bledow.md)
„No such file or directory".

## Wznawianie pobrania

Wznawianie działa na poziomie **całych plików**, nie bajtów. Launcher nie
prosi serwera o dalszy ciąg urwanej treści: po nieudanej próbie plik roboczy
jest kasowany, a kolejna próba zaczyna się od zera.

Sensu nabiera to przy spojrzeniu na całą synchronizację. Pobranie paczki
przerwane w połowie zostawia na dysku komplet plików, które zdążyły się
przenieść pod nazwę docelową, a każdy z nich ma już poprawny skrót. Przy
następnym uruchomieniu `plan` nie wystawia dla nich żadnej akcji, a te,
które i tak trafią do `fetch_one`, odpadają na sprawdzeniu `pasuje` bez
dotykania sieci. Koszt przerwanego pobierania to zawsze najwyżej jeden plik
liczony od nowa.

## Ścieżki z manifestu

Manifest to dane z sieci, więc każda ścieżka przechodzi przez `validate_rel`
z `crates/core/src/paths.rs`, zanim zostanie sklejona z katalogiem paczki
(`join_within`). Odrzucane są:

| Co | Dlaczego |
| --- | --- |
| pusta ścieżka | nie ma czego zapisać |
| ukośnik odwrotny `\` | na Windowsie bywa separatorem, więc mógłby ominąć kontrolę segmentów |
| ścieżka od `/` | ścieżka absolutna |
| litera dysku, np. `C:/` | ścieżka absolutna po windowsowemu |
| segment pusty albo `.` | zapis okrężny, nie ma powodu go przyjmować |
| segment `..` | wyjście poza katalog paczki |

Bez tego wpis `../../.bashrc` w manifeście pozwoliłby nadpisać dowolny plik
na dysku użytkownika. Błąd ścieżki wraca z `apply` jako `SyncError::Path`.

## Limity czasu

Wszystkie górne granice czasu leżą w jednym pliku,
`crates/core/src/limity.rs`, żeby nie rozjechały się między crate'ami.

Powód ich istnienia opisuje komentarz modułu: `reqwest` domyślnie nie nakłada
żadnego limitu, więc host, który przyjmował połączenie i milkł, zatrzymywał
launcher na zawsze. Pasek postępu stał w miejscu, przycisk GRAJ był
wygaszony, a jedynym wyjściem było zabicie procesu. Gorzej: cała maszyneria
odporności — trzy próby i zapasowe adresy — siedzi **za** tym oczekiwaniem,
więc przy zawieszeniu nie uruchamiała się ani razu.

| Stała | Wartość | Czego pilnuje | Dlaczego tyle |
| --- | --- | --- | --- |
| `POLACZENIE` | 10 s | Nawiązanie połączenia w obu klientach HTTP | Na tym etapie nie płyną jeszcze żadne dane, więc rozmiar pliku nie ma znaczenia |
| `MALE_ZADANIE` | 30 s | Całe żądanie w `klient_maly`: manifest, logowanie, odświeżenie tokenu | Przy kilku kilobajtach limit na całość jest właściwym narzędziem |
| `BRAK_POSTEPU` | 60 s | Przerwa między kawałkami pobieranego pliku oraz długość okna strażnika | Limit na całe żądanie urwałby uczciwe pobieranie na wolnym łączu |
| `NAJMNIEJ_BAJTOW_NA_OKNO` | 64 kB | Ile danych musi przyjść w oknie `BRAK_POSTEPU` | 64 kB na minutę to około kilobajta na sekundę — wolniej niż modem z lat dziewięćdziesiątych |
| `INSTALATOR` | 15 min | Instalator NeoForge | Mieli około minuty; kwadrans to zapas na wolny dysk i wolne łącze |
| `KOMENDA_GRACZA` | 2 min | Komenda wpisana w [Ustawieniach](../gracz/ustawienia.md), przed startem i po wyjściu | Dwie minuty starczą na kopię zapasową świata czy podmianę konfiguracji |
| `SPRAWDZENIE_JAVY` | 15 s | `java -version` | Zdrowa Java odpowiada w ułamku sekundy |

Dwa klienty HTTP różnią się właśnie doborem limitów:

```rust
pub fn klient_maly() -> reqwest::Client { /* connect_timeout + timeout */ }
pub fn klient_pobierania() -> reqwest::Client { /* connect_timeout + read_timeout */ }
```

Klient pobierania nie ma limitu na całość żądania. Komentarz przy
`BRAK_POSTEPU` mówi wprost dlaczego: JRE waży 180 MB i na wolnym łączu
uczciwie schodzi kilkanaście minut. Limit na przerwę przerywa martwe
połączenie, nie karząc powolnego. Relacje między wartościami pilnują testy:
`BRAK_POSTEPU >= MALE_ZADANIE`, `POLACZENIE` jest najkrótsze,
a `INSTALATOR > BRAK_POSTEPU * 10`.

Oba klienty przedstawiają się nagłówkiem `ChmurkowyLauncher/{wersja}`.

`KOMENDA_GRACZA` i `SPRAWDZENIE_JAVY` nie dotyczą sieci, ale leżą w tym samym
pliku, bo problem jest ten sam: launcher czeka na coś, na co nie ma wpływu.
Przy komendzie gracza limit jest nawet ważniejszy niż przy sieci, bo treść
wpisuje człowiek — literówka w skrypcie albo program czekający na wciśnięcie
klawisza zawiesiłby start gry na zawsze, a launcher nie ma jak pokazać
takiemu procesowi konsoli.

### Strażnik zatrzymanego pobierania

`read_timeout` pilnuje przerw **między odczytami** i na tym kończy się jego
użyteczność. Serwer, który sączy po kilka bajtów co kilkanaście sekund,
formalnie nigdy nie milczy: każdy odczyt mieści się w limicie, więc
połączenie żyje, a pobieranie stoi. Pobranie 180 MB trwałoby wtedy
miesiącami — z paskiem postępu stojącym w miejscu, bo meldunki idą co 400 kB.

Dlatego w pętli czytania strumienia (`Downloader::raz`) chodzi drugi
mechanizm: okno czasowe z licznikiem bajtów.

```rust
okno_bajty += kawalek.len() as u64;
if okno_start.elapsed() >= crate::limity::BRAK_POSTEPU {
    if okno_bajty < crate::limity::NAJMNIEJ_BAJTOW_NA_OKNO {
        return Err(format!(
            "pobieranie stoi w miejscu: {okno_bajty} B w ciągu {} s",
            crate::limity::BRAK_POSTEPU.as_secs()
        ));
    }
    okno_start = tokio::time::Instant::now();
    okno_bajty = 0;
}
```

Co minutę strażnik pyta, ile bajtów przyszło. Mniej niż 64 kB znaczy, że to
nie jest powolne łącze, tylko zepsute — i próba kończy się błędem.

Rzecz w tym, **czym** ten błąd jest. Strażnik nie przerywa pobierania paczki:
zwraca zwykły `Err` z funkcji `raz`, czyli trafia dokładnie w istniejącą
pętlę ponowień i listę zapasowych adresów. Zamiast stać w miejscu przy
jednym zepsutym serwerze, launcher sam przechodzi na następny adres.

## Co widzi gracz

Postęp synchronizacji melduje się jako `Stage::Pack` z licznikiem „plik
z ilu" i nazwą aktualnie pobieranego pliku. Notatki o pominięciach
i usunięciach trafiają do logu launchera.

Błędy zapisu mają własne kody i opisy — patrz
[Kody błędów](../gracz/kody-bledow.md). Największa część to `PLIK-01` (brak
miejsca), `PLIK-02` (brak prawa zapisu) i `PLIK-03` (plik zajęty przez inny
program).

Sama synchronizacja to jeden z etapów przygotowania gry; kolejność całości
opisuje [Od kliknięcia GRAJ do gry](przebieg-uruchomienia.md).

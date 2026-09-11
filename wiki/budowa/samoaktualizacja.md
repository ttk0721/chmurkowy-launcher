# Samoaktualizacja

Launcher podmienia sam siebie. Poprawki wychodzą czasem po kilka na godzinę
i nie da się prosić dziesięciolatka, żeby pobierał plik z GitHuba — ma
usłyszeć najwyżej „zamknij i odpal ponownie".

Decyzje i podmiana pliku leżą w `crates/core/src/aktualizacja.rs`. Moment,
w którym to się dzieje, oraz rozmowa z graczem — w `crates/launcher/src/app.rs`
i `crates/launcher/src/views/update.rs`.

## Skąd launcher wie, że jest nowsza wersja

Z [manifestu](manifest.md), z sekcji `launcher`:

| Pole | Zawartość |
| --- | --- |
| `latest_version` | Numer najnowszego wydania |
| `urls` | Mapa: klucz systemu → adres pliku wykonywalnego |

Klucz systemu ustala `klucz_systemu()`: `windows-x64` na Windowsie,
`linux-x64` w pozostałych przypadkach. Wersję bieżącą launcher bierze
z własnego `Cargo.toml` przez `env!("CARGO_PKG_VERSION")` — nie ma osobnego
miejsca, w którym numer mógłby się rozjechać.

### Porównanie wersji

```rust
pub fn nowsza(biezaca: &str, zdalna: &str) -> bool {
    match (numery(biezaca), numery(zdalna)) {
        (Some(a), Some(b)) => b > a,
        _ => biezaca.trim() != zdalna.trim(),
    }
}
```

`numery` rozbija `0.4.5` na trzy liczby i wymaga dokładnie trzech członów.
Gdy obie wersje mają ten kształt, porównanie jest liczbowe. Gdy któraś nie
ma — decyduje zwykła różnica tekstu, bo „coś się zmieniło" jest tu lepszą
odpowiedzią niż cisza: jedna aktualizacja za dużo szkodzi mniej niż
przegapiona poprawka.

| Bieżąca | Z manifestu | Wynik | Dlaczego |
| --- | --- | --- | --- |
| `0.4.9` | `0.4.10` | nowsza | Porównanie tekstowe uznałoby `0.4.9` za nowsze i gracz utknąłby na przedostatniej poprawce |
| `0.4.17` | `0.4.14` | nie | Cofanie się wstecz nie jest aktualizacją |
| `0.4.5` | `0.4.5` | nie | — |
| `0.4.5` | `0.4.6-rc1` | nowsza | Kształt nieznany, więc liczy się sama różnica |

Drugi wiersz to poprawka po konkretnej wpadce: ekran Ustawień porównywał
wersje zwykłym „różne od" i przy manifeście ogłoszonym chwilowo na starszą
wersję pisał graczowi „dostępna nowsza: 0.4.14", mając 0.4.17.

## Decyzja

`zdecyduj(biezaca, najnowsza, url, data)` odpowiada na pytanie „czy launcher
ma się teraz podmienić", nie dotykając sieci. Zwraca jeden z czterech
wariantów:

| Decyzja | Kiedy | Co robi launcher |
| --- | --- | --- |
| `Aktualna` | Manifest nie podaje nowszej wersji | Przy kliknięciu gracza: komunikat `Masz już najnowszą wersję (…)`. Przy sprawdzeniu automatycznym — milczy, bo to stan normalny przy każdym starcie |
| `Nowsza { wersja, url }` | Jest nowsza i jest plik dla tego systemu | Przechodzi na ekran aktualizacji i pobiera |
| `BrakAdresu` | Jest nowsza, ale `urls` nie ma klucza tego systemu | Notatka do logu: `Jest launcher {wersja}, ale manifest nie podaje pliku dla tego systemu.` |
| `JuzProbowano(wersja)` | Ta wersja już raz nie wskoczyła | Automatycznie: notatka `Aktualizacja do {w} raz się nie powiodła — zostaję przy {biezaca}.` Ręcznie: mimo wszystko próbuje |

Ręczne kliknięcie „Zaktualizuj teraz" celowo omija znacznik nieudanej próby —
skoro gracz prosi wprost, to nie launchera rzecz mu odmawiać, bo poprzednim
razem coś nie wyszło.

Cała funkcja `zaktualizuj` wychodzi też od razu, gdy launcher jest zajęty:
podmiana pliku spod działającego procesu to ostatnia rzecz, jakiej wtedy
trzeba.

## Znacznik nieudanej próby

Znacznik to plik `aktualizacja.txt` w katalogu danych launchera (patrz
[Gdzie leżą dane](dane.md)). Treść to numer wersji i czas zapisu w sekundach
od epoki, rozdzielone spacją.

Zapisuje się go **przed** podmianą, nie po niej:

```rust
// Znacznik stawiamy PRZED podmianą. Gdyby nowa wersja okazała się nie
// działać, kolejny start zobaczy, że raz już nie wskoczyła, i nie będzie
// podmieniał się w kółko — gracz wejdzie do gry na starej.
zapisz_znacznik(data, wersja);
```

Bez tego nowa wersja, która się nie uruchamia, dałaby pętlę: stary launcher
pobiera nową, uruchamia ją, ona pada, stary startuje znowu i pobiera znowu.
Gracz nigdy nie doszedłby do przycisku GRAJ.

### Wygasanie

| Stan znacznika | Skutek |
| --- | --- |
| Świeży, dotyczy tej samej wersji co manifest | `JuzProbowano` — automat odpuszcza |
| Starszy niż 30 minut | Traktowany tak, jakby go nie było |
| Dotyczy innej wersji niż manifest | Nie blokuje niczego |
| Bez czasu (zapisany przez starszą wersję launchera) | Nie blokuje niczego |
| Manifest przestał podawać nowszą wersję | Znacznik jest kasowany |

Ważność wynosi 30 minut (`WAZNOSC_ZNACZNIKA_S`). Znacznik ma chronić przed
kręceniem się w kółko, a nie zamykać drogę do poprawki na zawsze. Najczęstsza
przyczyna nieudanej próby to manifest, który ogłosił wersję, zanim wydanie
skończyło się budować — kwadrans później plik zwykle już jest i warto
spróbować ponownie.

Kasowanie znacznika dzieje się w `zdecyduj`, w gałęzi `Aktualna`: skoro
wersja z manifestu wskoczyła (albo launcher wyprzedził manifest), znacznik
nie jest już do niczego potrzebny. Gdyby został na dysku, zablokowałby powrót
do tej wersji po naprawie.

Znacznik bez czasu pochodzi ze starszej wersji launchera. Nie ma jak
stwierdzić, czy jest świeży, więc nie blokuje niczego.

## Pobranie i sprawdzenie pliku

`pobierz_i_podmien` pracuje na dwóch plikach roboczych leżących obok
launchera: `<nazwa>.nowy` i `<nazwa>.stary`.

Zanim cokolwiek się pobierze, `.nowy` jest kasowany. Powód jest w kodzie:
pobieranie idzie z `Expect::Any`, które przepuściłoby plik leżący po
poprzedniej, przerwanej próbie.

`Expect::Any` znaczy „nie sprawdzaj skrótu" i nie jest to przeoczenie.
Manifest nie podaje skrótu binarki, bo powstaje ona w CI dopiero po
zbudowaniu manifestu — nie ma go skąd wziąć. Zaufanie opiera się na HTTPS do
GitHuba, a lokalnie sprawdzane jest tylko to, czy pobrany plik w ogóle jest
programem:

| Kontrola | Wartość | Co wyłapuje |
| --- | --- | --- |
| Rozmiar | co najmniej 2 MiB | Stronę błędu albo urwane pobieranie — binarka waży kilkanaście megabajtów |
| Początek pliku | `MZ` na Windowsie, `\x7fELF` poza nim | Plik odpowiednio duży, ale niebędący programem dla tego systemu |

Nieudana kontrola daje `BladAktualizacji::NiePlikWykonywalny` i podmiana się
nie odbywa.

Potem na Linuksie (i każdym innym Uniksie) plik dostaje prawa `0755`.
Pobrany plik nie dziedziczy prawa uruchamiania po niczym, więc bez tego kroku
byłby bezużyteczny. Na Windowsie funkcja nic nie robi.

Postęp melduje się jako `Stage::Loader` z etykietą `Pobieram launcher
{wersja}`, a po sprawdzeniu pliku — `Podmieniam launcher…`.

## Podmiana pliku

Podmiana działa tak samo na obu systemach i opiera się na jednej własności:
pliku, który właśnie się wykonuje, nie da się nadpisać, ale **wolno go
przemianować**.

```rust
let _ = std::fs::remove_file(&stary);
std::fs::rename(&exe, &stary)?;
if let Err(e) = std::fs::rename(&nowy, &exe) {
    // Wracamy do stanu sprzed próby — stara wersja jest lepsza niż żadna.
    let _ = std::fs::rename(&stary, &exe);
    return Err(BladAktualizacji::Plik(exe.display().to_string(), e));
}
```

Kolejność: stary plik odsuwany jest na bok pod nazwę `.stary`, nowy wstawiany
pod nazwę, spod której launcher wystartował. Gdy drugie przemianowanie się
nie uda, pierwsze jest cofane — stara wersja wraca na swoje miejsce.

Plik `.stary` znika przy następnym starcie. Sprzątanie
(`posprzataj_po_podmianie`) siedzi w konstruktorze aplikacji właśnie
dlatego: dopiero wtedy poprzednia wersja na pewno nie jest już uruchomiona.

Różnice między systemami są poza samą podmianą:

| Rzecz | Windows | Linux |
| --- | --- | --- |
| Klucz w `launcher.urls` | `windows-x64` | `linux-x64` |
| Nagłówek sprawdzanego pliku | `MZ` | `\x7fELF` |
| Prawo uruchamiania | nie dotyczy | `chmod 0755` na nowym pliku |
| Wpis w menu systemu | zakłada instalator | launcher sam, patrz niżej |

## Ponowne uruchomienie

`pobierz_i_podmien` zwraca ścieżkę gotowego pliku i na tym kończy swoją
rolę. Uruchomienie go zostaje po stronie interfejsu, bo tylko on wie, czy
właśnie nie dzieje się coś, czego nie wolno przerwać.

Interfejs dostaje `Wiadomosc::ZrestartujDo(exe)` i woła `uruchom_ponownie`:

```rust
let mut cmd = std::process::Command::new(exe);
if let Some(katalog) = exe.parent() {
    cmd.current_dir(katalog);
}
cmd.spawn()?;
```

Nowy proces dostaje katalog launchera jako katalog roboczy, żeby nie
odziedziczył tego, w którym akurat stał proces stary. Zakończenie bieżącego
procesu należy do wołającego — inaczej przez chwilę biegłyby dwa okna;
interfejs ustawia wtedy `zakoncz = true`.

Gdy uruchomienie się nie powiedzie, launcher nie zostawia gracza na ekranie
aktualizacji: oddaje przyciski i wpisuje do logu `Nowy launcher jest już na
dysku, ale nie dał się uruchomić (…). Zamknij i odpal launcher ponownie.`

Przez cały czas podmiany widoczny jest osobny ekran (`Widok::Aktualizacja`)
z paskiem postępu i zdaniem „Za chwilę launcher zamknie się i otworzy sam
w nowej wersji. Nic nie musisz robić." Bez niego cała aktualizacja wyglądała
tak, że okno znika i po chwili wraca — dla dorosłego zagadka, dla dziecka
powód, żeby zawołać rodzica.

Podobnie nieudana aktualizacja kończy się notatką `Nie udało się
zaktualizować launchera (…). Gram na tej wersji.` i powrotem na ekran główny —
także wtedy, gdy aktualizację uruchomiono z Ustawień, bo obsługa wiadomości
`Wolny` przestawia widok z ekranu aktualizacji zawsze na główny. Nieudana aktualizacja nie może być powodem, dla którego ktoś nie
zagra.

## Pilnowanie co 10 minut

Przy starcie launcher pobiera manifest i, jeśli trzeba, podmienia się sam,
nikogo o nic nie pytając. To za mało: kto zostawia launcher otwarty na całe
popołudnie, dowiadywał się o poprawce dopiero następnego dnia.

Dlatego `przypilnuj_aktualizacji` wołane jest z każdej klatki rysowania
i raz na `ODSTEP_PILNOWANIA`, czyli 10 minut, pyta serwer o manifest.
Dziesięć minut to kilka kilobajtów manifestu na godzinę; krócej nie ma po co,
bo wydanie i tak buduje się dłużej.

```rust
fn przypilnuj_aktualizacji(&mut self, ctx: &egui::Context) {
    if self.gra_dziala || self.zajety {
        return;
    }
    let minelo = self.ostatnie_pilnowanie.map(|t| t.elapsed());
    match chmurka_core::aktualizacja::do_nastepnego_pilnowania(minelo) {
        None => { /* pora sprawdzić */ }
        Some(za) => ctx.request_repaint_after(za),
    }
}
```

Trzy rzeczy warte uwagi:

- **Zamówione przebudzenie.** Bezczynne okno egui przestaje się
  przerysowywać, więc bez `request_repaint_after` nie byłoby komu zauważyć,
  że dziesięć minut minęło. `do_nastepnego_pilnowania` zwraca właśnie tę
  wartość.
- **Licznik startuje przy uruchomieniu launchera.** `ostatnie_pilnowanie`
  dostaje `Instant::now()` już w konstruktorze, więc pierwsze samodzielne
  sprawdzenie wypada dziesięć minut po starcie — start i tak pobiera
  manifest osobno.
- **Po grze sprawdzenie dzieje się samo.** Gdy gra trwała dłużej niż
  dziesięć minut, odstęp jest po jej zakończeniu dawno przekroczony;
  `do_nastepnego_pilnowania` zwraca wtedy `None`, a nie ujemny czas.

Sprawdzenie „o które nikt nie prosił" jest oznaczone (`pilnowanie_w_toku`)
i traktowane inaczej, gdy sieć nie odpowiada: zamiast ekranu błędu do logu
idzie notatka `Nie udało się sprawdzić aktualizacji (…). Spróbuję później.`
Gracz o nic nie prosił, a wyrzucenie go na ekran błędu w środku zabawy byłoby
karą za pomysł launchera — co dziesięć minut, przy zerwanym Wi-Fi, karą
dotkliwą.

Ta sama flaga decyduje o tym, co się stanie po odebraniu manifestu:

| Skąd manifest | Zachowanie |
| --- | --- |
| Start launchera albo „Sprawdź ponownie" | `zaktualizuj_sie()` — podmiana bez pytania |
| Samodzielne sprawdzenie co 10 minut | `rozwaz_propozycje()` — pytanie w okienku |

Różnica bierze się stąd, że przy starcie nikt nie jest w połowie niczego,
a launcher, który już chodzi, może komuś zamknąć okno pod rękami.

## Okienko z propozycją

O tym, czy jest o czym mówić, decyduje osobna funkcja:

```rust
pub fn warto_zaproponowac(
    biezaca: &str,
    najnowsza: &str,
    jest_plik: bool,
    odlozona: Option<&str>,
) -> bool {
    jest_plik && nowsza(biezaca, najnowsza) && odlozona != Some(najnowsza)
}
```

Jest osobna od `zdecyduj`, bo to inne pytanie. `zdecyduj` odpowiada „czy
launcher ma się teraz podmienić", więc pilnuje też znacznika nieudanych prób.
Tutaj chodzi o to, czy jest o czym **mówić** — a o wersji, która raz nie
wskoczyła, warto powiedzieć: gracz może spróbować ręcznie i zobaczyć, co się
dzieje.

`jest_plik` sprawdza, czy manifest w ogóle podaje plik dla tego systemu.
Okienko z przyciskiem, który nic nie zrobi, jest gorsze niż cisza.

### Kiedy okienko się pokazuje

Poza samą propozycją musi być spełnione `pora_na_propozycje()`:

```rust
self.proponowana_wersja.is_some()
    && !self.gra_dziala
    && !self.zajety
    && !matches!(self.widok, Widok::Aktualizacja | Widok::Blad)
```

### Kiedy launcher milczy

| Sytuacja | Dlaczego |
| --- | --- |
| Trwa gra | Launcher siedzi wtedy schowany w zasobniku, więc okienko nie miałoby się gdzie pokazać. Podmiana pliku spod działającej gry to osobny kłopot. W trakcie gry launcher nie zaczepia nawet serwera |
| Launcher jest zajęty | Coś się instaluje albo pobiera i nie należy tego przerywać |
| Ekran błędu | Gracz czyta, co się stało |
| Ekran aktualizacji | Jest już w trakcie aktualizowania |
| Manifest nie podaje pliku dla tego systemu | Przycisk nie miałby czego zrobić |
| Gracz kliknął „Nie teraz" na tej wersji | Odłożenie obowiązuje do końca sesji |

Odłożenie dotyczy **jednego numeru wersji**. Kolejna, nowsza wersja to inne
pytanie i wolno je zadać — pilnuje tego test
`odlozona_wersja_nie_wraca_a_kolejna_owszem`.

Krzyżyk w rogu okienka znaczy dokładnie to samo, co „Nie teraz". Inaczej
okienko wracałoby za dziesięć minut do kogoś, kto właśnie je zamknął.

Okienko rysuje się po narysowaniu widoku, żeby nie schowało się pod panelami.
W treści stoi, że aktualizacja trwa kilkanaście sekund, że światy, paczka
modów i ustawienia zostają nietknięte, a pod przyciskami — „Przypomnę przy
następnym uruchomieniu launchera."

## „Sprawdź ponownie" w Ustawieniach

Przycisk w [Ustawieniach](../gracz/ustawienia.md) nie pyta serwera za każdym
razem. `warto_sprawdzic` przepuszcza żądanie dopiero po `ODSTEP_SPRAWDZANIA`,
czyli 30 sekundach; wcześniej launcher odpowiada tym, co już wie:
`Sprawdzano przed chwilą — masz najnowszą wersję (…)`.

Powód jest praktyczny: przycisk bywa klikany po kilka razy pod rząd,
zwłaszcza gdy ktoś czeka na poprawkę, a odpowiedź i tak byłaby ta sama.

## Zadomowienie

Wydanie zawiera instalator i gołą binarkę. Ta druga jest po to, żeby launcher
mógł podmienić sam siebie przy aktualizacji — ale na liście wydań wygląda
zachęcająco i to ją ludzie pobierają. Klikają, program działa, tylko zostaje
jednym plikiem w „Pobranych": nie ma go w menu, nie da się go znaleźć
wyszukiwarką, a przy każdej aktualizacji dochodzi kolejna kopia z numerkiem
w nazwie.

Zamiast tłumaczyć, że „trzeba było wziąć to drugie", launcher przenosi się
tam, gdzie jego miejsce, i dopisuje do menu. Raz, po cichu. Kod:
`crates/core/src/zadomowienie.rs` oraz `crates/core/src/skrot.rs`, wołane
z `main()` **przed** otwarciem okna.

### Przeniesienie binarki

Miejsce docelowe to katalog użytkownika launchera plus nazwa pliku:
`ChmurkowyLauncher.exe` na Windowsie, `ChmurkowyLauncher` poza nim. Obok
leży katalog z danymi, więc program i jego dane trzymają się razem,
a odinstalowanie sprowadza się do usunięcia jednego drzewa (patrz
[Gdzie leżą dane](dane.md)).

Sprawdzenie „czy jesteśmy na miejscu" porównuje ścieżki **po skanonizowaniu**:

```rust
pub fn poza_miejscem(exe: &Path, docelowy: &Path) -> bool {
    let kanoniczny = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    kanoniczny(exe) != kanoniczny(docelowy)
}
```

Bez kanonizacji dowiązanie albo uruchomienie przez `./plik` wyglądałoby na
coś innego niż plik docelowy i launcher instalowałby się w kółko, przy każdym
starcie.

Sama przeprowadzka to skasowanie pliku pod nazwą docelową, kopia i nadanie
praw `0755`. Kasowanie i wstawianie zamiast nadpisywania bierze się stąd, że
pliku, który się wykonuje, nie da się nadpisać na Windowsie, a na Linuksie
choć się da, lepiej tego nie robić.

!!! uwaga

    Plik źródłowy zostaje nietknięty. Gracz sam go pobrał i to jego rzecz, co
    z nim zrobi — kasowanie cudzych plików z „Pobranych" bez pytania byłoby
    przekroczeniem uprawnień. Do logu launchera idzie za to zdanie
    „Launcher zainstalował się w … i dopisał do menu aplikacji. Plik, który
    pobrałeś, możesz skasować."

Wynik przeprowadzki ma trzy warianty:

| Wynik | Co dalej |
| --- | --- |
| `JuzNaMiejscu` | Nic, launcher tylko odświeża wpis w menu |
| `Zainstalowany(sciezka)` | Dopisanie do menu, uruchomienie pliku spod nowej ścieżki i zakończenie siebie |
| `NieUdalo(powod)` | Launcher działa dalej stamtąd, skąd go uruchomiono; powód ląduje w logu |

Uruchomienie z docelowego miejsca nie jest kosmetyką: dzięki temu
samoaktualizacja podmieni właściwy plik, a nie ten leżący w „Pobranych".
Robi to ta sama funkcja `uruchom_ponownie`, której używa podmiana wersji.

### Wpis w menu systemu

Na Windowsie `skrot::zarejestruj` zwraca `Stan::NieDotyczy` i nie robi nic:
skróty w menu Start zakłada instalator, bo on jeden wie, co po sobie
posprzątać przy odinstalowaniu.

Poza Windowsem launcher zapisuje dwa pliki w katalogu danych współdzielonych
użytkownika (`$XDG_DATA_HOME`, a bez niego `~/.local/share`):

```text
applications/chmurkowy-launcher.desktop
icons/hicolor/256x256/apps/chmurkowy-launcher.png
```

Treść wpisu układa `zaplanuj` i nic przy tym nie zapisuje — dzięki temu da
się ją sprawdzić testem bez dotykania prawdziwego katalogu domowego. Pole
`Exec` jest bezwzględne i w cudzysłowie, bo ścieżka bywa ze spacją, a katalog
„Pobrane (1)" to nie teoria, tylko codzienność.

Zapis jest pomijany, gdy plik `.desktop` ma już dokładnie tę treść, a ikona
istnieje. Rejestracja wołana jest mimo to **przy każdym starcie** — gracz mógł
skasować wpis, a system zgubić go przy aktualizacji środowiska.

### Odświeżenie menu

Po zapisie launcher prosi pulpit o przebudowę dwóch **różnych** baz:

| Wywołanie | Czego dotyczy |
| --- | --- |
| `update-desktop-database` na katalogu `applications` | Spis „który program otwiera jaki plik" |
| `kbuildsycoca6`, a gdy go nie ma — `kbuildsycoca5` | Drzewo kategorii w działającym pulpicie |

Przez długi czas odświeżana była tylko jedna z nich, a objaw był mylący:
launchera dawało się znaleźć, wpisując jego nazwę w wyszukiwarkę menu, ale
nie było go w „Grach" — więc wyglądało to na problem z kategorią w pliku
`.desktop`, choć plik był cały czas poprawny. Wyszukiwarka pyta bazę usług na
żywo przy każdej wpisanej literze, dlatego widziała wpis od razu. Drzewo
kategorii pulpit buduje raz, przy starcie sesji, i odświeża je dopiero na
sygnał o przebudowie tej drugiej bazy.

Odświeżenie idzie **zawsze**, nie tylko po zapisie pliku. Wpis mógł zostać
zapisany poprawnie przez starszą wersję launchera, która nie umiała poprosić
pulpitu o przebudowę menu. Taka maszyna ma plik na miejscu i nie ma już czego
zmieniać, a launchera w „Grach" i tak nie ma — odświeżanie tylko po zmianie
zostawiłoby ją zepsutą aż do wylogowania. Kosztuje to około siedemdziesięciu
milisekund, gdy nie ma nic do zrobienia.

Każde z narzędzi dostaje najwyżej 10 sekund (`NAJDLUZEJ_NA_MENU`). Całość
dzieje się w `main()`, przed otwarciem okna: zawieszone wywołanie nie dałoby
ani okna, ani błędu — po kliknięciu ikony po prostu nie działoby się nic. Po
przekroczeniu limitu proces jest ubijany i sprzątany (`kill`, a po nim
`wait`), bo bez tego zostałby proces zombie na całe życie launchera. Brak
narzędzia nie jest błędem: na pulpicie, który go nie używa, to normalny stan
rzeczy.

## Powiązane strony

- [Manifest](manifest.md) — skąd biorą się `latest_version` i `urls`
- [Gdzie leżą dane](dane.md) — katalog ze znacznikiem i miejsce docelowe binarki
- [Wydanie nowej wersji](../projekt/wydanie.md) — co trzeba zrobić, żeby ta maszyneria miała co pobrać
- [Instalacja](../gracz/instalacja.md) — to samo widziane od strony gracza

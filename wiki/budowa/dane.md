# Gdzie leżą dane

Launcher trzyma na dysku cztery rzeczy: pliki samego Minecrafta, katalog
instancji z paczką i światami gracza, pobraną Javę oraz kilka małych plików
z własnym stanem. Wszystko to siedzi w jednym katalogu `data`, a ten leży
w katalogu użytkownika — zgodnie ze zwyczajem systemu.

Za wybór miejsca odpowiada `crates/core/src/miejsca.rs`.

## Katalog użytkownika

| System | Ścieżka |
| --- | --- |
| Windows | `%LOCALAPPDATA%\ChmurkowyLauncher` |
| Linux | `$XDG_DATA_HOME/chmurkowy-launcher` |
| Linux bez `XDG_DATA_HOME` | `~/.local/share/chmurkowy-launcher` |

Na Linuksie zmienna `XDG_DATA_HOME` jest brana pod uwagę tylko wtedy, gdy
zawiera ścieżkę bezwzględną — taki jest wymóg specyfikacji XDG, a ścieżka
względna wskazywałaby co chwilę gdzie indziej, zależnie od tego, skąd launcher
został uruchomiony.

Funkcja `katalog_uzytkownika` jest napisana ręcznie, bez biblioteki od katalogów
systemowych. To kilka linii kodu, a każda zależność w programie, który ma
działać na cudzych komputerach, to jedna rzecz więcej do pilnowania.

Gdy zmiennych środowiskowych zabraknie (`katalog_uzytkownika` zwraca wtedy
`None`), launcher używa katalogu, w którym leży jego własny plik —
`crates/launcher/src/app.rs` robi `unwrap_or_else(|| katalog.clone())`.
Program działa wtedy tak jak dawniej, w trybie przenośnym.

!!! wskazówka

    Gracz nie musi znać tej ścieżki na pamięć. W Ustawieniach, w sekcji PLIKI,
    jest przycisk „Otwórz folder gry", a przed odinstalowaniem launcher
    wypisuje pełną ścieżkę w zdaniu „Leżą w: …".

Obok katalogu `data` leży też sam plik programu — `ChmurkowyLauncher.exe`
na Windowsie, `ChmurkowyLauncher` na Linuksie. Launcher przenosi się tam sam
przy pierwszym uruchomieniu (`crates/core/src/zadomowienie.rs`), żeby program
i jego dane trzymały się razem, a odinstalowanie sprowadzało się do usunięcia
jednego drzewa.

## Drzewo katalogu data

```text
data/
├── mc/                        pliki Minecrafta, wspólne dla każdej instancji
│   ├── versions/
│   │   ├── 1.21.1/
│   │   │   ├── 1.21.1.json
│   │   │   └── 1.21.1.jar
│   │   └── neoforge-21.1.249/
│   │       └── neoforge-21.1.249.json
│   ├── libraries/
│   ├── assets/
│   │   ├── indexes/
│   │   └── objects/<dwa znaki hasza>/<hasz>
│   ├── natives/
│   ├── version_manifest_v2.json
│   └── launcher_profiles.json
├── instance/                  katalog gry: paczka i wszystko, co robi gracz
│   ├── mods/
│   ├── config/
│   ├── saves/
│   ├── resourcepacks/
│   ├── shaderpacks/
│   ├── audio_providers/
│   │   ├── yt-dlp/
│   │   └── ffmpeg/
│   └── options.txt
├── java/
│   └── 21/                    JRE pobrane z Adoptium
├── logs/
│   └── game.log
├── settings.json
├── state.json
├── aktualizacja.txt           znacznik nieudanej próby aktualizacji
└── auth.json
```

Podział na `mc` i `instance` nie jest ozdobny. `mc` to katalog wspólny
Minecrafta — launcher podstawia go do zmiennych `assets_root`,
`library_directory` i `natives_directory` w komendzie uruchamiającej grę.
`instance` to katalog roboczy: idzie jako `game_directory` i jest zarazem
katalogiem bieżącym procesu gry, czyli to, co gracz uważa za „swoje". Dzięki
temu naprawa instalacji może skasować całe `mc` i pobrać je od nowa, nie
dotykając ani jednego świata.

| Katalog albo plik | Kto go tworzy | Czy wolno skasować |
| --- | --- | --- |
| `mc/` | `crates/core/src/game_install.rs` | tak, odbuduje się przy następnym starcie |
| `instance/` | synchronizacja paczki i sama gra | nie — tu są światy gracza |
| `java/<numer>/` | `crates/core/src/java.rs` | tak, pobierze się od nowa |
| `logs/game.log` | launcher przy każdym uruchomieniu gry | tak, powstaje na nowo |
| `settings.json` | zapis ustawień w launcherze | tak, wrócą wartości domyślne |
| `state.json` | synchronizacja paczki | tak, ale patrz niżej |
| `auth.json` | logowanie | tak, trzeba będzie zalogować się ponownie |

### mc

`versions/1.21.1/` z plikami czystej gry launcher zapełnia sam, zanim uruchomi
instalator NeoForge. Instalator umie pobrać je we własnym zakresie, ale robi to
połączeniem z pięciosekundowym limitem wpisanym na sztywno — na wolnym łączu
albo przy niesprawnym IPv6 ten limit mija, zanim cokolwiek przyjdzie. Gdy pliki
już leżą na miejscu, instalator w ogóle nie wchodzi na tę ścieżkę.

`launcher_profiles.json` powstaje z pustej treści tylko po to, żeby instalator
NeoForge zgodził się ruszyć — sprawdza obecność tego pliku, zanim cokolwiek
zrobi.

`assets/objects/` ma układ z dwuznakowym podkatalogiem, bo taki narzuca Mojang:
plik leży pod `objects/<dwa pierwsze znaki SHA-1>/<pełne SHA-1>`.

`natives/` występuje wyłącznie jako ścieżka przekazywana grze — launcher
podstawia `mc/natives/<nazwa profilu>` pod zmienną `natives_directory` i sam
niczego tam nie rozpakowuje.

Plik instalatora `neoforge-21.1.249-installer.jar` i jego log są kasowane po
udanej instalacji.

### instance

To katalog, który gra widzi jako swój. Wszystko, co opisuje manifest, trafia
tutaj: `mods/`, `config/`, `options.txt`. Reszta powstaje w trakcie grania —
`saves/` ze światami, `resourcepacks/` i `shaderpacks/` z tym, co gracz sam
wrzucił.

`audio_providers/yt-dlp` i `audio_providers/ffmpeg` to katalogi narzędzi
wymaganych przez mod Create: Harmonics. Launcher sprawdza, czy coś w nich leży,
pomijając pliki `.md` — sam mod zostawia tam plik z instrukcją, więc sama
obecność czegokolwiek nic by nie znaczyła.

### java

JRE ląduje w `java/<numer wydania>`, czyli przy obecnej paczce w `java/21`.
Numer bierze się z pola `java.major` w manifeście, więc zmiana wymaganej Javy
nie nadpisuje poprzedniej — obie mogą leżeć obok siebie.

Pobrane archiwum (`java-21.tar.gz` na Linuksie, `java-21.zip` na Windowsie)
leży przez chwilę w `data/`, a po rozpakowaniu jest kasowane.

Archiwa Adoptium mają na wierzchu jeden katalog o nazwie zależnej od wydania,
na przykład `jdk-21.0.12+7-jre`, więc launcher nie zakłada z góry, gdzie leży
binarka — szuka jej o poziom głębiej.

### logs

Log gry leci prosto do pliku, a nie do pamięci launchera. Dzięki temu launcher
może schować się do zasobnika, a zapis i tak powstaje w całości. Plik jest
zakładany od nowa przy każdym uruchomieniu gry, więc zostaje w nim ostatnia
rozgrywka.

Skutkiem ubocznym tego rozwiązania było kiedyś to, że gracz nie miał żadnego
sposobu sprawdzić, co się dzieje, gdy okno gry jeszcze nie wstało. Dlatego
launcher czyta ogon tego pliku i pokazuje go w oknie
(`crates/core/src/konsola.rs`): trzyma ostatnie 800 linii, a gdy log milczy
dłużej niż 90 sekund, mówi o tym wprost.

## state.json

To pamięć launchera o tym, co sam wgrał. Struktura jest jedna i prosta
(`crates/core/src/state.rs`):

```json
{
  "written": {
    "config/fml.toml": "dda737dbfafc46c162935d0e2b17f4cb87bd6c4afda324f253e59adbdd3a296b19866634394ada91c68939ef0abd40202a62d6cf47563b24ac53cc61a7b330d4",
    "mods/AI-Improvements-1.21-0.5.3.jar": "ded870e90953ea915d24ac4c81799b95b321dda39936cd4cd620302b48882141fa3b9f1af658bdc6c25dc48de003223b91f3669b69efcc92c8666a3983f0fccc"
  }
}
```

Klucz to ścieżka pliku względem `instance/`, wartość to SHA-512 treści, którą
launcher tam położył.

Po co to jest: bez tego zapisu nie da się odróżnić „gracz zmienił config" od
„paczka się zaktualizowała". Oba przypadki wyglądają na dysku identycznie —
plik ma inny hash niż w manifeście. Dopiero porównanie z tym, co launcher sam
wgrał, rozstrzyga, czy wolno go nadpisać. Od tego zależy działanie polityki
`smart`, opisanej na stronie [Manifest](manifest.md).

Kilka decyzji wartych zapamiętania:

- **Brak pliku albo uszkodzony JSON daje pusty stan, nie błąd.** Najgorsze, co
  się wtedy stanie, to jedno pominięcie aktualizacji configu — a przerwanie
  uruchamiania gry z powodu nieczytelnego pliku pomocniczego byłoby gorsze.
- **Stan zapisywany jest dopiero po udanym pobraniu wszystkiego.** Gdyby
  pobieranie padło w połowie, `state.json` nie może twierdzić, że pliki są na
  miejscu.
- **Skasowanie obcego pliku usuwa też jego wpis** — inaczej stan opisywałby
  coś, czego nie ma.
- Plik zapisywany jest w czytelnej postaci (`to_vec_pretty`), żeby dało się do
  niego zajrzeć bez narzędzi.

Skasowanie `state.json` nie psuje instalacji, ale zmienia zachowanie przy
najbliższej synchronizacji: launcher zobaczy configi, których „nie pamięta",
i zostawi je w spokoju jako pliki nieznanego pochodzenia, zamiast je
zaktualizować.

## Pozostałe pliki stanu

`settings.json` trzyma ustawienia gracza: pamięć, dodatkowe parametry Javy,
rozmiar okna gry, własne komendy. Pola z pierwszych wersji zostają na najwyższym
poziomie pliku, a nowe grupy siedzą w zagnieżdżonych obiektach. To nie jest
kwestia porządku: przeniesienie istniejącego pola w głąb sprawiłoby, że stary
plik wczytuje się bez niego i gracz po aktualizacji zastaje domyślne 4096 MB
zamiast swoich ośmiu gigabajtów.

Obecność `settings.json` jest zarazem znakiem, czy to pierwsze uruchomienie.
Gdy pliku nie ma, launcher dobiera ilość pamięci do komputera; później już jej
nie rusza, bo to wybór gracza.

`auth.json` trzyma listę zapamiętanych kont i to, które jest wybrane. Wcześniej
launcher pamiętał jedno konto Microsoft i jeden nick offline — a przy jednym
komputerze siedzi rodzeństwo. Przycisk „Wyloguj wszystkie konta" w Ustawieniach
czyści ten plik; światy i pliki gry zostają.

## Przeprowadzka ze starego układu

Do wersji 0.4.13 katalog `data` leżał obok pliku launchera. Przy jednym
przenośnym pliku było to wygodne, ale przestaje działać, gdy program instaluje
się raz i potem aktualizuje: katalog instalacji nie jest miejscem na
dwugigabajtowe światy gracza.

Dane przeniosły się więc do katalogu użytkownika, a tym, którzy już mieli
launcher, przenosi je raz — przy pierwszym uruchomieniu nowej wersji. Nikt nie
pobiera paczki drugi raz i nikt nie traci świata z singleplayera.

Decyduje o tym funkcja `ustal(obok_programu, docelowy)`:

| Stan na dysku | Co się dzieje |
| --- | --- |
| Nowy katalog już istnieje | Nic. Przeprowadzka jest za nami. |
| Nowego nie ma, starego też nie | Nic. Świeża instalacja, dane powstaną od zera w nowym miejscu. |
| Nowego nie ma, stary jest | Przeprowadzka. |
| Przeprowadzka się nie udała | Launcher pracuje dalej na starym katalogu. |

Gdy nowy katalog istnieje, starego nie ruszamy nawet wtedy, gdy dalej tam leży.
Mógł zostać po nieudanej próbie i być jedyną kopią, jaką gracz ma.

Samo przeniesienie idzie w dwóch krokach:

1. Najpierw tanio, przez zmianę nazwy katalogu.
2. Gdy to nie wyjdzie — a nie wyjdzie, jeśli katalog domowy jest na innym
   dysku niż pendrive z launcherem — launcher kopiuje drzewo plik po pliku
   i dopiero po udanym kopiowaniu kasuje oryginał.

Nieudane kopiowanie zostawia po sobie śmieci, więc są sprzątane od razu:
niepełny katalog w nowym miejscu przy następnym starcie wyglądałby na gotową
przeprowadzkę i przesłoniłby prawdziwe dane. Gdy z kolei nie uda się skasować
oryginału po udanym kopiowaniu, launcher to ignoruje — dane są już na miejscu,
a stary katalog nikomu nie przeszkadza.

Katalogi kopiowane są z rozpoznawaniem typu wpisu bez podążania za
dowiązaniami. Dowiązanie do katalogu jest kopiowane jako plik, żeby launcher
nie wszedł w nie i nie zaczął ciągnąć cudzych danych spoza swojego drzewa.

O wyniku przeprowadzki gracz dowiaduje się jednym zdaniem w logu launchera:

| Wynik | Zdanie |
| --- | --- |
| `BezZmian` | nic nie jest wypisywane |
| `Przeniesione` | `Dane gry przeniesione z … do katalogu użytkownika. Światy i ustawienia zostały.` |
| `ZostajemyPrzyStarym` | `Nie udało się przenieść danych gry (…). Launcher korzysta ze starego katalogu — nic nie zginęło.` |

!!! uwaga

    Zasada przy każdym z tych przypadków jest ta sama: nieudana przeprowadzka
    nigdy nie może skasować świata gracza. Działanie w nietypowym miejscu jest
    mniejszym złem niż utrata dwóch gigabajtów.

## Kiedy launcher sam kasuje dane

| Sytuacja | Co znika |
| --- | --- |
| Samonaprawa po błędzie Javy | `data/java` |
| Samonaprawa po błędzie plików gry | `data/mc` |
| Przycisk „Napraw instalację" w Ustawieniach | `data/mc` |
| Synchronizacja paczki | obce pliki w katalogach z `mirror_dirs`, czyli w `mods/` |
| Odinstalowanie, odpowiedź „Zostaw moje światy" | nic z `data` |
| Odinstalowanie, odpowiedź o usunięciu wszystkiego | cały katalog `data` |

Kasowane są wyłącznie te katalogi, które launcher potrafi odtworzyć z sieci.
`instance/` nie jest wśród nich nigdy, poza jawnym odinstalowaniem z danymi.
Na Linuksie odinstalowanie usuwa dodatkowo wpis w menu aplikacji
(`~/.local/share/applications/chmurkowy-launcher.desktop`) i ikonę
(`~/.local/share/icons/hicolor/256x256/apps/chmurkowy-launcher.png`).

Szczegóły tego, co i kiedy jest pobierane do `instance/`, opisuje
[Synchronizacja paczki](synchronizacja.md).

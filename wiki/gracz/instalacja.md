# Instalacja

Launcher instaluje się raz i dalej dba o siebie sam: przy każdym uruchomieniu
sprawdza, czy wyszła nowsza wersja, i podmienia własny plik bez pytania.
Nie trzeba więc wracać na stronę wydań po każdej poprawce — wystarczy jedno
pobranie na początku.

## Co pobrać

Pliki leżą na stronie wydań projektu:
[github.com/ttk0721/chmurkowy-launcher/releases](https://github.com/ttk0721/chmurkowy-launcher/releases).
Przy każdym wydaniu jest ich cztery i tylko dwa z nich są dla gracza.

| Plik | Dla kogo | Do czego |
|---|---|---|
| `ChmurkowyLauncher-setup.exe` | Windows, dla gracza | Instalator. Zakłada skróty, dopisuje launcher do listy programów i zostawia po sobie deinstalator. |
| `ChmurkowyLauncher-linux-x64.tar.gz` | Linux, dla gracza | Archiwum z programem, ikoną, wpisem do menu oraz skryptami `install.sh` i `uninstall.sh`. |
| `ChmurkowyLauncher-windows-x64.exe` | Windows, nie dla gracza | Goła binarka, czyli sam program bez niczego dookoła. |
| `ChmurkowyLauncher-linux-x64` | Linux, nie dla gracza | To samo dla Linuksa. |

Gołe binarki leżą w wydaniu dlatego, że sięga po nie samoaktualizacja:
manifest paczki podaje adresy dokładnie tych dwóch plików i to je launcher
pobiera, gdy podmienia sam siebie. Dla człowieka są niewygodne — nie zakładają
skrótów, nie zapisują się na liście programów i nie zostawiają deinstalatora.

Jeśli mimo wszystko ktoś pobierze gołą binarkę i ją kliknie, program zadziała,
ale przy pierwszym uruchomieniu przeniesie się tam, gdzie jego miejsce
(`%LOCALAPPDATA%\ChmurkowyLauncher` na Windowsie,
`~/.local/share/chmurkowy-launcher` na Linuksie), na Linuksie dopisze się do
menu aplikacji i uruchomi się ponownie już stamtąd. Robi tak, bo inaczej
zostałby jednym plikiem w „Pobranych": nie do znalezienia wyszukiwarką
aplikacji, a przy każdej aktualizacji obok leżałaby kolejna kopia z numerkiem
w nazwie. Pobrany plik zostaje nietknięty — launcher nie kasuje cudzych
plików z „Pobranych". Możesz go usunąć sam.

!!! wskazówka

    Zaraz po ogłoszeniu wydania na liście plików bywają tylko te dla Linuksa,
    a w opisie stoi zdanie o tym, że wersja na Windowsa jeszcze się kompiluje.
    Tak ma być: Linux buduje się około sześciu minut, Windows około jedenastu,
    a wydanie publikuje się od razu po Linuksie. Odśwież stronę za kilka minut.

## Windows

1. Pobierz `ChmurkowyLauncher-setup.exe` i kliknij go dwa razy.
2. Przeklikaj instalator. Jest po polsku i nie pyta o hasło administratora.
3. Po drodze możesz zaznaczyć „Utwórz skrót na pulpicie".
4. Na ostatnim ekranie zostaw zaznaczone „Uruchom Chmurkowy Launcher".

Instalacja idzie na Twoje konto, a nie na cały komputer, i to jest świadoma
decyzja: launcher aktualizuje się, podmieniając własny plik, więc musi leżeć
tam, gdzie ma prawo zapisu. W `Program Files` każda poprawka wymagałaby
ponownej instalacji z uprawnieniami administratora.

W menu Start pojawiają się dwie pozycje: „Chmurkowy Launcher" i „Odinstaluj
Chmurkowy Launcher".

Kolejne wersje instalują się na wierzch poprzedniej — Windows rozpoznaje, że
to ta sama aplikacja. Jeżeli launcher akurat działa, instalator go zamknie,
zamiast podmieniać plik pod pracującym programem.

## Linux

Rozpakuj archiwum i uruchom `install.sh` z jego wnętrza:

```bash
tar xzf ChmurkowyLauncher-linux-x64.tar.gz
cd ChmurkowyLauncher-linux
./install.sh
```

Skrypt nie potrzebuje `sudo` i nie zagląda do `/usr`, z tego samego powodu co
instalator na Windowsie: launcher podmienia własny plik przy aktualizacji,
więc musi leżeć w katalogu, do którego ma prawo zapisu. Instalacja systemowa
odebrałaby mu tę możliwość. Nie zależy też od menedżera pakietów, więc
przebiega tak samo na Ubuntu, Mincie, Fedorze i Archu.

Po instalacji pliki leżą tak:

| Co | Gdzie |
|---|---|
| Program | `~/.local/share/chmurkowy-launcher/ChmurkowyLauncher` |
| Wpis w menu aplikacji | `~/.local/share/applications/chmurkowy-launcher.desktop` |
| Ikona | `~/.local/share/icons/hicolor/256x256/apps/chmurkowy-launcher.png` |
| Gra, mody, światy, ustawienia | `~/.local/share/chmurkowy-launcher/data` |

Gdy w systemie ustawiona jest zmienna `XDG_DATA_HOME`, wszystkie te ścieżki
idą za nią zamiast za `~/.local/share`.

Ścieżka do programu trafia do wpisu w menu dopiero podczas instalacji — w
archiwum jej nie ma, bo katalog domowy każdy ma inny. Na koniec skrypt prosi
system o odświeżenie spisu aplikacji; gdy w danym środowisku nie ma
odpowiedniego narzędzia, to nie jest błąd, ale skrót bywa wtedy widoczny
dopiero po wylogowaniu.

Launcher jest po instalacji w menu aplikacji. Można go też uruchomić wprost:

```bash
~/.local/share/chmurkowy-launcher/ChmurkowyLauncher
```

!!! uwaga

    `install.sh` trzeba uruchomić z rozpakowanego katalogu, bo bierze
    z niego plik programu, ikonę i wpis do menu. Uruchomiony gdzie indziej
    przerwie pracę komunikatem „Nie znalazłem pliku ChmurkowyLauncher obok
    tego skryptu".

## Wymagania

- **Komputer 64-bitowy.** Wydanie zawiera pliki wyłącznie dla architektury
  x64 — zarówno instalator, jak i binarki, po które sięga samoaktualizacja.
- **Linux: glibc 2.35 lub nowsze.** Spełniają to Ubuntu 22.04+, Debian 12+,
  Mint 21+, Fedora 36+ i Arch. Program budowany jest celowo na starszym
  systemie, żeby ruszył także na tych dystrybucjach, a przed każdym wydaniem
  sprawdzane jest, czy przypadkiem nie zaczął wymagać nowszej wersji.
  Poza tym potrzebne są tylko biblioteki graficzne i dźwiękowe, które są
  w każdym środowisku graficznym.
- **Pamięć: w praktyce 8 GB.** Paczka modów potrzebuje co najmniej 4 GB dla
  samej gry, a cały proces gry zajmuje przy takim przydziale blisko 6 GB —
  poza przydzieloną pamięcią leżą jeszcze bufory i tekstury sterownika
  grafiki. Poniżej mniej więcej 7 GB pamięci nie ma ustawienia, przy
  którym wszystko naraz się zmieści — launcher wtedy wprost o tym mówi,
  zamiast po cichu zaniżać przydział.
- **Java: nie trzeba jej instalować.** Launcher pobiera własną (Temurin 21)
  i trzyma ją u siebie. Java zainstalowana w systemie nie jest do niczego
  potrzebna i nie jest ruszana.
- **Połączenie z internetem** przy pierwszym uruchomieniu i przy każdej
  zmianie paczki modów.

## Ile się pobiera i ile to zajmuje

| Co | Ile |
|---|---|
| Pobieranie przy pierwszym uruchomieniu | około 1,8 GB |
| Gra z modami na dysku | około 2 GB |
| Wolne miejsce, jakie trzeba mieć | co najmniej 3 GB |

Pobiera się wtedy Java, pliki Minecrafta 1.21.1 z NeoForge i cała paczka
modów. Kolejne uruchomienia startują od razu: launcher porównuje to, co ma na
dysku, z manifestem paczki i dociąga wyłącznie pliki, które się zmieniły.
Aktualizacja samego launchera to jeden plik programu, a nie ponowne pobieranie
gry.

Kiedy miejsca zabraknie w trakcie pobierania, launcher pokazuje błąd
`PLIK-01` — opis wszystkich kodów jest na stronie
[Kody błędów](kody-bledow.md).

### Gdzie leżą dane gry

Gra, paczka modów, światy i ustawienia leżą osobno od samego programu:

| System | Katalog |
|---|---|
| Windows | `%LOCALAPPDATA%\ChmurkowyLauncher\data` |
| Linux | `~/.local/share/chmurkowy-launcher/data` |

Dzięki temu aktualizacja launchera nie rusza światów z singleplayera,
a odinstalowanie programu ich nie kasuje. Kto ma starszą wersję, w której
katalog `data` leżał obok pliku programu, nie musi nic robić — launcher
przeniesie dane sam przy pierwszym uruchomieniu, a gdyby mu się to nie udało,
zostanie przy starym katalogu, zamiast ryzykować zgubienie kilku gigabajtów.
Szczegóły: [Gdzie leżą dane](../budowa/dane.md).

## Odinstalowanie

### Przyciskiem w launcherze

Działa tak samo na obu systemach i nie trzeba niczego szukać poza programem:

1. Otwórz **Ustawienia**, zakładka **Ogólne**.
2. Zjedź na sam dół, do części **STREFA ZAGROŻENIA**.
3. Kliknij **Odinstaluj launcher**.
4. W okienku, które się pojawi, wybierz jedną z możliwości:

| Przycisk | Co robi |
|---|---|
| Zostaw moje światy | Usuwa sam program i skróty. Światy, ustawienia gry i paczka modów zostają na dysku. |
| Usuń wszystko, razem ze światami | Usuwa również katalog z danymi. Tego nie da się cofnąć. |
| Nie odinstalowuj | Zamyka okienko i nic nie zmienia. |

Okienko pokazuje pełną ścieżkę katalogu, którego dotyczy pytanie, żeby było
widać, co dokładnie zniknie. Po wybraniu launcher kończy pracę. Gdyby czegoś
nie dało się usunąć, powie o tym wprost, zamiast udawać, że zniknął.

Na Windowsie samego pliku programu nie może skasować, bo system nie pozwala
usunąć pliku, który właśnie się wykonuje. Gdy launcher pochodzi z instalatora,
robotę przejmuje zostawiony przez niego deinstalator — on jeden wie, co
dopisał do rejestru i do menu Start. Gołą binarkę sprząta mały skrypt, który
czeka, aż program zniknie z pamięci, kasuje plik, a potem sam siebie. Nie
otwiera przy tym żadnego czarnego okna. Na Linuksie nic takiego nie jest
potrzebne, bo tam działający plik wolno usunąć od razu.

### Windows, bez launchera

Tak samo jak każdy inny program — przez **Aplikacje i funkcje**, gdzie na
liście stoi „Chmurkowy Launcher". To samo robi pozycja **Odinstaluj Chmurkowy
Launcher** w menu Start.

### Linux, bez launchera

Skryptem z tego samego archiwum, z którego szła instalacja:

```bash
cd ChmurkowyLauncher-linux
./uninstall.sh
```

Usuwa program, wpis w menu i ikonę, a katalog `data` ze światami, ustawieniami
i paczką zostawia — i wypisuje, gdzie on jest. Żeby skasować również dane:

```bash
./uninstall.sh --wszystko
```

!!! uwaga

    `uninstall.sh` nie jest kopiowany na dysk przy instalacji — zostaje
    w rozpakowanym archiwum. Jeżeli archiwum zostało już skasowane, użyj
    przycisku w launcherze albo pobierz archiwum jeszcze raz ze strony wydań.

## Co zostaje po odinstalowaniu

Niezależnie od sposobu obowiązuje jedna zasada: odpowiedź „nie" na pytanie
o dane nigdy nie kasuje świata gracza. Skasowania światów z singleplayera nie
da się cofnąć, więc jest to zawsze osobny, świadomy wybór, a nie efekt
uboczny odinstalowania.

| Sposób | Co znika | Co zostaje |
|---|---|---|
| Windows, „Aplikacje i funkcje" lub menu Start | Program i skróty | `%LOCALAPPDATA%\ChmurkowyLauncher` razem ze światami, ustawieniami i paczką |
| Linux, `./uninstall.sh` | Program, wpis w menu, ikona | `~/.local/share/chmurkowy-launcher/data` |
| Linux, `./uninstall.sh --wszystko` | Wszystko powyższe razem z danymi | Nic |
| Launcher, „Zostaw moje światy" | Program i skróty | Katalog z danymi |
| Launcher, „Usuń wszystko, razem ze światami" | Wszystko | Nic |

Poza tym zostaje plik, który kiedyś pobrałeś ze strony wydań, jeśli był to
instalator albo goła binarka — nikt go za Ciebie nie kasuje. Na Linuksie po
usunięciu bez `--wszystko` katalog `~/.local/share/chmurkowy-launcher` znika
tylko wtedy, gdy naprawdę nic w nim nie zostało.

Gdy po jakimś czasie zechcesz wrócić do gry, wystarczy zainstalować launcher
jeszcze raz w tym samym systemie — zostawione dane zostaną znalezione i nic
nie będzie pobierane od nowa. Co dzieje się przy takim starcie, opisuje strona
[Pierwsze uruchomienie](pierwsze-uruchomienie.md).

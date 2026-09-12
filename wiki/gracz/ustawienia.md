# Ustawienia

Ustawienia otwiera przycisk **Ustawienia** w prawym górnym rogu głównego ekranu.
Wszystko jest podzielone na cztery zakładki, ułożone od najbezpieczniejszej do
najbardziej ryzykownej.

| Zakładka | Co w niej jest | Czy może zepsuć grę |
| --- | --- | --- |
| Ogólne | pamięć, dodatki, pliki gry, naprawa, konta, aktualizacja, odinstalowanie | nie |
| Gra | rozmiar okna, pełny ekran, zachowanie launchera | nie |
| Java | którą Javą uruchamiać grę, pamięć startowa, parametry Javy | tak |
| Zaawansowane | własne komendy i zmienne środowiskowe | tak |

Zmiany zapisują się same, od razu po kliknięciu. Nie ma przycisku „Zapisz" —
przycisk **Wróć** na dole wraca na główny ekran i też zapisuje. Na dole po
prawej stronie widać numer wersji launchera.

## Ostrzeżenie przed zakładkami Java i Zaawansowane

Przy pierwszym wejściu w **Javę** albo w **Zaawansowane** launcher pokazuje okno
z ostrzeżeniem i dwoma przyciskami: **Rozumiem, pokaż** i **Wolę nie**. Dopóki
nie klikniesz „Rozumiem, pokaż", zakładka się nie otworzy. Po kliknięciu
launcher zapamiętuje, że ostrzeżenie zostało przyjęte, i nie pyta więcej.

Niezależnie od tego, na górze obu tych zakładek **zawsze** stoi pasek
„TYLKO DLA ZAAWANSOWANYCH" z krótkim zdaniem, czym grozi zmiana akurat tutaj.
Okno klika się raz i zapomina; pasek przypomina, gdzie się jest, także wtedy,
gdy ktoś wróci tu za pół roku.

Obie te zakładki mają na dole przycisk **Przywróć domyślne**. Cofa on wszystkie
ustawienia z tej jednej zakładki do stanu, w jakim launcher przyszedł, i nie
rusza niczego poza nią. Bez takiej drogi powrotnej każde pole tutaj byłoby
pułapką bez wyjścia. Przycisk jest wyszarzony, gdy nie ma czego przywracać.

!!! uwaga

    Ostrzeżenia dotyczą wyłącznie tego, czy gra się uruchomi. Światy
    z singleplayera i pliki gry nie mają z tymi zakładkami nic wspólnego
    i nic im tam nie grozi.

## Zakładka „Ogólne"

### Pamięć

Suwak ustawia się w przedziale od 2048 do 16384 MB, co 512 MB.

Launcher sprawdza tę wartość przy **każdym** uruchomieniu, nie tylko pierwszym.
Gdy ustawienie grozi tym, że system zamknie grę w trakcie zabawy, a istnieje
niższa wartość, która się bezpiecznie mieści — launcher zbija suwak do niej
i zapisuje zmianę. Poza tym jednym przypadkiem wartość jest wyborem gracza
i nic jej nie rusza.

Pod suwakiem launcher pisze, ile modów ma paczka i że potrzebuje co najmniej
4 GB. Liczba modów bierze się z manifestu paczki, a nie z tekstu wpisanego na
stałe — inaczej rozjeżdżałaby się przy każdej zmianie paczki.

Obok jest przycisk **Dobierz automatycznie**. Ustawia wartość policzoną z pamięci
tego konkretnego komputera:

| Pamięć komputera | Co proponuje launcher |
| --- | --- |
| 4 GB | 3072 MB |
| 8 GB | 3584 MB |
| 12 GB | 6656 MB |
| 16 GB | 8192 MB |
| 32 GB i więcej | 8192 MB |

Powyżej 8192 MB launcher nie idzie nawet na dużych maszynach. Minecraft i tak
nie korzysta z większej sterty, a odśmiecanie pamięci zaczyna się przy niej wlec
i powoduje przycięcia.

Gdy ustawienie jest za wysokie dla tego komputera, pod suwakiem pojawia się
czerwone zdanie o tym, że system może zamknąć grę w trakcie zabawy. Nie jest to
przypuszczenie: dokładnie to spotkało testera przy 4 GB na komputerze z 8 GB.

### Dlaczego gra bierze więcej pamięci, niż pokazuje suwak

Suwak ustawia **stertę Javy** — miejsce na świat, mody i wszystko, czym gra
żongluje w trakcie zabawy. Poza stertą proces gry zajmuje jeszcze:

- pamięć na opisy klas (przy paczce z ponad dwustoma modami to kilkaset MB),
- pamięć na skompilowany kod i na wewnętrzne struktury odśmiecacza,
- stosy wątków,
- bufory i tekstury sterownika grafiki — zwykle najwięcej z całej tej listy.

Dlatego launcher pisze pod suwakiem, ile zajmie **cały proces**, a nie tylko
sterta:

| Ustawienie suwaka | Tyle zajmie cały proces gry |
| --- | --- |
| 3072 MB | około 4608 MB |
| 4096 MB | około 5888 MB |
| 6144 MB | około 8448 MB |
| 8192 MB | około 11008 MB |

Bez tego zdania gracze podnosili suwak „bo mam 8 GB" i system ubijał im grę
w połowie rozgrywki. Launcher zostawia też pamięć dla systemu — co najmniej
2,5 GB, a na większych maszynach czwartą część całości, bo tam i reszta
programów ma więcej miejsca.

!!! wskazówka

    Suwaka nie opłaca się też zaniżać. Poniżej 3 GB odśmiecacz nie nadąża
    zwalniać pamięci i gra staje: procesor stoi na jednym procencie, a komputer
    zapełnia plik wymiany. Nie wygląda to na brak pamięci, tylko na zawieszenie.

Suwak bywa wyłączony w jednej sytuacji: gdy w zakładce **Java**,
w dodatkowych parametrach, wpisałeś własne `-Xmx`. Dwa takie parametry w jednej
komendzie tylko mylą, więc pierwszeństwo ma to, co wpisałeś ręcznie. Launcher
pisze pod suwakiem, dlaczego jest wyszarzony.

Jest jeszcze drugi przypadek, który suwaka **nie** wyłącza: gdy w zakładce
**Java** ustawisz pamięć startową większą niż to maksimum. Suwak zostaje
aktywny, a launcher dopisuje pod nim na bursztynowo, jakiej wartości naprawdę
użyje — przy odwrotnych wartościach Java odmówiłaby startu.

### Dodatki

Jeden przełącznik: **Przygotuj biblioteki dźwięku przed pierwszym
uruchomieniem**, domyślnie włączony. Mod Create: Harmonics potrzebuje programów
yt-dlp i ffmpeg. Launcher pobiera je z ich oficjalnych źródeł (około 160 MB),
żeby mod nie pytał o to w trakcie gry. Przy wolnym łączu można to wyłączyć —
mod poradzi sobie sam.

### Pliki

| Przycisk | Co robi |
| --- | --- |
| Otwórz folder gry | otwiera katalog z modami, światami i konfiguracją; jeśli go jeszcze nie ma, najpierw go zakłada |
| Otwórz log gry | otwiera zapis tego, co gra wypisała ostatnim razem |

Przycisk od logu jest wyszarzony, dopóki gra nie uruchomiła się ani razu —
wcześniej nie ma czego otwierać.

### Naprawa

**Napraw instalację** usuwa pobrane pliki gry. Pobiorą się od nowa przy
następnym starcie. Światy i ustawienia gry zostają. Przycisk jest wyszarzony,
gdy gra nie jest jeszcze zainstalowana.

### Konto

**Wyloguj wszystkie konta** kasuje z launchera całą listę zapamiętanych kont.
Światy i pliki gry zostają. Sekcja pokazuje się tylko wtedy, gdy ktoś jest
zalogowany. Pojedynczymi kontami zarządza się na ekranie
[Konta](konta.md).

### Aktualizacja

Sekcja jest widoczna zawsze — także wtedy, gdy nie ma nic do pobrania. Pisze
albo „Masz najnowszą wersję", albo numer nowszej, jaka wyszła.

Przycisk pod spodem nazywa się **Zaktualizuj teraz**, gdy jest co pobierać, albo
**Sprawdź ponownie**, gdy nie ma. Przy aktualizacji launcher pobiera nową wersję,
podmienia się i uruchamia ponownie sam. Katalog z danymi zostaje nietknięty.

### Strefa zagrożenia

Na samym dole zakładki, osobno i na czerwono, jest przycisk **Odinstaluj
launcher**. To jedyne miejsce w launcherze, po którym nie ma odwrotu.

Po kliknięciu otwiera się okno z trzema wyjściami. Pokazuje ono ścieżkę do
katalogu z Twoimi danymi, żeby było widać, o czym mowa:

| Wybór | Co znika |
| --- | --- |
| Zostaw moje światy | tylko sam program i jego skróty |
| Usuń wszystko, razem ze światami | program oraz światy z singleplayera, ustawienia gry i paczka modów |
| Nie odinstalowuj | nic, okno się zamyka |

Pytanie o dane jest tu osobnym, świadomym wyborem, a nie polem do zaznaczenia
obok przycisku — skasowania światów nie da się cofnąć.

Na Windowsie robotę kończy deinstalator zostawiony przez instalator: to on wie,
co dopisał do rejestru i do menu Start. W kopii przenośnej, bez instalatora,
plik programu kasuje krótki skrypt, który czeka, aż launcher zniknie z pamięci —
Windows nie pozwala usunąć pliku, który właśnie się wykonuje. Na Linuksie
launcher radzi sobie sam: kasuje swój plik, wpis w menu aplikacji i ikonę.

Gdyby czegoś nie udało się usunąć, launcher wypisze, co zostało. Bez tego
wyglądałoby, że zniknął, a siedziałby dalej na dysku.

## Zakładka „Gra"

### Okno gry

**Ustaw własny rozmiar okna gry** jest domyślnie wyłączone i wtedy o rozmiarze
decyduje sama gra — pamięta ten, w jakim zamknąłeś ją ostatnio.

Po włączeniu dostajesz dwa pola: szerokość i wysokość, w pikselach. Obok są trzy
gotowe rozmiary, których nie trzeba wpisywać ręcznie:

| Nazwa | Rozmiar |
| --- | --- |
| Domyślny | 854 × 480 |
| HD | 1280 × 720 |
| Full HD | 1920 × 1080 |

Wpisać da się od 320 × 240 do 7680 × 4320. Dolna granica bierze się stąd, że
niżej interfejs gry robi się nieczytelny, a menu nie mieści przycisków; górna
to 8K, powyżej i tak decyduje sterownik grafiki.

**Uruchamiaj grę na pełnym ekranie** sprawia, że gra zajmuje cały ekran od razu
po starcie. Rozmiar okna powyżej nadal ma znaczenie — to on obowiązuje po
wyjściu z pełnego ekranu klawiszem `F11`.

### Zachowanie launchera

| Przełącznik | Domyślnie | Co robi |
| --- | --- | --- |
| Schowaj launcher do zasobnika po uruchomieniu gry | włączony | launcher znika z ekranu i przestaje zabierać zasoby, ale nadal czuwa — wróci sam, gdyby gra zamknęła się z błędem |
| Zamknij launcher po zakończeniu gry | wyłączony | launcher zamyka się razem z normalnym wyjściem z gry |

Drugi przełącznik dotyczy wyłącznie normalnego wyjścia. Kiedy gra padnie,
launcher zostaje i pokazuje, co się stało — bez tego zniknąłby razem z jedynym
wyjaśnieniem.

## Zakładka „Java"

Domyślnie nie ma tu nic do roboty. Launcher pobiera własną Javę i to działa na
każdym komputerze, na którym w ogóle da się grać. Zakładka istnieje dla
przypadków, w których to zawodzi: polityka firmowa blokująca pliki w katalogu
użytkownika, nietypowa biblioteka systemowa, ktoś, kto po prostu ma swoją Javę.

### Własna Java

Pole na ścieżkę jest puste i puste znaczy „użyj tej, którą launcher pobrał sam".
Wyczyszczenie pola zawsze wraca do tego bezpiecznego stanu. Pod polem są trzy
przyciski:

| Przycisk | Co robi |
| --- | --- |
| Wykryj | przeszukuje typowe miejsca, w których instaluje się Java, i pokazuje listę znalezionych |
| Przeglądaj | otwiera okno wyboru pliku |
| Sprawdź | pyta wskazaną Javę o wersję i mówi, czy pasuje do paczki |

Wykrywanie zagląda do `JAVA_HOME`, do katalogów z `PATH` oraz do miejsc typowych
dla systemu — na Linuksie `/usr/lib/jvm`, `/usr/lib64/jvm`, `/opt/java` i katalogi
z Javą pobraną przez Prism Launcher albo MultiMC; na Windowsie podkatalogi
`Java`, `Eclipse Adoptium` i `Microsoft\jdk` w Program Files i w katalogu
użytkownika; na macOS `JavaVirtualMachines`. Wykrywanie samo o nic nie pyta
znalezionych Jav — odpalenie kilkunastu procesów naraz przy otwarciu Ustawień
byłoby widoczne jako zacięcie. Pytanie o wersję wykonuje dopiero **Sprawdź**,
i tylko dla tej jednej, którą wskażesz. Kliknięcie pozycji na liście od razu ją
sprawdza.

### Sprawdzanie wersji

Wynik sprawdzenia to jedno zdanie w rodzaju „Java 21 (21.0.4), 64-bitowa".
Launcher porównuje numer główny z tym, którego wymaga paczka, i mówi wprost,
gdy się nie zgadzają.

Dwie rzeczy, o których wtedy pisze:

- **Zła wersja.** Launcher nie uruchomi gry taką Javą, dopóki nie zaznaczysz
  **Pomiń sprawdzanie zgodności wersji Javy**. To furtka dla kogoś, kto wie,
  co robi; domyślnie zamknięta.
- **Java 32-bitowa.** Taka nie obejmie tyle pamięci, ile potrzebuje paczka. Gra
  padłaby przy starcie na braku miejsca na stertę, a komunikat JVM nie tłumaczy
  z tego niczego.

Gdy wskazany plik nie istnieje, nie da się go uruchomić albo nie jest Javą,
launcher powie to tutaj, zanim ktokolwiek kliknie GRAJ. Pytanie o wersję ma
15 sekund limitu: zdrowa Java odpowiada w ułamku sekundy, ale plik w zawieszonym
zasobie sieciowym potrafiłby wisieć bez końca, a dzieje się to w Ustawieniach,
gdzie gracz patrzy na przycisk i czeka.

### Pamięć startowa

Pole z numerem i podpisem `-Xms` ustawia, ile pamięci Java bierze od razu na
starcie. Wpisać da się od 256 do 16384 MB; domyślne 512 MB to wartość, której
launcher używał od zawsze, więc domyślne ustawienie niczego nie zmienia.
Podniesienie skraca zacinanie się gry w pierwszych minutach, ale zajmuje pamięć
od pierwszej sekundy.

!!! uwaga

    Maksimum ustawia suwak w zakładce **Ogólne**, a to pole stoi w zakładce
    **Java** — łatwo więc ustawić je sprzecznie. Pamięć startowa większa od
    maksimum to nie ostrzeżenie, tylko koniec: Java odmawia startu i gra nie
    rusza wcale. Launcher przycina wtedy wartość w locie i pisze o tym w obu
    zakładkach.

### Dodatkowe parametry Javy

Wieloliniowe pole na parametry przekazywane Javie. Można je pisać po jednym
w wierszu albo po spacji; cudzysłowy są respektowane, więc ścieżka ze spacją
zostaje jednym parametrem. Launcher pisze pod polem, ile parametrów rozpoznał.

Puste pole jest bezpieczne i taka jest wartość domyślna. Jeśli po wpisaniu
czegoś gra przestanie się uruchamiać, wyczyszczenie pola przywraca poprzedni
stan.

Wpisanie tu własnego `-Xmx` wyłącza suwak pamięci w zakładce Ogólne — launcher
pisze o tym w obu miejscach.

**Przywróć domyślne** na dole tej zakładki czyści ścieżkę do Javy, pomijanie
sprawdzania, pamięć startową i dodatkowe parametry naraz.

## Zakładka „Zaawansowane"

Wszystko tutaj wykonuje się na Twoim komputerze przy każdym starcie gry.

### Własne komendy

Trzy pola tekstowe:

| Pole | Kiedy się wykonuje | Co się dzieje przy błędzie |
| --- | --- | --- |
| Przed uruchomieniem gry | zanim gra ruszy | launcher **nie uruchomi gry** |
| Komenda opakowująca | razem z grą, jako program, w którym gra się uruchamia | gra nie ruszy |
| Po zakończeniu gry | po wyjściu z gry | launcher tylko o tym wspomni |

Przerwanie startu przez pierwszą komendę jest zamierzone. Wpisuje się tam rzeczy
w rodzaju „zrób kopię świata", a granie na świecie, którego kopia się nie udała,
jest dokładnie tym, przed czym ta komenda miała chronić. Komenda po zakończeniu
gry niczego nie przerywa — gra już się skończyła, nie ma czego chronić, a okno
błędu po udanej rozgrywce tylko by przestraszyło.

Komenda opakowująca wstawia się przed ścieżką do Javy w tej samej komendzie,
w której jest gra. Tak włącza się na Linuksie `gamemoderun` albo `prime-run`.
Jako jedyna z trzech nie idzie przez powłokę systemu, tylko jest dzielona na
program i jego argumenty.

Pozostałe dwie uruchamiają się przez powłokę systemu (`/bin/sh -c`, na Windowsie
`cmd /C`), w folderze gry. Powłoka jest tu potrzebna, bo ludzie piszą
`cp swiat swiat.bak && echo gotowe`, a nie listę argumentów — bez powłoki `&&`
byłoby zwykłym argumentem programu `cp`.

!!! uwaga

    Każda z tych komend ma limit **dwóch minut**. Po nim launcher ją przerywa
    i zgłasza błąd. Limit jest tu ważniejszy niż gdziekolwiek indziej, bo treść
    wpisuje człowiek: literówka w skrypcie albo program czekający na wciśnięcie
    klawisza zawiesiłby start gry na zawsze, a launcher nie ma jak pokazać
    takiemu procesowi konsoli. Dwie minuty starczą na kopię zapasową świata czy
    podmianę konfiguracji.

### Zmienne `INST_*`

Komendy dostają w środowisku sześć zmiennych opisujących uruchamianą grę.
Nazwy są takie same jak w Prism Launcherze, żeby gotowe skrypty przenosiły się
między launcherami bez przeróbek.

| Zmienna | Co zawiera |
| --- | --- |
| `$INST_NAME` | nazwa paczki, wzięta z manifestu |
| `$INST_ID` | krótki identyfikator instancji — zawsze `chmurka` |
| `$INST_DIR` | folder z modami, światami i konfiguracją |
| `$INST_MC_DIR` | folder z plikami samego Minecrafta |
| `$INST_JAVA` | plik wykonywalny Javy, którym ruszy gra |
| `$INST_JAVA_ARGS` | parametry pamięci i dodatkowe, które dostanie Java |

Przykładowa komenda robiąca kopię światów przed graniem:

```sh
cp -r "$INST_DIR/saves" ~/kopie
```

### Zmienne środowiskowe

Tabela par nazwa–wartość, dokładanych do procesu gry **i** do własnych komend
powyżej. Na Linuksie to jedyna droga do rzeczy w rodzaju
`MESA_GL_VERSION_OVERRIDE` czy `DRI_PRIME`. Wiersze dodaje się przyciskiem
**Dodaj zmienną**, kasuje przyciskiem **Usuń** obok wiersza. Pod tabelą launcher
pisze, ile zmiennych naprawdę trafi do gry.

Pusty wiersz nie jest błędem — to wiersz, którego jeszcze nie wypełniono. Wpis
z nazwą, której systemowi nie da się przekazać, launcher pomija i pisze o tym
przy wierszu:

| Nazwa zostanie odrzucona, gdy | Powód |
| --- | --- |
| jest pusta | nie ma czego ustawiać |
| zawiera znak `=` | system rozdziela nazwę od wartości właśnie tym znakiem |
| zawiera spację | to samo — podział rozjechałby się |
| zawiera znak nie do przekazania systemowi | próba przekazania go przewróciłaby launcher |

Gdyby taki wpis przeszedł po cichu, zmienna z literówką po prostu nie doszłaby
do gry i nikt nie wiedziałby dlaczego.

Gdy ta sama nazwa pojawi się dwa razy, liczy się ostatni wiersz — tak samo, jak
zachowuje się powłoka systemu.

**Przywróć domyślne** na dole tej zakładki czyści wszystkie trzy komendy
i całą tabelę zmiennych naraz.

## Gdzie zapisują się ustawienia

Wszystko z tych czterech zakładek leży w jednym pliku:

```text
Windows:  %LOCALAPPDATA%\ChmurkowyLauncher\data\settings.json
Linux:    ~/.local/share/chmurkowy-launcher/data/settings.json
```

Uszkodzony plik nie blokuje launchera — wczytują się wtedy ustawienia domyślne.
Aktualizacja launchera nie kasuje żadnej ustawionej wartości.

## Klawisze ustawiane przez paczkę

Ustawienia **w samej grze** — głośność, czułość myszy, zasięg widzenia, grafika
— są tylko Twoje. Launcher ich nie dotyka.

Wyjątkiem są **przypisania klawiszy i język**. Te administracja może ustawić dla
wszystkich, żeby paczka działała tak samo u każdego. Ma to znaczenie praktyczne:
część modów przypisuje sobie domyślnie klawisze numeryczne albo piąty przycisk
myszy, których na wielu klawiaturach po prostu nie ma.

Działa to tak:

* administracja zmienia jakiś klawisz i publikuje nową paczkę,
* przy najbliższym uruchomieniu launcher podmienia **tylko te wpisy, które się
  zmieniły**, i pisze o tym w komunikacie,
* jeśli potem przestawisz sobie ten klawisz po swojemu, **zostanie tak, jak
  ustawiłeś** — aż do momentu, gdy administracja zmieni akurat ten klawisz.

!!! wskazówka

    Zmiana jednego klawisza przez administrację nie kasuje pozostałych Twoich
    przestawień. Jeśli przestawiłeś sobie skok, a administracja zmieniła atak,
    Twój skok zostaje.

Gdy gra mimo wszystko nie startuje, po kolejne kroki zajrzyj na stronę
[Gdy gra nie startuje](problemy.md).

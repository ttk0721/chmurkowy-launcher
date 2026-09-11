# Pierwsze uruchomienie

Pierwsze kliknięcie **GRAJ** jest inne niż wszystkie następne: launcher musi
najpierw ściągnąć na dysk całą grę razem z modami. Nic nie trzeba przy tym
klikać ani niczego doinstalowywać — ta strona opisuje po kolei, co się dzieje,
ile to trwa i po czym poznać, że praca idzie dalej.

## Zanim klikniesz GRAJ

Zaraz po starcie launcher pobiera **manifest** — jeden mały plik z listą
wszystkiego, co składa się na paczkę. Do czasu, aż przyjdzie, na środku okna
kręci się kółko z napisem „Sprawdzam paczkę…", a przycisk GRAJ jest nieaktywny.
To trwa chwilę.

Gdy manifest już jest, na ekranie pojawia się nazwa edycji paczki oraz linijka
w rodzaju `Minecraft 1.21.1 · NeoForge 21.1.249`. Dopiero wtedy przycisk staje
się klikalny.

!!! uwaga

    Jeśli na przycisku widnieje **ZALOGUJ SIĘ**, a nie **GRAJ**, to znaczy, że
    launcher nie ma jeszcze żadnego konta. Kliknięcie zaprowadzi Cię na ekran
    logowania — opisany na stronie [Konta](konta.md).

## Co się dzieje po kliknięciu GRAJ

Na dolnym pasku okna pojawia się napis z nazwą etapu i pasek postępu. Etapy
idą zawsze w tej samej kolejności:

| Napis w launcherze | Co się wtedy dzieje |
| --- | --- |
| Pobieram Javę | Launcher ściąga Javę 21 (Temurin) z serwerów Adoptium i rozpakowuje ją u siebie. |
| Instaluję NeoForge | Pobranie plików czystego Minecrafta 1.21.1 i uruchomienie oficjalnego instalatora NeoForge. |
| Pobieram biblioteki | Pliki `.jar`, bez których gra się nie uruchomi. Licznik pokazuje, ile z nich już jest. |
| Pobieram zasoby gry | Pliki wypisane w indeksie zasobów Minecrafta. Jest ich dużo i są drobne, więc licznik skacze szybko. |
| Pobieram mody | Cała paczka: mody i pliki konfiguracyjne wypisane w manifeście. |
| Pobieram biblioteki dźwięku | Dwa dodatkowe programy dla moda Create: Harmonics. Ten etap można wyłączyć w Ustawieniach. |
| Gotowe / Uruchamiam grę… | Launcher startuje Minecrafta. |

Kilka rzeczy, które w tej liście nie są oczywiste:

- **Java pobiera się sama i tylko dla launchera.** Nie instaluje się w systemie
  i nie miesza w tym, co masz już na komputerze. Ląduje w podkatalogu `java/21`
  katalogu danych launchera i tylko stamtąd jest uruchamiana.
- **Pliki czystego Minecrafta launcher kładzie na dysku sam, zanim odpali
  instalator NeoForge.** Instalator umie pobrać je samodzielnie, ale ma na
  nawiązanie połączenia wpisane na sztywno pięć sekund. Na wolnym łączu albo
  przy niesprawnym IPv6 ten czas mija, zanim cokolwiek przyjdzie, i instalacja
  przewraca się — mimo że przeglądarka na tym samym komputerze otwiera strony
  normalnie. Gdy pliki już leżą na miejscu, instalator sprawdza, widzi że są,
  i całą tę drogę pomija.
- **Instalator NeoForge dostaje trzy podejścia**, a od drugiego launcher każe
  Javie trzymać się IPv4 — właśnie dlatego, że zepsute IPv6 jest najczęstszą
  przyczyną, dla której reszta internetu działa, a instalator stoi.
- **Pliki pobierają się po osiem naraz**, każdy w trzech podejściach z rosnącą
  przerwą, i każdy jest po pobraniu sprawdzany sumą kontrolną. Uszkodzony plik
  jest odrzucany i pobierany od nowa, zamiast trafić do gry.

## Kiedy pasek stoi, a kiedy kręci się kółko

Pasek postępu pokazuje procenty tylko wtedy, gdy da się je policzyć. Przy
pobieraniu plików launcher zna ich liczbę albo rozmiar, więc nad paskiem widać
`45/249` albo `10.0 MB / 45.0 MB`. Same procenty są w środku paska.

Są jednak etapy, których nie da się zmierzyć z zewnątrz — przede wszystkim
praca instalatora NeoForge, który mieli około minuty i nie melduje po drodze
niczego. Zamiast paska stojącego uparcie na zerze launcher pokazuje wtedy
kręcące się kółko i zdanie mówiące, co robi, np. „Instaluję NeoForge — to
potrwa około minuty" albo „Rozpakowuję Javę…".

!!! wskazówka

    Na dolnym pasku, po prawej, jest przycisk **szczegóły**. Rozwija listę
    zdarzeń — co się udało, co launcher poprawił sam, co pominął. Na co dzień
    niepotrzebny; przydaje się, gdy coś poszło nie tak.

## Ile to trwa i ile waży

Rozmiary, które launcher zna wprost:

| Co | Ile waży |
| --- | --- |
| Java 21 | około 180 MB |
| Paczka modów (wersja `2026.09.10-21`) | 710 plików, w tym 264 mody |
| Biblioteki dźwięku (yt-dlp i ffmpeg) | około 160 MB |
| Wszystko razem na dysku | około 2 GB |

Przed pierwszym uruchomieniem miej na dysku **co najmniej 3 GB wolnego
miejsca** — tyle launcher podaje w komunikacie, gdy miejsca zabraknie
w trakcie pobierania.

Czas zależy przede wszystkim od łącza, bo prawie cały ten czas to pobieranie.
Dwie rzeczy są stałe niezależnie od internetu:

- instalator NeoForge mieli **około minuty** (launcher czeka na niego
  najwyżej kwadrans, po czym uznaje, że stanął, i próbuje jeszcze raz);
- od uruchomienia gry do pojawienia się jej okna mija **około dwóch minut** —
  tyle NeoForge potrzebuje na wczytanie kilkuset modów. Dzieje się to przy
  **każdym** uruchomieniu, nie tylko pierwszym.

!!! uwaga

    Launcher nie ma górnego limitu na czas pobierania — 180 MB Javy na wolnym
    łączu uczciwie schodzi kilkanaście minut i przerywanie tego byłoby
    lekarstwem gorszym od choroby. Przerywa dopiero wtedy, gdy przez minutę
    nie przyjdzie nawet 64 kB. To nie jest już „wolne łącze", tylko zerwane
    połączenie.

    Jeśli pobieranie zostanie przerwane, nic nie przepada. Przy kolejnym
    kliknięciu GRAJ launcher pobierze tylko to, czego brakuje.

## Ekran „Co się dzieje"

Kiedy gra wystartuje, launcher sam przełącza się na ekran **Co się dzieje**
i pokazuje na żywo to, co gra wypisuje w trakcie ładowania. Można tu też wejść
ręcznie — przyciskiem *Co się dzieje* w prawym górnym rogu ekranu głównego.
Gdy gra działa, przy nazwie przycisku widnieje kropka (`Co się dzieje ●`).

Ten ekran powstał z jednego pytania testera: *„nie wiem, czy program od
dziesięciu minut coś ładuje, czy po prostu umarł, i nie mam jak tego
sprawdzić"*. Odpowiada na nie **jedno zdanie u góry** — i to jedyna rzecz,
którą naprawdę trzeba tu przeczytać:

| Co widzisz u góry | Co to znaczy |
| --- | --- |
| Gra się ładuje — log rośnie. | Wszystko w porządku, czekaj. |
| Gra działa, ale nic nie wypisała od… | Gra od dłuższego czasu milczy. Przy dużej paczce jeszcze bywa to normalne. |
| Gra już nie działa. Poniżej jej ostatnie zapiski. | Gra się zakończyła. |
| Gra jeszcze nie ruszyła — nie ma czego pokazać. | Gry jeszcze nie uruchamiano, więc nie ma żadnych zapisków. |

Ostrzeżenie o ciszy pojawia się dopiero po **półtorej minuty** bez ani jednego
nowego wpisu. Próg jest tak wysoki celowo: wczytywanie modów potrafi zamilknąć
na kilkadziesiąt sekund, gdy gra skleja atlas tekstur. Fałszywy alarm co pół
minuty byłby gorszy od braku alarmu.

Poniżej zdania leci sam log — ostatnie 800 linii, odświeżane kilka razy na
sekundę, zawsze przewinięte na koniec. Linie z błędami są czerwone,
ostrzeżenia niebieskie, reszta szara. Dla gracza to materiał do wysłania
dalej, nie do czytania.

Na dole ekranu są trzy przyciski: **Wróć**, **Kopiuj log** i **Otwórz folder
z logiem**. Do czego służą dwa ostatnie, opisuje strona
[Gdy gra nie startuje](problemy.md).

### Kiedy tam zajrzeć

- Gdy od kliknięcia GRAJ minęło kilka minut i nie wiadomo, czy coś się jeszcze
  dzieje.
- Zanim napiszesz do administracji serwera — to stąd bierze się treść
  zgłoszenia.

Poza tymi dwoma przypadkami nie ma powodu tu zaglądać.

## Co się dzieje, gdy gra już ruszy

Launcher domyślnie chowa swoje okno do zasobnika systemowego, żeby nie
zabierało miejsca ani zasobów. Wraca jednym kliknięciem w ikonę. Gdy systemu
nie da się o taką ikonę poprosić, launcher zamiast schować okno tylko je
minimalizuje — inaczej ukryte okno byłoby nie do odzyskania.

Po normalnym wyjściu z gry launcher wraca na ekran główny z przyciskiem GRAJ.
Gdy gra kończy się błędem, okno wraca samo i pokazuje, co się stało — opisuje
to strona [Gdy gra nie startuje](problemy.md).

## Drugie i kolejne uruchomienie

Wszystko, co pobrało się raz, zostaje na dysku. Przy każdym następnym
kliknięciu GRAJ launcher sprawdza tylko, czy pliki paczki zgadzają się
z manifestem, i pobiera różnice — zwykle nic albo kilka modów po aktualizacji
paczki. Java, NeoForge i zasoby gry nie są pobierane drugi raz.

Gdzie dokładnie to wszystko leży, opisuje strona
[Gdzie leżą dane](../budowa/dane.md). Krok po kroku od strony technicznej —
[Od kliknięcia GRAJ do gry](../budowa/przebieg-uruchomienia.md).

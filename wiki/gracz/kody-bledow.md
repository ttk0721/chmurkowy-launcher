# Kody błędów

Gdy coś nie wyjdzie, launcher pokazuje okno „Coś poszło nie tak”. Na górze,
w czerwonej ramce, stoi krótki kod — na przykład `SIE-02`. Pod nim jedno zdanie
o tym, co się stało, a niżej ponumerowana lista „Co zrobić”.

Kod jest krótki po to, żeby dało się go podyktować przez telefon albo przepisać
w wiadomości do administracji. Ta strona wypisuje wszystkie kody, jakie launcher
potrafi pokazać.

Na dole okna są dwa przyciski:

- **Spróbuj ponownie** — zamyka okno i wraca na ekran główny.
- **Kopiuj szczegóły dla administracji** — wkłada do schowka gotową wiadomość
  z wersją launchera, kodem błędu i szczegółami technicznymi. Jeśli kopiowanie
  się nie uda, launcher napisze, żeby zrobić zdjęcie okna i wysłać je
  administracji.

Sekcja „Szczegóły techniczne” w oknie jest domyślnie zwinięta. Dla gracza to
szum; jest tam dla administracji i trafia do schowka razem z resztą.

Pod listą kroków launcher zawsze dopisuje to samo zdanie: jeśli nic nie pomogło,
napisz do administracji i podaj kod błędu. Administracja prowadzi serwer po
godzinach, więc odpowiedź może zająć dzień lub dwa.

!!! wskazówka

    Jeśli nie chcesz szukać w tabelach, zacznij od strony
    [Gdy gra nie startuje](problemy.md). Ta strona jest katalogiem — służy do
    sprawdzenia konkretnego kodu, który już zobaczyłeś.

## Kody, których możesz nigdy nie zobaczyć

Część rad z tego katalogu launcher wykonuje sam, zanim w ogóle pokaże okno.
Powód jest prosty: rada „wejdź w Ustawienia i kliknij Napraw instalację” jest
poprawna, ale dla dziesięciolatka to ściana tekstu, której nikt nie czyta.
Więc launcher robi to za gracza i pokazuje błąd dopiero wtedy, gdy to nie
pomogło.

Samonaprawa wykonuje do trzech podejść. Po każdej nieudanej próbie launcher
pisze w logu, co robi i które to podejście, na przykład
„Coś się nie udało. Pobieram pliki gry od nowa… (kod INST-03, podejście 2 z 3)”.
To samo zdanie pojawia się przy pasku postępu, żeby pasek nie zamilkł
w połowie i nie wyglądał na zawieszenie.

| Co launcher robi sam | Przy których kodach | Na czym to polega |
| --- | --- | --- |
| Powtórka | `SIE-01`, `SIE-02`, `SIE-03`, `SIE-04`, `PLIK-03` | Odczekuje 2 sekundy i próbuje jeszcze raz. Nic nie kasuje. Chwilowy problem z siecią albo z zapisem pliku często mija sam. |
| Java od nowa | `INST-01` | Kasuje folder `java` z katalogu danych launchera i pobiera Javę jeszcze raz. |
| Pliki gry od nowa | `INST-02`, `INST-03`, `INST-04` | Kasuje folder `mc` z katalogu danych launchera i instaluje grę od zera. Światy, ustawienia gry i paczka modów leżą w osobnym folderze i zostają nietknięte. |

Wszystkie pozostałe kody launcher pokazuje od razu, bez własnych prób.

!!! uwaga

    Samonaprawy nie ma tam, gdzie nic by nie dała albo wręcz zaszkodziła.
    Przy pełnym dysku (`PLIK-01`) skasowanie 2 GB plików gry i tak nie dałoby
    gdzie ich zapisać. Przy braku uprawnień (`PLIK-02`) i przy sprawach konta
    (`KONTO-*`) powtórka nie zmienia zupełnie nic. A przy własnej komendzie
    gracza (`USTAW-02`, `USTAW-03`) powtórka byłaby szkodliwa — komenda
    „zrób kopię świata” wykonałaby się trzy razy pod rząd, a literówka
    w skrypcie kazałaby czekać trzy limity czasu.

Jest jeszcze jeden przypadek bez samonaprawy: jeśli gra **już wystartowała**
i dopiero potem coś padło, launcher nie próbuje niczego ponawiać. Drugie okno
Minecrafta byłoby gorsze od każdego błędu.

## Spis wszystkich kodów

| Kod | Tytuł w oknie | Naprawia sam |
| --- | --- | --- |
| `SIE-01` | Brak połączenia z internetem | powtórka |
| `SIE-02` | Nie udało się pobrać pliku | powtórka |
| `SIE-03` | Pobrany plik był uszkodzony | powtórka |
| `SIE-04` | Instalator gry nie mógł pobrać plików | powtórka |
| `PLIK-01` | Skończyło się miejsce na dysku | nie |
| `PLIK-02` | Launcher nie ma prawa zapisywać w tym miejscu | nie |
| `PLIK-03` | Nie udało się zapisać pliku | powtórka |
| `INST-01` | Nie udało się rozpakować Javy / Java rozpakowała się niekompletnie | Java od nowa |
| `INST-02` | Nie udało się uruchomić instalatora gry / Instalacja modyfikacji nie powiodła się | pliki gry od nowa |
| `INST-03` | Pliki gry są niekompletne / Spis plików gry jest uszkodzony | pliki gry od nowa |
| `INST-04` | Nie da się uruchomić gry | pliki gry od nowa |
| `GRA-01` | Gra zamknęła się zaraz po uruchomieniu | nie |
| `GRA-02` | Grze zabrakło pamięci | nie |
| `GRA-03` | Java odrzuciła Twoje dodatkowe parametry | nie |
| `GRA-04` | Jeden z modów spowodował błąd | nie |
| `GRA-05` | Komputerowi zabrakło pamięci i zamknął grę | nie |
| `GRA-06` | Ten komputer ma za mało pamięci na tę paczkę | nie |
| `KONTO-01` | Kod logowania wygasł | nie |
| `KONTO-02` | To konto nie ma profilu Xbox | nie |
| `KONTO-03` | To konto dziecka i wymaga zgody rodzica | nie |
| `KONTO-04` | To konto nie ma kupionego Minecrafta | nie |
| `KONTO-05` | Microsoft odmówił logowania / Logowanie nie powiodło się | nie |
| `KONTO-06` | Xbox Live nie działa w kraju ustawionym na tym koncie | nie |
| `KONTO-07` | Microsoft chce potwierdzenia na stronie konta | nie |
| `KONTO-08` | To konto jest zablokowane przez Microsoft | nie |
| `KONTO-09` | Logowanie zostało odrzucone | nie |
| `KONTO-10` | Kod stracił ważność | nie |
| `KONTO-11` | Microsoft nie wpuszcza tego konta | nie |
| `KONTO-12` | Zapisane logowanie wygasło | nie |
| `USTAW-01` | Java wskazana w Ustawieniach nie działa | nie |
| `USTAW-02` | Twoja komenda nie wykonała się poprawnie / Komenda po zakończeniu gry nie wykonała się poprawnie | nie |
| `USTAW-03` | Twoja komenda się zawiesiła | nie |
| `PACZKA-01` | Nie udało się odczytać informacji o paczce | nie |
| `PACZKA-02` | Paczka modów zawiera błąd | nie |
| `INNY-01` | Coś poszło nie tak, ale nie wiemy co | nie |

Niektóre kody mają dwa różne tytuły, bo do tego samego problemu prowadzą dwie
różne drogi. Rady są wtedy nieco inne — poniżej opisany jest każdy przypadek
osobno.

## SIE — internet i pobieranie

`SIE-01` — Brak połączenia z internetem

:   Launcher nie mógł połączyć się z serwerami Microsoftu, żeby Cię zalogować.

    1. Sprawdź, czy internet działa — otwórz dowolną stronę w przeglądarce.
    2. Jeśli masz Wi-Fi, podejdź bliżej routera.
    3. Spróbuj ponownie za chwilę.

`SIE-02` — Nie udało się pobrać pliku

:   Launcher kilka razy próbował pobrać jeden z plików gry i za każdym razem
    się nie udało.

    1. Sprawdź, czy internet działa — otwórz dowolną stronę w przeglądarce.
    2. Uruchom launcher ponownie. To, co już się pobrało, nie pobierze się
       drugi raz.
    3. Jeśli korzystasz z sieci szkolnej albo firmowej, może ona blokować
       pobieranie.

`SIE-03` — Pobrany plik był uszkodzony

:   Launcher pobrał plik, ale okazał się uszkodzony i został odrzucony.
    Najczęściej winne jest niestabilne połączenie.

    1. Uruchom launcher ponownie — pobierze brakujący plik jeszcze raz.
    2. Jeśli masz Wi-Fi, podejdź bliżej routera albo podłącz kabel.
    3. Wyłącz na chwilę program antywirusowy — czasem psuje pobierane pliki.

`SIE-04` — Instalator gry nie mógł pobrać plików

:   Program instalujący modyfikacje próbował pobrać pliki z internetu i nie
    zdążył się połączyć. Dzieje się tak na wolnym łączu albo gdy sieć
    przepuszcza tylko część połączeń — nawet jeśli strony w przeglądarce
    otwierają się normalnie.

    1. Uruchom launcher ponownie — to, co już się pobrało, zostaje na dysku.
    2. Jeśli masz Wi-Fi, podejdź bliżej routera albo podłącz kabel.
    3. Jeśli to sieć szkolna albo firmowa, spróbuj na domowej lub na telefonie.

!!! wskazówka

    `SIE-02` i `SIE-03` to nie to samo. Przy `SIE-02` plik w ogóle nie dojechał.
    Przy `SIE-03` dojechał, ale nie zgadza się jego suma kontrolna, więc launcher
    go odrzucił — lepiej pobrać go jeszcze raz, niż uruchamiać grę z uszkodzonym
    plikiem.

    `SIE-04` ma osobny kod, choć wygląda jak problem z instalacją. Pliki ściąga
    tu instalator modyfikacji, a nie sam launcher — i skoro nie dosięgnął
    serwerów, rada „napraw instalację” niczego by nie zmieniła. Przyczyna jest
    po stronie sieci.

## PLIK — zapis na dysku

`PLIK-01` — Skończyło się miejsce na dysku

:   Gra z modami zajmuje około 2 GB. Na dysku zabrakło miejsca w trakcie
    pobierania.

    1. Zwolnij miejsce na dysku — usuń niepotrzebne pliki albo opróżnij kosz.
    2. Potrzeba co najmniej 3 GB wolnego miejsca.
    3. Możesz też przenieść cały folder launchera na inny dysk i uruchomić go
       stamtąd.

`PLIK-02` — Launcher nie ma prawa zapisywać w tym miejscu

:   System nie pozwala launcherowi zapisywać plików w folderze, w którym się
    znajduje.

    1. Przenieś folder z launcherem na Pulpit albo do folderu Dokumenty.
    2. Nie trzymaj launchera w „Program Files” ani na dysku systemowym poza
       folderem użytkownika.
    3. Uruchom launcher ponownie z nowego miejsca.

`PLIK-03` — Nie udało się zapisać pliku

:   Launcher nie mógł zapisać jednego z plików gry. Zwykle znaczy to, że plik
    jest w tej chwili używany przez inny program.

    1. Sprawdź, czy gra nie jest już uruchomiona — jeśli tak, zamknij ją.
    2. Zamknij launcher i uruchom go ponownie.
    3. Program antywirusowy potrafi blokować pliki gry — dodaj folder launchera
       do wyjątków.

!!! uwaga

    Brak miejsca launcher rozpoznaje po numerze błędu, który zwraca system
    (28 na Linuksie i macOS, 112 na Windowsie), a nie po treści komunikatu.
    Komunikaty systemowe są w różnych językach i zmieniają się między wersjami
    systemu, a numer jest zawsze ten sam. Dzięki temu `PLIK-01` nie zamienia się
    w ogólne „nie udało się zapisać pliku”.

    `PLIK-03` jest kodem zbiorczym: trafia tu każdy problem z zapisem, którego
    nie da się rozpoznać jako braku miejsca ani braku uprawnień.

## INST — instalacja gry i Javy

`INST-01` — Nie udało się rozpakować Javy

:   Launcher pobrał Javę potrzebną do uruchomienia gry, ale nie dał rady jej
    rozpakować. Zwykle znaczy to, że pobieranie zostało przerwane.

    1. Uruchom launcher ponownie — Java pobierze się jeszcze raz.
    2. Sprawdź, czy na dysku jest co najmniej 1 GB wolnego miejsca.
    3. Wyłącz na chwilę program antywirusowy i spróbuj ponownie.

`INST-01` — Java rozpakowała się niekompletnie

:   Launcher rozpakował Javę, ale nie znalazł w niej programu, który uruchamia
    grę.

    1. Usuń folder „data/java” z katalogu launchera.
    2. Uruchom launcher ponownie — Java pobierze się od nowa.

`INST-02` — Nie udało się uruchomić instalatora gry

:   Launcher nie mógł uruchomić programu instalującego modyfikacje do
    Minecrafta.

    1. Wyłącz na chwilę program antywirusowy — często blokuje takie programy.
    2. Wejdź w Ustawienia i kliknij „Napraw instalację”, potem uruchom launcher
       ponownie.

`INST-02` — Instalacja modyfikacji nie powiodła się

:   Program instalujący modyfikacje do Minecrafta zakończył pracę błędem.

    1. Wejdź w Ustawienia i kliknij „Napraw instalację”.
    2. Uruchom launcher ponownie — instalacja zacznie się od zera.
    3. Sprawdź, czy na dysku jest co najmniej 3 GB wolnego miejsca.

`INST-03` — Pliki gry są niekompletne

:   Launcher zainstalował grę, ale brakuje w niej pliku opisującego wersję.

    1. Wejdź w Ustawienia i kliknij „Napraw instalację”.
    2. Uruchom launcher ponownie — pliki gry pobiorą się od nowa.
    3. Twoje światy i ustawienia gry zostaną nietknięte.

`INST-03` — Spis plików gry jest uszkodzony

:   Launcher pobrał listę plików Minecrafta, ale nie potrafi jej odczytać.

    1. Wejdź w Ustawienia i kliknij „Napraw instalację”.
    2. Uruchom launcher ponownie.

`INST-04` — Nie da się uruchomić gry

:   Coś jest nie tak z plikami gry i launcher nie potrafi jej wystartować.

    1. Wejdź w Ustawienia i kliknij „Napraw instalację”.
    2. Uruchom launcher ponownie.

!!! wskazówka

    Całą rodzinę `INST-` launcher naprawia sam, więc zwykle jej nie zobaczysz —
    okno pojawia się dopiero po trzech nieudanych podejściach. Jeśli mimo to
    je widzisz, problem nie leży w plikach gry, bo te zostały już skasowane
    i pobrane od nowa.

## GRA — gra wystartowała i się zamknęła

Te kody dotyczą sytuacji, w której Minecraft już ruszył, ale zakończył się
szybko i nienormalnie. Launcher czyta wtedy koniec logu gry i na jego podstawie
dobiera kod.

`GRA-01` — Gra zamknęła się zaraz po uruchomieniu

:   Minecraft wystartował, ale zakończył się po kilku sekundach.

    1. Uruchom grę jeszcze raz — czasem wystarczy druga próba.
    2. Wejdź w Ustawienia i sprawdź, czy pamięć jest ustawiona na co najmniej
       4096 MB.
    3. Zaktualizuj sterowniki karty graficznej.
    4. Wejdź w Ustawienia i kliknij „Napraw instalację”.

`GRA-02` — Grze zabrakło pamięci

:   Ta paczka modów potrzebuje dużo pamięci. Tyle, ile jej przydzielono, nie
    wystarczyło.

    1. Wejdź w Ustawienia i kliknij „Dobierz automatycznie” przy suwaku pamięci.
    2. Zamknij przeglądarkę i inne programy przed uruchomieniem gry.
    3. Nie podnoś suwaka na siłę — gra zajmuje o półtora do dwóch gigabajtów
       więcej, niż on pokazuje, i przy zbyt wysokim ustawieniu system ją zamknie.

`GRA-03` — Java odrzuciła Twoje dodatkowe parametry

:   W Ustawieniach są wpisane dodatkowe parametry Javy, których Java nie
    rozumie, więc gra w ogóle nie wystartowała.

    1. Wejdź w Ustawienia i wyczyść pole „Dodatkowe parametry Javy”.
    2. Uruchom grę ponownie — powinna wystartować.
    3. Jeśli chcesz używać własnych parametrów, dodawaj je po jednym
       i sprawdzaj po każdym.

`GRA-04` — Jeden z modów spowodował błąd

:   Gra wystartowała, ale jedna z modyfikacji przerwała jej uruchamianie.
    To najczęściej problem z samą paczką, a nie z Twoim komputerem.

    1. Wejdź w Ustawienia i kliknij „Napraw instalację”, potem uruchom grę
       ponownie.
    2. Jeśli to nie pomoże, skopiuj szczegóły przyciskiem poniżej i wyślij je
       administracji.
    3. Prawdopodobnie ten sam błąd mają inni gracze i administracja już o nim wie.

`GRA-05` — Komputerowi zabrakło pamięci i zamknął grę

:   Gra nie zawiesiła się sama — to system ją zamknął, bo zabrakło mu pamięci
    na wszystko naraz. Zwykle znaczy to, że grze przydzielono jej za dużo:
    gra zajmuje o półtora do dwóch gigabajtów więcej, niż pokazuje suwak.

    1. Wejdź w Ustawienia i kliknij „Dobierz automatycznie” przy suwaku pamięci.
    2. Zamknij przeglądarkę i inne programy przed uruchomieniem gry.
    3. Jeśli to się powtarza, zejdź suwakiem jeszcze o 512 MB niżej.

`GRA-06` — Ten komputer ma za mało pamięci na tę paczkę

:   Grze zabrakło pamięci, a dostała już tyle, ile ten komputer może jej dać.
    Przydzielenie jej więcej odebrałoby pamięć systemowi i gra zostałaby
    zamknięta jeszcze wcześniej. To nie jest wina ustawień ani instalacji.

    1. Zamknij wszystkie inne programy — zwłaszcza przeglądarkę — i spróbuj raz
       jeszcze.
    2. Wejdź w „Paczki” i wyłącz shadery, jeśli są włączone. Na słabszej grafice
       zajmują bardzo dużo pamięci.
    3. Jeśli masz do wyboru inny komputer, zagraj na nim.
    4. Napisz do administracji i podaj ten kod — może przygotować lżejszą paczkę.

### Dlaczego są aż trzy kody o pamięci

`GRA-02`, `GRA-05` i `GRA-06` opisują trzy różne sytuacje i mają celowo różne
rady.

| Kod | Kto zamknął grę | Co z tym realnie da się zrobić |
| --- | --- | --- |
| `GRA-02` | Gra sama, bo skończyła jej się przydzielona pamięć, a suwak stoi niżej niż to, co komputer może dać | Dobrać pamięć automatycznie — jest jeszcze co zyskać |
| `GRA-05` | System, bo zabrakło pamięci jemu | Zejść z suwakiem niżej i zamknąć inne programy |
| `GRA-06` | Gra sama, ale suwak jest już na maksimum, jakie ten komputer bezpiecznie mieści | Odciążyć grę (shadery, inne programy) albo zagrać na innym komputerze |

Rozdzielenie `GRA-02` i `GRA-06` wzięło się z prawdziwego przypadku: komputer
z 8 GB pamięci i zintegrowaną grafiką, suwak już na maksimum, a mimo to gra
kończyła się brakiem pamięci. Rada „podnieś suwak” była tam ślepą uliczką —
wyżej system i tak zamknąłby grę. Bez osobnego kodu rodzic z dzieckiem
przesuwaliby suwak w kółko i wracali do tego samego.

!!! uwaga

    `GRA-05` launcher rozpoznaje **przed** czytaniem logu. Gdy system zamyka
    grę z braku pamięci, robi to sygnałem, którego nie da się przechwycić —
    zapis w logu urywa się w pół zdania i po samym logu wygląda to na
    przypadkową awarię. Gdyby launcher czytał najpierw log, stary i niezwiązany
    wpis o pamięci wskazałby zły kod.

    `GRA-03` pojawia się tylko wtedy, gdy w Ustawieniach naprawdę są wpisane
    własne parametry Javy. Bez nich ta sama treść w logu nie może obwiniać
    gracza o coś, czego nie ustawiał.

## KONTO — logowanie przez Microsoft

Więcej o samym logowaniu jest na stronie [Konta](konta.md). Przy każdym z tych
błędów da się na razie wejść do gry w trybie offline — przycisk jest na ekranie
logowania.

`KONTO-01` — Kod logowania wygasł

:   Kod jest ważny 15 minut. Ten już się przeterminował.

    1. Kliknij „Zaloguj przez Microsoft” jeszcze raz i wpisz nowy kod.

`KONTO-02` — To konto nie ma profilu Xbox

:   Minecraft wymaga profilu Xbox, a to konto Microsoft jeszcze go nie ma.

    1. Wejdź na xbox.com, zaloguj się tym kontem i załóż profil — to darmowe
       i zajmuje minutę.
    2. Wróć do launchera i zaloguj się ponownie.

`KONTO-03` — To konto dziecka i wymaga zgody rodzica

:   Microsoft blokuje logowanie kont dziecięcych, dopóki nie zostaną dodane do
    rodziny Microsoft.

    1. Rodzic powinien wejść na account.microsoft.com/family i dodać to konto
       do rodziny.
    2. Po dodaniu spróbuj zalogować się jeszcze raz.
    3. Do testów możesz na razie użyć trybu offline na ekranie logowania.

`KONTO-04` — To konto nie ma kupionego Minecrafta

:   Zalogowałeś się poprawnie, ale na tym koncie Microsoft nie ma gry Minecraft.

    1. Sprawdź, czy logujesz się na właściwe konto — to, na którym kupiliście grę.
    2. Jeśli w domu jest kilka kont Microsoft, spróbuj innego.
    3. Do testów możesz na razie użyć trybu offline na ekranie logowania.

`KONTO-05` — Microsoft odmówił logowania

:   Serwery Microsoftu odrzuciły logowanie i nie podały powodu, który launcher
    potrafi rozpoznać. Szczegóły w oknie są dla administracji — to z nich
    wynika, co poszło nie tak.

    1. Spróbuj zalogować się jeszcze raz za kilka minut.
    2. Sprawdź, czy możesz zalogować się na minecraft.net w przeglądarce.
    3. Skopiuj szczegóły przyciskiem poniżej i wyślij je administracji.
    4. Do testów możesz na razie użyć trybu offline na ekranie logowania.

`KONTO-05` — Logowanie nie powiodło się

:   Microsoft przerwał logowanie. Zdarza się, gdy okno logowania zostanie
    zamknięte albo kod wpisany źle.

    1. Kliknij „Zaloguj przez Microsoft” jeszcze raz.
    2. Użyj przycisku „Otwórz stronę logowania” — otworzy okno z już wpisanym
       kodem.
    3. Do testów możesz na razie użyć trybu offline na ekranie logowania.

`KONTO-06` — Xbox Live nie działa w kraju ustawionym na tym koncie

:   Konto Microsoft ma ustawiony kraj, w którym Xbox Live nie jest dostępny.
    To ustawienie samego konta, nie komputera ani gry.

    1. Wejdź na account.microsoft.com, otwórz „Twoje dane” i sprawdź kraj lub
       region.
    2. Po poprawieniu kraju zaloguj się w launcherze jeszcze raz.
    3. Do testów możesz na razie użyć trybu offline na ekranie logowania.

`KONTO-07` — Microsoft chce potwierdzenia na stronie konta

:   Zanim to konto zaloguje się do gry, Microsoft wymaga dokończenia czegoś na
    stronie konta — zwykle potwierdzenia wieku albo zgody rodzica.

    1. Wejdź na account.microsoft.com i zaloguj się tym samym kontem.
    2. Wykonaj to, o co poprosi strona, a potem wróć do launchera.
    3. Do testów możesz na razie użyć trybu offline na ekranie logowania.

`KONTO-08` — To konto jest zablokowane przez Microsoft

:   Microsoft zablokował temu kontu dostęp do Xbox Live. Launcher nie ma jak
    tego obejść.

    1. Wejdź na account.microsoft.com i sprawdź, czy Microsoft czegoś nie wymaga.
    2. Zaloguj się innym kontem, jeśli masz do niego dostęp.
    3. Do testów możesz na razie użyć trybu offline na ekranie logowania.

`KONTO-09` — Logowanie zostało odrzucone

:   Na stronie Microsoftu kliknięto „Nie” albo okno zostało zamknięte przed
    potwierdzeniem. Kod był dobry — zabrakło zgody.

    1. Kliknij „Zaloguj przez Microsoft” jeszcze raz.
    2. Na stronie Microsoftu potwierdź, że zgadzasz się zalogować — trzeba
       kliknąć „Tak”.
    3. Do grania na własnym świecie możesz na razie użyć trybu offline.

`KONTO-10` — Kod stracił ważność

:   Kod do wpisania na stronie Microsoftu jest jednorazowy i ważny tylko
    kilkanaście minut. Ten już się przeterminował albo został wcześniej użyty.

    1. Kliknij „Zaloguj przez Microsoft” jeszcze raz — dostaniesz nowy kod.
    2. Wpisz go od razu; nie odświeżaj strony z kodem i nie otwieraj jej dwa razy.
    3. Jeśli logujesz się z telefonu, miej launcher otwarty do końca — on czeka
       na potwierdzenie.

`KONTO-11` — Microsoft nie wpuszcza tego konta

:   Kod został wpisany poprawnie i jeszcze nie wygasł, ale Microsoft nie wydał
    dostępu. Tak odpowiada, gdy konto wymaga czegoś, czego okienko z kodem nie
    potrafi pokazać: potwierdzenia tożsamości, zgody rodzica albo akceptacji
    nowego regulaminu.

    1. Otwórz w przeglądarce account.microsoft.com i zaloguj się na to samo
       konto — Microsoft pokaże tam, czego mu brakuje.
    2. Jeśli to konto dziecka w rodzinie Microsoft, zgodę musi kliknąć rodzic
       ze swojego konta.
    3. Gdy przeglądarka przestanie o cokolwiek pytać, wróć do launchera
       i kliknij „Zaloguj przez Microsoft” jeszcze raz.
    4. Do tego czasu możesz grać w trybie offline — świat i postępy zostaną.

    **Ponawianie tutaj nie pomoże.** To jedyny kod z rodziny `KONTO`, przy
    którym kolejne kliknięcie „Zaloguj przez Microsoft” na pewno da ten sam
    wynik, dopóki nikt nie załatwi sprawy w przeglądarce.

`KONTO-12` — Zapisane logowanie wygasło

:   Launcher miał zapamiętane logowanie do tego konta, ale Microsoft już go nie
    przyjmuje. Dzieje się tak po zmianie hasła, po dłuższej przerwie w graniu
    albo gdy ktoś wylogował urządzenia w ustawieniach konta.

    1. Kliknij „Zaloguj przez Microsoft” — wystarczy zalogować się jeszcze raz.
    2. Nic nie przepadło: świat, ustawienia i paczka modów zostają na miejscu.
    3. Jeśli logowanie znów się nie uda, skopiuj szczegóły przyciskiem poniżej
       i wyślij je administracji.

!!! uwaga

    `KONTO-10`, `KONTO-11` i `KONTO-12` wyglądają w odpowiedzi Microsoftu tak
    samo — wszystkie trzy przychodzą jako `invalid_grant`. Launcher rozróżnia je
    po tym, **o co pytał i kiedy**, a nie po treści komunikatu:

    * przy odświeżaniu zapamiętanego logowania → `KONTO-12` (żadnego kodu wtedy
      na ekranie nie było, więc rady o przepisywaniu kodu byłyby bez sensu);
    * przy kodzie urządzenia, gdy termin ważności już minął → `KONTO-10`;
    * przy kodzie urządzenia, gdy termin jeszcze nie minął → `KONTO-11`, bo kod
      nie mógł wygasnąć, więc przyczyna leży po stronie konta.

!!! uwaga

    `KONTO-05` jest kodem zbiorczym i dostajesz go wtedy, gdy launcher nie
    rozpoznał powodu odmowy. Jeśli go widzisz, sama treść okna niewiele powie —
    warto wysłać administracji skopiowane szczegóły.

    W szczegółach jest teraz kod błędu **wraz z opisem od Microsoftu**, zwykle
    z numerem `AADSTS`. Wcześniej launcher pokazywał sam kod, więc gracz widział
    na przykład `invalid_grant` i nic poza tym — a to właśnie w opisie napisane
    jest, co konkretnie poszło nie tak.

    Kod dobierany jest po rozpoznanym powodzie odmowy, a nie po tym, co jest
    napisane w komunikacie. Dopóki launcher szukał w komunikacie słowa „Xbox”,
    zdanie „Xbox Live odrzucił logowanie, kod 2148916235” trafiało w radę
    o zakładaniu profilu Xbox, choć chodziło o `KONTO-06` — niedostępność usługi
    w danym kraju.

## USTAW — to, co sam wpisałeś w Ustawieniach

Te trzy kody są inne od wszystkich pozostałych: przyczyna nie leży ani
w launcherze, ani w plikach gry, tylko w tekście, który ktoś sam wpisał
w Ustawieniach. Rada „napraw instalację” byłaby tu myląca, bo instalacja jest
w porządku. Szczegóły ustawień opisuje strona [Ustawienia](ustawienia.md).

`USTAW-01` — Java wskazana w Ustawieniach nie działa

:   W Ustawieniach, w zakładce „Java”, wpisana jest własna ścieżka do Javy.
    Launcher nie potrafi jej użyć, więc gra nie ruszy.

    1. Wejdź w Ustawienia → Java i kliknij „Przywróć domyślne”. Launcher wróci
       wtedy do Javy, którą pobiera sam, i to zwykle załatwia sprawę.
    2. Jeśli chcesz zostać przy własnej Javie, kliknij obok pola „Sprawdź” —
       launcher powie, co jest z nią nie tak.

`USTAW-02` — Twoja komenda nie wykonała się poprawnie
    (albo: Komenda po zakończeniu gry nie wykonała się poprawnie)

:   Komenda, którą wpisałeś w Ustawieniach, zakończyła się błędem. Tytuł okna
    mówi, o który moment chodzi: komenda przed startem gry dostaje pierwszy
    tytuł, komenda po jej zakończeniu — drugi. Ta druga niczego już nie psuje,
    bo gra zdążyła się skończyć.

    1. Wejdź w Ustawienia → Zaawansowane i sprawdź wpisaną komendę. Szczegóły
       w oknie zawierają to, co sama wypisała.
    2. Wyczyść pole, jeśli nie wiesz, skąd się tam wzięło — puste jest bezpieczne
       i niczego nie psuje.
    3. Jeśli chodzi o komendę przed startem: dopóki się nie wykona, launcher nie
       uruchomi gry. Tak ma być, bo komendy przed startem zwykle coś przygotowują.

`USTAW-03` — Twoja komenda się zawiesiła

:   Komenda z Ustawień nie skończyła się w wyznaczonym czasie, więc launcher ją
    przerwał. Okno podaje, ile sekund czekał. Najczęściej znaczy to, że komenda
    na coś czeka — na hasło albo na wciśnięcie klawisza — a nie ma jak o to
    zapytać.

    1. Wejdź w Ustawienia → Zaawansowane i sprawdź wpisaną komendę.
    2. Wyczyść pole, jeśli nie wiesz, skąd się tam wzięło.

!!! wskazówka

    Komenda „po zakończeniu gry” niczego już nie psuje — gra się skończyła,
    zanim komenda zawiodła. Dlatego okno mówi wtedy spokojniej i nie straszy
    tym, że coś się nie uruchomi.

    Jeśli nigdy nie wpisywałeś nic w zakładce „Zaawansowane”, tych kodów nie
    zobaczysz. Puste pola są bezpieczne.

## PACZKA — opis paczki modów z serwera

`PACZKA-01` — Nie udało się odczytać informacji o paczce

:   Launcher pobrał opis paczki modów, ale nie potrafi go zrozumieć. To błąd po
    stronie serwera, nie Twój.

    1. Poczekaj kilka minut i uruchom launcher ponownie.
    2. Sprawdź, czy masz najnowszą wersję launchera.
    3. Jeśli błąd się powtarza, zgłoś go administracji.

`PACZKA-02` — Paczka modów zawiera błąd

:   Opis paczki modów jest nieprawidłowy i launcher odmówił jego użycia. To błąd
    po stronie serwera, nie Twój.

    1. Poczekaj chwilę i uruchom launcher ponownie — administracja mogła właśnie
       wgrywać zmiany.
    2. Jeśli błąd się powtarza, zgłoś go administracji.

!!! wskazówka

    Oba te kody znaczą, że coś jest nie tak z opisem paczki leżącym na serwerze.
    Kasowanie plików gry ani ponowna instalacja nic tu nie dadzą — pomoże tylko
    poprawka po stronie administracji. Jeśli `PACZKA-02` pojawia się przez chwilę
    i mija, prawdopodobnie trafiłeś na moment wgrywania nowej wersji paczki.

## INNY — błąd bez własnego opisu

`INNY-01` — Coś poszło nie tak, ale nie wiemy co

:   Launcher natknął się na problem, którego nie potrafi rozpoznać. To znaczy,
    że trafiłeś na coś naprawdę rzadkiego.

    1. Uruchom launcher ponownie — czasem to wystarcza.
    2. Wejdź w Ustawienia i kliknij „Napraw instalację”.
    3. Skopiuj szczegóły przyciskiem poniżej i wyślij je administracji. Ten błąd
       nie ma jeszcze własnego opisu, więc Twoje zgłoszenie realnie pomoże go
       dodać.

`INNY-01` jest siatką bezpieczeństwa. Jeśli się pojawia, znaczy to, że
w katalogu błędów brakuje kategorii na to, co się właśnie stało — i dlatego
skopiowane szczegóły są w tym jednym przypadku ważniejsze niż przy każdym innym
kodzie.

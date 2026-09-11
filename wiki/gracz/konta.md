# Konta

Żeby zagrać, launcher musi wiedzieć, kim jesteś. Służy do tego konto.
Launcher pamięta całą listę kont naraz, więc jeśli na jednym komputerze gra
rodzeństwo, każde ma swoje i przełącza się jednym kliknięciem.

## Dwa rodzaje kont

| Rodzaj | Co jest potrzebne | Gdzie zadziała |
| --- | --- | --- |
| Konto Microsoft | zalogowanie się na stronie Microsoftu kodem z launchera | wszędzie, także na serwerze Chmurki |
| Konto offline | sam nick | tylko na serwerze ustawionym na `online-mode=false` |

Konto offline jest w launcherze po to, żeby dało się sprawdzić paczkę bez
logowania. Nie zastępuje kupionego Minecrafta i nie wpuści na zwykły serwer.

## Gdzie się dodaje i przełącza konta

Na głównym ekranie, w prawym górnym rogu, jest przycisk z nickiem osoby, która
gra teraz. Zanim dodasz pierwsze konto, pisze na nim **Nie zalogowano**.

- Kliknięcie tego przycisku otwiera ekran **Konta** — listę wszystkich
  zapamiętanych kont.
- Gdy nie ma jeszcze żadnego konta, launcher pomija listę i przechodzi od razu
  do dodawania. Pusta lista z prośbą o kliknięcie jeszcze raz niczego by nie
  załatwiła.
- Duży przycisk na środku ekranu pisze **ZALOGUJ SIĘ**, dopóki nie ma wybranego
  konta, a gdy jest — zmienia się w **GRAJ**.

## Dodanie konta Microsoft — logowanie kodem urządzenia

Launcher nie prosi o hasło do konta Microsoft i nie ma gdzie go wpisać. Zamiast
tego pokazuje krótki kod, który przepisuje się na stronie Microsoftu — hasło
wpisuje się tam, w przeglądarce, na stronie Microsoftu.

Krok po kroku:

1. Na ekranie **Konta** kliknij **Dodaj konto Microsoft**.
2. Launcher pokaże duży kod, na przykład `V3REVW36`, a pod nim adres strony,
   na którą trzeba go wpisać. W odpowiedziach, na których był sprawdzany, jest
   to `https://www.microsoft.com/link`.
3. Kliknij **Otwórz stronę logowania**. Launcher otworzy osobne, małe okno
   przeglądarki — bez pasków kart i adresu, wyłącznie ze stroną Microsoftu.
   Okno wychodzi na pierwszy plan, więc nie da się go przeoczyć wśród innych
   kart. Kod jest już w adresie, więc nie trzeba go nigdzie wpisywać.
4. W tym oknie wybierz konto Microsoft, na którym kupiliście Minecrafta,
   zaloguj się i potwierdź.

!!! wskazówka "Gdy Microsoft wchodzi na złe konto"

    Jeśli w przeglądarce jest już zalogowane jakieś konto Microsoft, strona
    logowania wejdzie na nie **bez pytania** — nie pokaże listy do wyboru.
    Na taki wypadek jest drugi przycisk: **Zaloguj na inne konto**.

    Otwiera on okno z czystym, osobnym profilem przeglądarki. Nie ma tam
    żadnej zapamiętanej sesji, więc Microsoft zawsze zapyta, na które konto
    się logujesz. Trzeba wtedy wpisać hasło — zapamiętane hasła z Twojej
    zwykłej przeglądarki tam nie sięgają.

    Przy okazji tylko to okno trzyma się zadanego rozmiaru. Zwykłe przejmuje
    geometrię po już otwartej przeglądarce i potrafi wyjść na całą wysokość
    ekranu — to ograniczenie samych przeglądarek, nie launchera.
5. Wróć do launchera. Pod kodem kręci się kółko i napis „Czekam na
   potwierdzenie…". Gdy Microsoft potwierdzi, launcher sam przejdzie na główny
   ekran, a konto trafi na listę.

!!! wskazówka

    Kod nadal jest widoczny i nadal da się go zaznaczyć myszą — przydaje się,
    gdy ktoś loguje się z telefonu, wchodząc na `microsoft.com/link` ręcznie.
    Jest tak zrobiony celowo: schowek systemowy potrafi odmówić (na Windowsie
    zdarza się to najczęściej) i wtedy zaznaczenie kodu jest jedyną drogą do
    skopiowania go.

!!! uwaga "Gdy zamiast okienka otworzy się zwykła karta"

    Osobne okno potrafią otworzyć przeglądarki z rodziny Chromium: Chrome,
    Edge, Brave, Vivaldi, Opera. Przycisk **Otwórz stronę logowania** używa
    wyłącznie **Twojej domyślnej przeglądarki** — na Linuksie odczytanej
    z ustawień pulpitu, na Windowsie z rejestru systemu.

    Gdy domyślną przeglądarką jest Firefox, osobnego okna nie będzie: Firefox
    nie ma takiego trybu. Strona otworzy się wtedy jako zwykła karta —
    **w Twoim Firefoksie**, a nie w przypadkowej innej przeglądarce, którą
    ktoś ma obok zainstalowaną. To celowe: logowanie ma się odbywać tam, gdzie
    masz swoje zapamiętane hasła.

    Przycisk **Zaloguj na inne konto** ma inne zadanie i zachowuje się inaczej:
    startuje z czystego profilu, w którym i tak nie ma żadnych haseł ani sesji,
    więc bierze dowolną dostępną przeglądarkę z rodziny Chromium.

### Ile czasu jest na wpisanie kodu

Kod jest ważny 15 minut — tyle daje Microsoft. Launcher pyta jego serwery co
kilka sekund, czy już potwierdziłeś, i pilnuje terminu. Po jego upływie przerywa
czekanie błędem `KONTO-01`; wtedy trzeba kliknąć **Dodaj konto Microsoft**
jeszcze raz i dostać nowy kod.

Dwie rzeczy dzieją się w tle i warto o nich wiedzieć, bo wyglądają jak
bezczynność:

- Gdy Microsoft poprosi o wolniejsze pytanie, launcher wydłuża przerwy między
  próbami (najwyżej do minuty). Ignorowanie takiej prośby kończy się
  zablokowaniem całej sesji logowania, więc launcher czeka dłużej zamiast
  pytać częściej.
- Chwilowa awaria internetu nie przerywa logowania. Masz wpisany kod na stronie
  Microsoftu i nie musisz zaczynać od nowa tylko dlatego, że Wi-Fi mrugnęło.

### Co się dzieje po potwierdzeniu

Po potwierdzeniu launcher wykonuje cztery zapytania pod rząd: do Xbox Live, do
usługi XSTS, do logowania Minecrafta i na koniec po profil gracza. Dopiero
z ostatniego bierze nick i identyfikator gracza (UUID). Dlatego konto pojawia się
na liście z prawdziwym nickiem, a nie z adresem e-mail.

Jeśli ostatnie zapytanie odpowie „nie ma takiego profilu", znaczy to, że konto
Microsoft jest poprawne, ale nie ma na nim kupionego Minecrafta. Launcher mówi
to wprost, kodem `KONTO-04`.

### Gdy logowanie się nie uda

Każda odmowa dostaje własny kod i własną radę. To nie są ogólniki — launcher
rozpoznaje powód po tym, co odpowiedział Xbox Live, a nie po tym, jak wygląda
komunikat.

| Kod | Co się stało |
| --- | --- |
| `KONTO-01` | kod logowania wygasł |
| `KONTO-02` | konto Microsoft nie ma profilu Xbox |
| `KONTO-03` | konto dziecka, które nie należy do rodziny Microsoft |
| `KONTO-04` | na tym koncie nie ma kupionego Minecrafta |
| `KONTO-05` | Microsoft odmówił i nie podał powodu, który launcher rozpoznaje |
| `KONTO-06` | Xbox Live nie działa w kraju ustawionym na koncie |
| `KONTO-07` | Microsoft wymaga dokończenia czegoś na stronie konta |
| `KONTO-08` | konto zablokowane przez Microsoft |

Co zrobić przy każdym z nich, opisuje strona [Kody błędów](kody-bledow.md).

## Dodanie konta offline

Na ekranie **Konta**, w części „Dodaj konto", jest pole na nick i przycisk
**Dodaj**. Nick wystarczy — nie ma tu hasła ani logowania.

- Przycisk jest wyszarzony, dopóki pole jest puste.
- Jest też wyszarzony, gdy konto offline o tym nicku już jest na liście.
  Dodanie go drugi raz niczego by nie zmieniło, więc launcher mówi o tym
  wprost zamiast udawać, że coś się stało.

Przy pierwszym uruchomieniu, gdy nie ma jeszcze żadnego konta, to samo pole
z nickiem stoi na ekranie dodawania konta, pod przyciskiem logowania przez
Microsoft — obok niego jest przycisk **Graj**, który dodaje konto offline
i wraca na główny ekran.

Konto offline ma identyfikator gracza wyliczany z samego nicku, dokładnie tak
samo jak liczy go serwer Minecrafta w trybie offline. Dzięki temu ten sam nick
to zawsze ten sam gracz: postępy i rzeczy w ekwipunku zostają na miejscu, także
po usunięciu i ponownym dodaniu konta.

## Ile kont naraz

Launcher nie ogranicza liczby kont. Można mieć obok siebie kilka kont Microsoft
i kilka offline.

Konto rozpoznawane jest po rodzaju i nicku, co daje trzy praktyczne skutki:

- **Konto Microsoft i offline o tym samym nicku to dwa osobne konta.** Tak
  właśnie sprawdza się paczkę przed wpuszczeniem na nią dzieci.
- **Wielkość liter nie tworzy drugiego konta.** `Zosia` i `zosia` to ten sam
  wpis.
- **Ponowne zalogowanie na to samo konto Microsoft odświeża istniejący wpis**,
  zamiast dokładać drugi taki sam. Bez tego lista rosłaby po każdym logowaniu.

## Przełączanie kont

Na liście każde konto ma swój kafelek z nickiem i podpisem „konto Microsoft"
albo „konto offline". Przy koncie używanym teraz stoi napis **● używane teraz**;
przy pozostałych jest przycisk **Przełącz**.

| Rodzaj konta | Co się dzieje po kliknięciu „Przełącz" |
| --- | --- |
| offline | przełączenie jest natychmiastowe, launcher wraca na główny ekran |
| Microsoft | launcher odświeża w tle logowanie w Microsofcie; przez tę chwilę jest zajęty i przycisk GRAJ nie działa |

Przy koncie Microsoft nie trzeba nic wpisywać — launcher zapamiętał to, co
pozwala mu wrócić na konto bez kodu. Gdyby odświeżenie się nie udało, zobaczysz
okno błędu z kodem `KONTO-…`.

To samo dzieje się przy każdym starcie launchera: konto offline wraca od ręki,
a konto Microsoft odświeża się w tle. Gdy odświeżenie się nie uda, launcher
niczego nie krzyczy: zadanie w tle po prostu kończy się po cichu, a Ty widzisz
ekran główny z przyciskiem **ZALOGUJ SIĘ**. To co innego niż ręczne przełączenie
konta — tam nieudane logowanie pokazuje okno błędu z kodem.

## „Zapomnij" i wylogowanie

Przy każdym koncie na liście jest przycisk **Zapomnij**. Usuwa on konto
z launchera i nic poza tym: światy, ustawienia gry i paczka modów zostają na
dysku nietknięte.

Gdy zapomnisz konto, którego właśnie używasz, launcher przechodzi na pierwsze
z pozostałych. Nikt nie zostaje z niczym po skasowaniu jednego z kilku kont.

W **Ustawieniach**, na zakładce **Ogólne**, jest jeszcze przycisk **Wyloguj
wszystkie konta**. Kasuje on całą listę naraz — używa się go, gdy komputer
zmienia właściciela. Sekcja z tym przyciskiem pokazuje się tylko wtedy, gdy
ktoś jest zalogowany.

!!! uwaga

    Wylogowanie nie kasuje niczego z gry. Po ponownym zalogowaniu na to samo
    konto wszystko jest tam, gdzie było. Do usuwania plików służą inne przyciski,
    opisane na stronie [Ustawienia](ustawienia.md).

## Gdzie są zapamiętane konta

Lista kont leży w pliku `auth.json` w katalogu z danymi launchera:

```text
Windows:  %LOCALAPPDATA%\ChmurkowyLauncher\data\auth.json
Linux:    ~/.local/share/chmurkowy-launcher/data/auth.json
```

Pełny opis, co jeszcze leży w tym katalogu, jest na stronie
[Gdzie leżą dane](../budowa/dane.md).

Przy koncie Microsoft plik zawiera to, co pozwala launcherowi wrócić na konto
bez pytania o kod. Dlatego na Linuksie zapisuje się go z uprawnieniami tylko dla
właściciela (`0600`); na Windowsie chroni go samo położenie w katalogu
użytkownika.

Dwie rzeczy dzieją się z tym plikiem po cichu i tak ma być:

- **Uszkodzony plik nie blokuje launchera.** Lista wczytuje się wtedy jako
  pusta i wystarczy zalogować się ponownie. Launcher, który nie startuje przez
  zepsuty plik, byłby gorszy od launchera, który prosi o ponowne logowanie.
- **Aktualizacja launchera nikogo nie wylogowuje.** Starszy format pliku,
  z jednym kontem Microsoft, wczytuje się dalej i zamienia w jednoelementową
  listę.

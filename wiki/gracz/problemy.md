# Gdy gra nie startuje

Zanim cokolwiek zrobisz: **zwykle nie trzeba robić nic**. Launcher najpierw
próbuje naprawić rzecz samodzielnie i pokazuje błąd dopiero wtedy, gdy to nie
pomogło. Ta strona mówi, po czym poznać, że akurat to robi, oraz co zrobić,
gdy mimo wszystko pojawi się czerwony ekran.

## Najpierw launcher próbuje sam

Rady w rodzaju „wejdź w Ustawienia i kliknij Napraw instalację" są poprawne,
tylko że nikt ich nie wykonuje — dla dziesięciolatka to ściana tekstu. Dlatego
launcher wykonuje je za gracza.

Po kliknięciu GRAJ launcher ma **trzy podejścia**. Gdy któreś się nie powiedzie,
patrzy na kod błędu, sam usuwa to, co jest podejrzane, i zaczyna od nowa.
Dopiero trzecia nieudana próba kończy się ekranem błędu.

| Co się zepsuło | Co launcher robi sam | Co widzisz na ekranie |
| --- | --- | --- |
| Sieć albo zapis pliku | Czeka dwie sekundy i próbuje jeszcze raz, niczego nie kasując | „Coś się nie udało. Próbuję jeszcze raz…" |
| Java pobrała się niekompletnie | Kasuje pobraną Javę i bierze ją od nowa | „Coś się nie udało. Pobieram Javę od nowa…" |
| Pliki gry są niekompletne albo popsute | Kasuje pliki gry i instaluje je od nowa | „Coś się nie udało. Pobieram pliki gry od nowa…" |

W rozwijanych **szczegółach** na dolnym pasku pojawia się przy tym linijka
z kodem błędu i numerem podejścia, na przykład `(kod SIE-02, podejście 2 z 3)`.

Samonaprawa nie dotyka wszystkiego. Pełny dysk, brak uprawnień, wygasłe
logowanie, padnięta gra i wszystko, co sam wpisałeś w Ustawieniach — te błędy
trafiają prosto na ekran błędu. Powtórka nic by tu nie zmieniła, a kasowanie
plików tylko by zaszkodziło: gdyby launcher ponawiał własną komendę gracza
„zrób kopię świata", wykonałaby się trzy razy pod rząd.

!!! uwaga

    Gdy gra **już wystartowała**, launcher nie ponawia niczego, choćby błąd był
    z listy powyżej. Drugie okno Minecrafta byłoby gorsze od każdego błędu.

### Czego samonaprawa nie skasuje

Kasowane są wyłącznie pliki, które launcher potrafi pobrać z powrotem:
pobrana Java oraz pliki gry. **Twoje światy, ustawienia gry i paczka modów
leżą w osobnym katalogu i nikt ich nie rusza.**

## Ekran błędu

Gdy nie udało się nic uratować, launcher pokazuje ekran „Coś poszło nie tak".
Jeśli okno było schowane w zasobniku, wraca samo — inaczej zobaczyłbyś tylko
znikającą grę i nic poza tym.

Na ekranie są, w tej kolejności:

1. **Kod błędu** (np. `GRA-05`) i tytuł. Kod jest krótki po to, żeby dało się
   go podyktować przez telefon.
2. Jedno zdanie o tym, co się stało, bez żargonu.
3. Lista **Co zrobić** — konkretne kroki, ponumerowane.
4. Zwinięte **Szczegóły techniczne**. To materiał dla administracji serwera,
   nie do czytania.

Na dole są dwa przyciski: **Spróbuj ponownie** (wraca na ekran główny) oraz
**Kopiuj szczegóły dla administracji**.

!!! wskazówka

    Co znaczą poszczególne kody i co zrobić przy każdym z nich, opisuje strona
    [Kody błędów](kody-bledow.md). Tu jest tylko to, co robi się niezależnie
    od kodu.

Jedna sytuacja warta osobnej uwagi: gdy gra się wysypie, launcher **nie
zamyka się** nawet wtedy, gdy w Ustawieniach włączono zamykanie po grze.
Zamknięcie się w takiej chwili zabrałoby Ci jedyne wyjaśnienie, jakie masz.

## Przycisk „Napraw instalację"

Gdy ekran błędu każe naprawić instalację albo gra zachowuje się dziwnie mimo
braku błędu:

1. Kliknij **Ustawienia** w prawym górnym rogu.
2. W zakładce **Ogólne** zjedź do sekcji **NAPRAWA**.
3. Kliknij **Napraw instalację**.
4. Kliknij **GRAJ** — pliki gry pobiorą się od nowa.

Przycisk kasuje pliki gry: Minecrafta, NeoForge i pobrane biblioteki. **Światy,
ustawienia gry i mody zostają nietknięte** — leżą w osobnym katalogu.
Po skasowaniu launcher pokazuje komunikat „Pliki gry usunięte. Pobiorą się
ponownie przy następnym starcie".

Gdy gry nie ma jeszcze na dysku, przycisk jest wyszarzony i po najechaniu
myszą tłumaczy, dlaczego: nie ma czego naprawiać.

!!! uwaga

    Naprawa oznacza ponowne pobranie większości tego, co schodziło przy
    pierwszym uruchomieniu. Przy wolnym łączu to znowu kilkanaście minut,
    więc nie jest to przycisk „na wszelki wypadek".

## Gdzie leży log gry

Wszystko, co gra wypisuje, launcher zapisuje do pliku `logs/game.log`
w swoim katalogu danych:

```text
Windows:  %LOCALAPPDATA%\ChmurkowyLauncher\data\logs\game.log
Linux:    ~/.local/share/chmurkowy-launcher/data/logs/game.log
```

Nie trzeba tego wpisywać ręcznie. Są dwa przyciski, które otwierają to miejsce
za Ciebie:

- **Ustawienia → Ogólne → PLIKI → Otwórz log gry** — otwiera sam plik.
- **Co się dzieje → Otwórz folder z logiem** — otwiera folder, w którym leży.

Oba są wyszarzone, dopóki log nie powstanie, czyli do pierwszego uruchomienia
gry.

!!! uwaga

    Launcher tworzy `game.log` **od nowa przy każdym uruchomieniu gry**.
    Jeśli gra się wysypała, a Ty klikniesz GRAJ jeszcze raz, poprzedni log
    zniknie. Skopiuj albo wyślij go, zanim spróbujesz ponownie.

## Jak skopiować zgłoszenie dla administracji

Zależnie od tego, co widzisz na ekranie:

**Gdy jest ekran błędu** — kliknij **Kopiuj szczegóły dla administracji**.
Do schowka trafia gotowy tekst: wersja launchera, kod błędu, tytuł
i szczegóły techniczne. Przy padniętej grze szczegóły zawierają już ostatnich
60 linii logu, więc w wielu sprawach to wystarczy.

**Gdy gra po prostu stoi albo zniknęła bez błędu** — wejdź na ekran **Co się
dzieje** i kliknij **Kopiuj log**. Do schowka trafia to, co widać w oknie,
czyli ostatnie 800 linii.

Skopiowany tekst wklej w wiadomości do administracji serwera. Jeśli kopiowanie
się nie uda (na Windowsie schowek bywa przez ułamek sekundy zajęty przez inny
program), launcher powie o tym wprost i podpowie, co zrobić zamiast tego:
zrobić zdjęcie okna błędu albo wysłać sam plik `game.log`. Napis „Skopiowano."
jest jedynym potwierdzeniem, że tekst naprawdę jest w schowku.

### Co warto dołączyć

| Co | Skąd |
| --- | --- |
| Kod błędu | Wielki napis u góry ekranu błędu, np. `INST-02` |
| Szczegóły techniczne | Przycisk „Kopiuj szczegóły dla administracji" |
| Log gry | Przycisk „Kopiuj log" albo plik `game.log` |
| Co robiłeś przed błędem | Jedno zdanie własnymi słowami |

Administracja prowadzi serwer po godzinach, więc odpowiedź może zająć dzień
lub dwa. To normalne — nikt o Tobie nie zapomniał.

## Zanim napiszesz — trzy rzeczy do sprawdzenia

1. **Czy internet działa?** Otwórz dowolną stronę w przeglądarce. Duża część
   błędów to zerwane pobieranie.
2. **Czy na dysku jest miejsce?** Gra z modami zajmuje około 2 GB, a launcher
   potrzebuje co najmniej 3 GB wolnego miejsca na sam proces pobierania.
3. **Czy gra nie jest już uruchomiona?** Drugi, niewidoczny proces gry blokuje
   pliki i nie pozwala zapisać nowych.

Co dokładnie dzieje się przy uruchamianiu i na co patrzeć na ekranie „Co się
dzieje", opisuje strona [Pierwsze uruchomienie](pierwsze-uruchomienie.md).

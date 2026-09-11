# Chmurkowy Launcher

Launcher paczki modów Chmurkowego Serwera — Minecraft 1.21.1 z NeoForge.

Instalujesz raz, a potem launcher sam pobiera Javę, instaluje grę, utrzymuje
paczkę modów w zgodzie z serwerem i aktualizuje sam siebie. Gra uruchamia się
w jego własnym katalogu i nie dotyka `~/.minecraft` ani innych instalacji.

Ta dokumentacja jest podzielona według tego, po co się do niej sięga.

## Gram i coś nie działa

[Dla gracza](gracz/index.md) — instalacja na Windowsie i Linuksie, co się dzieje
przy pierwszym uruchomieniu, konta, ustawienia i co robić, gdy gra nie startuje.
Jest tam też [pełny katalog kodów błędów](gracz/kody-bledow.md): każdy kod, który
launcher potrafi pokazać, wraz z tym, co znaczy i co z nim zrobić.

## Przejmuję ten projekt

[Jak to działa](budowa/index.md) — podział na trzy crate'y, droga od kliknięcia
GRAJ do uruchomionej gry, format manifestu, synchronizacja paczki, samoaktualizacja
i miejsca, w których leżą dane.

Zacznij od [architektury](budowa/architektura.md), a potem przeczytaj
[przebieg uruchomienia](budowa/przebieg-uruchomienia.md). Te dwie strony tłumaczą
większość decyzji, które w kodzie wyglądają na dziwne.

## Opiekuję się paczką modów

[Paczka modów](paczka/index.md) — jak dodać albo usunąć moda, jak przebudować
manifest narzędziem `chmurka` i dlaczego kilka modów serwujemy z własnego
repozytorium zamiast prosto z CurseForge czy Modrinth.

## Wypuszczam nową wersję

[Praca nad projektem](projekt/index.md) — wydanie launchera krok po kroku,
kontrole uruchamiane na pull requestach, ochrona gałęzi i wymóg podpisanych
commitów.

---

## Zasada, która obowiązuje w całym projekcie

Przyczynę trzeba udowodnić, a nie zgadnąć.

Kilka najdroższych pomyłek w historii tego launchera wzięło się z poprawiania
czegoś, czego nikt wcześniej nie zmierzył: limit pamięci metaprzestrzeni miał
zapobiec jej rośnięciu, a zawiesił grę w trakcie zabawy; własne nastawy
odśmiecacza miały pomóc słabemu komputerowi, którego problem leżał zupełnie
gdzie indziej. Oba pomysły były rozsądne i oba szkodziły.

Dlatego komentarze w kodzie tłumaczą **dlaczego**, a nie tylko co, a testy pilnują
konkretnych zdarzeń, które kiedyś naprawdę się wydarzyły. Ta dokumentacja trzyma
się tego samego: jeśli coś jest tu napisane, wynika z kodu, a nie z pamięci.

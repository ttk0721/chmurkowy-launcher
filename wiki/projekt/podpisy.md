# Podpisywanie commitów

Commity muszą być podpisane kluczem GPG, a adres autora commita musi
występować w tym kluczu. Nie wystarczy jedno z dwóch: podpis bez zgodnego
adresu GitHub odrzuca tak samo jak brak podpisu.

## Ustawienia w repozytorium

```bash
git config user.signingkey <identyfikator klucza>
git config commit.gpgsign true
git config user.email <adres z klucza>
```

`commit.gpgsign true` sprawia, że podpisywany jest każdy commit, bez
dopisywania `-S` za każdym razem. `user.email` decyduje o tym, co wyląduje
w commicie jako adres autora — i to właśnie ten adres GitHub będzie porównywał
z kluczem.

!!! uwaga

    Ustawienie zapisane w repozytorium przykrywa ustawienie globalne. Jeśli
    gdzieś indziej masz inny adres, to liczy się ten z repozytorium, w którym
    właśnie commitujesz. Skąd pochodzi wartość, pokaże:

    ```bash
    git config --show-origin --get user.email
    ```

## Jak GitHub weryfikuje podpis

GitHub robi dwie rzeczy, jedną po drugiej:

1. **Sprawdza sam podpis.** Bierze klucze publiczne wgrane na konto i szuka
   takiego, którym da się zweryfikować podpis pod obiektem commita.
2. **Porównuje adresy.** Czyta adres autora z commita i sprawdza, czy taki
   adres jest wśród tożsamości (UID) tego klucza.

Drugi krok bywa zaskoczeniem, bo kryptograficznie nie jest do niczego
potrzebny — podpis jest poprawny albo nie, niezależnie od tego, co stoi
w polu autora. Chodzi o to, co GitHub tym znaczkiem twierdzi. „Verified”
nie znaczy „ktoś podpisał ten commit”, tylko „ten commit pochodzi od tej
osoby”. Gdyby adres nie musiał się zgadzać, dałoby się podpisać własnym
kluczem commit przypisany komuś innemu i dostać za to znaczek
potwierdzenia — czyli dokładnie odwrotność tego, po co znaczek istnieje.

Dodatkowo adres z klucza musi być potwierdzonym adresem na koncie GitHuba.
Bez tego nie ma jak połączyć podpisu z kontem, nawet gdy wszystko inne się
zgadza.

## Objaw niezgodności: `bad_email`

Gdy podpis jest poprawny, ale adres autora nie występuje w kluczu, GitHub
oznacza taki commit statusem `bad_email`. W interfejsie widać przy nim
„Unverified”, a nie „Verified”. Dla ochrony gałęzi to znaczy tyle samo, co
brak podpisu: zmiana nie zostaje przyjęta.

Pułapka polega na tym, że lokalnie wszystko wygląda dobrze. `git` podpisał
commit i nie zgłosił żadnego problemu, `gpg` też nie ma nic do powiedzenia —
niezgodność ujawnia się dopiero po stronie GitHuba, przy próbie wypchnięcia
albo scalenia.

Najczęstsza przyczyna to adres zastępczy GitHuba
(`<numer>+<login>@users.noreply.github.com`). Ktoś ustawia go jako
`user.email`, żeby nie pokazywać prywatnego adresu w publicznej historii,
ale w kluczu GPG ma tylko ten prywatny. Wtedy commit jest podpisany, adres
jest poprawny, konto się zgadza — a mimo to wychodzi `bad_email`, bo klucz
nic nie wie o adresie zastępczym.

## Jak to sprawdzić

Adres, którym podpiszesz następny commit:

```bash
git config --get user.email
```

Adresy zapisane w kluczu (wiersze `uid`):

```bash
gpg --list-keys <identyfikator klucza>
```

Podpis pod ostatnim commitem, razem z komunikatem `gpg`:

```bash
git log --show-signature -1
```

Kilka ostatnich commitów naraz — status podpisu i adres autora w jednej
linijce (`G` znaczy podpis dobry, `N` brak podpisu, `B` zły):

```bash
git log --format='%h %G? %ae %s' -5
```

Co o commicie sądzi sam GitHub — pole `reason` przyjmuje tam między innymi
wartość `bad_email`:

```bash
gh api repos/ttk0721/chmurkowy-launcher/commits/<sha> --jq .commit.verification
```

## Jak to naprawić

Są dwie drogi i wybór zależy od tego, który adres ma zostać.

### Adres w repozytorium ma się dopasować do klucza

Najprostsza droga, gdy klucz jest w porządku, a tylko to repozytorium ma
ustawiony inny adres:

```bash
git config user.email <adres, który jest w kluczu>
```

### Klucz ma się dopasować do adresu

Gdy adres ma zostać taki, jaki jest (na przykład zastępczy adres GitHuba),
trzeba go dołożyć do klucza jako kolejną tożsamość:

```bash
gpg --quick-add-uid <odcisk klucza> "Imię <adres>"
gpg --armor --export <identyfikator klucza>
```

Wynik eksportu wgrywa się na konto: GitHub → Settings → SSH and GPG keys.
Jeśli klucz o tym odcisku już tam jest, usuń go najpierw i dodaj wersję
z nową tożsamością — GitHub nie dociąga zmian w kluczu sam z siebie, zna
tylko to, co zostało wgrane.

### Commity, które już powstały

Adres autora jest zapisany w commicie w chwili jego utworzenia. Zmiana
ustawień nie rusza tego, co już jest — stare commity trzeba odtworzyć.

Ostatni commit:

```bash
git commit --amend --reset-author --no-edit -S
```

`--reset-author` podstawia autora z aktualnej konfiguracji, `-S` podpisuje
na nowo. Gdy wadliwych commitów jest więcej, to samo na całej gałęzi
względem `main`:

```bash
git rebase --exec 'git commit --amend --reset-author --no-edit -S' main
```

!!! wskazówka

    To przepisuje historię gałęzi, więc wypchnięcie wymaga
    `git push --force-with-lease`. Na gałęzi roboczej własnego pull requesta
    jest to bezpieczne — `--force-with-lease` odmówi, gdyby w międzyczasie
    ktoś coś tam dołożył. Na `main` i tak się nie uda, bo gałąź jest
    chroniona.

Po scaleniu przez **Squash and merge** commit na `main` tworzy GitHub
i podpisuje go własnym kluczem. Dlatego akurat ten sposób scalania nie kłóci
się z wymogiem podpisów — inaczej niż „Rebase and merge”, opisany
w [Kontrolach i ochronie gałęzi](kontrole.md).

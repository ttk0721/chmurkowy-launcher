#!/usr/bin/env python3
"""Generuje ikonę launchera.

Skrypt trzymamy w repozytorium, ale gotowe pliki też — dzięki temu budowanie
w CI nie potrzebuje Pythona ani bibliotek graficznych. Uruchamiaj ręcznie
tylko wtedy, gdy ikona ma się zmienić.

    python3 packaging/ikona.py
"""

import pathlib
from PIL import Image, ImageDraw

# Te same kolory, co akcent w interfejsie (theme.rs).
AKCENT = (76, 141, 255)
AKCENT_CIEMNY = (37, 99, 235)
BIALY = (255, 255, 255)

BOK = 256
# Rysujemy w powiększeniu i zmniejszamy — to najprostszy sposób na gładkie
# krawędzie bez sięgania po bibliotekę do antyaliasingu.
SKALA = 8


def gradient(rozmiar: int) -> Image.Image:
    """Pionowe przejście między dwoma odcieniami akcentu."""
    obraz = Image.new("RGB", (1, rozmiar))
    rysuj = ImageDraw.Draw(obraz)
    for y in range(rozmiar):
        t = y / max(rozmiar - 1, 1)
        kolor = tuple(
            round(a + (b - a) * t) for a, b in zip(AKCENT, AKCENT_CIEMNY)
        )
        rysuj.point((0, y), fill=kolor)
    return obraz.resize((rozmiar, rozmiar))


def chmurka(rysuj: ImageDraw.ImageDraw, bok: int) -> None:
    """Chmurka z trzech kół i zaokrąglonej podstawy."""
    s = bok / 256  # wszystko podane w proporcjach ikony 256 px

    def kolo(x, y, r):
        rysuj.ellipse(
            [(x - r) * s, (y - r) * s, (x + r) * s, (y + r) * s], fill=BIALY
        )

    kolo(98, 140, 34)
    kolo(140, 122, 44)
    kolo(176, 146, 30)
    rysuj.rounded_rectangle(
        [64 * s, 146 * s, 206 * s, 182 * s], radius=18 * s, fill=BIALY
    )


def zbuduj() -> Image.Image:
    duzy = BOK * SKALA

    tlo = gradient(duzy).convert("RGBA")

    # Maska zaokrąglonego kwadratu — poza nią ikona jest przezroczysta.
    maska = Image.new("L", (duzy, duzy), 0)
    ImageDraw.Draw(maska).rounded_rectangle(
        [0, 0, duzy - 1, duzy - 1], radius=56 * SKALA, fill=255
    )

    ikona = Image.new("RGBA", (duzy, duzy), (0, 0, 0, 0))
    ikona.paste(tlo, (0, 0), maska)

    warstwa = Image.new("RGBA", (duzy, duzy), (0, 0, 0, 0))
    chmurka(ImageDraw.Draw(warstwa), duzy)
    ikona = Image.alpha_composite(ikona, warstwa)

    return ikona.resize((BOK, BOK), Image.LANCZOS)


def main() -> None:
    korzen = pathlib.Path(__file__).resolve().parent.parent
    ikona = zbuduj()

    png = korzen / "crates/launcher/assets/ikona.png"
    png.parent.mkdir(parents=True, exist_ok=True)
    ikona.save(png)
    print("zapisano", png)

    ico = korzen / "packaging/windows/ikona.ico"
    ico.parent.mkdir(parents=True, exist_ok=True)
    # Windows dobiera rozmiar do miejsca — bez małych wariantów ikona
    # na pasku zadań wygląda na rozmytą.
    ikona.save(ico, sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print("zapisano", ico)


if __name__ == "__main__":
    main()

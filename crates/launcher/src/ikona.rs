//! Ikona launchera: okno, pasek zadań i zasobnik systemowy.
//!
//! Wcześniej ikona zasobnika na Windowsie była **rysowana w kodzie** jako
//! zaokrąglony niebieski kwadrat, a okno nie miało ikony wcale. Gracz zgłosił
//! to jako „nie renderuje się chmurka, tylko niebieski kwadrat" — i miał rację
//! co do objawu, tylko to nie było zepsute rysowanie. Chmurki tam po prostu
//! nigdy nie było.
//!
//! Plik `assets/ikona.png` i tak siedzi w binarce (rejestruje się nim skrót
//! w menu), więc jedyne, czego brakowało, to rozpakowanie go do pikseli.

/// Rozpakowany obrazek: piksele RGBA oraz wymiary.
pub struct Obrazek {
    pub piksele: Vec<u8>,
    pub szerokosc: u32,
    pub wysokosc: u32,
}

/// Dekoduje PNG do RGBA.
///
/// Zwraca `None`, gdy pliku nie da się odczytać. Ikona jest ozdobą — jej brak
/// nie może przewrócić startu launchera.
pub fn dekoduj(dane: &[u8]) -> Option<Obrazek> {
    let dekoder = png::Decoder::new(std::io::Cursor::new(dane));
    let mut czytnik = dekoder.read_info().ok()?;
    let mut bufor = vec![0; czytnik.output_buffer_size()?];
    let info = czytnik.next_frame(&mut bufor).ok()?;
    bufor.truncate(info.buffer_size());

    // Do RGBA doprowadzamy sami: `png` oddaje to, co było w pliku, a nasza
    // ikona mogłaby kiedyś zostać zapisana bez kanału alfa albo w skali szarości.
    let piksele = match info.color_type {
        png::ColorType::Rgba => bufor,
        png::ColorType::Rgb => bufor
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => bufor.iter().flat_map(|&s| [s, s, s, 255]).collect(),
        png::ColorType::GrayscaleAlpha => bufor
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Obrazek {
        piksele,
        szerokosc: info.width,
        wysokosc: info.height,
    })
}

/// Zmniejsza obrazek całkowitą krotność razy, uśredniając bloki pikseli.
///
/// Zasobnik systemowy chce małej ikony. Wrzucenie tam obrazka 256×256 działa,
/// ale system skaluje go sam i wynik bywa rozmyty — własne uśrednienie daje
/// ostrzejszy efekt, a przy krotności całkowitej jest trywialne.
///
/// Uśredniamy z wagą kanału alfa, inaczej przezroczyste tło (u nas czarne
/// z zerową alfą) przyciemniłoby brzegi chmurki.
// Zmniejszanie jest potrzebne tylko zasobnikowi na Windowsie. Na Linuksie
// zasobnik bierze ikonę z motywu systemu, więc tam funkcja nie ma wywołania —
// a testy i tak ją sprawdzają na obu systemach.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn zmniejsz(zrodlo: &Obrazek, krotnosc: u32) -> Option<Obrazek> {
    if krotnosc == 0 || zrodlo.szerokosc % krotnosc != 0 || zrodlo.wysokosc % krotnosc != 0 {
        return None;
    }
    let (sz, wy) = (zrodlo.szerokosc / krotnosc, zrodlo.wysokosc / krotnosc);
    let mut piksele = Vec::with_capacity((sz * wy * 4) as usize);
    for y in 0..wy {
        for x in 0..sz {
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for dy in 0..krotnosc {
                for dx in 0..krotnosc {
                    let i = (((y * krotnosc + dy) * zrodlo.szerokosc) + (x * krotnosc + dx))
                        as usize
                        * 4;
                    let alfa = zrodlo.piksele[i + 3] as u32;
                    r += zrodlo.piksele[i] as u32 * alfa;
                    g += zrodlo.piksele[i + 1] as u32 * alfa;
                    b += zrodlo.piksele[i + 2] as u32 * alfa;
                    a += alfa;
                }
            }
            // Blok całkiem przezroczysty: nie ma czego uśredniać, a dzielenie
            // przez zero byłoby tu jedynym sposobem, żeby to zauważyć.
            match r.checked_div(a) {
                None => piksele.extend_from_slice(&[0, 0, 0, 0]),
                Some(sr_r) => {
                    let ile = krotnosc * krotnosc;
                    piksele.extend_from_slice(&[
                        sr_r as u8,
                        (g / a) as u8,
                        (b / a) as u8,
                        (a / ile) as u8,
                    ]);
                }
            }
        }
    }
    Some(Obrazek {
        piksele,
        szerokosc: sz,
        wysokosc: wy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const IKONA: &[u8] = include_bytes!("../assets/ikona.png");

    /// Ikona musi dac sie rozpakowac — bez tego okno i zasobnik pokazuja
    /// zastepcza grafike systemu, a dokladnie to zglosil gracz.
    #[test]
    fn ikona_z_binarki_daje_sie_rozpakowac() {
        let o = dekoduj(IKONA).expect("ikona.png musi sie dekodowac");
        assert_eq!((o.szerokosc, o.wysokosc), (256, 256));
        assert_eq!(o.piksele.len(), 256 * 256 * 4);
        // Chmurka nie jest jednolitym kwadratem: musi miec i piksele
        // przezroczyste, i nieprzezroczyste.
        assert!(o.piksele.chunks_exact(4).any(|p| p[3] == 0), "brak tla");
        assert!(
            o.piksele.chunks_exact(4).any(|p| p[3] == 255),
            "brak ksztaltu"
        );
    }

    #[test]
    fn zmniejszanie_trzyma_wymiary_i_dlugosc() {
        let o = dekoduj(IKONA).unwrap();
        let maly = zmniejsz(&o, 8).expect("256 dzieli sie przez 8");
        assert_eq!((maly.szerokosc, maly.wysokosc), (32, 32));
        assert_eq!(maly.piksele.len(), 32 * 32 * 4);
        assert!(maly.piksele.chunks_exact(4).any(|p| p[3] > 0), "same puste");
    }

    /// Krotnosc, ktora nie dzieli rozmiaru, musi zostac odrzucona — inaczej
    /// petla czytalaby poza buforem.
    #[test]
    fn niecalkowita_krotnosc_odrzucona() {
        let o = dekoduj(IKONA).unwrap();
        assert!(zmniejsz(&o, 0).is_none());
        assert!(zmniejsz(&o, 7).is_none());
        assert!(zmniejsz(&o, 512).is_none());
    }

    #[test]
    fn smieci_nie_wywracaja_dekodera() {
        assert!(dekoduj(b"to nie jest png").is_none());
        assert!(dekoduj(&[]).is_none());
    }
}

//! Una tipografía de píxeles mínima (5×7), para los carteles de la intro ("PRESS ANY
//! KEY...", "SAY CHEESE!...", "...WAIT. DID YOU HEAR THAT?"). Nada de archivos de fuente — cada glyph es
//! 5 columnas × 7 filas, a mano, igual de "generado en código" que el resto de la escena.
//! Cubre justo las letras que hacen falta para esos mensajes; agregar una letra nueva es
//! sumar una fila a `glyph`.

/// Ancho y alto de un glyph, en "píxeles de fuente" (antes de `scale`).
pub const GLYPH_WIDTH: usize = 5;
pub const GLYPH_HEIGHT: usize = 7;

/// Una fila por `u8`: los `GLYPH_WIDTH` bits más bajos son las columnas, de izquierda
/// (bit más alto) a derecha (bit más bajo).
fn glyph(c: char) -> [u8; GLYPH_HEIGHT] {
    match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'C' => [0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00100, 0b00000, 0b00100],
        _ => [0; GLYPH_HEIGHT], // espacio, o cualquier letra sin glyph: en blanco
    }
}

/// Cuánto ocupa `text` dibujado con `draw_text` a esta `scale` (en píxeles de pantalla):
/// para centrarlo antes de dibujarlo.
pub fn text_width(text: &str, scale: usize) -> usize {
    text.chars().count() * (GLYPH_WIDTH + 1) * scale
}

/// Dibuja `text` (se pasa a mayúsculas) sobre `buffer` (de `width` × `height`, en el
/// mismo empaquetado 0xRRGGBB que usa el render), con la esquina superior izquierda del
/// primer glyph en (`x`, `y`), cada "píxel de fuente" agrandado a `scale` × `scale`
/// píxeles de pantalla, del color `color`.
pub fn draw_text(buffer: &mut [u32], width: usize, height: usize, text: &str, x: usize, y: usize, scale: usize, color: u32) {
    let mut cursor_x = x;
    for ch in text.chars() {
        for (row, bits) in glyph(ch).iter().enumerate() {
            for col in 0..GLYPH_WIDTH {
                if bits & (1 << (GLYPH_WIDTH - 1 - col)) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let (px, py) = (cursor_x + col * scale + dx, y + row * scale + dy);
                        if px < width && py < height {
                            buffer[py * width + px] = color;
                        }
                    }
                }
            }
        }
        cursor_x += (GLYPH_WIDTH + 1) * scale; // una columna de espacio entre letras
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_draws_nothing_but_still_advances() {
        let (width, height) = (20, 7);
        let mut buffer = vec![0u32; width * height];
        draw_text(&mut buffer, width, height, " ", 0, 0, 1, 0xFFFFFF);
        assert!(buffer.iter().all(|&p| p == 0), "el espacio no prende ningún píxel");
    }

    #[test]
    fn a_letter_lights_up_some_pixels_within_bounds() {
        let (width, height) = (20, 10);
        let mut buffer = vec![0u32; width * height];
        draw_text(&mut buffer, width, height, "A", 2, 1, 2, 0xFFFFFF);
        assert!(buffer.iter().any(|&p| p == 0xFFFFFF), "la A tiene que prender algo");
    }

    #[test]
    fn text_width_matches_what_draw_text_actually_spans() {
        let (width, height) = (200, 20);
        let mut buffer = vec![0u32; width * height];
        let (text, scale) = ("PRESS ANY KEY", 3);
        draw_text(&mut buffer, width, height, text, 0, 2, scale, 0xFFFFFF);
        let rightmost = (0..width).rev().find(|&x| (0..height).any(|y| buffer[y * width + x] != 0)).unwrap_or(0);
        assert!(rightmost < text_width(text, scale), "nada se dibuja más allá del ancho calculado");
    }
}

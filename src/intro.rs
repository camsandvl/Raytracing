//! La placa de título y la escena de introducción de la intro, antes del diorama: dos
//! imágenes generadas aparte (con Manus, a partir de referencias del autor), que se
//! muestran una tras otra hasta que se aprieta cualquier tecla. El autor pidió el
//! mecanismo y el cartel de aviso ("PRESS ANY KEY..."); las imágenes las pone él — ver
//! `TITLE_PATH`/`SCENE_PATH` para dónde van, y el mensaje final del cambio para el
//! formato esperado.
//!
//! Después de la foto, antes del diorama, una pantalla negra con un aviso ominoso
//! ("...WAIT. DID YOU HEAR THAT?") que aparece de a poco y sigue sola al diorama — que
//! arranca en el vitral reventado por los zombis.
//!
//! Es un agregado opcional, igual que el modo primera persona: si todavía no están los
//! archivos, se saltea sin romper nada (ni en `cargo run` ni en los tests).

use crate::font;
use minifb::{KeyRepeat, Window};
use std::time::{Duration, Instant};

const TITLE_PATH: &str = "assets/intro/title.png";
const SCENE_PATH: &str = "assets/intro/scene.png";
/// El cartel de cada imagen: la placa de título sigue con "continuar"; la escena (los
/// novios antes del ataque) invita a sacarles la foto del gran día.
const TITLE_MESSAGE: &str = "PRESS ANY KEY TO CONTINUE";
const SCENE_MESSAGE: &str = "SAY CHEESE! PRESS ANY KEY TO TAKE THEIR HAPPY DAY PICTURE";
const TEXT_SCALE: usize = 3;
/// La pantalla negra entre la foto y el diorama: el texto aparece desde el negro en
/// `OMEN_FADE` y el diorama arranca solo a los `OMEN_TOTAL` (cualquier tecla lo
/// adelanta). Sin cartel de "apretá una tecla": a esta altura ya se sabe.
const OMEN: &str = "...WAIT. DID YOU HEAR THAT?";
const OMEN_SCALE: usize = 4;
const OMEN_FADE: Duration = Duration::from_millis(2000);
const OMEN_TOTAL: Duration = Duration::from_millis(4000);

/// Muestra las dos imágenes, cada una hasta que se aprieta cualquier tecla, reusando la
/// `window` que después sigue con el diorama (sin parpadeo de abrir una ventana nueva).
/// Devuelve `false` si el usuario cerró la ventana durante la intro — en ese caso no hay
/// que seguir al diorama.
pub fn show(window: &mut Window, width: usize, height: usize) -> bool {
    for (path, message) in [(TITLE_PATH, TITLE_MESSAGE), (SCENE_PATH, SCENE_MESSAGE)] {
        let Some(mut frame) = load_scaled(path, width, height) else {
            println!("intro: no se encontró {path}, se saltea esa placa (PNG, cualquier tamaño; se reescala a la ventana)");
            continue;
        };
        overlay_message(&mut frame, width, height, message);
        if !wait_for_key(window, &frame, width, height) {
            return false;
        }
    }
    omen(window, width, height)
}

/// La pantalla negra con el aviso, que aparece de a poco y sigue sola al diorama (o con
/// cualquier tecla); `false` si se cierra la ventana antes.
fn omen(window: &mut Window, width: usize, height: usize) -> bool {
    let start = Instant::now();
    while window.is_open() {
        let elapsed = start.elapsed();
        if elapsed >= OMEN_TOTAL {
            return true;
        }
        let frame = omen_frame(width, height, elapsed.as_secs_f32() / OMEN_FADE.as_secs_f32());
        window.update_with_buffer(&frame, width, height).expect("fallo actualizando la ventana");
        if !window.get_keys_pressed(KeyRepeat::No).is_empty() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    false
}

/// Negro, con `OMEN` centrado en gris pálido, a `brightness` (0 = negro, 1 = del todo
/// visible).
fn omen_frame(width: usize, height: usize, brightness: f32) -> Vec<u32> {
    let mut frame = vec![0u32; width * height];
    let level = (0xD8 as f32 * brightness.clamp(0.0, 1.0)) as u32;
    if level > 0 {
        let x = width.saturating_sub(font::text_width(OMEN, OMEN_SCALE)) / 2;
        let y = height.saturating_sub(font::GLYPH_HEIGHT * OMEN_SCALE) / 2;
        font::draw_text(&mut frame, width, height, OMEN, x, y, OMEN_SCALE, level << 16 | level << 8 | level);
    }
    frame
}

/// Una franja oscurecida al pie de la imagen (para que el texto se lea encima de
/// cualquier fondo) con el mensaje centrado.
fn overlay_message(frame: &mut [u32], width: usize, height: usize, message: &str) {
    let text_h = font::GLYPH_HEIGHT * TEXT_SCALE;
    let y = height.saturating_sub(text_h + 28);
    let band = y.saturating_sub(10)..(y + text_h + 10).min(height);
    for row in band {
        for pixel in &mut frame[row * width..(row + 1) * width] {
            let (r, g, b) = (*pixel >> 16 & 0xFF, *pixel >> 8 & 0xFF, *pixel & 0xFF);
            *pixel = (r / 3) << 16 | (g / 3) << 8 | (b / 3);
        }
    }
    let x = (width.saturating_sub(font::text_width(message, TEXT_SCALE))) / 2;
    font::draw_text(frame, width, height, message, x, y, TEXT_SCALE, 0xFFFFFF);
}

/// Mantiene `frame` en pantalla hasta la primera tecla apretada (de borde, no mantenida);
/// `false` si se cierra la ventana antes.
fn wait_for_key(window: &mut Window, frame: &[u32], width: usize, height: usize) -> bool {
    while window.is_open() {
        window.update_with_buffer(frame, width, height).expect("fallo actualizando la ventana");
        if !window.get_keys_pressed(KeyRepeat::No).is_empty() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    false
}

/// Carga un PNG y lo reescala (vecino más cercano) a exactamente `width` × `height`, en
/// el mismo empaquetado 0xRRGGBB que usa el render. `None` si el archivo no existe o no
/// se pudo decodificar.
fn load_scaled(path: &str, width: usize, height: usize) -> Option<Vec<u32>> {
    let image = image::open(path).ok()?.into_rgb8();
    let (src_w, src_h) = (image.width() as usize, image.height() as usize);
    let mut buffer = vec![0u32; width * height];
    for y in 0..height {
        let sy = (y * src_h / height).min(src_h - 1) as u32;
        for x in 0..width {
            let sx = (x * src_w / width).min(src_w - 1) as u32;
            let p = image.get_pixel(sx, sy);
            buffer[y * width + x] = (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32;
        }
    }
    Some(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La pantalla del aviso arranca en negro y termina con el texto visible, centrado.
    #[test]
    fn omen_fades_in_from_black_to_centered_text() {
        let (width, height) = (800, 100); // más ancho que el texto (648 px a esta escala)
        assert!(omen_frame(width, height, 0.0).iter().all(|&p| p == 0), "arranca en negro");
        let lit = omen_frame(width, height, 1.0);
        let columns: Vec<usize> = (0..width).filter(|&x| (0..height).any(|y| lit[y * width + x] != 0)).collect();
        let (left, right) = (columns[0], width - 1 - columns[columns.len() - 1]);
        assert!(left.abs_diff(right) <= OMEN_SCALE * 2, "centrado: {left} px a la izquierda, {right} a la derecha");
        let dim = omen_frame(width, height, 0.5).into_iter().max().unwrap();
        assert!(dim < lit.into_iter().max().unwrap() && dim > 0, "a mitad del fundido, más tenue");
    }

    /// Falta el archivo: no revienta, devuelve `None` (lo que hace que `show` lo saltee).
    #[test]
    fn load_scaled_is_none_when_the_file_is_missing() {
        assert!(load_scaled("assets/intro/esto_no_existe.png", 4, 4).is_none());
    }

    /// Una imagen de 2×1 (mitad roja, mitad azul) estirada a 4×2: cada franja de origen
    /// se repite por el vecino más cercano, y el color queda bien empaquetado 0xRRGGBB.
    #[test]
    fn load_scaled_resizes_and_packs_colors() {
        let path = std::env::temp_dir().join("diorama_intro_test.png");
        let img = image::RgbImage::from_fn(2, 1, |x, _| if x == 0 { image::Rgb([255, 0, 0]) } else { image::Rgb([0, 0, 255]) });
        img.save(&path).expect("no se pudo escribir el PNG de prueba");

        let buffer = load_scaled(path.to_str().unwrap(), 4, 2).expect("tenía que encontrar el archivo recién escrito");
        assert_eq!(buffer.len(), 8);
        assert_eq!(buffer[0], 0xFF0000, "mitad izquierda: rojo");
        assert_eq!(buffer[3], 0x0000FF, "mitad derecha: azul");
        assert_eq!(buffer[0], buffer[4], "misma franja repetida en la segunda fila");

        std::fs::remove_file(&path).ok();
    }
}

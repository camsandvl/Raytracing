//! La placa de título y la escena de introducción de la intro, antes del diorama: dos
//! imágenes generadas aparte (con Manus, a partir de referencias del autor), que se
//! muestran una tras otra hasta que se aprieta cualquier tecla. La placa ocupa toda la
//! pantalla; la escena es "la foto": centrada sobre negro, con sus proporciones, y el
//! cartel debajo, fuera de la foto. El autor pidió el
//! mecanismo y el cartel de aviso ("PRESS ANY KEY..."); las imágenes las pone él — ver
//! `TITLE_PATH`/`SCENE_PATH` para dónde van, y el mensaje final del cambio para el
//! formato esperado.
//!
//! Al apretar una tecla sobre la foto, salta el flash de la cámara (un blanco que se
//! apaga en negro, sin texto: el flash es el "click") y arranca el diorama — la misma
//! boda, un instante después.
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
/// La foto de la escena ocupa, como mucho, esta fracción del alto y del ancho de la
/// ventana (con sus proporciones); el cartel va debajo, a `PHOTO_GAP` píxeles.
const PHOTO_HEIGHT: f32 = 0.48;
const PHOTO_WIDTH: f32 = 0.56;
const PHOTO_GAP: usize = 28;
/// El flash de la cámara después de la foto: blanco pleno durante `FLASH_HOLD` y después
/// se apaga hasta negro en `FLASH_FADE`.
const FLASH_HOLD: Duration = Duration::from_millis(90);
const FLASH_FADE: Duration = Duration::from_millis(650);

/// Muestra las dos imágenes, cada una hasta que se aprieta cualquier tecla, reusando la
/// `window` que después sigue con el diorama (sin parpadeo de abrir una ventana nueva).
/// Devuelve `false` si el usuario cerró la ventana durante la intro — en ese caso no hay
/// que seguir al diorama.
pub fn show(window: &mut Window, width: usize, height: usize) -> bool {
    let title = load_scaled(TITLE_PATH, width, height).map(|mut frame| {
        overlay_message(&mut frame, width, height, TITLE_MESSAGE);
        frame
    });
    let scene = image_size(SCENE_PATH).and_then(|(w, h)| {
        let (pw, ph) = photo_size(w, h, width, height);
        load_scaled(SCENE_PATH, pw, ph).map(|photo| photo_frame(&photo, pw, ph, width, height, SCENE_MESSAGE))
    });
    for (path, frame) in [(TITLE_PATH, title), (SCENE_PATH, scene)] {
        let Some(frame) = frame else {
            println!("intro: no se encontró {path}, se saltea esa placa (PNG, cualquier tamaño; se reescala a la ventana)");
            continue;
        };
        if !wait_for_key(window, &frame, width, height) {
            return false;
        }
    }
    flash(window, width, height)
}

/// El tamaño de la foto en pantalla: la imagen de `w` × `h` agrandada o achicada, con sus
/// proporciones, hasta llenar `PHOTO_HEIGHT` del alto o `PHOTO_WIDTH` del ancho de la
/// ventana (lo que llegue primero).
fn photo_size(w: usize, h: usize, width: usize, height: usize) -> (usize, usize) {
    let scale = (height as f32 * PHOTO_HEIGHT / h as f32).min(width as f32 * PHOTO_WIDTH / w as f32);
    (((w as f32 * scale) as usize).max(1), ((h as f32 * scale) as usize).max(1))
}

/// La pantalla de la foto: negro, la foto de `pw` × `ph` centrada y el cartel debajo (la
/// foto y el cartel, juntos, centrados en alto).
fn photo_frame(photo: &[u32], pw: usize, ph: usize, width: usize, height: usize, message: &str) -> Vec<u32> {
    let mut frame = vec![0u32; width * height];
    let text_h = font::GLYPH_HEIGHT * TEXT_SCALE;
    let top = height.saturating_sub(ph + PHOTO_GAP + text_h) / 2;
    let left = width.saturating_sub(pw) / 2;
    for y in 0..ph.min(height - top) {
        let row = (top + y) * width + left;
        let n = pw.min(width - left);
        frame[row..row + n].copy_from_slice(&photo[y * pw..y * pw + n]);
    }
    let x = width.saturating_sub(font::text_width(message, TEXT_SCALE)) / 2;
    font::draw_text(&mut frame, width, height, message, x, top + ph + PHOTO_GAP, TEXT_SCALE, 0xFFFFFF);
    frame
}

/// El tamaño en píxeles de un PNG, sin decodificarlo entero; `None` si no está.
fn image_size(path: &str) -> Option<(usize, usize)> {
    image::image_dimensions(path).ok().map(|(w, h)| (w as usize, h as usize))
}

/// El flash de la cámara: la pantalla en blanco que se apaga hasta negro; `false` si se
/// cierra la ventana mientras tanto.
fn flash(window: &mut Window, width: usize, height: usize) -> bool {
    let start = Instant::now();
    let mut frame = vec![0u32; width * height];
    while window.is_open() {
        let elapsed = start.elapsed();
        if elapsed >= FLASH_HOLD + FLASH_FADE {
            return true;
        }
        let level = flash_level(elapsed);
        frame.fill(level << 16 | level << 8 | level);
        window.update_with_buffer(&frame, width, height).expect("fallo actualizando la ventana");
        std::thread::sleep(Duration::from_millis(16));
    }
    false
}

/// El brillo del flash (0–255) a `elapsed` de haber saltado: pleno, y después se apaga
/// rápido al principio y más lento al final, como la luz que queda en el ojo.
fn flash_level(elapsed: Duration) -> u32 {
    let fade = (elapsed.saturating_sub(FLASH_HOLD).as_secs_f32() / FLASH_FADE.as_secs_f32()).clamp(0.0, 1.0);
    (255.0 * (1.0 - fade).powi(2)) as u32
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

/// Carga un PNG y lo reescala a exactamente `width` × `height`, en el mismo empaquetado
/// 0xRRGGBB que usa el render: si hay que achicarla, con un filtro suave (la foto de la
/// escena se achica casi a un tercio, y por vecino más cercano quedaba serruchada); si
/// hay que agrandarla, por vecino más cercano. `None` si el archivo no existe o no se
/// pudo decodificar.
fn load_scaled(path: &str, width: usize, height: usize) -> Option<Vec<u32>> {
    let mut image = image::open(path).ok()?.into_rgb8();
    if (width as u32) < image.width() && (height as u32) < image.height() {
        image = image::imageops::resize(&image, width as u32, height as u32, image::imageops::FilterType::Triangle);
    }
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

    /// El flash arranca en blanco pleno, se apaga sin volver a subir y termina en negro.
    #[test]
    fn the_flash_starts_white_and_fades_to_black() {
        assert_eq!(flash_level(Duration::ZERO), 255);
        assert_eq!(flash_level(FLASH_HOLD), 255, "blanco pleno hasta que empieza a apagarse");
        assert_eq!(flash_level(FLASH_HOLD + FLASH_FADE), 0, "termina en negro");
        let steps: Vec<u32> = (0..=20).map(|i| flash_level(FLASH_HOLD + FLASH_FADE * i / 20)).collect();
        assert!(steps.windows(2).all(|w| w[1] <= w[0]), "se apaga sin volver a subir: {steps:?}");
    }

    /// La foto queda centrada a lo ancho, con negro alrededor, sus proporciones y el
    /// cartel debajo, fuera de la foto.
    #[test]
    fn the_photo_is_centered_on_black_with_the_label_below() {
        let (width, height) = (1280, 720);
        let (pw, ph) = photo_size(1536, 1024, width, height);
        assert_eq!(ph, (720.0 * PHOTO_HEIGHT) as usize, "manda el alto");
        assert!((pw as f32 / ph as f32 - 1.5).abs() < 0.01, "mantiene las proporciones");
        let photo = vec![0x808080u32; pw * ph];
        let frame = photo_frame(&photo, pw, ph, width, height, "SAY CHEESE");
        let lit = |x: usize, y: usize| frame[y * width + x] != 0;
        let rows: Vec<usize> = (0..height).filter(|&y| lit(width / 2, y)).collect();
        let (top, bottom) = (rows[0], rows[rows.len() - 1]);
        assert!(top > 0 && !lit(0, top + 10) && !lit(width - 1, top + 10), "negro alrededor de la foto");
        let photo_left = (0..width).find(|&x| lit(x, top + 10)).unwrap();
        assert_eq!(photo_left, width - pw - photo_left, "centrada a lo ancho");
        let text_rows: Vec<usize> = (top + ph..height).filter(|&y| (0..width).any(|x| frame[y * width + x] == 0xFFFFFF)).collect();
        assert!(!text_rows.is_empty() && text_rows[0] >= top + ph + PHOTO_GAP, "el cartel, debajo y fuera de la foto");
        assert!(bottom < height, "todo entra en la pantalla");
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

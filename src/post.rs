//! Retoques sobre la imagen ya terminada (después del tone mapping), baratos porque no
//! lanzan ningún rayo: un par de pasadas por los píxeles.
//! - Resplandor ("bloom"): lo muy brillante (las llamas, el vidrio que brilla, las astillas)
//!   se desparrama en un halo suave.
//! - Viñeta: las esquinas se oscurecen y la mirada va al centro.
//! - Gradación: las sombras un poco más frías y las luces un poco más cálidas.
//!
//! La bruma de profundidad no está acá: va en `cast_ray`, porque necesita la distancia de
//! cada rayo.
//!
//! Todo se reparte por franjas de filas entre los núcleos, como el render: en un solo
//! núcleo, estos retoques llegaban a ser un 10–15% del cuadro.

/// Desde qué brillo (0–1, por canal) un píxel empieza a resplandecer.
const BLOOM_THRESHOLD: f32 = 0.6;
/// Cuánto del halo se suma de vuelta.
const BLOOM_STRENGTH: f32 = 1.6;
/// Cuánto se oscurecen las esquinas (0 = nada).
const VIGNETTE: f32 = 0.45;
/// Gradación: el tinte que toman las sombras y el que toman las luces.
const SHADOW_TINT: [f32; 3] = [0.94, 0.98, 1.08];
const LIGHT_TINT: [f32; 3] = [1.06, 1.0, 0.92];

pub fn apply(buffer: &mut [u32], width: usize, height: usize) {
    let mut image: Vec<[f32; 3]> = buffer.iter().map(|&px| unpack(px)).collect();
    bloom(&mut image, width, height);
    grade(&mut image, width, height);
    for (px, c) in buffer.iter_mut().zip(image) {
        *px = pack(c);
    }
}

/// Lo que pasa del umbral se difumina (tres pasadas de caja en cada eje, que se parecen a
/// una gaussiana) y se suma encima de la imagen. El eje vertical se difumina trasponiendo
/// la imagen, así las columnas pasan a ser filas y se reparten igual entre los núcleos.
fn bloom(image: &mut [[f32; 3]], width: usize, height: usize) {
    let mut glow: Vec<[f32; 3]> = image.iter().map(|c| c.map(|v| (v - BLOOM_THRESHOLD).max(0.0) / (1.0 - BLOOM_THRESHOLD))).collect();
    let radius = (width / 160).max(2);
    for _ in 0..3 {
        blur_rows(&mut glow, width, radius);
        let mut columns = transpose(&glow, width, height);
        blur_rows(&mut columns, height, radius);
        glow = transpose(&columns, height, width);
    }
    for (c, g) in image.iter_mut().zip(glow) {
        for i in 0..3 {
            c[i] = (c[i] + g[i] * BLOOM_STRENGTH * (1.0 - c[i])).min(1.0);
        }
    }
}

/// Reparte `rows` (filas de `width` píxeles) en franjas, una tanda por núcleo.
fn par_rows<T: Send>(data: &mut [T], width: usize, work: impl Fn(usize, &mut [T]) + Sync) {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let rows = data.len() / width;
    let per_chunk = rows.div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        for (i, chunk) in data.chunks_mut(per_chunk * width).enumerate() {
            let work = &work;
            scope.spawn(move || work(i * per_chunk, chunk));
        }
    });
}

fn transpose(data: &[[f32; 3]], width: usize, height: usize) -> Vec<[f32; 3]> {
    let mut out = vec![[0.0f32; 3]; data.len()];
    for y in 0..height {
        for x in 0..width {
            out[x * height + y] = data[y * width + x];
        }
    }
    out
}

/// Promedio corrido de `2 * radius + 1` píxeles a lo largo de cada fila de `width`.
fn blur_rows(data: &mut [[f32; 3]], width: usize, radius: usize) {
    let scale = 1.0 / (2 * radius + 1) as f32;
    par_rows(data, width, |_, chunk| {
        let mut line = vec![[0.0f32; 3]; width];
        for row in chunk.chunks_mut(width) {
            let sample = |i: isize| row[i.clamp(0, width as isize - 1) as usize];
            let mut sum = [0.0f32; 3];
            for i in -(radius as isize)..=radius as isize {
                let s = sample(i);
                (0..3).for_each(|k| sum[k] += s[k]);
            }
            for i in 0..width {
                line[i] = sum.map(|v| v * scale);
                let (add, sub) = (sample(i as isize + radius as isize + 1), sample(i as isize - radius as isize));
                (0..3).for_each(|k| sum[k] += add[k] - sub[k]);
            }
            row.copy_from_slice(&line);
        }
    });
}

/// Viñeta y gradación: las esquinas más oscuras; las sombras hacia el azul y las luces
/// hacia el ámbar, apenas.
fn grade(image: &mut [[f32; 3]], width: usize, height: usize) {
    par_rows(image, width, |first_row, chunk| grade_rows(chunk, first_row, width, height));
}

fn grade_rows(chunk: &mut [[f32; 3]], first_row: usize, width: usize, height: usize) {
    for (row, pixels) in chunk.chunks_mut(width).enumerate() {
        let y = first_row + row;
        for x in 0..width {
            let (dx, dy) = ((x as f32 + 0.5) / width as f32 - 0.5, (y as f32 + 0.5) / height as f32 - 0.5);
            let r = (dx * dx + dy * dy).sqrt() / 0.7071;
            let t = ((r - 0.35) / 0.65).clamp(0.0, 1.0);
            let vignette = 1.0 - VIGNETTE * t * t * (3.0 - 2.0 * t);
            let c = &mut pixels[x];
            let luma = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let (dark, bright) = ((1.0 - luma).powi(2), luma * luma);
            for i in 0..3 {
                let tint = 1.0 + (SHADOW_TINT[i] - 1.0) * dark + (LIGHT_TINT[i] - 1.0) * bright;
                c[i] = (c[i] * tint * vignette).clamp(0.0, 1.0);
            }
        }
    }
}

fn unpack(px: u32) -> [f32; 3] {
    [(px >> 16) & 0xFF, (px >> 8) & 0xFF, px & 0xFF].map(|v| v as f32 / 255.0)
}

fn pack(c: [f32; 3]) -> u32 {
    let [r, g, b] = c.map(|v| (v * 255.0).round() as u32);
    (r << 16) | (g << 8) | b
}

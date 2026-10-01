//! Cada material que cuenta para la rúbrica necesita su propia textura (ver skill de
//! rúbrica). La mayoría se generan proceduralmente aquí mismo (ruido sembrado,
//! reproducible); el material Metal carga un PNG real con la crate `image` — la misma
//! que usa el curso para las texturas del laberinto (rama `11-RC-05-MAZE-TEXTURES`).
//!
//! Las coordenadas (u, v) que llegan a `sample` están en unidades de CELDA (no
//! normalizadas): cada textura decide cuántas celdas cubre un tile completo (`span`).
//! Así una textura de mampostería puede abarcar 4×4 bloques distintos y no repetirse
//! en cada cubo.

use crate::color::Color;
use crate::rng::Rng;
use std::path::Path;

pub type TextureId = usize;

pub struct Texture {
    width: usize,
    height: usize,
    span: f32,
    pixels: Vec<Color>,
}

fn scaled(color: Color, factor: f32) -> Color {
    Color::new(
        (color.r as f32 * factor).clamp(0.0, 255.0) as u8,
        (color.g as f32 * factor).clamp(0.0, 255.0) as u8,
        (color.b as f32 * factor).clamp(0.0, 255.0) as u8,
    )
}

impl Texture {
    pub fn load_png<P: AsRef<Path>>(path: P, span: f32) -> image::ImageResult<Texture> {
        let rgb = image::open(path)?.into_rgb8();
        let (width, height) = (rgb.width() as usize, rgb.height() as usize);
        let pixels = rgb.pixels().map(|p| Color::new(p[0], p[1], p[2])).collect();
        Ok(Texture { width, height, span, pixels })
    }

    /// Mampostería: una grilla de `blocks`×`blocks` bloques de `block_px` texels cada
    /// uno, un bloque por celda. Cada bloque tiene su propio tono y su propia cantidad
    /// de musgo (algunas piedras están casi limpias, otras cubiertas), grano interno y
    /// una junta oscura de 1 texel en el borde — así cada voxel se lee como una piedra
    /// individual, igual que en los renders de referencia.
    #[allow(clippy::too_many_arguments)]
    pub fn block(
        block_px: usize,
        blocks: usize,
        base_color: Color,
        strength: f32,
        edge_darken: f32,
        moss_color: Color,
        moss_coverage: f32,
        seed: u32,
    ) -> Texture {
        let size = block_px * blocks;
        let mut rng = Rng::new(seed);
        let tones: Vec<f32> = (0..blocks * blocks).map(|_| 1.0 + rng.range(-0.12, 0.12)).collect();
        let mossiness: Vec<f32> = (0..blocks * blocks).map(|_| moss_coverage * rng.range(0.0, 2.0)).collect();

        let mut pixels = Vec::with_capacity(size * size);
        for y in 0..size {
            for x in 0..size {
                let block = (y / block_px) * blocks + (x / block_px);
                let (lx, ly) = (x % block_px, y % block_px);
                let on_edge = lx == 0 || ly == 0 || lx == block_px - 1 || ly == block_px - 1;

                let mut factor = tones[block] * (1.0 + rng.range(-strength, strength));
                if on_edge {
                    factor *= 1.0 - edge_darken;
                }
                let mut color = scaled(base_color, factor);
                if rng.next_f32() < mossiness[block] {
                    color = Color::lerp(color, moss_color, rng.range(0.4, 0.9));
                }
                pixels.push(color);
            }
        }
        Texture { width: size, height: size, span: blocks as f32, pixels }
    }

    /// Vetas horizontales onduladas — Madera. El patrón de líneas sale de un seno en
    /// `x`; la ondulación de cada fila sale de `Rng`, para que no se vea repetitivo.
    pub fn grained(size: usize, base_color: Color, strength: f32, seed: u32) -> Texture {
        let mut rng = Rng::new(seed);
        let mut pixels = Vec::with_capacity(size * size);
        for _y in 0..size {
            let wobble = rng.range(0.0, std::f32::consts::TAU);
            for x in 0..size {
                let band = (x as f32 * 0.9 + wobble).sin();
                let jitter = rng.range(-0.2, 0.2);
                let factor = 1.0 + (band + jitter) * strength;
                pixels.push(scaled(base_color, factor));
            }
        }
        Texture { width: size, height: size, span: 1.0, pixels }
    }

    /// Base de hueso pálido con parches de musgo verde donde el ruido supera
    /// `coverage` — para los tres esqueletos ("llevan ahí mucho tiempo").
    pub fn moss(size: usize, bone_color: Color, moss_color: Color, coverage: f32, seed: u32) -> Texture {
        let mut rng = Rng::new(seed);
        let mut pixels = Vec::with_capacity(size * size);
        for _ in 0..(size * size) {
            let base = scaled(bone_color, 1.0 + rng.range(-0.12, 0.12));
            if rng.next_f32() > 1.0 - coverage {
                pixels.push(Color::lerp(base, moss_color, rng.range(0.5, 1.0)));
            } else {
                pixels.push(base);
            }
        }
        Texture { width: size, height: size, span: 1.0, pixels }
    }

    /// Mármol: tono de base con variación suave y vetas finas onduladas que cruzan varias
    /// celdas (el tile abarca `span` celdas). El ruido y las vetas son periódicos, así el
    /// tile se repite sin costura y sin contorno por bloque.
    pub fn marble(size: usize, span: f32, base: Color, vein: Color, veins: f32, seed: u32) -> Texture {
        const LATTICE: usize = 8;
        let mut rng = Rng::new(seed);
        let lattice: Vec<f32> = (0..LATTICE * LATTICE).map(|_| rng.next_f32()).collect();
        let n = LATTICE as f32;
        // Ruido de valor periódico con interpolación suave; `u`, `v` en celdas de la red.
        let noise = |u: f32, v: f32| {
            let at = |x: f32, y: f32| lattice[y.rem_euclid(n) as usize * LATTICE + x.rem_euclid(n) as usize];
            let (x0, y0) = (u.floor(), v.floor());
            let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
            let (fx, fy) = (smooth(u - x0), smooth(v - y0));
            let top = at(x0, y0) + (at(x0 + 1.0, y0) - at(x0, y0)) * fx;
            let bottom = at(x0, y0 + 1.0) + (at(x0 + 1.0, y0 + 1.0) - at(x0, y0 + 1.0)) * fx;
            top + (bottom - top) * fy
        };

        let pixels = (0..size * size)
            .map(|i| {
                let (u, v) = ((i % size) as f32 / size as f32, (i / size) as f32 / size as f32);
                let broad = noise(u * n, v * n);
                let turbulence = broad + 0.5 * noise(u * n * 2.0, v * n * 2.0);
                let wave = (std::f32::consts::TAU * (2.0 * u + v + 1.2 * turbulence)).sin();
                let vein_amount = (1.0 - wave.abs()).powi(10) * veins;
                Color::lerp(scaled(base, 0.93 + 0.14 * broad), vein, vein_amount)
            })
            .collect();
        Texture { width: size, height: size, span, pixels }
    }

    /// Vidrio: grano casi imperceptible + líneas oscuras de "plomo" en los bordes del
    /// tile, imitando el emplomado entre paneles de un vitral. El tinte de color real
    /// lo aporta `Material.diffuse`, no esta textura — esto evita que el vidrio sea un
    /// color 100% plano (no contaría como material propio).
    pub fn leaded_glass(size: usize, seed: u32) -> Texture {
        let mut rng = Rng::new(seed);
        let mut pixels = Vec::with_capacity(size * size);
        let border = 1;
        for y in 0..size {
            for x in 0..size {
                let on_lead = x < border || y < border || x >= size - border || y >= size - border;
                if on_lead {
                    pixels.push(Color::new(40, 34, 28));
                } else {
                    pixels.push(scaled(Color::new(255, 255, 255), 1.0 + rng.range(-0.05, 0.05)));
                }
            }
        }
        Texture { width: size, height: size, span: 1.0, pixels }
    }

    /// Muestreo nearest-neighbor. `(u, v)` en unidades de celda; se envuelve cada
    /// `span` celdas para que el tile se repita sobre superficies más grandes.
    pub fn sample(&self, u: f32, v: f32) -> Color {
        let u = (u / self.span).rem_euclid(1.0);
        let v = (v / self.span).rem_euclid(1.0);
        let x = ((u * self.width as f32) as usize).min(self.width - 1);
        let y = (((1.0 - v) * self.height as f32) as usize).min(self.height - 1);
        self.pixels[y * self.width + x]
    }
}

/// Registro de texturas cargadas, indexadas por `TextureId` — cada `Material` guarda
/// un id en vez de una `Texture` completa, así se mantiene liviano y `Copy`.
pub struct TextureBank {
    textures: Vec<Texture>,
}

impl TextureBank {
    pub fn new() -> Self {
        TextureBank { textures: Vec::new() }
    }

    pub fn add(&mut self, texture: Texture) -> TextureId {
        self.textures.push(texture);
        self.textures.len() - 1
    }

    pub fn get(&self, id: TextureId) -> &Texture {
        &self.textures[id]
    }
}

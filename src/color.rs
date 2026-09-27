use std::fmt;
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    pub fn from_hex(hex: u32) -> Self {
        Color {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    pub fn to_hex(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Interpolación lineal entre dos colores, `t` en [0, 1] — usada por el skybox
    /// (gradiente del cielo) y por el downscale del antialiasing por supersampling.
    pub fn lerp(a: Color, b: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        Color {
            r: (a.r as f32 + (b.r as f32 - a.r as f32) * t) as u8,
            g: (a.g as f32 + (b.g as f32 - a.g as f32) * t) as u8,
            b: (a.b as f32 + (b.b as f32 - a.b as f32) * t) as u8,
        }
    }
}

impl Add for Color {
    type Output = Color;

    fn add(self, other: Color) -> Color {
        Color {
            r: self.r.saturating_add(other.r),
            g: self.g.saturating_add(other.g),
            b: self.b.saturating_add(other.b),
        }
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, scalar: f32) -> Color {
        Color {
            r: (self.r as f32 * scalar).clamp(0.0, 255.0) as u8,
            g: (self.g as f32 * scalar).clamp(0.0, 255.0) as u8,
            b: (self.b as f32 * scalar).clamp(0.0, 255.0) as u8,
        }
    }
}

/// Tinte multiplicativo canal-por-canal (cada canal en [0,1] representado como el
/// [0,255] del otro color) — hace falta para que la textura de un material module el
/// color final (`shade()`), y para que el vidrio filtre lo que se ve a través suyo por
/// su propio color en vez de dejarlo pasar sin teñir.
impl Mul<Color> for Color {
    type Output = Color;

    fn mul(self, other: Color) -> Color {
        Color {
            r: ((self.r as u32 * other.r as u32) / 255) as u8,
            g: ((self.g as u32 * other.g as u32) / 255) as u8,
            b: ((self.b as u32 * other.b as u32) / 255) as u8,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Color(r: {}, g: {}, b: {})", self.r, self.g, self.b)
    }
}

//! El curso, al día de hoy, devuelve un color de fondo plano (`BACKGROUND_COLOR`)
//! cuando un rayo no pega nada. Esto lo reemplaza por un cielo nocturno procedural:
//! gradiente vertical + luna + estrellas, función pura de la dirección del rayo — sin
//! textura que cargar ni riesgo de asset, y consistente con que la cámara puede orbitar
//! libremente alrededor de la iglesia (se ve bien desde cualquier ángulo).

use crate::color::Color;
use nalgebra_glm::Vec3;

/// Dirección hacia la luna — la usa el cielo para dibujarla y la escena para su luz
/// direccional, así la luna que se ve y la luz que proyecta siempre coinciden.
pub fn moon_direction() -> Vec3 {
    Vec3::new(0.6, 0.55, -0.6).normalize()
}

pub fn sample(dir: &Vec3) -> Color {
    let horizon = Color::new(18, 22, 40);
    let zenith = Color::new(3, 3, 10);
    let t = (dir.y * 0.5 + 0.5).clamp(0.0, 1.0);
    let mut color = Color::lerp(horizon, zenith, t);

    let moon_dir = moon_direction();
    let alignment = dir.dot(&moon_dir);
    if alignment > 0.9985 {
        color = Color::new(235, 232, 215);
    } else if alignment > 0.996 {
        let glow = (alignment - 0.996) / (0.9985 - 0.996);
        color = Color::lerp(color, Color::new(120, 120, 130), glow);
    }

    if dir.y > 0.0 {
        let twinkle = star_hash(dir);
        if twinkle > 0.9965 {
            let brightness = ((twinkle - 0.9965) / 0.0035).clamp(0.0, 1.0);
            color = color + Color::new(255, 255, 255) * brightness;
        }
    }

    color
}

/// Hash determinista de una dirección — sin ruido dependiente del tiempo, así las
/// estrellas quedan fijas en el cielo mientras la cámara orbita, como estrellas reales.
fn star_hash(dir: &Vec3) -> f32 {
    let p = dir * 500.0;
    let n = (p.x.floor() * 12.9898 + p.y.floor() * 78.233 + p.z.floor() * 37.719).sin() * 43758.5453;
    n.fract().abs()
}

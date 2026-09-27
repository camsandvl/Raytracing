use crate::color::Color;
use nalgebra_glm::Vec3;

/// El `Light` del curso (`position`, `color`, `intensity`) más un modo direccional.
/// Las velas son puntuales (se apagan con la distancia); la luna es direccional (luz
/// paralela, sin atenuación) — sin ella el exterior de la iglesia quedaría negro de
/// noche, porque las velas están adentro.
pub struct Light {
    /// Posición de la luz, o — si `directional` — la dirección normalizada HACIA ella.
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub directional: bool,
}

impl Light {
    pub fn new(position: Vec3, color: Color, intensity: f32) -> Self {
        Light { position, color, intensity, directional: false }
    }

    pub fn directional(toward_light: Vec3, color: Color, intensity: f32) -> Self {
        Light { position: toward_light.normalize(), color, intensity, directional: true }
    }
}

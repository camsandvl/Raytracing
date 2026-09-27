//! Mismo `RayIntersect`/`Intersect`/`Material` que el curso (`ray_intersect.rs` en
//! `18-RT-06-REFLECTIONS`), extendido con lo que pide la rúbrica y lo que el curso
//! todavía no publicó: transparencia + índice de refracción, textura propia por
//! material, y coordenadas (u, v) en el punto de impacto para poder muestrearla.
//!
//! `albedo` sigue siendo un arreglo de PESOS (no colores) tal como lo usa `shade()` del
//! curso, solo que ahora de 4 elementos: `[k_diffuse, k_specular, k_reflection,
//! k_transparency]`. `specular` sigue siendo el EXPONENTE de Phong, no un peso — mismo
//! nombre un poco engañoso que ya traía el curso, se mantiene por consistencia con su
//! código en vez de renombrarlo a mitad de proyecto.

use crate::color::Color;
use crate::texture::TextureId;
use nalgebra_glm::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    pub diffuse: Color,
    pub specular: f32,
    pub albedo: [f32; 4],
    pub refractive_index: f32,
    pub texture: Option<TextureId>,
    /// > 0 = el material emite su propio color sin depender de ninguna luz (las
    /// llamas de las velas). No es uno de los parámetros de la rúbrica — sin esto una
    /// llama sería un cubo amarillo apagado, porque la vela que la "enciende" es una
    /// luz puntual ubicada justo encima, no la llama en sí.
    pub emission: f32,
}

impl Material {
    pub fn new(diffuse: Color, specular: f32, albedo: [f32; 4], refractive_index: f32, texture: Option<TextureId>) -> Self {
        Material { diffuse, specular, albedo, refractive_index, texture, emission: 0.0 }
    }

    pub fn with_emission(mut self, emission: f32) -> Self {
        self.emission = emission;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Intersect {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub material: Material,
    pub u: f32,
    pub v: f32,
}

/// `: Sync` — sin esto `Box<dyn RayIntersect>` no se puede compartir por referencia
/// entre hilos (necesario para repartir el render por filas con `std::thread::scope`,
/// ver `render()` en `main.rs`). Cualquier implementor real (como `VoxelGrid`) ya es
/// `Sync` automáticamente por estar hecho solo de datos simples, así que este bound no
/// le exige nada extra a nadie — solo hace explícito lo que ya era cierto.
pub trait RayIntersect: Sync {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect>;
}

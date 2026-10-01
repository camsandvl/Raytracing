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
    /// Un brillo propio tenue que se SUMA a la iluminación normal (a diferencia de
    /// `emission`, que la reemplaza): el resplandor radiactivo de la piel de los zombis.
    pub glow: f32,
}

impl Material {
    pub fn new(diffuse: Color, specular: f32, albedo: [f32; 4], refractive_index: f32, texture: Option<TextureId>) -> Self {
        Material { diffuse, specular, albedo, refractive_index, texture, emission: 0.0, glow: 0.0 }
    }

    pub fn with_emission(mut self, emission: f32) -> Self {
        self.emission = emission;
        self
    }

    pub fn with_glow(mut self, glow: f32) -> Self {
        self.glow = glow;
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

    /// Como `ray_intersect`, pero sin buscar más allá de `max_distance`. Los rayos de
    /// sombra no necesitan nada que esté detrás de la luz, y un objeto que sabe cortar su
    /// recorrido ahí (el DDA del `VoxelGrid`) se ahorra el resto del camino.
    fn ray_intersect_within(&self, ray_origin: &Vec3, ray_direction: &Vec3, max_distance: f32) -> Option<Intersect> {
        self.ray_intersect(ray_origin, ray_direction).filter(|hit| hit.distance < max_distance)
    }

    /// A qué distancia entra el rayo a la caja envolvente del objeto (`None` = no la toca).
    /// Una prueba barata que deja saltear objetos que empiezan más lejos que algo ya
    /// encontrado: un grupo de zombis detrás de un muro no se recorre.
    fn entry_distance(&self, _ray_origin: &Vec3, _ray_direction: &Vec3) -> Option<f32> {
        Some(0.0)
    }

    /// ¿Hay algo sólido dentro de la caja [min, max] (mundo)? Solo lo usa el modo
    /// primera persona para las colisiones; un objeto que no lo implementa es atravesable.
    fn overlaps_box(&self, _min: &Vec3, _max: &Vec3) -> bool {
        false
    }
}

/// Una caja se prueba igual que lo que contiene (la escena es `Vec<Box<dyn RayIntersect>>`).
impl<T: RayIntersect + ?Sized> RayIntersect for Box<T> {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect> {
        (**self).ray_intersect(ray_origin, ray_direction)
    }

    fn ray_intersect_within(&self, ray_origin: &Vec3, ray_direction: &Vec3, max_distance: f32) -> Option<Intersect> {
        (**self).ray_intersect_within(ray_origin, ray_direction, max_distance)
    }

    fn entry_distance(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<f32> {
        (**self).entry_distance(ray_origin, ray_direction)
    }

    fn overlaps_box(&self, min: &Vec3, max: &Vec3) -> bool {
        (**self).overlaps_box(min, max)
    }
}

/// El impacto más cercano antes de `max_distance` entre `objects`. Un objeto cuya caja
/// envolvente empieza más lejos que lo ya encontrado ni se recorre (la catedral va
/// primero: casi siempre deja un impacto cercano que descarta a los grupos de atrás), y
/// los demás no buscan más allá de lo ya encontrado.
pub fn nearest_hit<T: RayIntersect>(objects: &[T], origin: &Vec3, direction: &Vec3, max_distance: f32) -> Option<Intersect> {
    let mut closest: Option<Intersect> = None;
    for object in objects {
        let limit = closest.map_or(max_distance, |c| c.distance);
        if object.entry_distance(origin, direction).is_none_or(|t| t >= limit) {
            continue;
        }
        if let Some(hit) = object.ray_intersect_within(origin, direction, limit) {
            closest = Some(hit);
        }
    }
    closest
}

//! Grupo de grids cercanos detrás de una sola caja envolvente. Cada rayo (también cada
//! rayo de sombra, uno por vela) prueba todos los objetos de la escena: con ~55 props
//! sueltos eso triplicaba el render. Un rayo que no toca la caja del grupo se saltea todos
//! sus props con una sola prueba; uno que la toca recorre solo los grids ajustados de
//! adentro.

use crate::ray_intersect::{Intersect, RayIntersect};
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::Vec3;

pub struct Group {
    min: Vec3,
    max: Vec3,
    items: Vec<VoxelGrid>,
}

impl Group {
    pub fn new(items: Vec<VoxelGrid>) -> Self {
        let (mut min, mut max) = (Vec3::repeat(f32::INFINITY), Vec3::repeat(f32::NEG_INFINITY));
        for item in &items {
            let (lo, hi) = item.bounds();
            min = min.inf(&lo);
            max = max.sup(&hi);
        }
        Group { min, max, items }
    }

    /// Prueba de losas contra la caja del grupo: a qué distancia entra el rayo.
    fn bounds_entry(&self, origin: &Vec3, direction: &Vec3) -> Option<f32> {
        let (mut t_min, mut t_max) = (0.0f32, f32::INFINITY);
        for axis in 0..3 {
            let inv = 1.0 / direction[axis];
            let (mut t0, mut t1) = ((self.min[axis] - origin[axis]) * inv, (self.max[axis] - origin[axis]) * inv);
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
        }
        (t_min <= t_max).then_some(t_min)
    }
}

impl RayIntersect for Group {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect> {
        self.ray_intersect_within(ray_origin, ray_direction, f32::INFINITY)
    }

    fn ray_intersect_within(&self, ray_origin: &Vec3, ray_direction: &Vec3, max_distance: f32) -> Option<Intersect> {
        if self.bounds_entry(ray_origin, ray_direction)? >= max_distance {
            return None;
        }
        crate::ray_intersect::nearest_hit(&self.items, ray_origin, ray_direction, max_distance)
    }

    fn entry_distance(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<f32> {
        self.bounds_entry(ray_origin, ray_direction)
    }

    fn overlaps_box(&self, min: &Vec3, max: &Vec3) -> bool {
        let touches = (0..3).all(|a| min[a] <= self.max[a] && max[a] >= self.min[a]);
        touches && self.items.iter().any(|item| item.overlaps_box(min, max))
    }
}

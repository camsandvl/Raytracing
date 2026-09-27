//! El diorama en sí. En vez de una lista de cubos (que obligaría a probar cada cubo
//! por cada rayo — primario, sombra por cada vela, reflejo, refracción — y no
//! escalaría a una iglesia completa), un solo grid denso con travesía DDA
//! (Amanatides & Woo, "fast voxel traversal"): el costo de un rayo depende de cuántas
//! celdas atraviesa en línea recta, no de cuántas celdas del grid están ocupadas.
//! Implementa `RayIntersect` como un objeto más, así encaja sin fricción en el
//! `Vec<Box<dyn RayIntersect>>` del curso.
//!
//! Cada celda guarda un índice `u8` a una paleta de materiales (0 = aire) en vez del
//! `Material` completo: con más de un millón de celdas, eso es ~1 MB en vez de ~75 MB,
//! y el loop del DDA (que lee celdas todo el tiempo) se mantiene dentro de la caché.

use crate::ray_intersect::{Intersect, Material, RayIntersect};
use nalgebra_glm::Vec3;

const AIR: u8 = 0;

pub struct VoxelGrid {
    pub dims: (usize, usize, usize),
    pub cell_size: f32,
    pub origin: Vec3,
    cells: Vec<u8>,
    palette: Vec<Material>,
}

impl VoxelGrid {
    pub fn new(dims: (usize, usize, usize), cell_size: f32, origin: Vec3) -> Self {
        let count = dims.0 * dims.1 * dims.2;
        VoxelGrid { dims, cell_size, origin, cells: vec![AIR; count], palette: Vec::new() }
    }

    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.dims.1 + y) * self.dims.0 + x
    }

    fn in_bounds(&self, x: isize, y: isize, z: isize) -> bool {
        x >= 0 && y >= 0 && z >= 0 && (x as usize) < self.dims.0 && (y as usize) < self.dims.1 && (z as usize) < self.dims.2
    }

    fn material_id(&mut self, material: Material) -> u8 {
        if let Some(pos) = self.palette.iter().position(|m| *m == material) {
            return (pos + 1) as u8;
        }
        assert!(self.palette.len() < 255, "la paleta del VoxelGrid admite hasta 255 materiales");
        self.palette.push(material);
        self.palette.len() as u8
    }

    fn cell_id(&self, x: isize, y: isize, z: isize) -> u8 {
        if !self.in_bounds(x, y, z) {
            return AIR;
        }
        self.cells[self.index(x as usize, y as usize, z as usize)]
    }

    fn material_of(&self, id: u8) -> Material {
        self.palette[id as usize - 1]
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, material: Material) {
        if x < self.dims.0 && y < self.dims.1 && z < self.dims.2 {
            let id = self.material_id(material);
            let idx = self.index(x, y, z);
            self.cells[idx] = id;
        }
    }

    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<Material> {
        match self.cell_id(x as isize, y as isize, z as isize) {
            AIR => None,
            id => Some(self.material_of(id)),
        }
    }

    /// Vacía una celda (la vuelve aire) — usada para tallar aberturas (puertas, vanos
    /// de ventana) en un muro ya sólido.
    pub fn clear(&mut self, x: usize, y: usize, z: usize) {
        if x < self.dims.0 && y < self.dims.1 && z < self.dims.2 {
            let idx = self.index(x, y, z);
            self.cells[idx] = AIR;
        }
    }

    pub fn is_occupied(&self, x: isize, y: isize, z: isize) -> bool {
        self.cell_id(x, y, z) != AIR
    }

    /// Convierte una posición de mundo a índices de celda (sin clamping).
    pub fn world_to_cell(&self, world: Vec3) -> (isize, isize, isize) {
        let local = (world - self.origin) / self.cell_size;
        (local.x.floor() as isize, local.y.floor() as isize, local.z.floor() as isize)
    }

    /// Fase amplia: intersección contra la caja envolvente completa del grid. Devuelve
    /// `(t_entrada, t_salida, normal_de_entrada)` — evita tocar el arreglo de celdas
    /// para rayos que ni siquiera rozan el diorama.
    fn intersect_bounds(&self, origin: &Vec3, dir: &Vec3) -> Option<(f32, f32, Vec3)> {
        let extent = Vec3::new(self.dims.0 as f32, self.dims.1 as f32, self.dims.2 as f32) * self.cell_size;
        let min = self.origin;
        let max = self.origin + extent;

        let mut t_min = f32::NEG_INFINITY;
        let mut t_max = f32::INFINITY;
        let mut normal = Vec3::new(0.0, 0.0, 0.0);

        for axis in 0..3 {
            let o = origin[axis];
            let d = dir[axis];
            let lo = min[axis];
            let hi = max[axis];

            if d.abs() < 1e-8 {
                if o < lo || o > hi {
                    return None;
                }
                continue;
            }

            let (mut t1, mut t2) = ((lo - o) / d, (hi - o) / d);
            let mut sign = -1.0;
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                sign = 1.0;
            }
            if t1 > t_min {
                t_min = t1;
                normal = Vec3::new(0.0, 0.0, 0.0);
                normal[axis] = sign;
            }
            if t2 < t_max {
                t_max = t2;
            }
            if t_min > t_max {
                return None;
            }
        }

        if t_max < 0.0 {
            return None;
        }

        Some((t_min.max(0.0), t_max, normal))
    }

    /// Para un eje: a qué `t` (a partir de `ray_origin`) cae el próximo cruce de
    /// celda, y cuánto avanza `t` por cada celda completa recorrida en este eje.
    /// `.max(t_enter)` es un colchón contra empates en el borde por redondeo de punto
    /// flotante, que romperían "avanzar siempre hacia adelante".
    fn axis_step(origin: f32, dir: f32, index: isize, grid_origin: f32, cell_size: f32, t_enter: f32) -> (f32, f32) {
        if dir.abs() < 1e-8 {
            return (f32::INFINITY, f32::INFINITY);
        }
        let next_boundary = if dir > 0.0 {
            grid_origin + (index + 1) as f32 * cell_size
        } else {
            grid_origin + index as f32 * cell_size
        };
        let t_max = ((next_boundary - origin) / dir).max(t_enter);
        let t_delta = cell_size / dir.abs();
        (t_max, t_delta)
    }

    /// Recorre el grid con DDA desde la celda `start` hasta que `stop(id)` sea cierto
    /// para la celda a la que se acaba de entrar (fuera del grid cuenta como aire).
    /// Devuelve `(t, eje, id)` de ese cruce, o `None` si pasa `t_exit` o sale del grid
    /// sin que `stop` se cumpla.
    fn march(
        &self,
        ray_origin: &Vec3,
        ray_direction: &Vec3,
        start: (isize, isize, isize),
        t_enter: f32,
        t_exit: f32,
        stop: impl Fn(u8) -> bool,
    ) -> Option<(f32, usize, u8)> {
        let (mut ix, mut iy, mut iz) = start;
        let step = |d: f32| -> isize {
            if d > 0.0 {
                1
            } else if d < 0.0 {
                -1
            } else {
                0
            }
        };
        let (sx, sy, sz) = (step(ray_direction.x), step(ray_direction.y), step(ray_direction.z));

        let (mut tx, dx) = Self::axis_step(ray_origin.x, ray_direction.x, ix, self.origin.x, self.cell_size, t_enter);
        let (mut ty, dy) = Self::axis_step(ray_origin.y, ray_direction.y, iy, self.origin.y, self.cell_size, t_enter);
        let (mut tz, dz) = Self::axis_step(ray_origin.z, ray_direction.z, iz, self.origin.z, self.cell_size, t_enter);

        loop {
            // Avanza por el eje cuyo próximo cruce de celda está más cerca — la
            // esencia de Amanatides-Woo: caminar celda por celda siguiendo la línea
            // recta del rayo, sin visitar ninguna celda que el rayo no atraviese.
            let (axis, t_next) = if tx <= ty && tx <= tz {
                ix += sx;
                tx += dx;
                (0, tx - dx)
            } else if ty <= tz {
                iy += sy;
                ty += dy;
                (1, ty - dy)
            } else {
                iz += sz;
                tz += dz;
                (2, tz - dz)
            };

            if t_next > t_exit {
                return None;
            }
            if !self.in_bounds(ix, iy, iz) {
                return if stop(AIR) { Some((t_next, axis, AIR)) } else { None };
            }

            let id = self.cells[self.index(ix as usize, iy as usize, iz as usize)];
            if stop(id) {
                return Some((t_next, axis, id));
            }
        }
    }

    fn hit(&self, ray_origin: &Vec3, ray_direction: &Vec3, t: f32, normal: Vec3, id: u8) -> Intersect {
        let point = ray_origin + ray_direction * t;
        let (u, v) = face_uv(&point, &normal, self.cell_size);
        Intersect { point, normal, distance: t, material: self.material_of(id), u, v }
    }
}

fn axis_normal(axis: usize, sign: f32) -> Vec3 {
    let mut n = Vec3::new(0.0, 0.0, 0.0);
    n[axis] = sign;
    n
}

impl RayIntersect for VoxelGrid {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect> {
        // Caso especial: el rayo arranca DENTRO de una celda ocupada. Solo pasa con el
        // rayo refractado que sigue viajando adentro de un material transparente (ver
        // `offset_origin` en `main.rs`, que lo ubica a propósito del lado del vidrio).
        // Si se aplicara la lógica normal de "primera celda ocupada = hit", el rayo se
        // re-golpearía contra su propia cara de entrada a t≈0 sin atravesar nunca el
        // vidrio — el vidrio se vería como un bloque opaco oscuro. En cambio, se sigue
        // avanzando mientras el material sea el MISMO (el vidrio puede ocupar varias
        // celdas contiguas) y se devuelve la cara donde cambia: la cara de SALIDA.
        let start = self.world_to_cell(*ray_origin);
        let start_id = self.cell_id(start.0, start.1, start.2);
        if start_id != AIR {
            let (t, axis, _) = self.march(ray_origin, ray_direction, start, 0.0, f32::INFINITY, |id| id != start_id)?;
            // Normal de SALIDA: apunta EN la dirección del avance (es la cara de atrás
            // del material que se está dejando, "afuera" es hacia donde el rayo ya va).
            let normal = axis_normal(axis, ray_direction[axis].signum());
            return Some(self.hit(ray_origin, ray_direction, t, normal, start_id));
        }

        let (t_enter, t_exit, entry_normal) = self.intersect_bounds(ray_origin, ray_direction)?;

        let entry_point = ray_origin + ray_direction * (t_enter + 1e-4);
        let local = (entry_point - self.origin) / self.cell_size;
        let entry = (
            (local.x.floor() as isize).clamp(0, self.dims.0 as isize - 1),
            (local.y.floor() as isize).clamp(0, self.dims.1 as isize - 1),
            (local.z.floor() as isize).clamp(0, self.dims.2 as isize - 1),
        );

        // Si la celda de entrada ya está ocupada, el impacto es la propia cara de
        // entrada del grid — su normal ya la calculó `intersect_bounds`.
        let entry_id = self.cell_id(entry.0, entry.1, entry.2);
        if entry_id != AIR {
            return Some(self.hit(ray_origin, ray_direction, t_enter, entry_normal, entry_id));
        }

        let (t, axis, id) = self.march(ray_origin, ray_direction, entry, t_enter, t_exit, |id| id != AIR)?;
        // Normal de ENTRADA: apunta contra el avance, hacia afuera del sólido tocado.
        let normal = axis_normal(axis, -ray_direction[axis].signum());
        Some(self.hit(ray_origin, ray_direction, t, normal, id))
    }
}

/// (u, v) sobre la cara golpeada, en unidades de CELDA (sin envolver): las dos
/// componentes que NO son el eje de la normal recorren la cara. Cada textura decide
/// después cuántas celdas abarca un tile (`Texture::sample`).
fn face_uv(point: &Vec3, normal: &Vec3, cell_size: f32) -> (f32, f32) {
    let p = point / cell_size;
    if normal.x.abs() > 0.5 {
        (p.z, p.y)
    } else if normal.y.abs() > 0.5 {
        (p.x, p.z)
    } else {
        (p.x, p.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    fn mat(r: u8) -> Material {
        Material::new(Color::new(r, 0, 0), 10.0, [1.0, 0.0, 0.0, 0.0], 1.0, None)
    }

    #[test]
    fn ray_from_outside_hits_first_occupied_cell_with_facing_normal() {
        let mut grid = VoxelGrid::new((10, 10, 10), 1.0, Vec3::new(0.0, 0.0, 0.0));
        grid.set(5, 5, 5, mat(1));
        let hit = grid
            .ray_intersect(&Vec3::new(5.5, 5.5, -3.0), &Vec3::new(0.0, 0.0, 1.0))
            .expect("debe pegarle al cubo");
        assert!((hit.distance - 8.0).abs() < 1e-3, "entra por la cara z=5, a 8 unidades del origen");
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, -1.0));
    }

    #[test]
    fn ray_inside_transparent_block_exits_through_far_face() {
        let mut grid = VoxelGrid::new((10, 10, 10), 1.0, Vec3::new(0.0, 0.0, 0.0));
        let glass = mat(2);
        for z in 3..6 {
            grid.set(5, 5, z, glass);
        }
        // Arranca adentro de la primera celda del bloque de 3 celdas de espesor.
        let hit = grid
            .ray_intersect(&Vec3::new(5.5, 5.5, 3.001), &Vec3::new(0.0, 0.0, 1.0))
            .expect("debe encontrar la cara de salida");
        assert!((hit.point.z - 6.0).abs() < 1e-3, "sale por z=6, atravesando las 3 celdas");
        assert_eq!(hit.normal, Vec3::new(0.0, 0.0, 1.0), "la normal de salida apunta en la dirección del avance");
    }

    #[test]
    fn distinct_materials_get_distinct_palette_entries() {
        let mut grid = VoxelGrid::new((4, 4, 4), 1.0, Vec3::new(0.0, 0.0, 0.0));
        grid.set(0, 0, 0, mat(1));
        grid.set(1, 0, 0, mat(2));
        grid.set(2, 0, 0, mat(1));
        assert_eq!(grid.palette.len(), 2);
        assert_eq!(grid.get(2, 0, 0), Some(mat(1)));
        assert_eq!(grid.get(3, 0, 0), None);
    }
}

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
    /// Normales suavizadas por celda (vacío = se usa la cara del cubo). Ver
    /// `smooth_normals`.
    normals: Vec<[i8; 3]>,
}

impl VoxelGrid {
    pub fn new(dims: (usize, usize, usize), cell_size: f32, origin: Vec3) -> Self {
        let count = dims.0 * dims.1 * dims.2;
        VoxelGrid { dims, cell_size, origin, cells: vec![AIR; count], palette: Vec::new(), normals: Vec::new() }
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

    /// Oclusión ambiental horneada en el color: cada celda de superficie se oscurece
    /// según cuánto la rodea su propio volumen (cuántas de las celdas a menos de
    /// `radius` están ocupadas). Una cara plana no cambia; un pliegue de la tela, una
    /// axila, la cuenca de un ojo o la comisura de la boca se oscurecen, como la sombra
    /// que junta el polvo en una escultura. Sin esto, con una sola luz, una figura de un
    /// solo color se lee como una silueta plana. Los materiales transparentes (el velo) y
    /// los que emiten luz no se tocan. Los tonos se cuantizan para no llenar la paleta.
    pub fn bake_occlusion(&mut self, radius: isize, strength: f32) {
        let (w, h, d) = (self.dims.0 as isize, self.dims.1 as isize, self.dims.2 as isize);
        let total = ((2 * radius + 1).pow(3)) as f32;
        let flat = (radius + 1) as f32 / (2 * radius + 1) as f32; // ocupación de una cara plana
        let mut shaded = self.cells.clone();
        for z in 0..d {
            for y in 0..h {
                for x in 0..w {
                    let id = self.cell_id(x, y, z);
                    if id == AIR {
                        continue;
                    }
                    let material = self.material_of(id);
                    if material.albedo[3] > 0.0 || material.emission > 0.0 {
                        continue;
                    }
                    let exposed = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]
                        .iter()
                        .any(|&(dx, dy, dz)| !self.is_occupied(x + dx, y + dy, z + dz));
                    if !exposed {
                        continue;
                    }
                    let mut filled = 0;
                    for dz in -radius..=radius {
                        for dy in -radius..=radius {
                            for dx in -radius..=radius {
                                filled += self.is_occupied(x + dx, y + dy, z + dz) as usize;
                            }
                        }
                    }
                    let shade = (1.0 - strength * (filled as f32 / total - flat)).clamp(0.45, 1.0);
                    let shade = (shade * 10.0).round() / 10.0;
                    if shade < 1.0 {
                        let darker = Material { diffuse: material.diffuse * shade, glow: material.glow * shade, ..material };
                        shaded[self.index(x as usize, y as usize, z as usize)] = self.material_id(darker);
                    }
                }
            }
        }
        self.cells = shaded;
    }

    /// Normales suavizadas para las figuras esculpidas: cada celda de superficie guarda la
    /// dirección contraria a la masa que la rodea (a menos de `radius` celdas), así la luz
    /// sigue la forma esculpida (el hombro redondo, la tela que cae) y no la escalera de
    /// cubos: sin esto, una superficie inclinada se ve rayada, cada escalón con su cara de
    /// arriba iluminada y la de adelante oscura. La silueta sigue siendo de vóxeles; solo
    /// cambia cómo se ilumina. Las celdas transparentes (el velo) quedan con la cara.
    pub fn smooth_normals(&mut self, radius: isize) {
        let (w, h, d) = (self.dims.0 as isize, self.dims.1 as isize, self.dims.2 as isize);
        let mut normals = vec![[0i8; 3]; self.cells.len()];
        for z in 0..d {
            for y in 0..h {
                for x in 0..w {
                    let id = self.cell_id(x, y, z);
                    if id == AIR || self.material_of(id).albedo[3] > 0.0 {
                        continue;
                    }
                    let mut away = Vec3::zeros();
                    let mut exposed = false;
                    for dz in -radius..=radius {
                        for dy in -radius..=radius {
                            for dx in -radius..=radius {
                                if dx * dx + dy * dy + dz * dz > radius * radius {
                                    continue;
                                }
                                if self.is_occupied(x + dx, y + dy, z + dz) {
                                    away -= Vec3::new(dx as f32, dy as f32, dz as f32);
                                } else if dx.abs() + dy.abs() + dz.abs() == 1 {
                                    exposed = true;
                                }
                            }
                        }
                    }
                    if exposed && away.magnitude() > 1e-3 {
                        let n = away.normalize() * 127.0;
                        normals[self.index(x as usize, y as usize, z as usize)] = [n.x as i8, n.y as i8, n.z as i8];
                    }
                }
            }
        }
        self.normals = normals;
    }

    /// Caja envolvente en mundo: `(mínimo, máximo)`.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let extent = Vec3::new(self.dims.0 as f32, self.dims.1 as f32, self.dims.2 as f32) * self.cell_size;
        (self.origin, self.origin + extent)
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
        let step = |d: f32| -> isize {
            if d > 0.0 {
                1
            } else if d < 0.0 {
                -1
            } else {
                0
            }
        };
        let steps = [step(ray_direction.x), step(ray_direction.y), step(ray_direction.z)];
        let dims = [self.dims.0 as isize, self.dims.1 as isize, self.dims.2 as isize];
        // Cuánto avanza el índice lineal de `cells` al pasar a la celda vecina en cada eje:
        // se lleva el índice al día sumando, sin volver a multiplicar en cada paso.
        let strides = [steps[0], steps[1] * dims[0], steps[2] * dims[0] * dims[1]];
        let mut cell = [start.0, start.1, start.2];
        let mut index = (start.2 * dims[1] + start.1) * dims[0] + start.0;

        let (tx, dx) = Self::axis_step(ray_origin.x, ray_direction.x, start.0, self.origin.x, self.cell_size, t_enter);
        let (ty, dy) = Self::axis_step(ray_origin.y, ray_direction.y, start.1, self.origin.y, self.cell_size, t_enter);
        let (tz, dz) = Self::axis_step(ray_origin.z, ray_direction.z, start.2, self.origin.z, self.cell_size, t_enter);
        let (mut t_max, t_delta) = ([tx, ty, tz], [dx, dy, dz]);

        loop {
            // Avanza por el eje cuyo próximo cruce de celda está más cerca — la
            // esencia de Amanatides-Woo: caminar celda por celda siguiendo la línea
            // recta del rayo, sin visitar ninguna celda que el rayo no atraviese.
            let axis = if t_max[0] <= t_max[1] && t_max[0] <= t_max[2] {
                0
            } else if t_max[1] <= t_max[2] {
                1
            } else {
                2
            };
            let t_next = t_max[axis];
            if t_next > t_exit {
                return None;
            }
            t_max[axis] += t_delta[axis];
            cell[axis] += steps[axis];
            index += strides[axis];
            // Solo cambió un eje: basta con mirar ese para saber si se salió del grid.
            if cell[axis] < 0 || cell[axis] >= dims[axis] {
                return if stop(AIR) { Some((t_next, axis, AIR)) } else { None };
            }

            let id = self.cells[index as usize];
            if stop(id) {
                return Some((t_next, axis, id));
            }
        }
    }

    fn hit(&self, ray_origin: &Vec3, ray_direction: &Vec3, t: f32, face: Vec3, id: u8) -> Intersect {
        let point = ray_origin + ray_direction * t;
        let (u, v) = face_uv(&point, &face, self.cell_size);
        Intersect { point, normal: self.smooth_normal(&point, face), distance: t, material: self.material_of(id), u, v }
    }

    /// La normal suavizada de la celda tocada en `point` por su cara `face`, si la hay,
    /// inclinada hacia la cara cuando casi la contradice (para que los rayos de sombra y
    /// de reflejo sigan saliendo del lado correcto del cubo).
    fn smooth_normal(&self, point: &Vec3, face: Vec3) -> Vec3 {
        if self.normals.is_empty() {
            return face;
        }
        let (x, y, z) = self.world_to_cell(point - face * (0.5 * self.cell_size));
        if !self.in_bounds(x, y, z) {
            return face;
        }
        let [nx, ny, nz] = self.normals[self.index(x as usize, y as usize, z as usize)];
        if nx == 0 && ny == 0 && nz == 0 {
            return face;
        }
        let smooth = Vec3::new(nx as f32, ny as f32, nz as f32).normalize();
        let along = smooth.dot(&face);
        if along < 0.3 {
            (smooth + face * (0.3 - along) * 1.5).normalize()
        } else {
            smooth
        }
    }
}

fn axis_normal(axis: usize, sign: f32) -> Vec3 {
    let mut n = Vec3::new(0.0, 0.0, 0.0);
    n[axis] = sign;
    n
}

impl RayIntersect for VoxelGrid {
    fn entry_distance(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<f32> {
        self.intersect_bounds(ray_origin, ray_direction).map(|(t, _, _)| t)
    }

    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<Intersect> {
        self.ray_intersect_within(ray_origin, ray_direction, f32::INFINITY)
    }

    /// El DDA se corta en `max_distance`: un rayo de sombra hacia una vela no sigue
    /// recorriendo el grid más allá de la llama.
    fn ray_intersect_within(&self, ray_origin: &Vec3, ray_direction: &Vec3, max_distance: f32) -> Option<Intersect> {
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
            let (t, axis, _) = self.march(ray_origin, ray_direction, start, 0.0, max_distance, |id| id != start_id)?;
            // Normal de SALIDA: apunta EN la dirección del avance (es la cara de atrás
            // del material que se está dejando, "afuera" es hacia donde el rayo ya va).
            let normal = axis_normal(axis, ray_direction[axis].signum());
            return Some(self.hit(ray_origin, ray_direction, t, normal, start_id));
        }

        let (t_enter, t_exit, entry_normal) = self.intersect_bounds(ray_origin, ray_direction)?;
        if t_enter >= max_distance {
            return None;
        }

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

        let (t, axis, id) = self.march(ray_origin, ray_direction, entry, t_enter, t_exit.min(max_distance), |id| id != AIR)?;
        // Normal de ENTRADA: apunta contra el avance, hacia afuera del sólido tocado.
        let normal = axis_normal(axis, -ray_direction[axis].signum());
        Some(self.hit(ray_origin, ray_direction, t, normal, id))
    }

    fn overlaps_box(&self, min: &Vec3, max: &Vec3) -> bool {
        let lo = self.world_to_cell(*min);
        let hi = self.world_to_cell(*max);
        if hi.0 < 0 || hi.1 < 0 || hi.2 < 0 || lo.0 >= self.dims.0 as isize || lo.1 >= self.dims.1 as isize || lo.2 >= self.dims.2 as isize {
            return false;
        }
        (lo.2..=hi.2).any(|z| (lo.1..=hi.1).any(|y| (lo.0..=hi.0).any(|x| self.is_occupied(x, y, z))))
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

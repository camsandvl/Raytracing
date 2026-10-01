//! Grid de 5 cm sobre el que se estampan cápsulas y conos en coordenadas de mundo. Lo usan
//! los zombis (`zombies.rs`) y la novia inclinada hacia atrás (`people.rs`): figuras en
//! poses que no se pueden armar con cajas alineadas a los ejes (también cajas torcidas).

use crate::ray_intersect::Material;
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::Vec3;

pub struct Canvas {
    grid: VoxelGrid,
    origin: Vec3,
}

impl Canvas {
    /// Un canvas que cubre la caja `min`–`max`, con celdas de `cell_size`. Si `min.y` es la
    /// altura del piso, la primera capa de celdas queda apoyada justo sobre el piso (para
    /// la sangre salpicada).
    pub fn new(min: Vec3, max: Vec3, cell_size: f32) -> Self {
        let size = (max - min) / cell_size;
        let dims = (size.x.ceil() as usize + 1, size.y.ceil() as usize + 1, size.z.ceil() as usize + 1);
        Canvas { grid: VoxelGrid::new(dims, cell_size, min), origin: min }
    }

    /// Rellena todas las celdas a menos de `r` del segmento `a`–`b`; `paint` elige el
    /// material de cada celda según su posición (para las manchas).
    pub fn capsule(&mut self, a: Vec3, b: Vec3, r: f32, paint: &dyn Fn(Vec3) -> Material) {
        self.cone(a, b, r, r, paint);
    }

    /// Como `capsule`, pero con el radio pasando de `ra` en `a` a `rb` en `b` (la falda).
    pub fn cone(&mut self, a: Vec3, b: Vec3, ra: f32, rb: f32, paint: &dyn Fn(Vec3) -> Material) {
        let r = ra.max(rb);
        let (lo, hi) = (self.grid.world_to_cell(a.inf(&b) - Vec3::repeat(r)), self.grid.world_to_cell(a.sup(&b) + Vec3::repeat(r)));
        let ab = b - a;
        let len2 = ab.dot(&ab).max(1e-6);
        for z in lo.2.max(0)..=hi.2 {
            for y in lo.1.max(0)..=hi.1 {
                for x in lo.0.max(0)..=hi.0 {
                    let p = self.center(x, y, z);
                    let t = ((p - a).dot(&ab) / len2).clamp(0.0, 1.0);
                    if (p - (a + ab * t)).magnitude() <= ra + (rb - ra) * t {
                        self.grid.set(x as usize, y as usize, z as usize, paint(p));
                    }
                }
            }
        }
    }

    /// Una caja orientada: centro, sus tres ejes (unitarios, perpendiculares) y la mitad de
    /// su tamaño en cada eje. Las cabezas cuadradas de la novia inclinada y del zombi que
    /// la muerde, que van torcidas.
    pub fn cuboid(&mut self, center: Vec3, axes: [Vec3; 3], half: Vec3, paint: &dyn Fn(Vec3) -> Material) {
        let reach = Vec3::repeat(half.magnitude());
        let (lo, hi) = (self.grid.world_to_cell(center - reach), self.grid.world_to_cell(center + reach));
        for z in lo.2.max(0)..=hi.2 {
            for y in lo.1.max(0)..=hi.1 {
                for x in lo.0.max(0)..=hi.0 {
                    let p = self.center(x, y, z);
                    let d = p - center;
                    if (0..3).all(|i| d.dot(&axes[i]).abs() <= half[i]) {
                        self.grid.set(x as usize, y as usize, z as usize, paint(p));
                    }
                }
            }
        }
    }

    /// Un triángulo plano `a`–`b`–`c` de `half_thickness` de medio espesor: las astillas de
    /// vidrio en vuelo.
    pub fn triangle(&mut self, [a, b, c]: [Vec3; 3], half_thickness: f32, paint: &dyn Fn(Vec3) -> Material) {
        let (e1, e2) = (b - a, c - a);
        let normal = e1.cross(&e2);
        if normal.magnitude() < 1e-6 {
            return;
        }
        let normal = normal.normalize();
        let pad = Vec3::repeat(half_thickness);
        let (lo, hi) = (self.grid.world_to_cell(a.inf(&b).inf(&c) - pad), self.grid.world_to_cell(a.sup(&b).sup(&c) + pad));
        let (d11, d12, d22) = (e1.dot(&e1), e1.dot(&e2), e2.dot(&e2));
        let det = d11 * d22 - d12 * d12;
        for z in lo.2.max(0)..=hi.2 {
            for y in lo.1.max(0)..=hi.1 {
                for x in lo.0.max(0)..=hi.0 {
                    let p = self.center(x, y, z);
                    let w = p - a;
                    if w.dot(&normal).abs() > half_thickness {
                        continue;
                    }
                    let (w1, w2) = (w.dot(&e1), w.dot(&e2));
                    let (u, v) = ((d22 * w1 - d12 * w2) / det, (d11 * w2 - d12 * w1) / det);
                    if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
                        self.grid.set(x as usize, y as usize, z as usize, paint(p));
                    }
                }
            }
        }
    }

    /// Pinta, en la capa de celdas más baja, las que cumplen `inside` (posición en mundo):
    /// manchas planas sobre el piso.
    pub fn floor_layer(&mut self, center: Vec3, reach: f32, material: Material, inside: &dyn Fn(Vec3) -> bool) {
        let (lo, hi) = (self.grid.world_to_cell(center - Vec3::repeat(reach)), self.grid.world_to_cell(center + Vec3::repeat(reach)));
        for z in lo.2.max(0)..=hi.2 {
            for x in lo.0.max(0)..=hi.0 {
                if inside(self.center(x, 0, z)) {
                    self.grid.set(x as usize, 0, z as usize, material);
                }
            }
        }
    }

    /// Recorre las celdas de la caja `lo`–`hi` y pinta las que `inside` devuelve con un
    /// material (las que devuelve `None` quedan como estaban). Es la base de la escultura
    /// por campos de distancia de `sculpt.rs`.
    pub fn fill(&mut self, lo: Vec3, hi: Vec3, inside: &dyn Fn(Vec3) -> Option<Material>) {
        let (lo, hi) = (self.grid.world_to_cell(lo), self.grid.world_to_cell(hi));
        for z in lo.2.max(0)..=hi.2 {
            for y in lo.1.max(0)..=hi.1 {
                for x in lo.0.max(0)..=hi.0 {
                    if let Some(material) = inside(self.center(x, y, z)) {
                        self.grid.set(x as usize, y as usize, z as usize, material);
                    }
                }
            }
        }
    }

    pub fn cell_size(&self) -> f32 {
        self.grid.cell_size
    }

    fn center(&self, x: isize, y: isize, z: isize) -> Vec3 {
        self.origin + Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) * self.grid.cell_size
    }

    pub fn into_grid(self) -> VoxelGrid {
        self.grid
    }
}

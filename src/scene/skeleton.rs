//! Los tres esqueletos de la boda — novio, novia y oficiante — con musgo y plantitas
//! creciéndoles encima: llevan ahí mucho tiempo esperando una boda que nunca terminó.
//!
//! Cada esqueleto vive en su propio `VoxelGrid` con celdas 4 veces más chicas que las
//! de la iglesia (`DETAIL`): a la escala de la iglesia (4 celdas por metro) una
//! persona mediría apenas 7 celdas y un esqueleto sería una mancha. Con celdas finas
//! mide 28, suficiente para costillas, cuencas de los ojos y dedos. Son objetos
//! `RayIntersect` independientes que el render recorre junto con la iglesia.

use crate::materials::MaterialSet;
use crate::ray_intersect::Material;
use crate::scene::builders::hash;
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::Vec3;

pub const DETAIL: f32 = 0.25;
const HALF: isize = 8;
const DIMS: (usize, usize, usize) = (17, 37, 17);

/// Hacia dónde mira el esqueleto (en coordenadas de mundo).
#[derive(Clone, Copy)]
pub enum Facing {
    PosX,
    NegX,
    NegZ,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    Groom,
    Bride,
    Officiant,
}

struct Body<'a> {
    grid: VoxelGrid,
    facing: Facing,
    m: &'a MaterialSet,
    seed: u32,
}

impl Body<'_> {
    /// Coordenadas locales del esqueleto: `x` hacia el costado, `y` hacia arriba
    /// desde los pies, `z` hacia adelante (hacia donde mira). Se rotan según `facing`.
    fn set(&mut self, x: isize, y: isize, z: isize, material: Material) {
        let (gx, gz) = match self.facing {
            Facing::PosX => (HALF + z, HALF - x),
            Facing::NegX => (HALF - z, HALF + x),
            Facing::NegZ => (HALF - x, HALF - z),
        };
        if gx >= 0 && gz >= 0 && y >= 0 {
            self.grid.set(gx as usize, y as usize, gz as usize, material);
        }
    }

    fn fill(&mut self, x: (isize, isize), y: (isize, isize), z: (isize, isize), material: Material) {
        for xi in x.0..x.1 {
            for yi in y.0..y.1 {
                for zi in z.0..z.1 {
                    self.set(xi, yi, zi, material);
                }
            }
        }
    }

    fn skeleton(&mut self) {
        let bone = self.m.bone_moss;

        // Piernas (tibia + fémur en una línea, rótula marcada) y pies hacia adelante.
        for lx in [-2, 2] {
            self.fill((lx, lx + 1), (0, 1), (-1, 3), bone);
            self.fill((lx, lx + 1), (1, 14), (0, 1), bone);
            self.set(lx, 7, 1, bone);
        }

        // Pelvis y crestas ilíacas.
        self.fill((-3, 4), (13, 14), (0, 1), bone);
        for lx in [-3, 3] {
            self.fill((lx, lx + 1), (14, 16), (-1, 2), bone);
        }

        // Columna, por detrás de la caja torácica.
        self.fill((0, 1), (14, 26), (-1, 0), bone);

        // Costillas: anillos que se abren al frente a ambos lados del esternón, más
        // angostos abajo.
        for (y, half) in [(17, 2isize), (19, 3), (21, 3), (23, 3)] {
            for lx in -half..=half {
                for lz in -1..=2 {
                    let outline = lx.abs() == half || lz == -1 || lz == 2;
                    let gap_by_sternum = lz == 2 && lx.abs() == 1;
                    if outline && !gap_by_sternum {
                        self.set(lx, y, lz, bone);
                    }
                }
            }
        }
        self.fill((0, 1), (18, 24), (2, 3), bone);

        // Clavículas, cuello, mandíbula y cráneo.
        self.fill((-5, 6), (24, 25), (0, 1), bone);
        self.set(0, 25, 0, bone);
        self.fill((-1, 2), (26, 27), (0, 3), bone);
        self.fill((-2, 3), (27, 32), (-2, 3), bone);

        // Cuencas de los ojos y nariz: huecos oscuros de dos celdas de profundidad.
        for (x, y) in [(-1, 29), (1, 29), (0, 28)] {
            self.set(x, y, 2, self.m.soot);
            self.set(x, y, 1, self.m.soot);
        }
        // Dientes: la fila superior de la mandíbula, alternada.
        for x in [-1, 1] {
            self.set(x, 26, 2, self.m.soot);
        }
    }

    fn arm_hanging(&mut self, side: isize) {
        let bone = self.m.bone_moss;
        let x = 5 * side;
        self.fill((x, x + 1), (12, 24), (0, 1), bone);
        self.fill((x, x + 1), (10, 12), (0, 2), bone);
    }

    /// Brazo con el codo doblado y el antebrazo hacia adelante hasta `reach`.
    fn arm_forward(&mut self, side: isize, elbow_y: isize, reach: isize) {
        let bone = self.m.bone_moss;
        let x = 5 * side;
        self.fill((x, x + 1), (elbow_y, 24), (0, 1), bone);
        self.fill((x, x + 1), (elbow_y, elbow_y + 1), (1, reach), bone);
        self.fill((x, x + 1), (elbow_y, elbow_y + 2), (reach, reach + 1), bone);
    }

    /// Musgo y plantitas: un colchón de musgo alrededor de los pies, brotes que
    /// suben desde el piso, y hojitas y alguna flor sobre los hombros y la pelvis.
    fn overgrowth(&mut self) {
        let (leaves, petal) = (self.m.leaves, self.m.petal);
        for x in -7..=7 {
            for z in -7..=7 {
                let d2 = x * x + z * z;
                let r = hash(x, z, self.seed);
                if d2 <= 36 && r < 0.45 && !(x.abs() == 2 && (-1..3).contains(&z)) {
                    self.set(x, 0, z, leaves);
                    if r < 0.08 {
                        let height = 1 + (hash(z, x, self.seed + 1) * 3.0) as isize;
                        self.fill((x, x + 1), (1, 1 + height), (z, z + 1), leaves);
                        if r < 0.03 {
                            self.set(x, 1 + height, z, petal);
                        }
                    }
                }
            }
        }
        for (x, y, z) in [(-4, 25, 0), (4, 25, 0), (-3, 16, 0), (3, 16, 1)] {
            if hash(x, y, self.seed + 2) < 0.7 {
                self.set(x, y, z, leaves);
            }
        }
        self.set(4, 26, 0, petal);
    }

    fn bride_details(&mut self) {
        // Velo: tela semitransparente sobre el cráneo que cae por la espalda, con el
        // borde inferior deshilachado.
        let veil = self.m.veil;
        self.fill((-3, 4), (32, 33), (-3, 3), veil);
        for x in -3..=3 {
            let bottom = 11 + (hash(x, 0, self.seed + 3) * 4.0) as isize;
            self.fill((x, x + 1), (bottom, 32), (-3, -2), veil);
        }
        for x in [-3, 3] {
            self.fill((x, x + 1), (22, 32), (-3, 0), veil);
        }

        // Ramo marchito en la mano que cuelga.
        for (x, y, z) in [(-5, 12, 2), (-6, 12, 2), (-5, 13, 2), (-5, 12, 3), (-4, 12, 2), (-5, 11, 2)] {
            self.set(x, y, z, self.m.leaves);
        }
        for (x, y, z) in [(-5, 14, 2), (-6, 13, 2), (-4, 13, 3), (-6, 12, 3)] {
            self.set(x, y, z, self.m.petal);
        }
    }

    fn groom_details(&mut self) {
        let soot = self.m.soot;
        // Moño y sombrero de copa.
        self.fill((-1, 2), (25, 26), (1, 2), soot);
        self.fill((-3, 4), (32, 33), (-3, 4), soot);
        self.fill((-2, 3), (33, 37), (-2, 3), soot);
    }

    fn officiant_details(&mut self) {
        // Estola colgando al frente de la caja torácica.
        for x in [-2, 2] {
            self.fill((x, x + 1), (12, 25), (3, 4), self.m.leather);
        }
        // Libro abierto sostenido con ambas manos: tapas de cuero y hojas encima.
        self.fill((-4, 5), (19, 20), (3, 7), self.m.leather);
        for x in -4..5 {
            if x != 0 {
                self.fill((x, x + 1), (20, 21), (3, 7), self.m.wax);
            }
        }
    }
}

/// Construye un esqueleto parado con los pies centrados en `feet` (mundo).
pub fn build(m: &MaterialSet, role: Role, feet: Vec3, facing: Facing, seed: u32) -> VoxelGrid {
    let origin = feet - Vec3::new(HALF as f32 + 0.5, 0.0, HALF as f32 + 0.5) * DETAIL;
    let mut body = Body { grid: VoxelGrid::new(DIMS, DETAIL, origin), facing, m, seed };

    body.skeleton();
    match role {
        // Los novios se toman de la mano: cada uno estira el brazo que queda del
        // mismo lado del mundo (por eso los lados son opuestos en coordenadas locales).
        Role::Bride => {
            body.arm_forward(1, 18, 7);
            body.arm_hanging(-1);
            body.bride_details();
        }
        Role::Groom => {
            body.arm_forward(-1, 18, 7);
            body.arm_hanging(1);
            body.groom_details();
        }
        Role::Officiant => {
            body.arm_forward(-1, 19, 4);
            body.arm_forward(1, 19, 4);
            body.officiant_details();
        }
    }
    body.overgrowth();
    body.grid
}

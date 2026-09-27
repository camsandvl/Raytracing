//! La iglesia parroquial gótica, construida a partir de la planta y las fachadas de
//! referencia (Manus): una nave única con ábside poligonal al este, torre cuadrada
//! centrada en la fachada oeste (con la puerta y el rosetón), techo a dos aguas
//! escalonado, cinco ventanas ojivales por lado, todo sobre una base de diorama con
//! plaza de adoquines, lápidas y vegetación.
//!
//! Escala: el plano está en "unidades" (u); cada unidad son `4` celdas. Las constantes
//! de abajo ya están en celdas, con su medida original en u al lado.
//!
//! Ejes (coordenadas locales de la iglesia, antes de sumar el margen de la plaza):
//! X = ancho (0..48), Z = largo (0 = fachada de la torre, crece hacia el altar),
//! Y = altura sobre el piso interior.

use crate::color::Color;
use crate::light::Light;
use crate::materials::{self, MaterialSet};
use crate::ray_intersect::Material;
use crate::scene::builders::{carve_pointed_arch_opening, crenellate, fill_box, hash, place_rose_window, set_checked, Axis};
use crate::scene::skeleton::{self, Facing, Role};
use crate::scene::{CameraPreset, Scene};
use crate::skybox;
use crate::texture::TextureBank;
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::Vec3;

const MARGIN: isize = 20; // 5 u de plaza alrededor de la iglesia
const BASE_H: isize = 8; // espesor de la base del diorama
const PLINTH: isize = 3; // la iglesia se asienta sobre un zócalo (de ahí la escalinata)
const FLOOR_Y: isize = BASE_H + PLINTH;
const OX: isize = MARGIN;
const OZ: isize = MARGIN;

const W: isize = 48; // 12 u — ancho de la nave
const WALL: isize = 3;
const NAVE_Z0: isize = 24; // 6 u — la torre solapa 2 u sobre el arranque de la nave
const APSE_Z0: isize = 104; // 26 u
const END_Z: isize = 128; // 32 u — huella total
const APSE_END_X: (isize, isize) = (16, 32); // 4..8 u — cara plana del ábside
const TOWER_X: (isize, isize) = (8, 40); // 2..10 u — torre de 8×8 u, centrada
const TOWER_Z: (isize, isize) = (0, 32); // 0..8 u
const EAVE: isize = 40; // 10 u — muro al alero
const RIDGE: isize = 60; // 15 u — cumbrera
const TOWER_H: isize = 72; // 18 u — 1.2 × cumbrera
const ROOF_T: isize = 3;
const OVERHANG: isize = 2;
const MERLON_H: isize = 6;

const GRID_DIMS: (usize, usize, usize) = (
    (W + 2 * MARGIN) as usize,
    (FLOOR_Y + TOWER_H + MERLON_H + 2) as usize,
    (END_Z + 2 * MARGIN) as usize,
);

const CANDLE: Color = Color { r: 255, g: 168, b: 88 };

/// Distancia con signo del punto `p` a la recta `a`-`b`, positiva del lado de `inside`.
fn line_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32), inside: (f32, f32)) -> f32 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dz * dz).sqrt();
    let n = (dz / len, -dx / len);
    let d = n.0 * (p.0 - a.0) + n.1 * (p.1 - a.1);
    let s = n.0 * (inside.0 - a.0) + n.1 * (inside.1 - a.1);
    if s >= 0.0 { d } else { -d }
}

/// Qué tan adentro del contorno nave+ábside está el centro de la celda `(lx, lz)`:
/// `(roof_d, wall_d)`, en celdas, positivas hacia adentro. `roof_d` ignora la fachada
/// oeste (ahí el techo termina en hastial, no cae en pendiente); `wall_d` la incluye.
/// Con esto un único criterio arma muros, techo a dos aguas y el techo en faldones
/// del ábside, sin tratar cada cara por separado.
fn footprint(lx: isize, lz: isize) -> (f32, f32) {
    let p = (lx as f32 + 0.5, lz as f32 + 0.5);
    let w = W as f32;
    let inside = (w / 2.0, APSE_Z0 as f32);
    let left = p.0;
    let right = w - p.0;
    let chamfer_l = line_distance(p, (0.0, APSE_Z0 as f32), (APSE_END_X.0 as f32, END_Z as f32), inside);
    let chamfer_r = line_distance(p, (w, APSE_Z0 as f32), (APSE_END_X.1 as f32, END_Z as f32), inside);
    let end = END_Z as f32 - p.1;
    let front = p.1 - NAVE_Z0 as f32;
    let roof_d = left.min(right).min(chamfer_l).min(chamfer_r).min(end);
    (roof_d, roof_d.min(front))
}

/// Altura del techo sobre el piso según qué tan adentro del contorno está la celda:
/// sube 5 u cada 6 u horizontales (la pendiente de la fachada lateral de referencia).
fn roof_height(roof_d: f32) -> isize {
    let slope = (RIDGE - EAVE) as f32 / (W as f32 / 2.0);
    (EAVE as f32 + roof_d * slope).min(RIDGE as f32).round() as isize
}

fn in_tower(lx: isize, lz: isize) -> bool {
    (TOWER_X.0..TOWER_X.1).contains(&lx) && (TOWER_Z.0..TOWER_Z.1).contains(&lz)
}

fn nave_interior(lx: isize, lz: isize) -> bool {
    !in_tower(lx, lz) && footprint(lx, lz).1 >= WALL as f32
}

/// Coordenadas locales de la iglesia → celda del grid.
fn at(lx: isize, y: isize, lz: isize) -> (isize, isize, isize) {
    (OX + lx, FLOOR_Y + y, OZ + lz)
}

struct Builder<'a> {
    grid: &'a mut VoxelGrid,
    m: &'a MaterialSet,
    lights: Vec<Light>,
}

impl Builder<'_> {
    fn set(&mut self, (x, y, z): (isize, isize, isize), material: Material) {
        set_checked(self.grid, x, y, z, material);
    }

    fn column(&mut self, lx: isize, lz: isize, y: (isize, isize), material: Material) {
        for yy in y.0..y.1 {
            self.set(at(lx, yy, lz), material);
        }
    }

    /// Una vela: cuerpo de cera, llama emisiva encima, y la luz puntual justo sobre
    /// la llama (no dentro de ella — si no, la propia llama le haría sombra a todo).
    fn candle(&mut self, lx: isize, y: isize, lz: isize, height: isize, intensity: f32) {
        for dy in 0..height {
            self.set(at(lx, y + dy, lz), self.m.wax);
        }
        self.set(at(lx, y + height, lz), self.m.flame);
        let (x, yy, z) = at(lx, y + height, lz);
        self.lights.push(Light::new(Vec3::new(x as f32 + 0.5, yy as f32 + 1.8, z as f32 + 0.5), CANDLE, intensity));
    }

    fn base(&mut self) {
        let (gx, gz) = (GRID_DIMS.0 as isize, GRID_DIMS.2 as isize);
        for x in 0..gx {
            for z in 0..gz {
                // Borde derruido: las celdas más cercanas al borde de la base pierden
                // altura al azar, y algunas del filo desaparecen — la base se lee como
                // un trozo de terreno arrancado, no como una losa perfecta.
                let edge = x.min(gx - 1 - x).min(z).min(gz - 1 - z);
                let r = hash(x, z, 11);
                if edge == 0 && r < 0.2 {
                    continue;
                }
                let drop = match edge {
                    0 => 1 + (r * 3.0) as isize,
                    1 => (r < 0.5) as isize,
                    2 => (r < 0.2) as isize,
                    _ => 0,
                };
                let top = BASE_H - drop;
                let bottom = (hash(x, z, 12) * 3.0) as isize + if edge < 2 { 2 } else { 0 };
                for y in bottom..top {
                    let material = if y == top - 1 {
                        self.m.cobble
                    } else if hash(x * 7 + y, z, 13) < 0.08 {
                        self.m.stone
                    } else {
                        self.m.earth
                    };
                    set_checked(self.grid, x, y, z, material);
                }
            }
        }

        // Escalinata frente a la puerta: del nivel de la plaza al zócalo, en 3 escalones.
        for step in 0..PLINTH {
            let height = PLINTH - step;
            fill_box(
                self.grid,
                (OX + 14, OX + 34),
                (BASE_H, BASE_H + height),
                (OZ - 2 * (step + 1), OZ - 2 * step),
                self.m.stone,
            );
        }
    }

    fn nave_and_apse(&mut self) {
        for lx in -OVERHANG..W + OVERHANG {
            for lz in NAVE_Z0 - OVERHANG..END_Z + OVERHANG {
                if in_tower(lx, lz) {
                    continue;
                }
                let (roof_d, wall_d) = footprint(lx, lz);
                let top = roof_height(roof_d);

                if wall_d >= 0.0 {
                    // Zócalo bajo toda la huella; piso interior encima donde no hay muro.
                    self.column(lx, lz, (-PLINTH, 0), self.m.stone);
                    if wall_d >= WALL as f32 {
                        self.set(at(lx, -1, lz), self.m.floor);
                    } else {
                        // Muro: sube hasta tocar el techo — en la fachada oeste eso
                        // rellena el triángulo del hastial.
                        self.column(lx, lz, (0, top), self.m.stone);
                    }
                }

                if roof_d >= -(OVERHANG as f32) && lz >= NAVE_Z0 - 1 {
                    self.column(lx, lz, (top - ROOF_T, top), self.m.roof);
                }
            }
        }
    }

    fn tower(&mut self) {
        let (x0, x1) = TOWER_X;
        let (z0, z1) = TOWER_Z;
        for lx in x0..x1 {
            for lz in z0..z1 {
                self.column(lx, lz, (-PLINTH, 0), self.m.stone);
                let perimeter = lx < x0 + WALL || lx >= x1 - WALL || lz < z0 + WALL || lz >= z1 - WALL;
                if perimeter {
                    self.column(lx, lz, (0, TOWER_H), self.m.stone);
                } else {
                    self.set(at(lx, -1, lz), self.m.floor);
                }
                // Azotea plana, sobre la que se apoyan las almenas.
                self.column(lx, lz, (TOWER_H - 2, TOWER_H), self.m.stone);
            }
        }

        // Cornisa: un anillo que sobresale una celda justo bajo las almenas, y una
        // imposta a la altura del alero de la nave (solo donde la torre queda exenta).
        for lx in x0 - 1..=x1 {
            for lz in z0 - 1..=z1 {
                let ring = lx == x0 - 1 || lx == x1 || lz == z0 - 1 || lz == z1;
                if !ring {
                    continue;
                }
                self.column(lx, lz, (TOWER_H - 3, TOWER_H - 1), self.m.stone);
                if lz < NAVE_Z0 {
                    self.set(at(lx, EAVE, lz), self.m.stone);
                }
            }
        }

        let (x, y, z) = at(x0, TOWER_H, z0);
        let (x_end, _, z_end) = at(x1, 0, z1);
        crenellate(self.grid, (x, x_end), (z, z_end), y, MERLON_H, WALL, 6, 7, self.m.stone);
    }

    fn openings(&mut self) {
        let m = self.m;
        let front = (OZ + TOWER_Z.0, OZ + TOWER_Z.0 + WALL);

        // Puerta con arquivolta: un arco más grande tallado una sola celda de
        // profundidad deja el marco escalonado de los renders; la puerta en sí
        // atraviesa el muro completo.
        carve_pointed_arch_opening(self.grid, Axis::Z, (front.0, front.0 + 1), OX + 15, FLOOR_Y, 18, 12, None);
        carve_pointed_arch_opening(self.grid, Axis::Z, front, OX + 18, FLOOR_Y, 12, 10, None);

        // Rosetón: 4.5 u de diámetro, centrado en la torre a 13 u de altura, con un
        // aro de piedra que sobresale de la fachada.
        let (rose_along, rose_y) = ((OX + W / 2) as f32, (FLOOR_Y + 52) as f32);
        place_rose_window(self.grid, Axis::Z, front, rose_along, rose_y, 9.0, 8, m.stone, m.glass);
        for du in -11..=11 {
            for dv in -11..=11 {
                let r = ((du as f32 + 0.5).powi(2) + (dv as f32 + 0.5).powi(2)).sqrt();
                if (9.0..10.8).contains(&r) {
                    set_checked(self.grid, rose_along as isize + du, rose_y as isize + dv, front.0 - 1, m.stone);
                }
            }
        }

        // Arco de la torre hacia la nave: desde la puerta se ve el pasillo completo
        // hasta el altar.
        let tower_back = (OZ + TOWER_Z.1 - WALL, OZ + TOWER_Z.1);
        carve_pointed_arch_opening(self.grid, Axis::Z, tower_back, OX + 14, FLOOR_Y, 20, 18, None);
        for lx in 14..34 {
            for lz in TOWER_Z.1 - WALL..TOWER_Z.1 {
                self.set(at(lx, -1, lz), m.floor);
            }
        }

        // Troneras del campanario, en ambos costados de la torre.
        for side in [(OX + TOWER_X.0, OX + TOWER_X.0 + WALL), (OX + TOWER_X.1 - WALL, OX + TOWER_X.1)] {
            carve_pointed_arch_opening(self.grid, Axis::X, side, OZ + 14, FLOOR_Y + 44, 4, 8, None);
        }

        // Ventanitas de las alas de la fachada, a ambos lados de la torre.
        let wing_front = (OZ + NAVE_Z0, OZ + NAVE_Z0 + WALL);
        for along in [OX + WALL, OX + W - WALL - 4] {
            carve_pointed_arch_opening(self.grid, Axis::Z, wing_front, along, FLOOR_Y + 14, 4, 7, Some(m.glass));
        }

        // Cinco ventanas ojivales por muro lateral, repartidas parejo a lo largo de la
        // nave, con el alféizar al 35% de la altura del muro.
        for center in Self::side_window_centers() {
            for side in [(OX, OX + WALL), (OX + W - WALL, OX + W)] {
                carve_pointed_arch_opening(self.grid, Axis::X, side, OZ + center - 3, FLOOR_Y + 14, 7, 10, Some(m.glass));
            }
        }

        // Ventana alta en la cara plana del ábside, detrás del altar.
        let apse_end = (OZ + END_Z - WALL, OZ + END_Z);
        carve_pointed_arch_opening(self.grid, Axis::Z, apse_end, OX + 20, FLOOR_Y + 10, 8, 16, Some(m.glass));
    }

    fn side_window_centers() -> [isize; 5] {
        [39, 54, 68, 82, 97]
    }

    fn interior(&mut self) {
        let m = self.m;

        // Bancas a ambos lados del pasillo central, mirando al altar (+Z). Escala
        // humana (1 celda = 25 cm): asiento a 50 cm, respaldo hasta 1.25 m.
        for row in 0..8 {
            let z0 = 40 + row * 7;
            for (x0, x1) in [(5, 20), (28, 43)] {
                for lx in x0..x1 {
                    self.column(lx, z0, (2, 5), m.wood); // respaldo
                    for lz in z0 + 1..z0 + 4 {
                        self.set(at(lx, 2, lz), m.wood); // asiento
                    }
                }
                for lx in [x0, x1 - 1] {
                    for lz in z0..z0 + 4 {
                        self.column(lx, lz, (0, 4), m.wood); // laterales
                    }
                }
            }
        }

        // Candelabros colgantes sobre el pasillo, de hierro forjado (oscuro — en latón
        // el aro quedaba quemado por la luz de sus propias velas): aro con cuatro velas,
        // varillas a una corona, y una cadena hasta el techo. Una sola luz por
        // candelabro, en el hueco del aro.
        for lz in [50, 70, 90] {
            let lx = 24;
            for dx in -3isize..=3 {
                for dz in -3isize..=3 {
                    let ring = dx.abs().max(dz.abs()) == 3;
                    if ring {
                        self.set(at(lx + dx, 22, lz + dz), m.soot);
                        self.set(at(lx + dx, 28, lz + dz), m.soot);
                    }
                }
            }
            for (dx, dz) in [(-3, -3), (3, -3), (-3, 3), (3, 3)] {
                self.column(lx + dx, lz + dz, (23, 28), m.soot);
            }
            for (dx, dz) in [(-3, 0), (3, 0), (0, -3), (0, 3)] {
                self.set(at(lx + dx, 23, lz + dz), m.wax);
                self.set(at(lx + dx, 24, lz + dz), m.flame);
            }
            for y in 29..RIDGE {
                let (x, yy, z) = at(lx, y, lz);
                if self.grid.is_occupied(x, yy, z) {
                    break;
                }
                self.set((x, yy, z), m.soot);
            }
            let (x, y, z) = at(lx, 25, lz);
            self.lights.push(Light::new(Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5), CANDLE, 140.0));
        }

        // Presbiterio elevado: un escalón y después una plataforma de dos celdas.
        for lx in 0..W {
            for lz in 97..END_Z {
                if nave_interior(lx, lz) {
                    let raise = if lz < 100 { 1 } else { 2 };
                    self.column(lx, lz, (0, raise), m.floor);
                }
            }
        }

        // Altar de piedra (1 m de alto sobre el presbiterio) con tapa que sobresale.
        fill_box(self.grid, (OX + 18, OX + 30), (FLOOR_Y + 2, FLOOR_Y + 6), (OZ + 113, OZ + 119), m.stone);
        fill_box(self.grid, (OX + 17, OX + 31), (FLOOR_Y + 6, FLOOR_Y + 7), (OZ + 112, OZ + 120), m.stone);

        // Cruz de metal sobre el altar, recortada contra el vitral del ábside.
        fill_box(self.grid, (OX + 23, OX + 25), (FLOOR_Y + 7, FLOOR_Y + 19), (OZ + 116, OZ + 117), m.metal);
        fill_box(self.grid, (OX + 20, OX + 28), (FLOOR_Y + 14, FLOOR_Y + 16), (OZ + 116, OZ + 117), m.metal);

        // Velas sobre el altar.
        self.candle(19, 7, 114, 3, 40.0);
        self.candle(28, 7, 114, 2, 40.0);

        // Candelabros de metal a ambos lados del altar.
        for lx in [13, 34] {
            let lz = 116;
            fill_box(self.grid, (OX + lx - 1, OX + lx + 2), (FLOOR_Y + 2, FLOOR_Y + 3), (OZ + lz - 1, OZ + lz + 2), m.metal);
            self.column(lx, lz, (3, 12), m.metal);
            fill_box(self.grid, (OX + lx - 1, OX + lx + 2), (FLOOR_Y + 12, FLOOR_Y + 13), (OZ + lz - 1, OZ + lz + 2), m.metal);
            self.candle(lx, 13, lz, 3, 70.0);
        }

        // Una vela en el alféizar interior de cada ventana lateral — son las que hacen
        // brillar las ventanas vistas desde afuera.
        for center in Self::side_window_centers() {
            self.candle(WALL - 1, 14, center, 3, 32.0);
            self.candle(W - WALL, 14, center, 3, 32.0);
        }

        // Velas en el piso de la torre, recibiendo a quien entra.
        for (lx, lz, h) in [(19, 5, 3), (20, 6, 2), (28, 5, 3), (27, 6, 2)] {
            self.candle(lx, 0, lz, h, 30.0);
        }

        // Farol colgado dentro de la torre, detrás del rosetón: es lo que hace que el
        // rosetón brille desde afuera. Plato con la vela, cuatro varillas hasta una
        // corona, y una cadena única desde la corona hasta la azotea — la luz queda
        // en el hueco entre plato y corona, sin geometría encima que la tape.
        let (lx, lz) = (24, 20);
        fill_box(self.grid, (OX + lx - 1, OX + lx + 2), (FLOOR_Y + 45, FLOOR_Y + 46), (OZ + lz - 1, OZ + lz + 2), m.metal);
        for (cx, cz) in [(lx - 1, lz - 1), (lx + 1, lz - 1), (lx - 1, lz + 1), (lx + 1, lz + 1)] {
            self.column(cx, cz, (46, 52), m.metal);
        }
        fill_box(self.grid, (OX + lx - 1, OX + lx + 2), (FLOOR_Y + 52, FLOOR_Y + 53), (OZ + lz - 1, OZ + lz + 2), m.metal);
        self.column(lx, lz, (53, TOWER_H - 2), m.metal);
        self.candle(lx, 46, lz, 1, 400.0);
    }

    fn surroundings(&mut self) {
        let m = self.m;

        // Lápidas a los costados de la nave y dos cruces en las esquinas del frente.
        for lz in [40, 58, 76, 94] {
            for lx in [-10, W + 8] {
                let height = 6 + (hash(lx, lz, 21) * 3.0) as isize;
                let (x, _, z) = at(lx, 0, lz);
                fill_box(self.grid, (x, x + 2), (BASE_H, BASE_H + height - 1), (z, z + 5), m.stone);
                fill_box(self.grid, (x, x + 2), (BASE_H + height - 1, BASE_H + height), (z + 1, z + 4), m.stone);
            }
        }
        for lx in [-8, W + 6] {
            let (x, _, z) = at(lx, 0, 4);
            fill_box(self.grid, (x, x + 2), (BASE_H, BASE_H + 13), (z, z + 2), m.stone);
            fill_box(self.grid, (x - 2, x + 4), (BASE_H + 8, BASE_H + 10), (z, z + 2), m.stone);
        }

        // Hiedra: hebras verticales pegadas a los muros exteriores, subiendo desde el
        // suelo mientras haya muro detrás (se cortan al llegar a una ventana).
        let mut strands: Vec<(isize, isize, isize, isize)> = Vec::new(); // (x, z, dx, dz) del muro
        for lz in NAVE_Z0..APSE_Z0 {
            strands.push((OX - 1, OZ + lz, 1, 0));
            strands.push((OX + W, OZ + lz, -1, 0));
        }
        for lx in TOWER_X.0..TOWER_X.1 {
            strands.push((OX + lx, OZ - 1, 0, 1));
        }
        for (x, z, dx, dz) in strands {
            if hash(x, z, 31) > 0.14 {
                continue;
            }
            let max_h = 10 + (hash(x, z, 32) * 34.0) as isize;
            for y in BASE_H..BASE_H + max_h {
                if !self.grid.is_occupied(x + dx, y, z + dz) || self.grid.is_occupied(x, y, z) {
                    break;
                }
                set_checked(self.grid, x, y, z, m.leaves);
            }
        }

        // Arbustos sueltos en la plaza.
        let (gx, gz) = (GRID_DIMS.0 as isize, GRID_DIMS.2 as isize);
        for i in 0..70 {
            let x = 4 + (hash(i, 0, 41) * (gx - 8) as f32) as isize;
            let z = 4 + (hash(i, 1, 41) * (gz - 8) as f32) as isize;
            let (lx, lz) = (x - OX, z - OZ);
            let near_church = (-2..W + 2).contains(&lx) && (-8..END_Z + 2).contains(&lz);
            if near_church {
                continue;
            }
            let radius = 1 + (hash(i, 2, 41) * 2.0) as isize;
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    for dy in 0..=radius {
                        let (cx, cy, cz) = (x + dx, BASE_H + dy, z + dz);
                        if dx * dx + dz * dz + dy * dy * 2 <= radius * radius + 1 && !self.grid.is_occupied(cx, cy, cz) {
                            set_checked(self.grid, cx, cy, cz, m.leaves);
                        }
                    }
                }
            }
        }
    }
}

pub fn build(textures: &mut TextureBank) -> Scene {
    let m = materials::build(textures);
    let mut grid = VoxelGrid::new(GRID_DIMS, 1.0, Vec3::new(0.0, 0.0, 0.0));

    let mut builder = Builder { grid: &mut grid, m: &m, lights: Vec::new() };
    builder.base();
    builder.nave_and_apse();
    builder.tower();
    builder.openings();
    builder.interior();
    builder.surroundings();
    let mut lights = builder.lights;

    // La boda: los novios frente a frente tomados de la mano sobre el presbiterio, y
    // el oficiante detrás de ellos, delante del altar, mirando hacia las bancas.
    let chancel_top = (FLOOR_Y + 2) as f32;
    let aisle_x = (OX + W / 2) as f32;
    let couple_z = (OZ + 104) as f32 + 0.5;
    let grids = vec![
        grid,
        skeleton::build(&m, Role::Bride, Vec3::new(aisle_x - 1.75, chancel_top, couple_z), Facing::PosX, 71),
        skeleton::build(&m, Role::Groom, Vec3::new(aisle_x + 1.75, chancel_top, couple_z), Facing::NegX, 72),
        skeleton::build(&m, Role::Officiant, Vec3::new(aisle_x, chancel_top, (OZ + 109) as f32 + 0.5), Facing::NegZ, 73),
    ];

    // La luna: luz fría y tenue que deja leer el exterior de noche. Adentro solo entra
    // por las ventanas, proyectando su forma sobre el piso.
    lights.push(Light::directional(skybox::moon_direction(), Color::new(140, 155, 200), 1.3));

    let center = Vec3::new((OX + W / 2) as f32, (FLOOR_Y + 30) as f32, (OZ + END_Z / 2) as f32);
    let presets = vec![
        CameraPreset {
            name: "exterior",
            eye: center + Vec3::new(80.0, 52.0, -106.0),
            target: center + Vec3::new(0.0, -4.0, -12.0),
        },
        CameraPreset {
            name: "frente",
            eye: Vec3::new(center.x, (FLOOR_Y + 30) as f32, (OZ - 92) as f32),
            target: Vec3::new(center.x, (FLOOR_Y + 36) as f32, OZ as f32),
        },
        CameraPreset {
            name: "aerea",
            eye: center + Vec3::new(110.0, 115.0, 60.0),
            target: center,
        },
        CameraPreset {
            name: "abside",
            eye: Vec3::new(center.x + 38.0, (FLOOR_Y + 34) as f32, (OZ + END_Z + 82) as f32),
            target: Vec3::new(center.x, (FLOOR_Y + 28) as f32, (OZ + END_Z - 14) as f32),
        },
        CameraPreset {
            name: "interior",
            eye: Vec3::new(center.x, (FLOOR_Y + 14) as f32, (OZ + 36) as f32),
            target: Vec3::new(center.x, (FLOOR_Y + 10) as f32, (OZ + 118) as f32),
        },
        CameraPreset {
            name: "boda",
            eye: Vec3::new(center.x, (FLOOR_Y + 12) as f32, (OZ + 94) as f32),
            target: Vec3::new(center.x, (FLOOR_Y + 7) as f32, (OZ + 107) as f32),
        },
    ];

    Scene { grids, lights, presets }
}

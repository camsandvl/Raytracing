//! Catedral Sainte-Cécile, solo interior — ver `docs/cathedral_design_spec.md` (la
//! sección de adaptación a solo-interior manda sobre el spec original de Manus).
//!
//! El interior se describe como una lista de SALAS (`ROOMS`): nártex, vestíbulos de las
//! torres, nave, naves laterales, brazos del crucero y coro + ábside, cada una con su
//! bóveda. Construir es:
//! 1. poner piedra alrededor de cada sala, hasta la altura de su bóveda;
//! 2. vaciar la sala.
//!
//! Los muros, su altura y el cierre hacia afuera salen solos (entre dos salas queda el
//! muro de la más alta), y no hay que dibujar ningún muro a mano. Después se tallan los
//! vanos: portadas, rosetón, ventanas.
//!
//! Todo se escribe en las coordenadas del spec, en vóxeles de 20 cm: `x` = ancho
//! (negativo al norte), `y` = hacia el este (0 = fachada oeste), `z` = altura sobre el
//! piso terminado. `cell`/`world` las pasan a las del grid, que tiene Y hacia arriba.

use crate::color::Color;
use crate::light::{Aperture, Light};
use crate::materials::{self, MaterialSet};
use crate::ray_intersect::Material;
use crate::scene::builders::{carve_pointed_arch_opening, clear_checked, hash, place_rose_window, pointed_arch_profile, set_checked, Axis};
use crate::scene::props;
use crate::scene::people;
use crate::scene::zombies::{self, Debris, Feast, Opening};
use crate::rng::Rng;
use crate::scene::{CameraPreset, Scene};
use crate::skybox;
use crate::texture::TextureBank;
use crate::voxel_grid::VoxelGrid;
use crate::walker::Spawn;
use nalgebra_glm::Vec3;

const VOXELS_PER_METER: f32 = 5.0;
const HALF_WIDTH: isize = 70; // ±14 m: los brazos del crucero
const SUBFLOOR: isize = 2; // losa de 0.4 m bajo el piso terminado
const WALL: isize = 4; // muros de 0.8 m
const VAULT_CROWN: isize = 110; // clave de la bóveda de la nave, 22 m
const SHELL: isize = 2; // cascarón de las bóvedas, 0.4 m

/// Planta, de oeste a este: nártex (0–8 m), `NAVE_BAYS` tramos de nave de 8 m, el tramo
/// del crucero, el coro (6.8 m) y el ábside semicircular (radio 5.2 m). Con 2 tramos de
/// nave la iglesia mide 44 m (el spec tenía 4 tramos antes del crucero y uno después:
/// 68 m).
const BAY: isize = 40;
const NAVE_START: isize = 40;
const NAVE_BAYS: isize = 2;
const CROSSING: (isize, isize) = (NAVE_START + NAVE_BAYS * BAY, NAVE_START + (NAVE_BAYS + 1) * BAY);
const CROSSING_CENTER: isize = (CROSSING.0 + CROSSING.1) / 2;
const NAVE_END: isize = CROSSING.1;
const CHOIR_END: isize = NAVE_END + 34;
const LENGTH: isize = CHOIR_END + APSE_RADIUS;

/// Ancho: la nave mide 8.8 m entre las caras de la arquería (7.2 m entre pilares) y cada
/// nave lateral 4 m, la proporción gótica clásica de 2 : 1. El ancho total entre muros
/// exteriores es el del spec; solo se corrió la arquería hacia adentro.
const NAVE_HALF: isize = 22;
const ARCADE: (isize, isize) = (NAVE_HALF, NAVE_HALF + WALL);
const AISLE_WALL: isize = 46;
/// Cara interior del testero de cada brazo del crucero (13.2 m del eje).
const ARM_END: isize = 66;
/// Arranque de las bóvedas de las naves laterales (9.6 m).
const AISLE_SPRING: isize = 48;
/// Radio exterior del ábside semicircular, centrado al final del coro: del ancho de la
/// nave más sus muros.
const APSE_RADIUS: isize = NAVE_HALF + WALL;

const GRID_DIMS: (usize, usize, usize) = ((2 * HALF_WIDTH) as usize, (SUBFLOOR + VAULT_CROWN + SHELL) as usize, LENGTH as usize);

/// Luz de vela: ámbar profundo, bien cálido contra la luna fría.
const CANDLE: Color = Color { r: 255, g: 136, b: 52 };

/// Área libre de una sala: `(x0, x1, y0, y1)`, extremos exclusivos.
type Rect = (isize, isize, isize, isize);

#[derive(Clone, Copy)]
enum Vault {
    /// Cañón apuntado cuyo arco cruza el eje X (la sala corre a lo largo de Y).
    AcrossX,
    /// Cañón apuntado cuyo arco cruza el eje Y (la sala corre a lo largo de X).
    AcrossY,
    /// Bóveda que sube desde todos los muros hacia el centro: sirve para cualquier
    /// planta.
    Dome,
    /// Cañón de medio punto (semicircular) cuyo arco cruza el eje X. Es el del coro: visto
    /// desde la nave su boca es un gran arco redondo (el arco triunfal), y donde el ábside
    /// se angosta baja solo, formando la media cúpula.
    RoundAcrossX,
    /// Crucería por tramos de `bay` vóxeles a lo largo de Y (desde el `y0` del primer
    /// rectángulo). Es la unión de dos cañones apuntados: uno cruza la sala y el otro
    /// cruza cada tramo. Así sobre cada ventana alta queda un luneto, y donde los dos
    /// cañones se encuentran quedan las aristas diagonales. Encima van nervios de piedra
    /// clara de `rib` vóxeles (fajones en los límites de tramo, diagonales y espinazo)
    /// sobre una plementería de plata. `semicircular`: cañones de medio punto en vez de
    /// apuntados.
    Groin { bay: isize, rib: isize, semicircular: bool },
}

struct Room {
    rects: &'static [Rect],
    /// Suma a la sala el interior del polígono del ábside.
    apse: bool,
    vault: Vault,
    /// Arranque y clave de la bóveda, en vóxeles sobre el piso.
    spring: isize,
    crown: isize,
}

/// Las salas, según la planta del spec y la adaptación (sección 3 del doc). El arco del
/// nártex sube hasta 18 m para que el rosetón (centro a 14 m) quede adentro.
const ROOMS: [Room; 6] = [
    Room { rects: &[(-18, 18, 4, 36)], apse: false, vault: Vault::AcrossX, spring: 60, crown: 90 }, // nártex
    Room { rects: &[(-50, -22, 4, 36), (22, 50, 4, 36)], apse: false, vault: Vault::Dome, spring: 36, crown: 50 }, // vestíbulos de las torres
    Room {
        rects: &[(-NAVE_HALF, NAVE_HALF, NAVE_START, NAVE_END)],
        apse: false,
        vault: Vault::Groin { bay: BAY, rib: 4, semicircular: false },
        spring: 80,
        crown: VAULT_CROWN,
    }, // nave
    // Naves laterales: crucería de medio punto (la luz de 4 m sube 2 m, un semicírculo
    // exacto) con nervios finos de 0.4 m.
    Room {
        rects: &[(-AISLE_WALL, -ARCADE.1, NAVE_START, NAVE_END), (ARCADE.1, AISLE_WALL, NAVE_START, NAVE_END)],
        apse: false,
        vault: Vault::Groin { bay: BAY, rib: 2, semicircular: true },
        spring: AISLE_SPRING,
        crown: AISLE_SPRING + (AISLE_WALL - ARCADE.1) / 2,
    },
    Room { rects: &[(-ARM_END, -ARCADE.1, CROSSING.0 + 4, CROSSING.1 - 4), (ARCADE.1, ARM_END, CROSSING.0 + 4, CROSSING.1 - 4)], apse: false, vault: Vault::AcrossY, spring: 56, crown: 76 }, // brazos del crucero
    // Coro + ábside: medio punto de 8.8 m de luz, del arranque (11.6 m) a la clave (16 m).
    Room { rects: &[(-NAVE_HALF, NAVE_HALF, NAVE_END, CHOIR_END)], apse: true, vault: Vault::RoundAcrossX, spring: 80 - NAVE_HALF, crown: 80 },
];

/// Vóxel del spec → celda del grid.
fn cell(x: isize, y: isize, z: isize) -> (isize, isize, isize) {
    (x + HALF_WIDTH, z + SUBFLOOR, y)
}

/// Punto del spec en vóxeles (admite fracciones) → coordenadas de mundo.
fn world(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x + HALF_WIDTH as f32, z + SUBFLOOR as f32, y)
}

fn put(grid: &mut VoxelGrid, (x, y, z): (isize, isize, isize), material: Material) {
    let (cx, cy, cz) = cell(x, y, z);
    set_checked(grid, cx, cy, cz, material);
}

/// Interior del ábside semicircular a más de `inset` de su muro curvo (el lado oeste es
/// la unión abierta con el coro).
fn in_apse(x: isize, y: isize, inset: isize) -> bool {
    let r = (x as f32 + 0.5).hypot((y - CHOIR_END) as f32 + 0.5);
    y >= CHOIR_END && r <= (APSE_RADIUS - inset) as f32
}

impl Room {
    fn contains(&self, x: isize, y: isize) -> bool {
        self.rects.iter().any(|&(x0, x1, y0, y1)| (x0..x1).contains(&x) && (y0..y1).contains(&y)) || (self.apse && in_apse(x, y, WALL))
    }

    /// Caja envolvente, con margen para el muro y un anillo de "afuera".
    fn bounds(&self) -> Rect {
        let mut b = self.rects.iter().fold((isize::MAX, isize::MIN, isize::MAX, isize::MIN), |b, r| (b.0.min(r.0), b.1.max(r.1), b.2.min(r.2), b.3.max(r.3)));
        if self.apse {
            b = (b.0.min(-APSE_RADIUS), b.1.max(APSE_RADIUS), b.2.min(CHOIR_END), b.3.max(LENGTH));
        }
        let m = WALL + 1;
        (b.0 - m, b.1 + m, b.2 - m, b.3 + m)
    }
}

/// Distancia de cada celda a la celda más cercana con `mask == target` (chaflán 1/√2
/// en dos pasadas).
fn chamfer(mask: &[bool], w: usize, h: usize, target: bool) -> Vec<f32> {
    let diagonal = std::f32::consts::SQRT_2;
    let mut d: Vec<f32> = mask.iter().map(|&m| if m == target { 0.0 } else { f32::INFINITY }).collect();
    let neighbors_before = [(-1, 0, 1.0), (0, -1, 1.0), (-1, -1, diagonal), (1, -1, diagonal)];
    let mut relax = |x: usize, y: usize, sign: isize| {
        let mut v = d[y * w + x];
        for &(dx, dy, cost) in &neighbors_before {
            let (nx, ny) = (x as isize + dx * sign, y as isize + dy * sign);
            if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                v = v.min(d[ny as usize * w + nx as usize] + cost);
            }
        }
        d[y * w + x] = v;
    };
    for y in 0..h {
        for x in 0..w {
            relax(x, y, 1);
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            relax(x, y, -1);
        }
    }
    d
}

/// Distancia al borde de la sala medida solo a lo largo de un eje (para los cañones).
fn run_distance(mask: &[bool], w: usize, h: usize, along_x: bool) -> Vec<f32> {
    let (outer, inner) = if along_x { (h, w) } else { (w, h) };
    let at = |o: usize, i: usize| if along_x { o * w + i } else { i * w + o };
    let mut d = vec![0.0; w * h];
    for o in 0..outer {
        let mut run = 0.0;
        for i in 0..inner {
            run = if mask[at(o, i)] { run + 1.0 } else { 0.0 };
            d[at(o, i)] = run;
        }
        run = 0.0;
        for i in (0..inner).rev() {
            run = if mask[at(o, i)] { run + 1.0 } else { 0.0 };
            d[at(o, i)] = f32::min(d[at(o, i)], run);
        }
    }
    d
}

/// Perfil de arco apuntado equilátero normalizado: `t` = 0 en el arranque, 1 en la clave.
fn pointed(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (4.0 - (t - 2.0).powi(2)).sqrt() / 3f32.sqrt()
}

/// Perfil de medio punto normalizado: un cuarto de círculo de 0 (arranque) a 1 (clave).
fn round(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (1.0 - (1.0 - t).powi(2)).sqrt()
}

/// Piedra alrededor de cada sala hasta la altura de su bóveda + cascarón; después se
/// vacía cada sala hasta su bóveda.
fn shell(grid: &mut VoxelGrid, m: &MaterialSet) {
    let width = 2 * HALF_WIDTH;
    let mut top = vec![-1isize; (width * LENGTH) as usize];

    struct Field {
        bounds: Rect,
        w: usize,
        mask: Vec<bool>,
        to_room: Vec<f32>,
        to_edge: Vec<f32>,
    }
    let fields: Vec<Field> = ROOMS
        .iter()
        .map(|room| {
            let bounds = room.bounds();
            let (w, h) = ((bounds.1 - bounds.0) as usize, (bounds.3 - bounds.2) as usize);
            let mask: Vec<bool> = (0..w * h).map(|i| room.contains(bounds.0 + (i % w) as isize, bounds.2 + (i / w) as isize)).collect();
            let to_room = chamfer(&mask, w, h, true);
            let to_edge = match room.vault {
                Vault::AcrossX | Vault::RoundAcrossX | Vault::Groin { .. } => run_distance(&mask, w, h, true),
                Vault::AcrossY => run_distance(&mask, w, h, false),
                Vault::Dome => chamfer(&mask, w, h, false),
            };
            Field { bounds, w, mask, to_room, to_edge }
        })
        .collect();

    for (room, f) in ROOMS.iter().zip(&fields) {
        for (i, &distance) in f.to_room.iter().enumerate() {
            let (x, y) = (f.bounds.0 + (i % f.w) as isize, f.bounds.2 + (i / f.w) as isize);
            if distance <= WALL as f32 && (-HALF_WIDTH..HALF_WIDTH).contains(&x) && (0..LENGTH).contains(&y) {
                let t = &mut top[(y * width + x + HALF_WIDTH) as usize];
                *t = (*t).max(room.crown + SHELL);
            }
        }
    }
    for (i, &t) in top.iter().enumerate() {
        let (x, y) = (i as isize % width - HALF_WIDTH, i as isize / width);
        for z in -SUBFLOOR..t {
            let material = if z == -1 { floor(m, x, y) } else { m.marble };
            put(grid, (x, y, z), material);
        }
    }

    for (room, f) in ROOMS.iter().zip(&fields) {
        let inside: Vec<usize> = (0..f.mask.len()).filter(|&i| f.mask[i]).collect();
        let span = inside.iter().map(|&i| f.to_edge[i] - 0.5).fold(0.0, f32::max);
        for i in inside {
            let (x, y) = (f.bounds.0 + (i % f.w) as isize, f.bounds.2 + (i / f.w) as isize);
            let across = (f.to_edge[i] - 0.5) / span;
            let (shape, rib) = match room.vault {
                Vault::Groin { bay, rib, semicircular } => groin(across, span, y - room.rects[0].2, bay, rib, semicircular),
                Vault::RoundAcrossX => (round(across), None),
                _ => (pointed(across), None),
            };
            let ceiling = room.spring + ((room.crown - room.spring) as f32 * shape).round() as isize;
            for z in 0..ceiling {
                let (cx, cy, cz) = cell(x, y, z);
                clear_checked(grid, cx, cy, cz);
            }
            match room.vault {
                // Moldura pálida siguiendo el arco triunfal, en la cara que da a la nave.
                Vault::RoundAcrossX if y == room.rects[0].2 => {
                    for z in ceiling..ceiling + 3 {
                        put(grid, (x, y, z), m.trim);
                    }
                }
                Vault::Groin { .. } | Vault::Dome | Vault::RoundAcrossX => put(grid, (x, y, ceiling), m.silver), // plementería y cuencas de plata
                _ => {}
            }
            if let Some(depth) = rib {
                for z in ceiling - depth..ceiling {
                    put(grid, (x, y, z), m.marble);
                }
            }
        }
    }
}

/// Radio del sol del crucero (2.8 m) y media anchura de la alfombra procesional (1.6 m).
const SUN_RADIUS: f32 = 14.0;
const RUNNER_HALF: isize = 8;

/// El piso se lee como en una iglesia de verdad, de la puerta al altar: sencillo donde
/// se entra, más rico hacia el centro sagrado. Campos calmos y de poco contraste, y
/// contraste fuerte solo en los motivos adonde tiene que ir la vista: un damero de alto
/// contraste en baldosas de 1 m se vuelve moiré a lo lejos.
/// - Todo lo demás (nártex, naves laterales, brazos, vestíbulos): mármol pálido liso, el
///   mundo cotidiano.
/// - Bandas beige bajo cada límite de tramo y bajo cada arquería: el piso repite la
///   estructura de la bóveda que tiene encima.
/// - Nave: una alfombra procesional de piedra (borde oscuro, campo pálido) con un rombo
///   por tramo, alineado con los pilares: el camino del peregrino y sus estaciones.
/// - Crucero: un gran sol dorado de 12 rayos (el "Sol de justicia", Malaquías 4:2; los
///   doce apóstoles o meses). Los novios están parados en su centro, y la luna entra por
///   el vitral y cae sobre ellos: sol y luna, la imagen antigua del matrimonio, se
///   encuentran en la boda.
/// - Umbral del coro: una banda dorada, el límite entre la nave y el presbiterio.
/// - Presbiterio: campo oscuro con borde pálido, una alfombra de piedra apartada, para
///   que el altar y el vitral resalten.
fn floor(m: &MaterialSet, x: isize, y: isize) -> Material {
    let (dx, dy) = (x as f32 + 0.5, (y - CROSSING_CENTER) as f32 + 0.5);
    let r = dx.hypot(dy);
    if r < SUN_RADIUS {
        return sun(m, r, dy.atan2(dx));
    }
    if (NAVE_END..NAVE_END + 2).contains(&y) && x.abs() < NAVE_HALF {
        return m.floor_gold;
    }
    if y >= NAVE_END + 2 {
        let set_apart = (y < CHOIR_END && x.abs() < NAVE_HALF - 2) || in_apse(x, y, WALL + 2);
        return if set_apart { m.floor_dark } else { m.floor_pale };
    }
    if (NAVE_START..CROSSING.0).contains(&y) && x.abs() < RUNNER_HALF {
        let from_center = (x.abs() as f32 + 0.5) + ((y - NAVE_START).rem_euclid(BAY) as f32 + 0.5 - BAY as f32 / 2.0).abs();
        return match from_center {
            d if d < 3.0 => m.floor_gold,
            d if d < 8.0 => m.floor_dark,
            _ if x.abs() >= RUNNER_HALF - 2 => m.floor_dark,
            _ => m.floor_pale,
        };
    }
    let under_station = (y - NAVE_START + 1).rem_euclid(BAY) < 2 && (NAVE_START - 1..=NAVE_END).contains(&y);
    let under_arcade = (ARCADE.0..ARCADE.1).contains(&x.abs()) && (NAVE_START..NAVE_END).contains(&y);
    if under_station || under_arcade {
        m.floor_beige
    } else {
        m.floor_pale
    }
}

/// El sol del crucero: disco dorado, anillo oscuro, 12 rayos dorados que se afinan hacia
/// afuera (uno apunta al altar) y un anillo oscuro de borde.
fn sun(m: &MaterialSet, r: f32, angle: f32) -> Material {
    let (inner, rays_end) = (4.0, SUN_RADIUS - 2.0);
    if r < inner - 1.0 {
        return m.floor_gold;
    }
    if r < inner || r >= rays_end {
        return m.floor_dark;
    }
    let sector = std::f32::consts::TAU / 12.0;
    let a = (angle - std::f32::consts::FRAC_PI_2).rem_euclid(sector);
    let off_axis = a.min(sector - a);
    let half_width = 0.45 * sector * (1.0 - (r - inner) / (rays_end - inner));
    if off_axis < half_width {
        m.floor_gold
    } else {
        m.floor_pale
    }
}

/// Crucería en una celda. `across` va de 0 (muro) a 1 (eje de la sala) y `span` es la
/// media luz en vóxeles; `along` es la posición a lo largo de la sala desde el primer
/// límite de tramo. Devuelve la altura normalizada de la bóveda y, si la celda cae
/// bajo un nervio, cuánto cuelga ese nervio.
fn groin(across: f32, span: f32, along: isize, bay: isize, rib: isize, semicircular: bool) -> (f32, Option<isize>) {
    let half = bay as f32 / 2.0;
    let phase = (along as f32 + 0.5).rem_euclid(bay as f32);
    let from_station = phase.min(bay as f32 - phase);
    let profile: fn(f32) -> f32 = if semicircular { round } else { pointed };
    let shape = profile(across).max(profile(from_station / half));

    let from_ridge = (1.0 - across) * span;
    let from_center = half - from_station;
    let r = rib as f32 / 2.0;
    let diagonal = (from_ridge * half - from_center * span).abs() / span.hypot(half);
    let on_rib = from_station <= r || from_ridge <= r || diagonal <= r;
    let boss = from_ridge <= 3.0 && from_center <= 3.0; // clave: cuelga un poco más
    (shape, if boss { Some(rib + 2) } else { on_rib.then_some(rib) })
}

/// Vano ojival a través de un muro, en coordenadas del spec. `wall` es el rango del
/// espesor (en `y` si el muro corre a lo largo de X, en `x` si corre a lo largo de Y);
/// `start` es donde empieza a lo largo del muro. `fill` pone un panel en la capa
/// central del espesor (si no, el vano queda abierto).
#[allow(clippy::too_many_arguments)]
fn arch(grid: &mut VoxelGrid, axis: Axis, wall: (isize, isize), start: isize, base: isize, width: isize, height: isize, fill: Option<Material>) {
    let (wall, start) = match axis {
        Axis::X => ((wall.0 + HALF_WIDTH, wall.1 + HALF_WIDTH), start),
        Axis::Z => (wall, start + HALF_WIDTH),
    };
    let jamb = height - (width as f32 * 3f32.sqrt() / 2.0).round() as isize;
    carve_pointed_arch_opening(grid, axis, wall, start, base + SUBFLOOR, width, jamb, fill);
}

/// Capilla lateral en el testero de un brazo del crucero: un panel de plata enmarcado con
/// una cruz dorada, sobre el muro, encima del altar lateral (los props del altar y las
/// velas votivas van en `furnishings`).
fn side_chapel_panel(grid: &mut VoxelGrid, m: &MaterialSet, side: isize) {
    let face = if side > 0 { ARM_END } else { -ARM_END - 1 }; // primera celda del muro
    let (y0, y1) = (CROSSING_CENTER - 8, CROSSING_CENTER + 8);
    for y in y0..y1 {
        for z in 6..28 {
            let frame = y == y0 || y == y1 - 1 || z == 6 || z == 27;
            put(grid, (face, y, z), if frame { m.trim } else { m.silver });
        }
    }
    // Cruz latina en relieve, una capa delante del panel.
    let relief = face - side;
    for (ys, zs) in [((-1, 1), (9, 25)), ((-5, 5), (18, 20))] {
        for y in ys.0..ys.1 {
            for z in zs.0..zs.1 {
                put(grid, (relief, CROSSING_CENTER + y, z), m.floor_gold);
            }
        }
    }
}

/// Paneles de plata con marco pálido en los bolsillos poco profundos que quedan al final
/// de cada nave lateral, junto al coro (entre el brazo del crucero y el muro del fondo).
/// Tenían un sol y una luna en relieve; el autor los sacó y dejó solo el metal.
fn aisle_end_panels(grid: &mut VoxelGrid, m: &MaterialSet) {
    for side in [-1isize, 1] {
        let center = side * (ARCADE.1 + AISLE_WALL) / 2;
        let (x0, x1, z0, z1) = (center - 8, center + 8, 6, 46);
        for x in x0..x1 {
            for z in z0..z1 {
                let frame = x == x0 || x == x1 - 1 || z == z0 || z == z1 - 1;
                put(grid, (x, NAVE_END, z), if frame { m.trim } else { m.silver });
            }
        }
    }
}

/// El tramo modular, repetido a lo largo de la nave (sección 2 del spec + correcciones 1
/// y 4 de la adaptación). Por tramo y por lado, sobre el muro que separa la nave de su
/// nave lateral:
/// - un arco de la arquería (arranque a 5.6 m, clave a 11 m); en el tramo del crucero,
///   un arco alto hacia el brazo;
/// - dos nichos de triforio ciego, grises con tracería pintada (12–15.6 m);
/// - una ventana del claristorio, con el vitral reventado (16.8–20 m).
///
/// Después van los pilares en cada límite de tramo y las pilastras del muro exterior.
fn bays(grid: &mut VoxelGrid, m: &MaterialSet) {
    for bay in 0..=NAVE_BAYS {
        let s = NAVE_START + bay * BAY;
        for wall in [(-ARCADE.1, -ARCADE.0), ARCADE] {
            // Arco de la arquería con dos órdenes: el vano atraviesa el muro y un arco
            // 0.2 m más grande entra 0.2 m en cada cara, así el borde se lee como dos
            // escalones finos y no como un muro grueso cortado.
            let height = if bay == NAVE_BAYS { 72 } else { 55 };
            arch(grid, Axis::X, wall, s + 4, 0, BAY - 8, height, None);
            for face in [(wall.0, wall.0 + 1), (wall.1 - 1, wall.1)] {
                arch(grid, Axis::X, face, s + 3, 0, BAY - 6, height + 1, None);
            }
            if bay != NAVE_BAYS {
                // Nicho de 0.4 m de fondo (el fondo es el panel central de un rango de 5
                // capas medido desde la cara de la nave), en gris con tracería pintada: con
                // el fondo oscuro se confundían con ventanas.
                let niche_wall = if wall.0 < 0 { (-NAVE_HALF - 5, -NAVE_HALF) } else { (NAVE_HALF, NAVE_HALF + 5) };
                for start in [s + 6, s + 22] {
                    arch(grid, Axis::X, niche_wall, start, 60, 12, 18, Some(m.marble_grey));
                    painted_tracery(grid, m, (niche_wall.0 + niche_wall.1 - 1) / 2, start, 60, (12, 18));
                }
            }
            // Ventana alta: el vitral reventado por los que caen de ella (ver `invasion`).
            arch(grid, Axis::X, wall, s + 14, 84, 12, 16, None);
            let pane = (wall.0 + wall.1) / 2;
            shattered_pane(grid, m, Axis::X, pane, s + 14, 84, (12, 16), 3, 93 + (s + pane) as u32);
        }
    }

    for station in (NAVE_START..=NAVE_END).step_by(BAY as usize) {
        for side in [-1, 1] {
            pier(grid, m, side, station);
            // Pilastra del muro exterior: 0.4 m hacia la nave lateral, hasta el arranque
            // de su bóveda, con un panel de mármol gris enmarcado en pálido en la cara
            // que da a la nave lateral.
            for x in AISLE_WALL - 2..AISLE_WALL {
                for y in (station - 4..station + 4).filter(|&y| (NAVE_START..NAVE_END).contains(&y)) {
                    for z in 0..AISLE_SPRING {
                        let panel = x == AISLE_WALL - 2 && (station - 3..station + 3).contains(&y) && (4..AISLE_SPRING - 4).contains(&z);
                        put(grid, (side * x, y, z), if panel { m.marble_grey } else { m.marble });
                    }
                }
            }
        }
    }
}

/// Tracería pintada (no tallada) en mármol pálido sobre el fondo gris de un nicho de
/// `size` (ancho, alto), como una ventana ciega dibujada: un filete por el borde, un
/// parteluz al medio que sube hasta un óculo en lo alto del arco, y puntos en rombo en
/// las dos luces. Mismo criterio que los dibujos de las columnas.
fn painted_tracery(grid: &mut VoxelGrid, m: &MaterialSet, layer: isize, start: isize, sill: isize, (width, height): (isize, isize)) {
    let columns = lancet_columns(width, height);
    let jamb = height - (width as f32 * 3f32.sqrt() / 2.0).round() as isize;
    let center = ((width - 1) as f32 / 2.0, jamb as f32 + 3.0);
    let radius = (width as f32 / 4.5).max(1.5);
    for c in 0..width {
        for v in 0..columns[c as usize] {
            let border = c == 0 || c == width - 1 || v == 0 || v == columns[c as usize] - 1;
            let ring = ((c as f32 - center.0).hypot(v as f32 - center.1) - radius).abs() < 0.6;
            let mullion = (c as f32 - center.0).abs() < 1.0 && (v as f32) < center.1 - radius;
            let dots = (c + v) % 4 == 2 && (c - v).rem_euclid(4) == 2 && (v as f32) < center.1 - radius;
            if border || ring || mullion || dots {
                put(grid, (layer, start + c, sill + v), m.trim);
            }
        }
    }
}

/// Columna de la arquería en un límite de tramo, centrada en el eje del muro de la
/// arquería. De abajo hacia arriba:
/// - basa: plinto de plata de 2 × 2 m y el bloque de 1.6 m original con las esquinas
///   achaflanadas (hasta 0.8 m);
/// - un anillo de transición;
/// - el fuste, un cilindro esbelto de ~1.3 m de mármol gris azulado con dos anillos pálidos;
/// - un capitel de mármol pálido y un ábaco cuadrado, del que arrancan los arcos (5.6 m).
///
/// Arriba del ábaco sigue el muro de la arquería, y por su cara de la nave sube un fuste
/// adosado de media caña, también gris, con un anillo pálido y un remate pálido en el
/// arranque de la bóveda (16 m), donde apoyan los nervios. El detalle sale de las
/// iglesias de referencia (paneles de mármol de color enmarcados en blanco), en gris.
/// Radio del fuste de las columnas de la arquería.
const SHAFT: f32 = 3.2;

fn pier(grid: &mut VoxelGrid, m: &MaterialSet, side: isize, station: isize) {
    let axis = side * (NAVE_HALF + WALL / 2); // centro del muro de la arquería
    let (x0, x1) = (axis - 5, axis + 5);
    let (y0, y1) = (station - 5, station + 5);
    let radius = |x: isize, y: isize| ((x - axis) as f32 + 0.5).hypot((y - station) as f32 + 0.5);
    // Solo dentro del largo de la nave: las columnas de los extremos quedan como medias
    // columnas embebidas en el muro del nártex y en el del coro, sin perforarlos.
    let in_nave = |y: isize| (NAVE_START..NAVE_END).contains(&y);
    let arcade_wall = |x: isize| (ARCADE.0..ARCADE.1).contains(&x.abs());

    for x in x0..x1 {
        for y in (y0..y1).filter(|&y| in_nave(y)) {
            let r = radius(x, y);
            let in_block = (x0 + 1..x1 - 1).contains(&x) && (y0 + 1..y1 - 1).contains(&y);
            let block_corner = (x == x0 + 1 || x == x1 - 2) && (y == y0 + 1 || y == y1 - 2);
            for z in 0..28 {
                let material = match z {
                    0..2 => Some(m.silver),                                       // plinto
                    2..4 if in_block && !block_corner => Some(m.marble),          // bloque original
                    4 if r <= SHAFT + 1.0 => Some(m.marble),                      // anillo
                    12 | 19 if r <= SHAFT + 0.6 => Some(m.trim),                  // anillos del fuste
                    5..26 if r <= SHAFT => Some(m.marble_grey),                   // fuste
                    26 if r <= SHAFT + 1.3 => Some(m.marble),                     // capitel
                    27 if in_block => Some(m.trim),                               // ábaco
                    _ => None,
                };
                match material {
                    Some(material) => put(grid, (x, y, z), material),
                    // Vacía lo que quedaba del muro de la arquería alrededor de la columna.
                    None if arcade_wall(x) => {
                        let (cx, cy, cz) = cell(x, y, z);
                        clear_checked(grid, cx, cy, cz);
                    }
                    None => {}
                }
            }
        }
    }

    // Dibujos al estilo de Durham en el fuste, solo en las dos caras que se ven al
    // recorrer la iglesia (la que mira a la entrada y la que mira al altar). No van
    // tallados: son líneas de mármol pálido sobre el gris, como dibujadas. El dibujo
    // cambia de columna en columna: chevrones, espirales, rombos.
    let pattern = ((station - NAVE_START) / BAY + (side > 0) as isize) % 3;
    for x in x0..x1 {
        for y in (y0..y1).filter(|&y| in_nave(y)) {
            let (dx, dy) = ((x - axis) as f32 + 0.5, (y - station) as f32 + 0.5);
            let r = dx.hypot(dy);
            let on_face = dy.abs() > dx.abs() && (SHAFT - 1.0..=SHAFT).contains(&r);
            if !on_face {
                continue;
            }
            let h = dx.floor() as isize; // posición a lo ancho de la cara, -3..2
            for z in (5..26).filter(|&z| z != 12 && z != 19) {
                let line = match pattern {
                    0 => (z + h.abs()).rem_euclid(4) == 0,                                  // chevrones
                    1 => (z + h * dy.signum() as isize).rem_euclid(4) == 0,                 // espiral
                    _ => (z + h).rem_euclid(5) == 0 || (z - h).rem_euclid(5) == 0,          // rombos
                };
                if line {
                    put(grid, (x, y, z), m.trim);
                }
            }
        }
    }

    // Fuste adosado de media caña sobre la cara del muro que da a la nave.
    let face = side * NAVE_HALF;
    for x in face - 3..face + 3 {
        for y in (station - 3..station + 3).filter(|&y| in_nave(y)) {
            let toward_nave = if side > 0 { x < face } else { x >= face };
            if toward_nave && ((x - face) as f32 + 0.5).hypot((y - station) as f32 + 0.5) <= 2.2 {
                for z in 28..80 {
                    let material = match z {
                        28 | 53 => m.trim,
                        77.. => m.trim,
                        _ => m.marble_grey,
                    };
                    put(grid, (x, y, z), material);
                }
            }
        }
    }
}

/// Portada en el muro oeste vista desde adentro: un vano ojival de 0.4 m de fondo con
/// las hojas de roble (y dos bandas de hierro) al fondo; el tímpano queda en piedra.
fn portal(grid: &mut VoxelGrid, m: &MaterialSet, center: isize, width: isize, height: isize, door: (isize, isize)) {
    arch(grid, Axis::Z, (WALL - 2, WALL), center - width / 2, 0, width, height, None);
    let (door_width, door_height) = door;
    for x in center - door_width / 2..center + door_width / 2 {
        for z in 0..door_height {
            let band = z == door_height / 4 || z == door_height * 3 / 4;
            put(grid, (x, WALL - 3, z), if band { m.iron } else { m.wood });
        }
    }
}

fn openings(grid: &mut VoxelGrid, m: &MaterialSet) {
    // Portada principal y portadas laterales (a los vestíbulos), con las puertas cerradas.
    portal(grid, m, 0, 20, 40, (16, 28));
    for x in [-35, 35] {
        portal(grid, m, x, 10, 25, (8, 22));
    }

    // Rosetón abierto al cielo en el muro oeste del nártex: Ø 4.8 m, centro a 14 m.
    let (cx, cy, _) = cell(0, 0, 70);
    place_rose_window(grid, Axis::Z, (0, WALL), cx as f32, cy as f32, 12.0, 8, m.trim, None);

    // Arco del nártex a la nave, y de cada vestíbulo a su nave lateral.
    arch(grid, Axis::Z, (36, 40), -16, 0, 32, 84, None);
    let aisle_center = (ARCADE.1 + AISLE_WALL) / 2;
    for start in [-aisle_center - 6, aisle_center - 6] {
        arch(grid, Axis::Z, (36, 40), start, 0, 12, 36, None);
    }

    // Testeros del crucero: una capilla lateral (panel con el santo sobre el altar) y,
    // encima, una ventana abierta (alféizar a 6 m, para dejarle lugar al panel).
    for side in [-1, 1] {
        let wall = if side > 0 { (ARM_END, ARM_END + WALL) } else { (-ARM_END - WALL, -ARM_END) };
        arch(grid, Axis::X, wall, CROSSING_CENTER - 10, 30, 20, 40, None);
        shattered_pane(grid, m, Axis::X, (wall.0 + wall.1 - 1) / 2, CROSSING_CENTER - 10, 30, (20, 40), 7, (96 + side) as u32);
        side_chapel_panel(grid, m, side);
    }
    aisle_end_panels(grid, m);

    stained_glass(grid, m);
    aisle_windows(grid, m);
}

/// El único vitral, en el eje del ábside: una lanceta alta de 4 × 8.4 m (20 × 42 vóxeles)
/// que arranca a 2.8 m, justo sobre el retablo, y llega casi al arranque de la media
/// cúpula. Está reventado por la masa de zombis que entra por él: solo quedan astillas
/// pegadas al marco, de hondo muy desparejo, con huecos donde se rompió hasta la piedra. Borde de rubí de 2 vóxeles, el anillo de
/// emplomado y, en las astillas más hondas, el campo de cobalto. Las astillas que se
/// cayeron están en el piso, entre los cuerpos (`zombies::Debris::Glass`).
const GLASS_WIDTH: isize = 20;
const GLASS_HEIGHT: isize = 42;
const GLASS_SILL: isize = 14;

fn stained_glass(grid: &mut VoxelGrid, m: &MaterialSet) {
    // El muro del ábside es curvo: el rango del espesor es más hondo que el muro, para que
    // el vano lo atraviese completo también en los bordes del vitral.
    const WALL_RANGE: (isize, isize) = (LENGTH - WALL - 3, LENGTH);
    arch(grid, Axis::Z, WALL_RANGE, -GLASS_WIDTH / 2, GLASS_SILL, GLASS_WIDTH, GLASS_HEIGHT, None);

    let pane = (WALL_RANGE.0 + WALL_RANGE.1 - 1) / 2;
    shattered_pane(grid, m, Axis::Z, pane, -GLASS_WIDTH / 2, GLASS_SILL, (GLASS_WIDTH, GLASS_HEIGHT), 7, 91);
}

/// Lo que queda de un vitral reventado en un vano apuntado de `size` (ancho, alto): solo
/// astillas pegadas al marco, de hondo muy desparejo (de 1 a `deepest` vóxeles, en trozos
/// de 2 × 4), con huecos en el borde de rubí donde se rompió hasta la piedra. Borde de
/// rubí de 2 vóxeles, el anillo de emplomado y, en las astillas más hondas, cobalto.
/// `pane` es la capa del muro donde va el vidrio (una `y` con `Axis::Z`, una `x` con
/// `Axis::X`) y `start` la primera columna a lo largo del muro.
#[allow(clippy::too_many_arguments)]
fn shattered_pane(grid: &mut VoxelGrid, m: &MaterialSet, axis: Axis, pane: isize, start: isize, sill: isize, (width, height): (isize, isize), deepest: isize, seed: u32) {
    let columns = lancet_columns(width, height);
    let inside = |c: isize, v: isize| (0..width).contains(&c) && v >= 0 && v < columns[c as usize];
    let near_edge = |c: isize, v: isize, k: isize| (-k..=k).any(|dc| (-k..=k).any(|dv| !inside(c + dc, v + dv)));
    for c in 0..width {
        for v in 0..columns[c as usize] {
            let depth = (1..=deepest + 1).find(|&k| near_edge(c, v, k)).unwrap_or(deepest + 2);
            let broken_border = depth == 2 && hash(c, v / 2, seed + 1) < 0.3;
            if depth > 1 + (hash(c / 2, v / 4, seed) * deepest as f32) as isize || broken_border {
                continue;
            }
            let material = match depth {
                ..=2 => m.glass_ruby,
                3 => m.iron,
                _ => m.glass_cobalt,
            };
            let cell = match axis {
                Axis::Z => (start + c, pane, sill + v),
                Axis::X => (pane, start + c, sill + v),
            };
            put(grid, cell, material);
        }
    }
}

/// Alto de cada columna de un vano apuntado de `width` × `height` vóxeles: el mismo perfil
/// que talla `arch`, para que el vidrio llene el vano justo.
fn lancet_columns(width: isize, height: isize) -> Vec<isize> {
    let jamb = height - (width as f32 * 3f32.sqrt() / 2.0).round() as isize;
    (0..width).map(|c| pointed_arch_profile(c as f32 + 0.5, width as f32, jamb as f32).round() as isize).collect()
}

/// Los vitrales de los muros de las naves laterales: una lanceta grande por tramo (2.4 ×
/// 7.2 m, alféizar a 1.6 m), centrada entre las pilastras. Vidrio emplomado liso, sin motivo: rombos de
/// colores mezclados (cobalto, rubí, ámbar) entre plomos de hierro, con un marco de
/// hierro. Todavía enteros, sin zombis, pero empezando a rajarse: desde un punto de golpe
/// salen fisuras claras que zigzaguean por los paños (dos muy rajadas, con un agujerito y
/// un anillo de telaraña; dos con unas pocas). La luna no llega a estos muros (viene del
/// este), así que el vidrio tiene un resplandor propio tenue para que se vea de noche.
const AISLE_GLASS: (isize, isize, isize) = (12, 36, 8); // ancho, alto, alféizar
const AISLE_GLASS_START: isize = (BAY - 12) / 2; // dentro del tramo: centrada
const AISLE_GLASS_GLOW: f32 = 1.1;

fn aisle_windows(grid: &mut VoxelGrid, m: &MaterialSet) {
    let (width, height, sill) = AISLE_GLASS;
    let columns = lancet_columns(width, height);
    // Sin pálido en los paños: lo blanco son solo las fisuras.
    let colors = [m.glass_cobalt, m.glass_ruby, m.glass_amber, m.glass_cobalt, m.glass_ruby, m.glass_amber].map(|g| g.with_glow(AISLE_GLASS_GLOW));
    let lead = m.iron;
    for side in [-1isize, 1] {
        let wall = if side > 0 { (AISLE_WALL, AISLE_WALL + WALL) } else { (-AISLE_WALL - WALL, -AISLE_WALL) };
        let pane = side * (AISLE_WALL + WALL / 2);
        for bay in 0..NAVE_BAYS {
            let start = NAVE_START + bay * BAY + AISLE_GLASS_START;
            arch(grid, Axis::X, wall, start, sill, width, height, None);
            let seed = (start * 2 + side) as u32;
            for c in 0..width {
                for v in 0..columns[c as usize] {
                    let frame = c == 0 || c == width - 1 || v == 0 || v == columns[c as usize] - 1;
                    let leading = (c + v) % 4 == 0 || (c - v).rem_euclid(4) == 0;
                    let material = if frame || leading {
                        lead
                    } else {
                        colors[(hash((c + v) / 4, (c - v).div_euclid(4), seed) * colors.len() as f32) as usize % colors.len()]
                    };
                    put(grid, (pane, start + c, sill + v), material);
                }
            }
            let badly = (bay + (side > 0) as isize) % 2 == 1;
            glass_cracks(grid, m, pane, start, sill, &columns, badly, seed);
        }
    }
}

/// Fisuras en un vitral entero de columnas `columns`: líneas claras (vidrio pálido con
/// brillo, la luz en la rajadura) que salen de un punto de golpe y zigzaguean. Si está
/// `badly` rajado: más fisuras y más largas, un anillo de telaraña y un agujerito en el
/// centro del golpe.
#[allow(clippy::too_many_arguments)]
fn glass_cracks(grid: &mut VoxelGrid, m: &MaterialSet, pane: isize, start: isize, sill: isize, columns: &[isize], badly: bool, seed: u32) {
    let crack = m.glass_pale.with_glow(1.4);
    let width = columns.len() as isize;
    let inside = |c: isize, v: isize| c > 0 && c < width - 1 && v > 0 && v < columns[c as usize] - 1;
    let mut rng = Rng::new(seed.wrapping_mul(7) + 3);
    let (c0, v0) = (rng.range(3.0, width as f32 - 3.0), rng.range(14.0, 24.0));
    let (count, length) = if badly { (7, (10.0, 20.0)) } else { (3, (4.0, 8.0)) };
    for i in 0..count {
        let mut angle = i as f32 / count as f32 * std::f32::consts::TAU + rng.range(-0.3, 0.3);
        let (mut c, mut v) = (c0, v0);
        for _ in 0..rng.range(length.0, length.1) as usize {
            angle += rng.range(-0.35, 0.35);
            c += angle.cos();
            v += angle.sin();
            let (ci, vi) = (c.round() as isize, v.round() as isize);
            if inside(ci, vi) {
                put(grid, (pane, start + ci, sill + vi), crack);
            }
        }
    }
    if badly {
        for c in 0..width {
            for v in 0..columns[c as usize] {
                let r = (c as f32 - c0).hypot(v as f32 - v0);
                if inside(c, v) && (r - 2.6).abs() < 0.5 && hash(c, v, seed) < 0.6 {
                    put(grid, (pane, start + c, sill + v), crack);
                }
                if r < 0.9 {
                    let (cx, cy, cz) = cell(pane, start + c, sill + v);
                    clear_checked(grid, cx, cy, cz);
                }
            }
        }
    }
}

/// Metros del spec → mundo.
fn meters(x: f32, y: f32, z: f32) -> Vec3 {
    world(x * VOXELS_PER_METER, y * VOXELS_PER_METER, z * VOXELS_PER_METER)
}

/// Centros de los tramos en metros: los de la nave y, último, el del crucero.
const BAY_CENTERS: [f32; 3] = [12.0, 20.0, 28.0];
const CROSSING_Y: f32 = CROSSING_CENTER as f32 / VOXELS_PER_METER;

/// El mobiliario (sección 8 del spec + adaptación): presbiterio, ceremonia, púlpito,
/// candelabros, lámparas colgantes y los esqueletos. Cada prop tiene su propio grid de
/// 5 cm, ajustado, y los props cercanos van en el mismo grupo (una sola caja envolvente
/// por grupo, ver `group.rs`). Todas las luces de velas salen de acá, de la llama de
/// cada prop.
fn furnishings(grid: &mut VoxelGrid, m: &MaterialSet, lights: &mut Vec<Light>) -> Vec<Vec<VoxelGrid>> {
    let at = |x: f32, y: f32| meters(x, y, 0.0); // esquina mínima de cada prop, en metros
    let mut lit = |group: &mut Vec<VoxelGrid>, (prop, light): (VoxelGrid, Vec3), intensity: f32| {
        group.push(prop);
        lights.push(Light::new(light, CANDLE, intensity));
    };

    // Presbiterio: las posiciones del spec, corridas 24 m al oeste con el ábside.
    let mut sanctuary = vec![
        props::altar(m, at(-0.9, 37.4)),
        props::retable(m, at(-0.9, 39.2)),
        props::ambo(m, at(-4.0, 33.2)),
        props::bishop_chair(m, at(-0.6, 34.2)),
        props::reliquary(m, at(-4.0, 38.0)),
    ];
    lit(&mut sanctuary, props::paschal_candle(m, at(-0.3, 36.8)), 22.0);
    for x in [-1.8, 1.4] {
        lit(&mut sanctuary, props::altar_stand(m, at(x, 37.8)), 22.0);
    }

    // Ceremonia, sobre el sol del crucero, de oeste a este: los novios en su centro, los
    // reclinatorios, el oficiante y la cruz procesional detrás de él.
    let y = CROSSING_Y;
    let ceremony = vec![
        props::kneeler(m, at(-0.9, y + 0.8)),
        props::kneeler(m, at(0.3, y + 0.8)),
        props::processional_cross(m, at(-0.3, y + 3.2)),
        // La novia está en `invasion`, en brazos del zombi que la muerde.
        people::groom(m, meters(1.15, y - 0.3, 0.0), Vec3::new(-0.55, 0.0, -0.83), Vec3::new(-0.25, 0.08, -0.97)),
        people::officiant(m, meters(0.0, y + 2.2, 0.0), Vec3::new(0.0, 0.0, -1.0)),
    ];

    // Candelabros de pie (0.8 m de lado; las posiciones son su centro). En la nave, uno
    // por tramo salvo el del crucero, que tiene los suyos. Las naves laterales quedan sin
    // candelabros, en penumbra.
    let candelabrum = |x: f32, y: f32| props::candelabrum(m, at(x - 0.4, y - 0.4));
    let mut groups = Vec::new();
    for side in [-1.0, 1.0] {
        let mut nave = Vec::new();
        for y in BAY_CENTERS.into_iter().filter(|&y| y != CROSSING_Y) {
            lit(&mut nave, candelabrum(side * 3.6, y), 38.0);
        }
        groups.push(nave);
    }
    // Pares: junto a los novios (al borde del sol), nártex y coro.
    // Cada par es su propio grupo, aparte de la ceremonia: si no, estirarían su caja.
    for (x, y, intensity) in [(3.2, CROSSING_Y, 33.0), (2.4, 5.2, 28.0), (3.6, 35.2, 28.0)] {
        let mut pair = Vec::new();
        for side in [-1.0, 1.0] {
            lit(&mut pair, candelabrum(side * x, y), intensity);
        }
        groups.push(pair);
    }

    // Capillas laterales en los testeros del crucero: altar contra el muro, bajo el panel
    // del santo, con un soporte de velas votivas a cada lado. Las velas votivas son la luz
    // de la capilla.
    let arm_end = ARM_END as f32 / VOXELS_PER_METER;
    for side in [-1.0, 1.0] {
        let mut chapel = vec![props::side_altar(m, at(if side > 0.0 { arm_end - 0.8 } else { -arm_end }, CROSSING_Y - 0.9))];
        let rack_x = if side > 0.0 { arm_end - 0.6 } else { -arm_end };
        for y in [CROSSING_Y - 3.0, CROSSING_Y + 1.8] {
            lit(&mut chapel, props::votive_rack(m, at(rack_x, y)), 22.0);
        }
        groups.push(chapel);
    }

    // Lámparas colgantes sobre el eje, una por tramo, a 12 m, cada una con un tirante de
    // hierro de 0.4 m (en el grid de la catedral) hasta la clave de la bóveda.
    let mut hanging = Vec::new();
    for y in BAY_CENTERS {
        lit(&mut hanging, props::chandelier(m, meters(-0.8, y - 0.8, 11.6)), 140.0);
        let yc = (y * VOXELS_PER_METER) as isize;
        for (x, yy) in [(-1, yc - 1), (0, yc - 1), (-1, yc), (0, yc)] {
            for z in 62..VAULT_CROWN - 6 {
                put(grid, (x, yy, z), m.iron);
            }
        }
    }

    groups.extend([sanctuary, ceremony, hanging, vec![props::pulpit(m, at(-8.8, 22.0))]]);
    groups
}

/// Pinta con sangre las celdas ya ocupadas (no agrega bloques sueltos en el aire, por
/// ejemplo dentro de un nicho).
fn stain(grid: &mut VoxelGrid, m: &MaterialSet, (x, y, z): (isize, isize, isize)) {
    let (cx, cy, cz) = cell(x, y, z);
    if grid.is_occupied(cx, cy, cz) {
        put(grid, (x, y, z), m.blood);
    }
}

/// Charco de sangre de borde irregular sobre el piso, centrado en `(cx, cy)` (vóxeles del spec).
fn blood_pool(grid: &mut VoxelGrid, m: &MaterialSet, (cx, cy): (isize, isize), radius: f32, seed: u32) {
    let reach = radius as isize + 3;
    for x in cx - reach..=cx + reach {
        for y in cy - reach..=cy + reach {
            let edge = radius * (0.7 + 0.6 * hash(x, y, seed));
            if ((x - cx) as f32).hypot((y - cy) as f32) <= edge {
                stain(grid, m, (x, y, -1));
            }
        }
    }
}

/// Rastro de arrastre de `from` a `to` (vóxeles del spec): manchas chicas cada 3
/// vóxeles, con un leve zigzag.
fn blood_trail(grid: &mut VoxelGrid, m: &MaterialSet, from: (f32, f32), to: (f32, f32), seed: u32) {
    let steps = ((to.0 - from.0).hypot(to.1 - from.1) / 3.0) as usize;
    for i in 0..=steps {
        let t = i as f32 / steps.max(1) as f32;
        let wobble = (t * 12.0).sin() * 1.5;
        let (x, y) = (from.0 + (to.0 - from.0) * t + wobble, from.1 + (to.1 - from.1) * t);
        blood_pool(grid, m, (x.round() as isize, y.round() as isize), 1.6 - 0.6 * t, seed + i as u32);
    }
}

/// Chorreaduras de sangre bajando por la cara de un muro desde `top`, en `count` columnas
/// al azar a lo largo de `span`. `face` es la celda del muro que da al interior: una `x`
/// para los muros normales a `x` (`Axis::X`), una `y` para los normales a `y` (`Axis::Z`,
/// como la fachada oeste).
#[allow(clippy::too_many_arguments)]
fn blood_drips(grid: &mut VoxelGrid, m: &MaterialSet, wall: Axis, face: isize, span: (isize, isize), top: isize, count: usize, rng: &mut Rng) {
    for _ in 0..count {
        let t = rng.range(span.0 as f32, span.1 as f32) as isize;
        let length = rng.range(6.0, 30.0) as isize;
        for z in top - length..top {
            let cell = match wall {
                Axis::X => (face, t, z),
                Axis::Z => (t, face, z),
            };
            stain(grid, m, cell);
        }
    }
}

/// La invasión zombi (ver `zombies.rs`): la puerta principal rota, una pirámide humana en
/// cada abertura (las 6 ventanas altas, el rosetón, las 2 ventanas de los brazos del
/// crucero, la puerta y el vitral del ábside, que también cedió), los que se separaron de
/// ellas, zombis bajando por las columnas, caminando y cayendo, el que muerde a la novia,
/// y sangre en el piso y chorreando por los muros.
fn invasion(grid: &mut VoxelGrid, m: &MaterialSet) -> Vec<Vec<VoxelGrid>> {
    let (along_y, along_x) = (Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0));
    let mut rng = Rng::new(666);
    let mut seed = 1000;
    let mut next_seed = || {
        seed += 97;
        seed
    };

    // Ventanas altas de la nave: una pila 4-3-2-1 en el vano, dos que bajan por el muro,
    // uno o dos que se caen de la ventana (el primero volcándose por el alféizar) y las
    // astillas del vitral volando alrededor.
    let mut clerestory = [Vec::new(), Vec::new()];
    for bay in 0..=NAVE_BAYS {
        let s = NAVE_START + bay * BAY;
        for (i, side) in [-1isize, 1].into_iter().enumerate() {
            let face = side * NAVE_HALF;
            let o = Opening {
                bottom: world(face as f32, (s + 20) as f32, 84.0),
                along: along_y,
                inward: Vec3::new(-side as f32, 0.0, 0.0),
                width: 2.4,
                height: 3.2,
                round: false,
                sill: 16.8,
                stack: 4,
                loose: 2,
                debris: Debris::None,
            };
            clerestory[i].extend(zombies::pyramid(m, &o, next_seed()));
            clerestory[i].extend(zombies::falling_from(m, &o, 2 + ((bay as usize + i) % 2), next_seed()));
            clerestory[i].push(zombies::shard_burst(m, &o, (0.3, 2.6), 14, (1.0, 2.5), next_seed()));
            let wall_face = if side > 0 { NAVE_HALF } else { -NAVE_HALF - 1 };
            blood_drips(grid, m, Axis::X, wall_face, (s + 15, s + 25), 84, 10, &mut rng);
        }
    }

    // Fachada oeste: la puerta principal rota (el vano atraviesa todo el muro), con una
    // estampida entrando por ella (`zombies::surge`: la multitud empujando en el vano, los
    // de adelante tropezando a mitad de la caída, los caídos pisoteados y las hojas rotas
    // entre medio); y el rosetón reventado, con su pila y los que bajan por el muro.
    arch(grid, Axis::Z, (0, WALL), -10, 0, 20, 40, None);
    let door = Opening {
        bottom: world(0.0, WALL as f32, 0.0),
        along: along_x,
        inward: along_y,
        width: 4.0,
        height: 8.0,
        round: false,
        sill: 0.0,
        stack: 0,
        loose: 0,
        debris: Debris::Door,
    };
    let rose = Opening { bottom: world(0.0, WALL as f32, 58.0), width: 4.8, height: 4.8, round: true, sill: 11.6, stack: 6, loose: 4, debris: Debris::None, ..door };
    let mut west = vec![zombies::surge(m, &door, next_seed())];
    west.extend(zombies::pyramid(m, &rose, next_seed()));

    // Sangre, sobre todo en la entrada: un charco grande bajo el montón y otros alrededor,
    // rastros de arrastre desde la puerta hacia la nave, chorreaduras por la cara interior
    // de la fachada (y bajo el rosetón), y salpicaduras frescas en el nártex.
    for ((x, y), radius) in [((0, 18), 12.0), ((0, 27), 9.0), ((-12, 14), 5.0), ((12, 22), 6.0), ((-6, 36), 3.0), ((8, 40), 4.0)] {
        blood_pool(grid, m, (x, y), radius, next_seed());
    }
    for (start, end) in [((-4.0, 24.0), (-10.0, 70.0)), ((3.0, 26.0), (6.0, 82.0)), ((8.0, 22.0), (14.0, 60.0)), ((-1.0, 28.0), (-3.0, 100.0))] {
        blood_trail(grid, m, start, end, next_seed());
    }
    blood_drips(grid, m, Axis::Z, WALL - 1, (-24, 24), 50, 30, &mut rng);
    blood_drips(grid, m, Axis::Z, WALL - 1, (-12, 12), 58, 12, &mut rng);
    for ((x, y), scale) in [((-9.0, 26.0), 1.0), ((10.0, 32.0), 0.8), ((-6.0, 46.0), 0.7), ((13.0, 12.0), 0.9), ((8.0, 52.0), 0.6)] {
        west.push(zombies::splatter(m, world(x, y, 0.0), scale, next_seed()));
    }

    // Ventanas de los testeros del crucero, sobre las capillas, junto al sol y la luna:
    // revientan y se vacían hacia adentro siguiendo la gravedad (`zombies::torrent`): pocos
    // en el vano, la mayoría en el aire a mitad de la caída y los que ya llegaron, rotos en
    // el piso de la capilla. Dos cuelgan de las jambas, y las astillas salen disparadas
    // lejos, cruzando el brazo hacia el crucero.
    let mut arms = Vec::new();
    for side in [-1isize, 1] {
        let o = Opening {
            bottom: world((side * ARM_END) as f32, CROSSING_CENTER as f32, 30.0),
            along: along_y,
            inward: Vec3::new(-side as f32, 0.0, 0.0),
            width: 4.0,
            height: 8.0,
            round: false,
            sill: 6.0,
            stack: 0,
            loose: 0,
            debris: Debris::None,
        };
        arms.push(zombies::torrent(m, &o, 22, 12, (-1.6, 1.4), (0, 0.0, 0.0), next_seed()));
        arms.extend(zombies::hanging(m, &o, [2.8, 2.0], next_seed()));
        arms.push(zombies::shard_burst(m, &o, (0.3, 4.3), 40, (5.0, 13.0), next_seed()));
        let face = if side > 0 { ARM_END } else { -ARM_END - 1 };
        blood_drips(grid, m, Axis::X, face, (CROSSING_CENTER - 10, CROSSING_CENTER + 10), 30, 12, &mut rng);
        blood_pool(grid, m, (side * (ARM_END - 8), CROSSING_CENTER - 6), 5.0, next_seed());
    }

    // El vitral del ábside reventado, vaciándose hacia adentro con la gravedad, como los
    // del crucero (`zombies::torrent`): pocos en el alféizar, la mayoría cayendo (el
    // alféizar está a 2.8 m: caídas cortas) y los que ya llegaron, rotos detrás del retablo
    // entre las astillas. Afuera, la montaña por la que suben hasta la ventana llena el
    // vano desde atrás, recortada contra la luna. Dos cuelgan de las jambas y el vidrio
    // vuela en el aire. Lo alto del vano queda abierto y entra la luna.
    let apse = Opening {
        bottom: world(0.0, (LENGTH - WALL) as f32, GLASS_SILL as f32),
        along: along_x,
        inward: -along_y,
        width: GLASS_WIDTH as f32 / VOXELS_PER_METER,
        height: GLASS_HEIGHT as f32 / VOXELS_PER_METER,
        round: false,
        sill: GLASS_SILL as f32 / VOXELS_PER_METER,
        stack: 0,
        loose: 0,
        debris: Debris::Glass,
    };
    let mut apse_zombies = vec![zombies::torrent(m, &apse, 16, 14, (-1.8, 1.8), (110, 4.0, 3.0), next_seed())];
    apse_zombies.extend(zombies::hanging(m, &apse, [2.6, 1.9], next_seed()));
    apse_zombies.push(zombies::shard_burst(m, &apse, (0.3, 4.3), 45, (2.0, 4.5), next_seed()));

    // Además, sueltos por toda la iglesia, cada uno por su cuenta:
    // - bajando cabeza abajo por las columnas: dos en cada columna exenta, a distintas
    //   alturas y del lado que se ve, y uno en cada fuste adosado alto de la nave;
    // - dos bajando por los muros altos de la nave;
    // - tres que ya cayeron de las ventanas altas y se estrellaron contra el piso, con la
    //   sangre saltando del golpe (los que están cayendo salen de cada ventana, arriba);
    // - caminando por la nave, las naves laterales, el nártex y los brazos del crucero,
    //   todos hacia los novios;
    // - uno arrastrándose por el piso de la nave, dejando un rastro de sangre.
    let flat = |x: f32, y: f32| Vec3::new(x, 0.0, y);
    let mut strays = Vec::new();
    let climb = |rng: &mut Rng, pelvis: Vec3, toward: Vec3, seed: u32| zombies::single(m, &zombies::climbing(rng, pelvis, toward), seed);
    for station in [NAVE_START + BAY, NAVE_START + 2 * BAY] {
        for side in [-1isize, 1] {
            let axis = (side * (NAVE_HALF + WALL / 2)) as f32;
            // Los lados que dan a la nave central y a lo largo de ella.
            let faces = [flat(-side as f32, 0.0), flat(-side as f32 * 0.7, 0.7), flat(-side as f32 * 0.7, -0.7), flat(0.0, 1.0), flat(0.0, -1.0)];
            let first = rng.range(0.0, 5.0) as usize;
            for (k, z) in [(first, rng.range(8.0, 14.0)), ((first + 2) % 5, rng.range(17.0, 23.0))] {
                let out = faces[k].normalize();
                let pelvis = world(axis, station as f32, z) + out * (SHAFT + 0.9);
                strays.push(climb(&mut rng, pelvis, -out, next_seed()));
            }
            let face = (side * (NAVE_HALF - 4)) as f32;
            let pelvis = world(face, station as f32, rng.range(36.0, 66.0));
            strays.push(climb(&mut rng, pelvis, flat(side as f32, 0.0), next_seed()));
        }
    }
    for (x, y, z) in [(20.5, 60.0, 50.0), (-20.5, 100.0, 66.0)] {
        strays.push(climb(&mut rng, world(x, y, z), flat(x.signum(), 0.0), next_seed()));
    }
    for (x, y) in [(12.0, 76.0), (-14.0, 92.0), (12.0, 114.0)] {
        strays.push(zombies::impact(m, world(x, y, 0.0), next_seed()));
    }

    let couple = world(0.0, CROSSING_CENTER as f32, 0.0);
    let walkers = [
        (-14.0, 26.0), (13.0, 20.0),                // nártex, recién entrados
        (-9.0, 52.0),                               // nave (los demás, comiéndose a los padres)
        (-34.0, 72.0), (35.0, 76.0), (-35.0, 102.0), (37.0, 114.0), // naves laterales
        (-50.0, 146.0), (48.0, 132.0),              // brazos del crucero
    ];
    for (x, y) in walkers {
        let feet = world(x, y, 0.0);
        let to_couple = couple - feet;
        let heading = (flat(to_couple.x, to_couple.z).normalize() + flat(rng.range(-0.5, 0.5), rng.range(-0.5, 0.5))).normalize();
        strays.push(zombies::single(m, &zombies::walking(&mut rng, feet, heading), next_seed()));
    }
    // Los padres de los novios, atacados por los que venían caminando por la nave, a lo
    // largo del camino hacia la boda (no junto a la puerta), cada uno con su charco:
    // - el padre del novio, de pie, peleando, de cara a la puerta (se le ve la cara al
    //   entrar);
    // - la madre de la novia, arrastrándose hacia el pasillo central, de perfil para quien
    //   entra;
    // - la madre del novio, desplomada contra un reclinatorio, mirando hacia la entrada;
    // - el padre de la novia, tirado de espaldas, la cabeza hacia el centro de la nave.
    // Los zombis que comen vienen siempre del lado de atrás de cada uno (el que no da a la
    // entrada), así desde la nave se ve a la víctima entera y al zombi de frente mordiendo.
    // Parejos: a 2 m del eje, uno de cada lado del pasillo central por turno, cada 3.6–4 m.
    let parents = [
        (Feast::FightingFather, (10.0f32, 62.0f32), (-0.3f32, -1.0f32)),
        (Feast::CrawlingMother, (-10.0, 80.0), (1.0, 0.4)),
        (Feast::CollapsedMother, (10.0, 100.0), (-0.6, -0.8)),
        (Feast::PinnedFather, (-10.0, 118.0), (1.0, 0.0)),
    ];
    for (which, (x, y), (fx, fy)) in parents {
        strays.push(zombies::feast(m, which, world(x, y, 0.0), Vec3::new(fx, 0.0, fy), next_seed()));
        blood_pool(grid, m, (x as isize, y as isize), 4.0, next_seed());
        if let Feast::CollapsedMother = which {
            // El reclinatorio a su espalda (0.6 m de lado; `meters` toma la esquina).
            let back = Vec3::new(-fx, 0.0, -fy).normalize() * 0.55;
            let (cx, cy) = (x / VOXELS_PER_METER + back.x, y / VOXELS_PER_METER + back.z);
            strays.push(props::kneeler(m, meters(cx - 0.3, cy - 0.3, 0.0)));
        }
    }

    // Uno que se arrastra por la nave dejando un rastro de sangre, del lado del pasillo
    // contrario a la madre desplomada (antes quedaba tirado delante de ella, tapándola).
    let crawler = zombies::wild_pose(&mut rng, world(-4.0, 99.0, 0.8), Vec3::new(0.0, 0.1, 1.0), Vec3::new(0.0, -0.3, 1.0), 0.3);
    strays.push(zombies::single(m, &crawler, next_seed()));
    for y in (86..99).step_by(5) {
        blood_pool(grid, m, (-4, y), 2.0, next_seed());
    }
    for ((x, y), radius) in [((-16, 58), 3.0), ((15, 96), 3.0), ((-8, 128), 4.0), ((20, 80), 3.0), ((-20, 80), 2.5), ((20, 120), 2.5), ((-20, 120), 3.0), ((-36, 90), 3.0), ((36, 100), 2.5)] {
        blood_pool(grid, m, (x, y), radius, next_seed());
    }

    // El que muerde a la novia: la tiene inclinada hacia atrás, hacia el norte, como en el
    // beso de una boda. El novio lo agarra por detrás.
    strays.push(zombies::dip(m, meters(-0.15, CROSSING_Y, 0.0), -along_x, next_seed()));

    // Los sueltos van repartidos por toda la iglesia: en cuatro grupos (norte o sur, antes
    // o después del crucero) cada rayo se saltea los que están lejos.
    let mut quarters: [Vec<VoxelGrid>; 4] = Default::default();
    for zombie in strays {
        let (lo, hi) = zombie.bounds();
        let center = (lo + hi) / 2.0 - world(0.0, CROSSING.0 as f32, 0.0);
        quarters[(center.x > 0.0) as usize * 2 + (center.z > 0.0) as usize].push(zombie);
    }
    let [north, south] = clerestory;
    let mut groups = vec![north, south, west, arms, apse_zombies];
    groups.extend(quarters);
    groups
}

pub fn build(textures: &mut TextureBank) -> Scene {
    let m = materials::build(textures);
    let mut grid = VoxelGrid::new(GRID_DIMS, 1.0, Vec3::zeros());
    shell(&mut grid, &m);
    openings(&mut grid, &m);
    bays(&mut grid, &m);

    let mut lights = Vec::new();
    let mut groups = furnishings(&mut grid, &m, &mut lights);
    groups.extend(invasion(&mut grid, &m));
    // La luna viene del este, así que adentro solo entra por el vitral del ábside: todo lo
    // demás le da la espalda o la tiene tapada por la piedra. Con el vano marcado, los
    // puntos que no lo ven por ahí ni lanzan su rayo de sombra.
    let apse_window = Aperture {
        center: world(0.0, (LENGTH - WALL) as f32 + 0.5, (GLASS_SILL + GLASS_HEIGHT / 2) as f32),
        half_width: Vec3::new(GLASS_WIDTH as f32 / 2.0 + 0.2, 0.0, 0.0),
        half_height: Vec3::new(0.0, GLASS_HEIGHT as f32 / 2.0 + 0.2, 0.0),
    };
    lights.push(Light::directional(skybox::moon_direction(), Color::new(140, 155, 200), 1.1).through(apse_window));
    // Un haz de luna frío sobre la boda, desde lo alto de la nave, del lado de la entrada
    // (a 14 m, 6 m antes de los novios, lejos de la lámpara colgante del crucero): los
    // ilumina de frente para quien llega por la nave. Con la iglesia oscura se perdían.
    let couple = meters(-0.3, CROSSING_Y, 0.6);
    lights.push(Light::spot(meters(0.0, CROSSING_Y - 6.0, 14.0), couple, 0.14, Color::new(170, 190, 255), 5000.0));
    // Luna que entra por los vitrales rajados de las naves laterales: un foco afuera de
    // cada uno, 7 m afuera y 8 m más alto, apuntando a través del vidrio hacia el piso de
    // la nave lateral. La luz pasa teñida por los rombos (y blanca por los agujeritos),
    // así cada vitral pinta su dibujo en el piso.
    let (width, height, sill) = AISLE_GLASS;
    for side in [-1.0f32, 1.0] {
        for bay in 0..NAVE_BAYS {
            let y = (NAVE_START + bay * BAY + AISLE_GLASS_START + width / 2) as f32;
            let pane = (AISLE_WALL + WALL / 2) as f32;
            let center_z = (sill + height / 2) as f32;
            let light = world(side * (pane + 35.0), y, center_z + 40.0);
            let floor = world(side * (pane - 35.0 * center_z / 40.0), y, 0.0);
            let window = Aperture {
                center: world(side * pane + 0.5, y, center_z),
                half_width: Vec3::new(0.0, 0.0, width as f32 / 2.0 + 0.2),
                half_height: Vec3::new(0.0, height as f32 / 2.0 + 0.2, 0.0),
            };
            lights.push(Light::spot(light, floor, 0.36, Color::new(170, 190, 255), 8500.0).through(window));
        }
    }

    // Vistas 1, 2, 3 y 6 del spec (las exteriores 4 y 5 no aplican), más una hacia el
    // rosetón y una detrás de los novios. "boda" y "mordida" miran desde el lado de la
    // nave, de donde viene el haz de luna que ilumina a los novios.
    let view = |name, eye: [f32; 3], target: [f32; 3]| CameraPreset { name, eye: world(eye[0], eye[1], eye[2]), target: world(target[0], target[1], target[2]) };
    let c = CROSSING_CENTER as f32;
    let presets = vec![
        // La vista inicial: el vitral reventado del ábside, con los zombis entrando, desde
        // pasando el altar (sin que tape el cuadro) y delante de la cascada, a 4 m de
        // altura y mirando derecho. Alejarse (S) la lleva hacia atrás a esa misma altura,
        // por encima del altar y de la boda, hasta la entrada: la iglesia entera.
        view("inicio", [0.0, 192.0, 20.0], [0.0, 214.0, 20.0]),
        view("nave", [0.0, 34.0, 8.0], [0.0, 200.0, 26.0]),
        view("boda", [-12.0, c - 12.0, 8.0], [-1.0, c, 5.0]),
        view("vitral", [0.0, LENGTH as f32 - 54.0, 9.0], [0.0, LENGTH as f32 - 2.0, 37.0]),
        view("crucero", [-42.0, c - 4.0, 8.0], [0.0, c, 80.0]),
        view("roseton", [0.0, 110.0, 8.0], [0.0, 0.0, 70.0]),
        // Como la imagen de referencia: detrás de los novios, mirando al altar.
        view("altar", [0.0, c - 40.0, 8.0], [0.0, c + 60.0, 22.0]),
        // Desde el crucero hacia el panel de plata al final de la nave lateral sur.
        view("panel", [8.0, c - 14.0, 8.0], [36.0, NAVE_END as f32, 26.0]),
        // Por la nave lateral norte, mirando al este.
        view("lateral", [-36.0, 44.0, 8.0], [-36.0, 150.0, 30.0]),
        // Para revisar el piso: casi en picada desde 20 m, sobre el final de la nave.
        view("planta", [0.0, c - 15.0, 100.0], [0.0, c - 9.0, 0.0]),
        // La invasión: la montaña de la puerta rota bajo el rosetón, y la mordida de cerca.
        view("puerta", [8.0, 80.0, 12.0], [0.0, 0.0, 26.0]),
        view("mordida", [-2.5, c - 9.0, 6.5], [-2.5, c, 4.5]),
        // La capilla del brazo norte, con su ventana reventándose hacia adentro.
        view("capilla", [-24.0, c - 8.0, 9.0], [-66.0, c, 22.0]),
    ];

    // Primera persona: al fondo del nártex, delante de la montaña de zombis que entra por
    // la puerta, mirando al altar (+y del spec).
    let walk_spawn = Spawn { feet: world(0.0, 34.0, 0.0), yaw: 0.0, units_per_meter: VOXELS_PER_METER };

    // Solo interior: la cámara orbital no puede salir del edificio (ver
    // `Scene::camera_bounds`). Una sola caja envolvente no alcanza para esta planta en
    // cruz — el rectángulo que cubre a la vez la nave (angosta) y el crucero (ancho) deja
    // sus cuatro esquinas como "adentro de la caja" aunque en realidad son aire vacío de
    // afuera, nunca tallado —, así que es una caja por cada sala real, en la misma altura
    // (piso a la clave de la nave, de sobra para las salas más bajas).
    Scene { grids: vec![grid], groups, lights, presets, walk_spawn, camera_bounds: camera_rooms() }
}

/// Hasta dónde llega (a lo largo de la iglesia) el frente de la montaña de zombis de la
/// puerta principal, y desde dónde empieza la cascada del vitral del ábside: la cámara se
/// queda siempre delante de las dos, nunca detrás (medido sobre los grids de la invasión:
/// la montaña llega hasta y ≈ 30, la cascada arranca en y ≈ 196).
const DOOR_HEAP_FRONT: f32 = 31.0;
const APSE_TORRENT_FRONT: f32 = 196.0;

/// Una caja por cada sala real, para `Scene::camera_bounds` (ver ahí por qué una sola caja
/// envolvente no alcanza). Todas llegan hasta la clave de la nave, de sobra para las salas
/// más bajas. No hay caja para el nártex: queda detrás de la montaña de zombis de la
/// puerta, igual que el fondo del ábside detrás de la cascada del vitral.
fn camera_rooms() -> Vec<(Vec3, Vec3)> {
    let (z0, z1) = (0.0, VAULT_CROWN as f32);
    let room = |x0: f32, x1: f32, y0: f32, y1: f32| (world(x0, y0, z0), world(x1, y1, z1));
    vec![
        room(-AISLE_WALL as f32, AISLE_WALL as f32, DOOR_HEAP_FRONT, NAVE_END as f32), // nave + naves laterales
        room(-ARM_END as f32, ARM_END as f32, (CROSSING.0 - 4) as f32, NAVE_END as f32), // crucero
        room(-APSE_RADIUS as f32, APSE_RADIUS as f32, (NAVE_END - 4) as f32, APSE_TORRENT_FRONT), // coro + ábside
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;

    /// Cada caja de `camera_bounds` toca el piso y llega hasta la clave de la nave, así
    /// que todas se superponen en altura: si el punto está dentro de alguna en (x, z,
    /// largo), está adentro.
    fn inside_any_room(p: Vec3, bounds: &[(Vec3, Vec3)]) -> bool {
        bounds.iter().any(|(min, max)| (min.x..=max.x).contains(&p.x) && (min.y..=max.y).contains(&p.y) && (min.z..=max.z).contains(&p.z))
    }

    /// La cámara orbital, en la vista por defecto ("nave") y en cualquier otra, no puede
    /// salir del edificio: ni alejándose (zoom) ni girando hacia un costado (orbit). Esto
    /// pasa con la escena real, no una aproximación.
    ///
    /// La comprobación es contra `camera_rooms()`, llamada acá aparte (no contra
    /// `scene.camera_bounds`, que es lo que de hecho limita a la cámara): si `build()`
    /// alguna vez volviera a usar una sola caja envolvente para `camera_bounds` (en vez de
    /// una por sala), la cámara quedaría mal limitada pero el test seguiría comparando
    /// contra la forma real — y detectaría que, por ejemplo, girar la vista "nave" hacia
    /// el costado la mandó al hueco junto a la nave, dentro del ancho del crucero pero
    /// fuera de las salas reales.
    #[test]
    fn preset_eyes_are_not_moved_by_their_own_bounds() {
        // Las cajas de `camera_rooms()` tienen que cubrir de verdad las 13 vistas: si
        // alguna se corriera al construir la cámara, la vista arrancaría distinta a como
        // se la revisó en los renders.
        let scene = build(&mut TextureBank::new());
        for preset in &scene.presets {
            let camera = Camera::new(preset.eye, preset.target, Vec3::new(0.0, 1.0, 0.0)).with_bounds(&scene.camera_bounds);
            assert!((camera.eye - preset.eye).magnitude() < 1e-3, "{}: el propio punto de partida quedó afuera de sus salas: {:?} -> {:?}", preset.name, preset.eye, camera.eye);
        }
    }

    /// Alejándose o acercándose desde cualquier vista, la cámara no pasa por detrás de la
    /// montaña de zombis de la puerta ni de la cascada del vitral del ábside.
    #[test]
    fn camera_never_gets_behind_the_zombie_masses() {
        let scene = build(&mut TextureBank::new());
        for preset in &scene.presets {
            for amount in [9_000.0f32, -9_000.0] {
                let mut camera = Camera::new(preset.eye, preset.target, Vec3::new(0.0, 1.0, 0.0)).with_bounds(&scene.camera_bounds);
                camera.max_radius = 10_000.0;
                camera.zoom(amount);
                let along = camera.eye.z; // `world` lleva el largo de la iglesia a z
                assert!((DOOR_HEAP_FRONT..=APSE_TORRENT_FRONT).contains(&along), "{}: la cámara quedó detrás de los zombis ({along:.1})", preset.name);
            }
        }
    }

    /// Desde la vista inicial (el vitral del ábside), alejándose al máximo o girando una
    /// vuelta entera, la montaña de zombis de la puerta queda siempre a la espalda de la
    /// cámara: nunca entra en cuadro.
    #[test]
    fn the_start_view_never_shows_the_door_zombies() {
        let scene = build(&mut TextureBank::new());
        let start = &scene.presets[0];
        assert_eq!(start.name, "inicio");
        let heap = world(0.0, DOOR_HEAP_FRONT / 2.0, 5.0);
        let check = |camera: &Camera, what: &str| {
            let forward = (camera.center - camera.eye).normalize();
            let toward_heap = (heap - camera.eye).normalize();
            assert!(forward.dot(&toward_heap) < 0.0, "{what}: la montaña de la puerta queda adelante de la cámara");
        };
        for zoom in [0.0f32, 30.0, 60.0, 9_000.0] {
            let mut camera = Camera::new(start.eye, start.target, Vec3::new(0.0, 1.0, 0.0)).with_bounds(&scene.camera_bounds);
            camera.zoom(zoom);
            check(&camera, "alejándose");
            for _ in 0..48 {
                camera.orbit(2.0 * std::f32::consts::PI / 48.0, 0.02);
                check(&camera, "girando");
            }
        }
    }

    /// Desde la vista inicial, alejándose (S) se llega hasta la entrada (delante de la
    /// montaña de la puerta), por encima del piso: se ve la iglesia entera.
    #[test]
    fn zooming_out_from_the_start_reaches_the_entrance() {
        let scene = build(&mut TextureBank::new());
        let start = &scene.presets[0];
        let mut camera = Camera::new(start.eye, start.target, Vec3::new(0.0, 1.0, 0.0)).with_bounds(&scene.camera_bounds);
        for _ in 0..400 {
            camera.zoom(camera.radius() * 0.05);
        }
        let along = camera.eye.z;
        assert!(along < DOOR_HEAP_FRONT + 6.0, "se frena antes de la entrada ({along:.1})");
        assert!(along >= DOOR_HEAP_FRONT, "pasó por detrás de la montaña de la puerta ({along:.1})");
        assert!(camera.eye.y > world(0.0, 0.0, 15.0).y, "llega a la entrada por encima del piso, no arrastrándose por él");
    }

    #[test]
    fn orbital_camera_cannot_leave_the_cathedral() {
        let scene = build(&mut TextureBank::new());
        let rooms = camera_rooms();
        assert!(!scene.camera_bounds.is_empty(), "la catedral es solo interior: tiene que limitar la cámara");
        for preset in &scene.presets {
            let mut camera = Camera::new(preset.eye, preset.target, Vec3::new(0.0, 1.0, 0.0)).with_bounds(&scene.camera_bounds);
            camera.max_radius = 10_000.0; // aísla el límite de bounds del límite de radio, que ya existía
            camera.zoom(9_000.0);
            assert!(inside_any_room(camera.eye, &rooms), "{}: se fue del edificio al alejar la cámara: {:?}", preset.name, camera.eye);

            let mut camera = Camera::new(preset.eye, preset.target, Vec3::new(0.0, 1.0, 0.0)).with_bounds(&scene.camera_bounds);
            let steps = 48;
            for i in 0..steps {
                camera.orbit(2.0 * std::f32::consts::PI / steps as f32, 0.0);
                camera.orbit(0.0, 0.3);
                assert!(inside_any_room(camera.eye, &rooms), "{} paso {i}: se fue del edificio al girar: {:?}", preset.name, camera.eye);
            }
        }
    }

    #[test]
    fn every_candle_light_is_in_open_air() {
        let scene = build(&mut TextureBank::new());
        let candles: Vec<Vec3> = scene.lights.iter().filter(|l| !l.directional && l.spot.is_none()).map(|l| l.position).collect();
        assert_eq!(candles.len(), 20, "3 del altar + 10 candelabros + 4 soportes votivos + 3 lámparas colgantes");
        let objects = crate::scene::into_objects(scene.grids, scene.groups);
        let r = Vec3::new(0.05, 0.05, 0.05);
        for p in candles {
            assert!(!objects.iter().any(|o| o.overlaps_box(&(p - r), &(p + r))), "luz adentro de algo sólido en {p:?}");
        }
    }

    #[test]
    fn shell_follows_the_plan() {
        let scene = build(&mut TextureBank::new());
        let grid = &scene.grids[0];
        let solid = |x: isize, y: isize, z: isize| {
            let (cx, cy, cz) = cell(x, y, z);
            grid.is_occupied(cx, cy, cz)
        };
        let bay_center = NAVE_START + BAY + BAY / 2; // segundo tramo de la nave
        let station = NAVE_START + BAY;
        let (arcade, aisle) = ((ARCADE.0 + ARCADE.1) / 2, (ARCADE.1 + AISLE_WALL) / 2);

        assert!(solid(0, bay_center, -1), "piso bajo la nave");
        assert!(!solid(0, 20, 30), "nártex vacío");
        assert!(solid(20, 20, 30), "muro entre el nártex y el vestíbulo");
        assert!(!solid(6, bay_center, 100), "nave vacía casi hasta la clave (al costado del tirante de la lámpara)");
        assert!(solid(0, bay_center, VAULT_CROWN), "cascarón sobre la clave");
        assert!(!solid(aisle, bay_center, 70), "sobre la bóveda lateral ya es afuera");
        assert!(!solid(aisle, bay_center, 30), "nave lateral vacía");
        assert!(!solid(arcade, bay_center, 10), "arco de la arquería abierto hacia la nave lateral");
        assert!(solid(NAVE_HALF - 2, station, 40), "fuste adosado sobre la columna");
        assert!(solid(NAVE_HALF + 1, station, 15), "fuste redondo de la columna");
        let (cx, cy, cz) = cell(NAVE_HALF + 1, station, 15);
        assert_ne!(grid.get(cx as usize, cy as usize, cz as usize), grid.get(cx as usize, cy as usize, (cz - 13) as usize), "el fuste es de otro mármol que la basa");
        assert!(!solid(NAVE_HALF, station + 3, 15), "el muro alrededor de la columna quedó vaciado");
        assert!(!solid(NAVE_HALF + 4, station + 3, 15), "la esquina del pilar cuadrado ya no está");
        assert!(solid(NAVE_HALF - 3, station, 1), "plinto ancho en la base");
        assert!(solid(ARCADE.0, NAVE_START - 2, 15), "el muro del nártex sigue entero junto a la media columna");
        assert!(solid(NAVE_HALF - 2, station, 90), "la bóveda arranca sobre el pilar");
        assert!(!solid(arcade, bay_center, 90), "ventana del claristorio abierta");
        assert!(!solid(NAVE_HALF - 1, bay_center, 105), "luneto sobre la ventana alta");
        assert!(!solid(-arcade, CROSSING_CENTER, 60), "arco alto del crucero hacia el brazo");
        assert!(!solid(-50, CROSSING_CENTER, 30), "brazo norte del crucero vacío");
        assert!(!solid(0, LENGTH - 15, 30), "ábside vacío");
        assert!(!solid(0, NAVE_END, 75), "arco triunfal redondo a la entrada del coro");
        assert!(!solid(NAVE_HALF - 6, NAVE_END, 70), "el arco sigue alto hacia los costados");
        assert!(solid(0, NAVE_END, 80), "moldura en la clave del arco triunfal");
        assert!(solid(17, CHOIR_END + 17, 30), "muro curvo del ábside, a 45°");
        assert!(!solid(24, CHOIR_END + 22, 30), "fuera del semicírculo no hay nada");
        assert!(!solid(0, LENGTH - 1, 30), "derrame exterior del vitral vacío");
        assert!(!solid(0, LENGTH - 4, 30), "el vitral está reventado: el centro del vano quedó abierto");
        let (cx, cy, cz) = cell(-GLASS_WIDTH / 2, LENGTH - 4, GLASS_SILL + 10);
        let shard = grid.get(cx as usize, cy as usize, cz as usize).expect("astilla pegada al marco");
        assert!(shard.albedo[3] > 0.0, "la astilla es vidrio transparente");
        assert!(!solid(0, 38, 60), "arco del nártex a la nave");
        let (cx, cy, cz) = cell(AISLE_WALL + WALL / 2, NAVE_START + AISLE_GLASS_START + 3, AISLE_GLASS.2 + 10);
        let aisle_pane = grid.get(cx as usize, cy as usize, cz as usize).expect("vitral de la nave lateral");
        assert!(aisle_pane.albedo[3] > 0.0, "el vitral de la nave lateral es vidrio");
        assert!(!solid(0, 1, 10), "la puerta principal está rota: el vano atraviesa el muro");
    }

    #[test]
    fn floor_reads_from_door_to_altar() {
        let m = materials::build(&mut TextureBank::new());
        assert_eq!(floor(&m, 0, CROSSING_CENTER), m.floor_gold, "centro del sol, donde están los novios");
        assert_eq!(floor(&m, 0, CROSSING_CENTER + 8), m.floor_gold, "el rayo que apunta al altar");
        assert_eq!(floor(&m, 0, NAVE_START + BAY / 2), m.floor_gold, "centro del rombo del primer tramo");
        assert_eq!(floor(&m, RUNNER_HALF - 1, NAVE_START + 5), m.floor_dark, "borde de la alfombra procesional");
        assert_eq!(floor(&m, 0, NAVE_END), m.floor_gold, "umbral del coro");
        assert_eq!(floor(&m, 0, NAVE_END + 10), m.floor_dark, "presbiterio apartado");
        assert_eq!(floor(&m, 36, NAVE_START + 20), m.floor_pale, "nave lateral lisa");
        assert_eq!(floor(&m, 36, NAVE_START + BAY), m.floor_beige, "banda bajo el límite de tramo");
    }
}

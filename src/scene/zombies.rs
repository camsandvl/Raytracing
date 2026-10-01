//! La invasión, congelada a mitad de movimiento: una pirámide humana en cada abertura
//! (la puerta rota, el vitral del ábside, las ventanas altas, el rosetón y las ventanas
//! del crucero), los que se separaron de ellas y siguen solos, los que bajan por las
//! columnas, caminan o caen, el que tiene a la novia inclinada, mordiéndole el cuello, y
//! los padres de los novios atacados a lo largo de la nave (`feast`).
//!
//! Cada zombi es un puñado de "cápsulas" (segmentos con radio) estampadas en un grid de
//! 5 cm: torso, cabeza, mandíbula, brazos y piernas. Cualquier pose sale de las
//! direcciones de cada tramo, así que las mismas pocas líneas hacen zombis que trepan,
//! caen, se arrastran o empujan. Están desnudos: todo es carne podrida amarillo verdosa
//! con un resplandor tenue, parches de sangre y hueso a la vista (el Hueso con musgo de
//! la rúbrica).
//!
//! Todo en coordenadas de mundo (1 unidad = un vóxel de 20 cm de la catedral).

use crate::materials::MaterialSet;
use crate::ray_intersect::Material;
use crate::rng::Rng;
use crate::scene::builders::hash;
use crate::scene::canvas::Canvas;
use crate::scene::anatomy::{self, Body, Build, Skeleton};
use crate::scene::figure::{canvas_bounds, canvas_for, capsules, frame, leg_to, random_dir, Part, Pose, M, UP};
use crate::scene::people;
use crate::scene::sculpt::{perpendicular, stamp, Layer, Shape};
use crate::scene::skeleton::DETAIL;
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::Vec3;

/// El detalle de la escena de la boda (novios, oficiante y el zombi que la muerde): el
/// doble de fino que el resto de la invasión (2.5 cm en vez de 5), para que la silueta —
/// no el detalle fino — se lea mejor en las vistas de cerca ("mordida", "boda", "altar").
/// Es la única escena a esta resolución: todo lo demás (el resto de los zombis, los
/// props) se queda en `DETAIL`, así el costo extra queda acotado a esta figura chica.
pub const SCULPT_DETAIL: f32 = DETAIL / 2.0;


/// Ruido en manchas de 20 cm, para que la sangre y la podredumbre sean parches grandes y
/// no un moteado (de lejos, el moteado fino se leía como follaje).
fn blotch(p: Vec3, seed: u32) -> f32 {
    let q = p / (DETAIL * 4.0);
    hash(q.x.floor() as isize * 7919 + q.y.floor() as isize * 104_729, q.z.floor() as isize, seed)
}

/// Carne podrida con parches de sangre y de hueso a la vista.
fn flesh_paint(m: &MaterialSet, seed: u32) -> impl Fn(Vec3) -> Material + '_ {
    move |p| match blotch(p, seed) {
        n if n < 0.07 => m.blood,
        n if n < 0.12 => m.bone_moss,
        _ => m.zombie_skin,
    }
}

/// Pose al azar: el torso apunta más o menos a `up_hint`, los brazos se estiran hacia
/// `reach` (garras hacia adelante) y las piernas cuelgan del lado opuesto. `wild`
/// controla cuánto se desordena todo, torso y miembros.
pub fn wild_pose(rng: &mut Rng, pelvis: Vec3, up_hint: Vec3, reach: Vec3, wild: f32) -> Pose {
    let up_dir = up_hint.normalize() + random_dir(rng) * wild;
    let front_hint = reach + random_dir(rng) * wild;
    let (up, front) = frame(up_dir, front_hint);
    let (a, b) = (0.3 + 0.35 * wild, 0.25 + 0.3 * wild);
    let mut limb = |base: Vec3, amount: f32| (base.normalize() + random_dir(rng) * amount).normalize();
    let arms = [(); 2].map(|_| (limb(reach * 0.7 + up * 0.3, a), limb(reach, b)));
    let legs = [(); 2].map(|_| (limb(-up, b), limb(-up - front * 0.3, a)));
    Pose { pelvis, up, front, arms, legs, look: front }
}

/// Un zombi caminando con los pies en `feet` hacia `heading` (horizontal): el paso a
/// medias, rengo, inclinado hacia adelante y con los dos brazos estirados.
pub fn walking(rng: &mut Rng, feet: Vec3, heading: Vec3) -> Pose {
    let heading = heading.normalize();
    let side = UP.cross(&heading).normalize();
    let (up, front) = frame(UP + heading * rng.range(0.1, 0.35) + side * rng.range(-0.2, 0.2), heading);
    let stride = if rng.next_f32() < 0.5 { 1.0 } else { -1.0 };
    let mut jitter = |v: Vec3, amount: f32| (v + random_dir(rng) * amount).normalize();
    let arms = [(); 2].map(|_| (jitter(heading - UP * 0.25, 0.25), jitter(heading - UP * 0.1, 0.2)));
    let legs = [
        (jitter(-UP + heading * (0.35 * stride), 0.08), jitter(-UP + heading * (0.2 * stride), 0.05)),
        (jitter(-UP - heading * (0.3 * stride), 0.08), jitter(-UP - heading * (0.5 * stride), 0.05)),
    ];
    Pose { pelvis: feet + UP * (0.85 * M), up, front, arms, legs, look: front }
}

/// Un zombi aferrado a una columna o a un muro, bajando cabeza abajo: el pecho contra la
/// superficie (`toward` apunta hacia ella), las manos más abajo agarrándose a los
/// costados y las rodillas dobladas arriba.
pub fn climbing(rng: &mut Rng, pelvis: Vec3, toward: Vec3) -> Pose {
    let toward = toward.normalize();
    let (up, front) = frame(-UP - toward * 0.15 + random_dir(rng) * 0.2, toward);
    let side = up.cross(&front).normalize();
    let mut jitter = |v: Vec3| (v + random_dir(rng) * 0.2).normalize();
    let arms = [-1.0, 1.0].map(|s| (jitter(-UP * 0.6 + toward * 0.3 + side * (s * 0.6)), jitter(-UP * 0.8 + toward * 0.5 + side * (s * 0.2))));
    let legs = [-1.0, 1.0].map(|s| (jitter(UP * 0.7 - toward * 0.3 + side * (s * 0.4)), jitter(UP * 0.3 + toward * 0.8)));
    // La cara vuelta hacia la iglesia, mirando hacia abajo: adonde va.
    let look = (-UP * 0.5 - toward * 0.8).normalize();
    Pose { pelvis, up, front, arms, legs, look }
}

/// Estampa un zombi completo. Los ojos y la mandíbula van al final, encima de la cabeza.
fn body(c: &mut Canvas, m: &MaterialSet, pose: &Pose, seed: u32) {
    let flesh = flesh_paint(m, seed);
    let mut parts = capsules(pose);
    parts.sort_by_key(|&(_, _, _, part)| matches!(part, Part::Eye | Part::Jaw));
    for (a, b, r, part) in parts {
        let paint: &dyn Fn(Vec3) -> Material = match part {
            Part::Eye => &|_| m.soot,
            Part::Jaw => &|_| m.blood,
            _ => &flesh,
        };
        c.capsule(a, b, r, paint);
    }
}

/// Un zombi suelto en la pose dada.
pub fn single(m: &MaterialSet, pose: &Pose, seed: u32) -> VoxelGrid {
    let mut c = canvas_for(std::slice::from_ref(pose), &[]);
    body(&mut c, m, pose, seed);
    c.into_grid()
}

/// Una abertura por la que entra la invasión. `bottom` es el centro del borde inferior
/// en la cara interior del muro; `along` corre a lo largo del muro y `inward` apunta hacia
/// adentro de la iglesia.
pub struct Opening {
    pub bottom: Vec3,
    pub along: Vec3,
    pub inward: Vec3,
    /// Ancho y alto del vano, en metros. Con `round`, el vano es un círculo de diámetro
    /// `width` (el rosetón).
    pub width: f32,
    pub height: f32,
    pub round: bool,
    /// Altura de `bottom` sobre el piso, en metros.
    pub sill: f32,
    /// Filas de la pirámide en el vano (ver `pyramid`).
    pub stack: usize,
    /// Los que se separaron de la pirámide y bajan solos por el muro.
    pub loose: usize,
    /// Lo que quedó tirado en el piso (ver `Wreckage`).
    pub debris: Debris,
}

/// Lo que quedó tirado entre los cuerpos.
#[derive(Clone, Copy, PartialEq)]
pub enum Debris {
    None,
    /// Las hojas rotas de la puerta (roble y herrajes).
    Door,
    /// Astillas del vitral, de todos sus colores, desparramadas en el piso.
    Glass,
}

/// Medidas de un zombi en cuatro patas dentro de una pirámide, en metros: separación a lo
/// ancho y alto de una fila (su espalda).
const WIDE: f32 = 0.62;
const TIER: f32 = 0.58;

/// Un zombi en cuatro patas sobre `support` (el punto bajo la pelvis), avanzando hacia
/// `heading` con la cara levantada: los ladrillos de las pirámides.
pub fn crawling(rng: &mut Rng, support: Vec3, heading: Vec3) -> Pose {
    let heading = heading.normalize();
    let (up, front) = frame(heading + UP * rng.range(0.15, 0.35) + random_dir(rng) * 0.08, -UP);
    let side = up.cross(&front).normalize();
    let mut jitter = |v: Vec3, amount: f32| (v + random_dir(rng) * amount).normalize();
    let arms = [-1.0f32, 1.0].map(|s| {
        let d = jitter(-UP + heading * 0.15 + side * (s * 0.1), 0.1);
        (d, jitter(d, 0.1))
    });
    let legs = [(); 2].map(|_| (jitter(-UP - heading * 0.05, 0.05), jitter(-heading - UP * 0.1, 0.1)));
    let look = jitter(heading + UP * 0.5, 0.2);
    Pose { pelvis: support + UP * (0.48 * M), up, front, arms, legs, look }
}

/// Una pirámide humana en el vano de una ventana, armada cuerpo por cuerpo y no al azar:
/// `o.stack` filas de zombis en cuatro patas que se achican hacia arriba (4-3-2-1, o
/// recortadas al círculo del rosetón), cada una apoyada en las espaldas de la de abajo,
/// la de adelante asomada por encima del alféizar. Devuelve la pirámide y, aparte, cada
/// uno de los `o.loose` que se separaron de ella y bajan solos por el muro, cabeza abajo,
/// escalonados y bien espaciados para que se lea cada uno.
pub fn pyramid(m: &MaterialSet, o: &Opening, seed: u32) -> Vec<VoxelGrid> {
    let mut rng = Rng::new(seed);
    let in_frame = |a: f32, u: f32, d: f32| o.bottom + (o.along * a + UP * u + o.inward * d) * M;
    let span = |u: f32| if o.round { 2.0 * ((o.width / 2.0).powi(2) - (u - o.width / 2.0).powi(2)).max(0.0).sqrt() } else { o.width };
    let first = (span(0.4) / WIDE) as usize;
    let mut poses = Vec::new();
    for k in 0..o.stack {
        let u = k as f32 * TIER;
        let n = first.saturating_sub(k).min((span(u + 0.4) / WIDE) as usize);
        if n == 0 || u + 0.6 > o.height {
            break;
        }
        for i in 0..n {
            let a = (i as f32 - (n as f32 - 1.0) / 2.0) * WIDE + rng.range(-0.06, 0.06);
            let heading = o.inward + o.along * rng.range(-0.15, 0.15);
            poses.push(crawling(&mut rng, in_frame(a, u, 0.15 - 0.55 * k as f32), heading));
        }
    }
    let loose: Vec<Pose> = (0..o.loose)
        .map(|i| {
            let t = i as f32 - (o.loose as f32 - 1.0) / 2.0;
            let a = t * (o.width / o.loose as f32).max(0.9) + rng.range(-0.2, 0.2);
            let u = -(1.1 + 1.3 * (i % 3) as f32 + rng.range(0.0, 0.4));
            climbing(&mut rng, in_frame(a, u, 0.22), -o.inward)
        })
        .collect();

    let mut c = canvas_for(&poses, &[]);
    for (i, pose) in poses.iter().enumerate() {
        body(&mut c, m, pose, seed.wrapping_add(i as u32 * 31));
    }
    let mut grids = vec![c.into_grid()];
    grids.extend(loose.iter().enumerate().map(|(i, pose)| single(m, pose, seed.wrapping_add(7919 + i as u32))));
    grids
}

/// Lo que quedó tirado en el piso delante de una abertura, hasta `reach` metros hacia
/// adentro: las hojas rotas de la puerta o las astillas del vitral (`o.debris`).
struct Wreckage {
    boards: Vec<(Vec3, Vec3)>,
    shards: Vec<(Vec3, Vec3)>,
}

const BOARD_RADIUS: f32 = 0.09 * M;
const SHARD_RADIUS: f32 = 0.03 * M;

impl Wreckage {
    fn new(o: &Opening, rng: &mut Rng, reach: f32) -> Self {
        let floor = o.bottom - UP * (o.sill * M);
        let on_floor = |a: f32, h: f32, d: f32| floor + (o.along * a + UP * h + o.inward * d) * M;
        let boards = (0..if o.debris == Debris::Door { 4 } else { 0 })
            .map(|_| {
                let base = on_floor(rng.range(-2.5, 2.5), 0.1, rng.range(0.4, reach + 0.3));
                let dir = (o.along * rng.range(-1.0, 1.0) + o.inward * rng.range(-0.5, 0.5) + UP * rng.range(0.05, 0.3)).normalize();
                (base, base + dir * (rng.range(1.2, 2.0) * M))
            })
            .collect();
        let shards = (0..if o.debris == Debris::Glass { 70 } else { 0 })
            .map(|_| {
                let p = on_floor(rng.range(-2.5, 2.5), 0.02, reach * rng.next_f32().sqrt() + 0.3);
                let dir = (o.along * rng.range(-1.0, 1.0) + o.inward * rng.range(-1.0, 1.0)).normalize();
                (p, p + dir * (rng.range(0.04, 0.14) * M))
            })
            .collect();
        Wreckage { boards, shards }
    }

    /// Los extremos de cada pieza con su radio, para ajustar el canvas.
    fn bounds(&self) -> Vec<(Vec3, f32)> {
        let ends = |pieces: &[(Vec3, Vec3)], r: f32| pieces.iter().flat_map(|&(a, b)| [(a, r), (b, r)]).collect::<Vec<_>>();
        [ends(&self.boards, BOARD_RADIUS), ends(&self.shards, SHARD_RADIUS)].concat()
    }

    fn stamp(self, c: &mut Canvas, m: &MaterialSet) {
        for (a, b) in self.boards {
            c.capsule(a, b, BOARD_RADIUS, &|p| if blotch(p, 77) < 0.85 { m.wood } else { m.iron });
        }
        let glass = [m.glass_cobalt, m.glass_cobalt, m.glass_ruby, m.glass_amber, m.glass_pale, m.iron];
        for (i, (a, b)) in self.shards.into_iter().enumerate() {
            c.capsule(a, b, SHARD_RADIUS, &|_| glass[i % glass.len()]);
        }
    }
}

/// Dos colgados de las jambas rotas de un vano, a `heights` metros sobre el alféizar: las
/// dos manos enganchadas en el canto, una sobre otra (los brazos llegan con la IK de dos
/// tramos), el cuerpo colgando contra el muro al costado del vano, las piernas en el aire
/// y la cara hacia la iglesia.
pub fn hanging(m: &MaterialSet, o: &Opening, heights: [f32; 2], seed: u32) -> Vec<VoxelGrid> {
    let mut rng = Rng::new(seed);
    let at = |a: f32, u: f32, d: f32| o.bottom + (o.along * a + UP * u + o.inward * d) * M;
    [-1.0f32, 1.0]
        .into_iter()
        .zip(heights)
        .enumerate()
        .map(|(i, (s, u))| {
            let grip = at(s * (o.width / 2.0 - 0.08), u, 0.05);
            let (up, front) = frame(UP + o.along * (s * 0.12) + random_dir(&mut rng) * 0.05, o.inward);
            let side = up.cross(&front).normalize();
            let neck = grip - UP * (0.5 * M) + o.along * (s * 0.3 * M) + o.inward * (0.15 * M);
            let pelvis = neck - up * (0.55 * M);
            let arms = [-1.0f32, 1.0].map(|k| {
                let shoulder = neck + side * (k * 0.2 * M) - up * (0.04 * M);
                leg_to(shoulder, grip + UP * (k * 0.1 * M), 0.29 * M, o.inward)
            });
            let kick = rng.range(0.1, 0.5);
            let legs = [((-UP + side * 0.15 + o.inward * 0.1).normalize(), (-UP - o.inward * 0.15).normalize()), ((-UP - side * 0.15 + o.inward * kick).normalize(), (-UP + o.inward * 0.1).normalize())];
            let pose = Pose { pelvis, up, front, arms, legs, look: (o.inward - UP * 0.3).normalize() };
            single(m, &pose, seed.wrapping_add(i as u32 * 131))
        })
        .collect()
}

/// Una ventana que revienta y se vacía hacia adentro siguiendo la gravedad (las del
/// crucero, el vitral del ábside), en un solo instante:
/// - pocos quedan en el vano, trepando por encima del alféizar, asomados hacia abajo;
/// - la mayoría está en el aire, cada uno en un momento distinto de su caída, empujado
///   desde el borde con algo de velocidad hacia adentro: se amontonan cerca de la ventana,
///   donde todavía van lentos, y se abren a medida que caen;
/// - los que ya llegaron quedan rotos en el piso, uno sobre otro donde terminan las
///   parábolas, con la sangre saltando del golpe.
///
/// `airborne` y `landed` son cuántos hay en el aire y en el piso; `spread` es entre qué
/// posiciones a lo largo del vano (metros) caen los del piso, para no tapar lo que haya
/// al pie del muro. `outside` es la montaña de afuera por la que suben hasta la ventana
/// (cuerpos, metros de largo y metros por encima del alféizar), apoyada en el suelo del
/// otro lado del muro: se ve por el vano, recortada contra la luna. Todo en un solo grid, apoyado en el piso (para las
/// salpicaduras), con lo que haya quedado tirado (`o.debris`).
#[allow(clippy::too_many_arguments)]
pub fn torrent(m: &MaterialSet, o: &Opening, airborne: usize, landed: usize, spread: (f32, f32), outside: (usize, f32, f32), seed: u32) -> VoxelGrid {
    let mut rng = Rng::new(seed);
    let at = |a: f32, u: f32, d: f32| o.bottom + (o.along * a + UP * u + o.inward * d) * M;
    let floor = o.bottom - UP * (o.sill * M);
    let half = o.width / 2.0 - 0.3;
    let mut poses = Vec::new();

    for i in 0..4 {
        let a = (i as f32 - 1.5) * (o.width / 4.0) + rng.range(-0.1, 0.1);
        let heading = o.inward - UP * 0.5 + o.along * rng.range(-0.2, 0.2);
        poses.push(crawling(&mut rng, at(a, 0.0, -0.35), heading));
    }

    // Afuera, la montaña por la que suben: más alta contra el muro, bajando hasta el suelo.
    // El muro tiene 0.8 m de espesor.
    let (count, depth, top) = outside;
    for _ in 0..count {
        let x = rng.next_f32().powf(0.7);
        let d = -0.9 - depth * x;
        let u = (o.sill + top) * (1.0 - x) * rng.next_f32().sqrt() - o.sill;
        let pelvis = at(rng.range(-o.width / 2.0 - 0.8 * x, o.width / 2.0 + 0.8 * x), u, d);
        poses.push(wild_pose(&mut rng, pelvis, UP * 0.6 + o.inward * 0.4, o.inward + UP * 0.5, 0.35));
    }

    // En el aire, hasta que la pelvis queda a 1.2 m del piso. Empujados desde el borde a
    // `PUSH` m/s hacia adentro: los del piso caen donde terminan esas parábolas.
    const PUSH: (f32, f32) = (1.2, 2.8);
    let last = ((o.sill - 1.2).max(0.2) / 4.9).sqrt();
    let land = (o.sill / 4.9).sqrt();
    for k in 0..airborne {
        let t = (k as f32 + rng.range(0.2, 0.8)) / airborne as f32 * last;
        let (push, lift) = (rng.range(PUSH.0, PUSH.1), rng.range(0.0, 0.8));
        let a = (rng.range(-half, half) + rng.range(-0.5, 0.5) * t).clamp(-half, half);
        poses.push(falling(&mut rng, at(a, 0.3 + lift * t - 4.9 * t * t, 0.35 + push * t)));
    }

    // En el piso, donde terminan las parábolas: una capa y unos pocos encima.
    let reach = (0.35 + PUSH.0 * land, 0.35 + PUSH.1 * land);
    let mut spots = Vec::new();
    for k in 0..landed {
        let spot = floor + (o.along * rng.range(spread.0, spread.1) + o.inward * rng.range(reach.0, reach.1)) * M;
        let height = if k < landed * 2 / 3 { 0.15 } else { 0.45 };
        let angle = rng.range(0.0, std::f32::consts::TAU);
        let lying = o.along * angle.cos() + o.inward * angle.sin();
        let arms = Vec3::new(rng.range(-1.0, 1.0), -0.6, rng.range(-1.0, 1.0));
        poses.push(wild_pose(&mut rng, spot + UP * (height * M), lying, arms, 0.8));
        spots.push(spot);
    }
    let wreckage = Wreckage::new(o, &mut rng, reach.1);
    finish(m, &poses, &spots, wreckage, floor.y, seed)
}

/// Estampa en un grid apoyado en el piso (a la altura `floor_y`) los cuerpos, las
/// salpicaduras de sangre en `splashes` y lo que quedó tirado.
fn finish(m: &MaterialSet, poses: &[Pose], splashes: &[Vec3], wreckage: Wreckage, floor_y: f32, seed: u32) -> VoxelGrid {
    let mut rng = Rng::new(seed ^ 0x5eed);
    let around = Vec3::new(1.5, 0.0, 1.5) * M;
    let mut extra: Vec<(Vec3, f32)> = splashes.iter().flat_map(|&p| [(p - around, 0.0), (p + around + UP * (0.8 * M), 0.0)]).collect();
    extra.extend(wreckage.bounds());
    let (mut min, max) = canvas_bounds(poses, &extra);
    min.y = floor_y;
    let mut c = Canvas::new(min, max, DETAIL);
    for &spot in splashes {
        splash(&mut c, m, spot, 0.8, &mut rng);
    }
    for (i, pose) in poses.iter().enumerate() {
        body(&mut c, m, pose, seed.wrapping_add(i as u32 * 31));
    }
    wreckage.stamp(&mut c, m);
    c.into_grid()
}

/// Uno que tropieza hacia adelante y está cayendo: pivotea sobre el pie que le queda en el
/// piso (`feet`), con el cuerpo inclinado `angle` radianes desde la vertical hacia
/// `heading`, el otro pie levantado atrás y los brazos adelante y abajo para parar el
/// golpe.
fn tripping(rng: &mut Rng, feet: Vec3, heading: Vec3, angle: f32) -> Pose {
    let heading = heading.normalize();
    let lean = |a: f32| UP * a.cos() + heading * a.sin();
    let (up, front) = frame(lean(angle) + random_dir(rng) * 0.05, heading - UP * angle.sin());
    let side = up.cross(&front).normalize();
    let pelvis = feet + lean(angle * 0.8) * (0.84 * M);
    let arms = [-1.0f32, 1.0].map(|k| {
        let d = (heading * 0.6 - UP * 0.8 + side * (k * 0.25) + random_dir(rng) * 0.1).normalize();
        (d, d)
    });
    let behind = feet - heading * (0.35 * M) + UP * (0.25 * M);
    let legs = [feet, behind].map(|foot| leg_to(pelvis, foot, 0.42 * M, heading));
    Pose { pelvis, up, front, arms, legs, look: (heading - UP * 0.3).normalize() }
}

/// La puerta rota, con la gravedad de una estampida y no de una caída: nadie cae de
/// alto, se aplastan. Una multitud apretada empuja en el vano; los de adelante tropiezan y
/// se van de boca, a mitad de la caída, con los brazos para parar el golpe, y alguno se
/// tira de cabeza por encima de los caídos; los que ya cayeron quedan boca abajo,
/// pisoteados, algunos encima de otros, y unos pocos se arrastran para salir de abajo.
/// El montón queda bajo y se extiende hacia adentro, como se derrama una multitud, con la
/// sangre saltando donde golpean y las hojas rotas de la puerta entre los cuerpos.
pub fn surge(m: &MaterialSet, o: &Opening, seed: u32) -> VoxelGrid {
    let mut rng = Rng::new(seed);
    let floor = o.bottom - UP * (o.sill * M);
    let on_floor = |a: f32, d: f32| floor + (o.along * a + o.inward * d) * M;
    let heading = |rng: &mut Rng| (o.inward + o.along * rng.range(-0.3, 0.3)).normalize();
    let half = o.width / 2.0 - 0.25;
    let mut poses = Vec::new();

    // La multitud en el vano y afuera, empujando.
    for (d, n) in [(-2.1, 6), (-1.5, 6), (-0.9, 6), (-0.3, 6)] {
        for i in 0..n {
            let a = (i as f32 - (n as f32 - 1.0) / 2.0) * (o.width / n as f32) + rng.range(-0.1, 0.1);
            let h = heading(&mut rng);
            let feet = on_floor(a, d + rng.range(-0.1, 0.1));
            poses.push(walking(&mut rng, feet, h));
        }
    }
    // Los de adelante, tropezando: más inclinados cuanto más adentro.
    for k in 0..10 {
        let d = 0.4 + 2.2 * k as f32 / 10.0 + rng.range(0.0, 0.3);
        let angle = (0.35 + 0.35 * d).min(1.3) + rng.range(-0.15, 0.15);
        let (a, h) = (rng.range(-half, half), heading(&mut rng));
        poses.push(tripping(&mut rng, on_floor(a, d), h, angle));
    }
    // Dos tirándose de cabeza por encima de los caídos.
    for a in [-0.8f32, 0.9] {
        let h = heading(&mut rng);
        let pelvis = on_floor(a, rng.range(2.0, 2.8)) + UP * (0.8 * M);
        poses.push(wild_pose(&mut rng, pelvis, h - UP * 0.2, h - UP * 0.5, 0.3));
    }
    // Los caídos: boca abajo, una capa y algunos encima; y unos pocos arrastrándose.
    let mut spots = Vec::new();
    for k in 0..15 {
        let d = rng.range(0.8, 4.2);
        let a = rng.range(-half, half) * if d > 3.4 { 0.8 } else { 1.0 };
        let spot = on_floor(a, d);
        let h = heading(&mut rng);
        let height = if k < 11 { 0.14 } else { 0.42 };
        poses.push(wild_pose(&mut rng, spot + UP * (height * M), h, -UP * 0.8 + h * 0.2, 0.4));
        if k % 5 == 0 {
            spots.push(spot + h * (0.7 * M));
        }
    }
    for _ in 0..4 {
        let (a, d, h) = (rng.range(-half, half), rng.range(1.5, 3.5), heading(&mut rng));
        poses.push(crawling(&mut rng, on_floor(a, d) + UP * (0.2 * M), h));
    }
    let wreckage = Wreckage::new(o, &mut rng, 3.8);
    finish(m, &poses, &spots, wreckage, floor.y, seed)
}

/// El vidrio de un vano en el instante en que revienta: `count` astillas planas, grandes
/// (10 a 28 cm) y de un solo color cada una, casi todas del campo de cobalto y algunas del
/// borde de rubí, congeladas en el aire. Cada una sale del vano entre las alturas `from`
/// (metros sobre `bottom`) a una velocidad hacia adentro entre `speed` (m/s) y sigue su
/// parábola un instante. Tienen el resplandor tenue de los vitrales laterales: vidrio
/// transparente contra la noche se veía negro.
pub fn shard_burst(m: &MaterialSet, o: &Opening, from: (f32, f32), count: usize, speed: (f32, f32), seed: u32) -> VoxelGrid {
    let mut rng = Rng::new(seed);
    let at = |a: f32, u: f32| o.bottom + (o.along * a + UP * u) * M;
    const HALF_THICKNESS: f32 = 0.03 * M;
    let shards: Vec<([Vec3; 3], Material)> = (0..count)
        .map(|_| {
            let origin = at(rng.range(-o.width / 2.0 + 0.2, o.width / 2.0 - 0.2), rng.range(from.0, from.1));
            let velocity = o.inward * rng.range(speed.0, speed.1) + o.along * rng.range(-1.2, 1.2) + UP * rng.range(-0.3, 1.2);
            let t = rng.range(0.1, 0.7);
            let center = origin + (velocity * t - UP * (4.9 * t * t)) * M;
            // Un triángulo irregular en un plano al azar (la astilla va girando).
            let normal = random_dir(&mut rng);
            let e1 = normal.cross(&random_dir(&mut rng)).normalize();
            let e2 = normal.cross(&e1);
            let size = rng.range(0.1, 0.28) * M;
            let mut angle = rng.range(0.0, std::f32::consts::TAU);
            let corners = [(); 3].map(|_| {
                let corner = center + (e1 * angle.cos() + e2 * angle.sin()) * (size * rng.range(0.6, 1.2));
                angle += rng.range(1.7, 2.4);
                corner
            });
            let color = match rng.next_f32() {
                n if n < 0.55 => m.glass_cobalt,
                n if n < 0.8 => m.glass_ruby,
                n if n < 0.9 => m.glass_amber,
                _ => m.glass_pale,
            };
            (corners, color.with_glow(0.9))
        })
        .collect();
    let bounds: Vec<(Vec3, f32)> = shards.iter().flat_map(|(corners, _)| corners.map(|p| (p, HALF_THICKNESS))).collect();
    let mut c = canvas_for(&[], &bounds);
    for (corners, color) in shards {
        c.triangle(corners, HALF_THICKNESS, &|_| color);
    }
    c.into_grid()
}

/// Los que se caen de una ventana alta, en el instante: el primero volcándose por encima
/// del alféizar, cabeza abajo, con la cadera todavía en el borde, y los demás ya en el
/// aire, más abajo y más adentro cuanto más cayeron (la parábola de quien sale empujado
/// del vano), así se ve de dónde vienen.
pub fn falling_from(m: &MaterialSet, o: &Opening, count: usize, seed: u32) -> Vec<VoxelGrid> {
    let mut rng = Rng::new(seed);
    let at = |a: f32, u: f32, d: f32| o.bottom + (o.along * a + UP * u + o.inward * d) * M;
    (0..count)
        .map(|i| {
            let a = rng.range(-o.width / 2.0 + 0.3, o.width / 2.0 - 0.3);
            let pose = if i == 0 {
                wild_pose(&mut rng, at(a, 0.35, 0.25), o.inward * 0.5 - UP * 0.9, -UP + o.inward * 0.3, 0.3)
            } else {
                let drop = 1.5 + 2.8 * (i - 1) as f32 + rng.range(0.0, 1.0);
                let out = 0.5 + 1.6 * (2.0 * drop / 9.8).sqrt() + rng.range(0.0, 0.4);
                falling(&mut rng, at(a, -drop, out))
            };
            single(m, &pose, seed.wrapping_add(i as u32 * 131))
        })
        .collect()
}

fn flat(angle: f32) -> Vec3 {
    Vec3::new(angle.cos(), 0.0, angle.sin())
}

/// Sangre que acaba de golpear el piso en `center` (sobre el piso), congelada en el
/// instante del impacto: una mancha central de borde irregular con chorros que salen en
/// estrella y terminan en una gota, una corona de puntas que se levantan alrededor, y
/// gotas todavía en el aire, cada una estirada en la dirección en la que vuela. `scale`
/// agranda o achica todo (1 = el golpe de un cuerpo).
fn splash(c: &mut Canvas, m: &MaterialSet, center: Vec3, scale: f32, rng: &mut Rng) {
    use std::f32::consts::{PI, TAU};
    let core = 0.28 * scale * M;
    let rays: Vec<(f32, f32)> = (0..14).map(|_| (rng.range(0.0, TAU), rng.range(0.4, 1.1) * scale * M)).collect();
    let wobble: [f32; 8] = std::array::from_fn(|_| rng.range(0.75, 1.25));
    c.floor_layer(center, 1.3 * scale * M, m.blood, &|p| {
        let v = Vec3::new(p.x - center.x, 0.0, p.z - center.z);
        let (r, angle) = (v.magnitude(), v.z.atan2(v.x));
        let lobe = wobble[((angle + PI) / TAU * 8.0) as usize % 8];
        r < core * lobe
            || rays.iter().any(|&(a, len)| {
                let off = (angle - a + PI).rem_euclid(TAU) - PI;
                let width = 0.07 * scale * M * (1.0 - r / len).max(0.0) + 0.02 * M;
                let tip = center + flat(a) * (len + 0.07 * scale * M);
                (r < len && off.abs() * r < width) || Vec3::new(p.x - tip.x, 0.0, p.z - tip.z).magnitude() < 0.05 * scale * M + 0.02 * M
            })
    });
    // La corona: puntas que se levantan desde el borde de la mancha, con una gota suelta
    // un poco más arriba de cada una.
    for i in 0..16 {
        let dir = flat(i as f32 / 16.0 * TAU + rng.range(-0.15, 0.15));
        let base = center + dir * core * 0.9 + UP * (0.02 * M);
        let tip = base + (dir * 0.1 + UP * rng.range(0.08, 0.22)) * (scale * M);
        c.capsule(base, tip, 0.03 * M, &|_| m.blood);
        let drop = tip + (dir * 0.05 + UP * rng.range(0.03, 0.08)) * (scale * M);
        c.capsule(drop, drop, 0.03 * M, &|_| m.blood);
    }
    // Gotas en vuelo, sobre parábolas que salen del centro.
    for _ in 0..30 {
        let dir = flat(rng.range(0.0, TAU));
        let (range, top) = (rng.range(0.8, 1.5) * scale, rng.range(0.25, 0.6) * scale);
        let t = rng.range(0.15, 0.85);
        let at = |t: f32| center + (dir * (range * t) + UP * (4.0 * top * t * (1.0 - t))) * M;
        c.capsule(at(t - 0.05), at(t), rng.range(0.025, 0.04) * M, &|_| m.blood);
    }
}

/// Un zombi que cae desde las ventanas altas, a mitad de la caída: los brazos y las piernas
/// sueltos, en cualquier dirección.
pub fn falling(rng: &mut Rng, pelvis: Vec3) -> Pose {
    let up = random_dir(rng) - UP * 0.5; // más bien cabeza abajo
    let reach = random_dir(rng);
    wild_pose(rng, pelvis, up, reach, 0.6)
}

/// Sangre recién salpicada contra el piso en `spot`, sin cuerpo (ver `splash`).
pub fn splatter(m: &MaterialSet, spot: Vec3, scale: f32, seed: u32) -> VoxelGrid {
    let mut rng = Rng::new(seed);
    let around = Vec3::new(1.6, 0.0, 1.6) * (scale * M);
    let mut c = Canvas::new(spot - around, spot + around + UP * (0.8 * scale * M), DETAIL);
    splash(&mut c, m, spot, scale, &mut rng);
    c.into_grid()
}

/// Uno que ya cayó: el cuerpo roto contra el piso en `spot` y la sangre saltando del golpe.
pub fn impact(m: &MaterialSet, spot: Vec3, seed: u32) -> VoxelGrid {
    let mut rng = Rng::new(seed);
    let lying = Vec3::new(rng.range(-1.0, 1.0), 0.0, rng.range(-1.0, 1.0));
    let reach = Vec3::new(rng.range(-1.0, 1.0), -0.6, rng.range(-1.0, 1.0));
    let pose = wild_pose(&mut rng, spot + UP * (0.14 * M), lying, reach, 0.8);
    let around = Vec3::new(1.6, 0.0, 1.6) * M;
    let (mut min, max) = canvas_bounds(std::slice::from_ref(&pose), &[(spot - around, 0.0), (spot + around + UP * (0.8 * M), 0.0)]);
    min.y = spot.y;
    let mut c = Canvas::new(min, max, DETAIL);
    splash(&mut c, m, spot, 1.0, &mut rng);
    body(&mut c, m, &pose, seed);
    c.into_grid()
}

/// Como el novio que inclina a la novia hacia atrás para besarla, pero es un zombi y le
/// muerde el cuello. `feet` son los pies de la novia; ella cae hacia `back`, con la cara
/// girada hacia `-side` (hacia la nave). El zombi está de pie, erguido, detrás del cuello
/// de ella y de frente a la nave, sosteniéndola como en el paso de baile de una boda, con
/// la boca en el costado de su cuello, un brazo por debajo de su espalda y el otro agarrándole la cintura; ella le
/// agarra esa muñeca. Es el mismo zombi desnudo y amarillento de la invasión, pero
/// esculpido (`anatomy.rs`) y más podrido: costillas y columna marcadas, el vientre
/// abierto con las tripas colgando, cuencas vacías, sin nariz, sin labios, los dientes al
/// aire. De la herida la sangre baja por el corpiño y gotea hasta el piso, donde salpica.
pub fn dip(m: &MaterialSet, feet: Vec3, back: Vec3, seed: u32) -> VoxelGrid {
    let mut rng = Rng::new(seed);
    let back = back.normalize();
    let side = UP.cross(&back).normalize(); // del lado del zombi, lejos de la nave
    let corners = [feet + back * (1.4 * M), feet - back * (1.0 * M), feet + side * (1.1 * M), feet - side * (1.0 * M)];
    let (mut min, mut max) = corners.iter().fold((Vec3::repeat(f32::INFINITY), Vec3::repeat(f32::NEG_INFINITY)), |(lo, hi), p| (lo.inf(p), hi.sup(p)));
    min.y = feet.y;
    max.y = feet.y + 1.8 * M;
    let mut c = Canvas::new(min, max, SCULPT_DETAIL);

    let mut bride = people::bride_skeleton(feet, back, side);
    let points = people::bride_dip_points(&bride);

    let (zombie, grip) = dip_zombie(&bride, &points, feet, side);
    let k = zombie.k;

    // Ella le agarra la muñeca que la tiene de la cintura, tratando de soltarse: el brazo
    // baja por su costado, sin taparle la cara.
    let hold = zombie.wrists[grip] - (zombie.wrists[grip] - zombie.elbows[grip]).normalize() * (0.05 * k) - side * (0.03 * k);
    bride.reach_arm(people::BRIDE_GRIP, hold, -side - back * 0.3);
    bride.palms[people::BRIDE_GRIP] = zombie.wrists[grip] - bride.wrists[people::BRIDE_GRIP];

    let legs = anatomy::body(&zombie);
    let pressed: Vec<Shape> = (0..2).flat_map(|i| legs.thighs[i].iter().chain(&legs.shins[i]).chain(&legs.feet[i]).map(|s| s.grow(0.025 * k)).collect::<Vec<_>>()).collect();
    people::dipped_bride(&mut c, m, &bride, feet, back, &pressed);
    sculpted_zombie(&mut c, m, &zombie, seed);

    // La herida y la sangre: por el corpiño hacia la cintura (que queda más abajo que el
    // cuello), y gotas que caen del cuello al piso, cortadas en tramos, del lado de atrás
    // (así no tapan la cara de ella).
    c.capsule(points.bite, points.bite, 0.06 * M, &|_| m.blood);
    for s in [-0.6f32, 0.0, 0.7] {
        c.capsule(points.bite, points.waist + side * (s * 0.06 * M), 0.035 * M, &|_| m.blood);
    }
    for k in 0..3 {
        let top = points.bite + side * ((0.06 + k as f32 * 0.04) * M) + back * ((k as f32 - 1.0) * 0.04 * M); // del lado de atrás del cuello
        let mut y = top.y - 0.05 * M;
        let mut piece = 0.25 * M;
        while y > feet.y + 0.1 * M {
            let bottom = (y - piece).max(feet.y + 0.05 * M);
            c.capsule(Vec3::new(top.x, y, top.z), Vec3::new(top.x, bottom, top.z), 0.03 * M, &|_| m.blood);
            y = bottom - rng.range(0.06, 0.14) * M;
            piece = rng.range(0.04, 0.1) * M;
        }
        splash(&mut c, m, Vec3::new(top.x, feet.y, top.z), 0.35, &mut rng);
    }
    people::sculpted(c)
}

/// El zombi de la mordida (ver `dip`), con la boca en `points.bite`. Devuelve también
/// cuál de sus brazos agarra la cintura de la novia.
fn dip_zombie(bride: &Skeleton, points: &people::BrideDip, feet: Vec3, side: Vec3) -> (Skeleton, usize) {
    // De pie, erguido, sosteniéndola como el novio que inclina a la novia en el baile: un
    // brazo por debajo de la espalda y otro en la cintura, parado detrás del cuello de ella
    // (del lado de `side`), de frente a la nave, con la cabeza agachada mordiéndola. Así,
    // desde la nave, el pecho y los hombros le asoman por encima de la cabeza de ella y
    // las piernas por debajo de su cuerpo, y no le tapa la cara (que queda delante).
    let spot = points.bite + side * (0.55 * M);
    let toward = Vec3::new(points.bite.x - spot.x, 0.0, points.bite.z - spot.z).normalize(); // de él hacia el cuello de ella
    let build = |lean: f32| {
        let up = (UP * lean.to_radians().cos() + toward * lean.to_radians().sin()).normalize();
        let front = perpendicular(-UP, up, toward);
        let look = (toward * 0.5 - UP * 0.8).normalize();
        let pose = Pose { pelvis: Vec3::zeros(), up, front, arms: [(-UP, -UP); 2], legs: [(-UP, -UP); 2], look };
        let mut zombie = Skeleton::new(&pose, 1.8, Build::Corpse);
        zombie.turn_head(look, up);
        let mouth = anatomy::face(&zombie).mouth;
        zombie.shift(points.bite - mouth + side * (0.01 * M));
        zombie
    };
    let standing = feet.y + 0.81 * M;
    let lean = (20..=80).map(|d| d as f32).min_by(|&a, &b| (build(a).pelvis.y - standing).abs().total_cmp(&(build(b).pelvis.y - standing).abs())).unwrap_or(50.0);
    let mut zombie = build(lean);
    let k = zombie.k;

    // Las piernas en una estocada: la de adelante bajo la pelvis, la de atrás estirada
    // hacia atrás, los dos pies en el piso (dentro del ruedo de la falda, que se abre
    // alrededor de sus piernas: ver `dipped_bride`).
    let behind = -toward;
    for i in 0..2 {
        let under = Vec3::new(zombie.hips[i].x, feet.y + 0.075 * k, zombie.hips[i].z);
        let s = if i == 0 { -1.0 } else { 1.0 };
        let front_leg = (zombie.hips[i] - zombie.hips[1 - i]).dot(&toward) > 0.0;
        let mut ankle = under + zombie.side * (s * 0.05 * k) + behind * ((if front_leg { 0.02 } else { 0.3 }) * k);
        let reach = 0.87 * k;
        for _ in 0..60 {
            let pull = Vec3::new(zombie.hips[i].x - ankle.x, 0.0, zombie.hips[i].z - ankle.z);
            if (ankle - zombie.hips[i]).magnitude() <= reach || pull.magnitude() < 0.01 * M {
                break;
            }
            ankle += pull.normalize() * (0.01 * M);
        }
        zombie.reach_leg(i, ankle, toward + UP * 0.1);
    }
    zombie.toes = [toward, toward];
    // Los brazos: el más cercano a la cabeza de ella por debajo de su espalda, el otro a
    // su cintura.
    let toward_head = (zombie.shoulders[0] - bride.head).magnitude() < (zombie.shoulders[1] - bride.head).magnitude();
    let (under, grip) = if toward_head { (0, 1) } else { (1, 0) };
    zombie.reach_arm(under, bride.at(0.34, -0.14, 0.02), side + UP * 0.3);
    zombie.palms[under] = bride.at(0.34, 0.0, 0.0) - zombie.wrists[under];
    zombie.reach_arm(grip, bride.at(0.13, 0.07, 0.06), side + UP * 0.6);
    zombie.palms[grip] = bride.pelvis - zombie.wrists[grip];

    (zombie, grip)
}

/// Carne podrida al detalle de la escultura: los parches grandes de sangre y hueso de
/// toda la invasión, más llagas chicas y zonas de carne oscura y pasada.
fn rotten_paint(m: &MaterialSet, seed: u32) -> impl Fn(Vec3) -> Material + '_ {
    move |p| {
        let fine = p / (SCULPT_DETAIL * 2.5);
        let sore = hash(fine.x.floor() as isize * 7919 + fine.y.floor() as isize * 104_729, fine.z.floor() as isize, seed ^ 0x5eed);
        match blotch(p, seed) {
            n if n < 0.07 => m.blood,
            n if n < 0.11 => m.bone_moss,
            n if n < 0.3 => m.rot,
            _ if sore < 0.06 => m.blood,
            _ if sore < 0.16 => m.rot,
            _ => m.zombie_skin,
        }
    }
}

/// El zombi esculpido de la mordida (ver `dip`).
fn sculpted_zombie(c: &mut Canvas, m: &MaterialSet, sk: &Skeleton, seed: u32) {
    let k = sk.k;
    let b = anatomy::body(sk);
    let flesh = rotten_paint(m, seed);
    // El vientre abierto, del lado de la nave, y las costillas a la vista de ese lado.
    let wound = sk.at(0.13, 0.08, -0.04);
    let mut carve = b.carve.clone();
    carve.push(Shape::ellipsoid(wound, sk.torso_axes(), Vec3::new(0.055, 0.04, 0.065) * k));
    let skin = Layer::new(b.all(), move |p| {
        let d = (p - sk.pelvis) / k;
        let (h, fwd, sd) = (d.dot(&sk.up), d.dot(&sk.front), d.dot(&sk.side));
        if (0.25..0.46).contains(&h) && fwd > 0.05 && sd < -0.02 {
            let cell = p / SCULPT_DETAIL;
            let n = hash(cell.x as isize * 31 + cell.y as isize * 977, cell.z as isize, seed);
            return Some(if n < 0.55 { m.bone_moss } else { m.blood });
        }
        Some(flesh(p))
    })
    .blend(Body::blend(sk))
    .carve(carve, 0.008 * k);

    // Las tripas: cuelgan del vientre abierto por su propio peso.
    let mut guts = vec![Shape::sphere(wound - sk.front * (0.02 * k), 0.045 * k)];
    let mut rng = Rng::new(seed ^ 0x6475);
    for _ in 0..3 {
        let start = wound + sk.side * (rng.range(-0.03, 0.03) * k) + sk.front * (0.02 * k);
        let mid = start - UP * (rng.range(0.06, 0.1) * k) + sk.side * (rng.range(-0.04, 0.04) * k);
        let end = mid - UP * (rng.range(0.08, 0.16) * k) + sk.front * (rng.range(-0.03, 0.03) * k);
        guts.push(Shape::capsule(start, mid, 0.02 * k));
        guts.push(Shape::capsule(mid, end, 0.018 * k));
    }
    let guts = Layer::new(guts, move |p| {
        let cell = p / SCULPT_DETAIL;
        Some(if hash(cell.x as isize, cell.y as isize * 131 + cell.z as isize, seed) < 0.6 { m.lips } else { m.blood })
    })
    .blend(0.01 * k);

    // La cara: cuencas vacías, la boca ensangrentada y dos hileras de dientes, con huecos.
    let f = anatomy::face(sk);
    let mut layers = vec![skin, guts, Layer::solid(f.eyes.iter().map(|&e| Shape::sphere(e, 0.021 * k)).collect(), m.soot), Layer::solid(vec![Shape::sphere(f.mouth_back, 0.032 * k)], m.blood)];
    let rows = [sk.head_at(-0.068, 0.088, 0.0), sk.head_at(-0.124, 0.072, 0.0)];
    let teeth = Layer::new(rows.iter().map(|&r| Shape::block(r, sk.head_axes(), Vec3::new(0.026, 0.008, 0.012) * k, 0.0)).collect(), move |p| {
        let cell = p / SCULPT_DETAIL;
        (hash(cell.x as isize * 17, cell.y as isize + cell.z as isize * 53, seed) > 0.25).then_some(m.bone_moss)
    });
    layers.push(teeth);
    stamp(c, &layers);
}

/// De los dos lados de una víctima (`side` o `-side`, horizontales), el de atrás: el que
/// queda lejos de la entrada (más adelante en la nave, `+z`). Los zombis que comen
/// vienen de ahí, así no tapan a la víctima para quien llega por la nave.
fn far_side(side: Vec3) -> Vec3 {
    let flat = Vec3::new(side.x, 0.0, side.z);
    if flat.z >= 0.0 { flat.normalize() } else { -flat.normalize() }
}

/// Los padres de los novios, atacados a lo largo de la nave, rumbo a la boda (vestidos a
/// la moda de los 80, ver `people::Outfit`).
#[derive(Clone, Copy)]
pub enum Feast {
    /// El padre del novio, todavía de pie, peleando: un zombi colgado de su brazo,
    /// mordiéndoselo, mientras él le empuja la cabeza a otro.
    FightingFather,
    /// La madre de la novia, arrastrándose boca abajo con una mano estirada, y un zombi
    /// arrodillado a su lado mordiéndole el hombro.
    CrawlingMother,
    /// La madre del novio, desplomada contra un reclinatorio, con un zombi arrodillado
    /// mordiéndole el cuello y el sombrero tirado al lado.
    CollapsedMother,
    /// El padre de la novia, tirado de espaldas, con un zombi arrodillado comiéndole el
    /// cuello mientras él lo empuja con un brazo.
    PinnedFather,
}

/// Un zombi esculpido (1.75 m) con la boca en `bite`, viniendo desde el lado contrario a
/// `toward` (horizontal, de él hacia la mordida): se prueba cada inclinación del torso,
/// con la boca puesta en la mordida, y se queda la que deja la pelvis a `hip` metros del
/// piso. De pie, con las piernas en una estocada; o arrodillado, con las rodillas en el
/// piso y las piernas estiradas hacia atrás.
fn biting_zombie(bite: Vec3, toward: Vec3, floor_y: f32, hip: f32, kneeling: bool) -> Skeleton {
    let toward = Vec3::new(toward.x, 0.0, toward.z).normalize();
    let build = |lean: f32| {
        let up = (UP * lean.to_radians().cos() + toward * lean.to_radians().sin()).normalize();
        let front = perpendicular(-UP, up, toward);
        let look = (toward * 0.5 - UP * 0.8).normalize();
        let pose = Pose { pelvis: Vec3::zeros(), up, front, arms: [(-UP, -UP); 2], legs: [(-UP, -UP); 2], look };
        let mut zombie = Skeleton::new(&pose, 1.75, Build::Corpse);
        zombie.turn_head(look, up);
        let mouth = anatomy::face(&zombie).mouth;
        zombie.shift(bite - mouth);
        zombie
    };
    let target = floor_y + hip * M;
    let lean = (0..=85).map(|d| d as f32).min_by(|&a, &b| (build(a).pelvis.y - target).abs().total_cmp(&(build(b).pelvis.y - target).abs())).unwrap_or(30.0);
    let mut zombie = build(lean);
    let k = zombie.k;
    let behind = -toward;
    for i in 0..2 {
        let s = if i == 0 { -1.0 } else { 1.0 };
        let under = Vec3::new(zombie.hips[i].x, floor_y, zombie.hips[i].z) + zombie.side * (s * 0.05 * k);
        if kneeling {
            let knee = under + toward * (0.05 * k) + UP * (0.05 * k);
            zombie.knees[i] = zombie.hips[i] + (knee - zombie.hips[i]).normalize() * (0.45 * k);
            zombie.ankles[i] = Vec3::new(zombie.knees[i].x, floor_y + 0.045 * k, zombie.knees[i].z) + behind * (0.42 * k);
            zombie.toes[i] = behind;
        } else {
            let front_leg = (zombie.hips[i] - zombie.hips[1 - i]).dot(&toward) > 0.0;
            let mut ankle = under + UP * (0.075 * k) + behind * ((if front_leg { 0.02 } else { 0.28 }) * k);
            for _ in 0..60 {
                let pull = Vec3::new(zombie.hips[i].x - ankle.x, 0.0, zombie.hips[i].z - ankle.z);
                if (ankle - zombie.hips[i]).magnitude() <= 0.87 * k || pull.magnitude() < 0.01 * M {
                    break;
                }
                ankle += pull.normalize() * (0.01 * M);
            }
            zombie.reach_leg(i, ankle, toward + UP * 0.1);
            zombie.toes[i] = toward;
        }
    }
    zombie
}

/// Lleva las dos manos del zombi a `grips` (la más cercana a cada uno), con las palmas
/// hacia lo que agarran.
fn grab(zombie: &mut Skeleton, grips: [Vec3; 2], bend: Vec3) {
    let swap = (zombie.shoulders[0] - grips[1]).magnitude() + (zombie.shoulders[1] - grips[0]).magnitude()
        < (zombie.shoulders[0] - grips[0]).magnitude() + (zombie.shoulders[1] - grips[1]).magnitude();
    for i in 0..2 {
        let target = grips[if swap { 1 - i } else { i }];
        // La muñeca queda un poco antes del punto: la mano es lo que lo toca.
        let wrist = target - (target - zombie.shoulders[i]).normalize() * (0.1 * zombie.k);
        zombie.reach_arm(i, wrist, bend);
        zombie.palms[i] = target - zombie.wrists[i];
    }
}

/// Las escenas de los padres, esculpidas como la boda (`anatomy.rs`, a 2.5 cm):
/// - el padre del novio, de pie, de cara a la entrada, con un brazo estirado hacia el
///   costado y un zombi mordiéndoselo, mientras con la otra mano le empuja la cara a otro
///   que se le tira encima;
/// - la madre de la novia, arrastrándose boca abajo hacia el pasillo con un brazo estirado,
///   con un zombi arrodillado del lado de atrás mordiéndole el hombro;
/// - la madre del novio, desplomada contra un reclinatorio, con un zombi arrodillado
///   mordiéndole el cuello y el sombrero tirado al lado;
/// - el padre de la novia, tirado de espaldas, con un zombi arrodillado al cuello, al que
///   empuja de la cara.
fn sculpted_feast(m: &MaterialSet, which: Feast, spot: Vec3, f: Vec3, seed: u32) -> VoxelGrid {
    let mut rng = Rng::new(seed);
    let floor_y = spot.y;
    let reach = Vec3::new(1.5, 0.0, 1.5) * M;
    let height = if let Feast::FightingFather = which { 2.2 } else { 1.5 };
    let mut c = Canvas::new(spot - reach, spot + reach + UP * (height * M), SCULPT_DETAIL);
    let mut zombies = Vec::new();
    let mut wounds = Vec::new();

    match which {
        Feast::FightingFather => {
            let (up, front) = frame(UP - f * 0.12, f);
            let side = up.cross(&front).normalize();
            let k = M * 1.78 / 1.8;
            let pose = Pose { pelvis: spot + UP * (0.93 * k) - f * (0.05 * M), up, front, arms: [(-UP, -UP); 2], legs: [(-UP, -UP); 2], look: (f + side * 0.35).normalize() };
            let mut father = Skeleton::new(&pose, 1.78, Build::Man);
            father.reach_leg(0, spot - side * (0.17 * M) - f * (0.22 * M) + UP * (0.075 * k), f);
            father.reach_leg(1, spot + side * (0.17 * M) + f * (0.05 * M) + UP * (0.075 * k), f);
            father.toes = [(f - side * 0.2).normalize(), (f + side * 0.2).normalize()];
            // El brazo mordido, estirado hacia el costado.
            let out = (side * 0.85 + f * 0.3 - UP * 0.2).normalize();
            father.reach_arm(1, father.shoulders[1] + out * (0.53 * k), -UP + f * 0.2);
            father.palms[1] = -UP;
            let bite = father.elbows[1].lerp(&father.wrists[1], 0.55) + UP * (0.045 * k);
            let mut feeder = biting_zombie(bite, (-side + f * 0.15).normalize(), floor_y, 0.86, false);
            grab(&mut feeder, [father.elbows[1] + UP * (0.03 * k), father.wrists[1]], -UP + side * 0.3);
            wounds.push(bite);

            // El otro se le tira encima desde el otro costado, con los brazos hacia él; el
            // padre le empuja la cara (que queda echada para atrás).
            let at = Vec3::new(father.pelvis.x, floor_y, father.pelvis.z) - side * (0.62 * M) + f * (0.12 * M);
            let toward = (father.neck - at).component_mul(&Vec3::new(1.0, 0.0, 1.0)).normalize();
            let (wup, wfront) = frame(UP + toward * 0.35, toward);
            let look = (toward + UP * 0.45).normalize();
            let wpose = Pose { pelvis: at + UP * (0.86 * M * 1.75 / 1.8), up: wup, front: wfront, arms: [(-UP, -UP); 2], legs: [(-UP, -UP); 2], look };
            let mut walker = Skeleton::new(&wpose, 1.75, Build::Corpse);
            walker.turn_head(look, wup);
            let wk = walker.k;
            for i in 0..2 {
                let s = if i == 0 { -1.0 } else { 1.0 };
                let stride = if i == 0 { 0.22 } else { -0.18 };
                let ankle = Vec3::new(walker.hips[i].x, floor_y + 0.075 * wk, walker.hips[i].z) + toward * (stride * wk) + walker.side * (s * 0.04 * wk);
                walker.reach_leg(i, ankle, toward + UP * 0.1);
                walker.toes[i] = toward;
            }
            grab(&mut walker, [father.shoulders[0] + up * (0.02 * k), father.neck - side * (0.05 * k)], -UP - toward * 0.3);
            let forehead = walker.head_at(0.03, 0.08, 0.0);
            let wrist = forehead - (forehead - father.shoulders[0]).normalize() * (0.12 * k);
            father.reach_arm(0, wrist, -UP - f * 0.3);
            father.palms[0] = forehead - father.wrists[0];

            people::suited_father(&mut c, m, &father, &people::Suit::groom_father(m));
            zombies.extend([feeder, walker]);
        }
        Feast::CrawlingMother => {
            let up = (f + UP * 0.3).normalize();
            let front = perpendicular(-UP, up, -UP);
            let pose = Pose { pelvis: spot + UP * (0.13 * M), up, front, arms: [(-UP, -UP); 2], legs: [(-UP, -UP); 2], look: (f * 0.6 + UP * 0.6).normalize() };
            let mut mother = Skeleton::new(&pose, 1.65, Build::Woman);
            let k = mother.k;
            let side = mother.side;
            let floor_at = |p: Vec3, h: f32| Vec3::new(p.x, floor_y + h * k, p.z);
            for i in 0..2 {
                let s = if i == 0 { -1.0 } else { 1.0 };
                let ankle = floor_at(mother.pelvis - f * (0.85 * k) + side * (s * 0.1 * k), 0.045);
                mother.reach_leg(i, ankle, UP);
            }
            mother.toes = [-f, -f];
            // El brazo de atrás (lejos de la entrada) apoyado en el codo; el otro, estirado
            // hacia el pasillo, arañando el piso.
            let far = if mother.shoulders[0].z > mother.shoulders[1].z { 0 } else { 1 };
            let near = 1 - far;
            let s_far = if far == 0 { -1.0 } else { 1.0 };
            mother.reach_arm(far, floor_at(mother.shoulders[far] + f * (0.3 * k) - side * (s_far * 0.04 * k), 0.035), -UP + side * (s_far * 0.5));
            mother.reach_arm(near, floor_at(mother.shoulders[near] + f * (0.52 * k), 0.035), UP);
            mother.palms = [-UP, -UP];
            let bite = mother.shoulders[far] + UP * (0.05 * k);
            let mut zombie = biting_zombie(bite, -far_side(side), floor_y, 0.5, true);
            grab(&mut zombie, [mother.at(0.32, -0.11, 0.0), mother.elbows[far] + UP * (0.03 * k)], -UP);
            wounds.push(bite);
            people::eighties_mother(&mut c, m, &mother, floor_y, &people::Dress::bride_mother(m));
            zombies.push(zombie);
        }
        Feast::CollapsedMother => {
            // Sentada en el piso, desplomada hacia atrás contra el reclinatorio, una pierna
            // estirada y la otra con la rodilla levantada; la cabeza caída hacia la entrada
            // deja el cuello al aire del lado de atrás, donde muerde el zombi.
            let up = (UP * 0.75 - f * 0.6).normalize();
            let pose = Pose { pelvis: spot + UP * (0.14 * M), up, front: f, arms: [(-UP, -UP); 2], legs: [(-UP, -UP); 2], look: f };
            let mut mother = Skeleton::new(&pose, 1.68, Build::Woman);
            let k = mother.k;
            let far = far_side(mother.side);
            let far_i = if mother.side.dot(&far) > 0.0 { 1 } else { 0 };
            let near_i = 1 - far_i;
            let floor_at = |p: Vec3, h: f32| Vec3::new(p.x, floor_y + h * k, p.z);
            mother.reach_leg(near_i, floor_at(mother.hips[near_i] + f * (0.86 * k), 0.05), UP);
            mother.reach_leg(far_i, floor_at(mother.hips[far_i] + f * (0.5 * k), 0.075), UP);
            mother.toes[near_i] = UP;
            mother.toes[far_i] = f;
            mother.turn_head((f * 0.75 + UP * 0.15 - far * 0.6).normalize(), up - far * 0.35);
            let axis = (mother.neck_top - mother.neck).normalize();
            let bite = (mother.neck + mother.neck_top) / 2.0 + perpendicular(far + UP * 0.2, axis, far) * (0.05 * k);
            let mut zombie = biting_zombie(bite, -far, floor_y, 0.5, true);
            grab(&mut zombie, [mother.shoulders[far_i] + UP * (0.02 * k), mother.at(0.32, 0.08, 0.0)], -UP);
            // Un brazo caído en el piso, del lado de la entrada; con el otro empuja,
            // débil, el pecho del zombi.
            mother.reach_arm(near_i, floor_at(mother.hips[near_i] - far * (0.22 * k) + f * (0.12 * k), 0.03), -f + UP * 0.3);
            let chest = zombie.at(0.38, 0.1, 0.0);
            mother.reach_arm(far_i, chest - (chest - mother.shoulders[far_i]).normalize() * (0.1 * k), -UP + f * 0.3);
            mother.palms[far_i] = chest - mother.wrists[far_i];
            mother.palms[near_i] = UP;
            wounds.push(bite);
            people::eighties_mother(&mut c, m, &mother, floor_y, &people::Dress::groom_mother(m));
            people::fallen_hat(&mut c, spot + f * (0.6 * M) - far * (0.5 * M), m.dress_teal, m.dress_fuchsia);
            zombies.push(zombie);
        }
        Feast::PinnedFather => {
            // Tirado de espaldas, la cabeza hacia el centro de la nave, una rodilla
            // levantada; grita hacia la entrada con el cuello al aire del lado de atrás,
            // donde muerde el zombi, al que empuja de la cara. El otro brazo, tirado en el
            // piso hacia quien llega.
            let (up, front) = frame(f, UP);
            let pose = Pose { pelvis: spot + UP * (0.12 * M), up, front, arms: [(-UP, -UP); 2], legs: [(-UP, -UP); 2], look: UP };
            let mut father = Skeleton::new(&pose, 1.75, Build::Man);
            let k = father.k;
            let far = far_side(father.side);
            let far_i = if father.side.dot(&far) > 0.0 { 1 } else { 0 };
            let near_i = 1 - far_i;
            let floor_at = |p: Vec3, h: f32| Vec3::new(p.x, floor_y + h * k, p.z);
            father.reach_leg(near_i, floor_at(father.hips[near_i] - f * (0.87 * k) - far * (0.04 * k), 0.06), UP);
            father.reach_leg(far_i, floor_at(father.hips[far_i] - f * (0.5 * k), 0.075), UP);
            father.toes[near_i] = UP;
            father.toes[far_i] = -f;
            father.turn_head((UP * 0.65 - far * 0.55).normalize(), f);
            let axis = (father.neck_top - father.neck).normalize();
            let bite = (father.neck + father.neck_top) / 2.0 + perpendicular(far + UP * 0.3, axis, far) * (0.055 * k);
            let mut zombie = biting_zombie(bite, -far, floor_y, 0.5, true);
            grab(&mut zombie, [father.shoulders[far_i] + UP * (0.02 * k), father.at(0.3, 0.1, 0.0)], -UP);
            father.reach_arm(near_i, floor_at(father.shoulders[near_i] - far * (0.5 * k) + f * (0.15 * k), 0.035), UP);
            father.palms[near_i] = UP;
            let forehead = zombie.head_at(0.03, 0.08, 0.0);
            father.reach_arm(far_i, forehead - (forehead - father.shoulders[far_i]).normalize() * (0.12 * k), -f + UP * 0.2);
            father.palms[far_i] = forehead - father.wrists[far_i];
            wounds.push(bite);
            people::suited_father(&mut c, m, &father, &people::Suit::bride_father(m));
            zombies.push(zombie);
        }
    }

    for (i, zombie) in zombies.iter().enumerate() {
        sculpted_zombie(&mut c, m, zombie, seed.wrapping_add(i as u32 * 31));
    }
    let below = |p: Vec3| Vec3::new(p.x, floor_y, p.z);
    for &w in &wounds {
        c.capsule(w, w, 0.05 * M, &|_| m.blood);
        c.capsule(w, w.lerp(&below(w), 0.6), 0.025 * M, &|_| m.blood);
        splash(&mut c, m, below(w), 0.4, &mut rng);
    }
    people::sculpted(c)
}

/// Una de las escenas de los padres en `spot` (sobre el piso), orientada por `facing`
/// (horizontal): hacia dónde mira el padre que pelea, hacia dónde se arrastra la madre,
/// hacia dónde mira la que se desplomó (el reclinatorio queda a su espalda) o hacia dónde
/// apunta la cabeza del que está tirado. Todo en un grid apoyado en el piso, con las
/// heridas y la sangre salpicada debajo.
pub fn feast(m: &MaterialSet, which: Feast, spot: Vec3, facing: Vec3, seed: u32) -> VoxelGrid {
    sculpted_feast(m, which, spot, Vec3::new(facing.x, 0.0, facing.z).normalize(), seed)
}

#[cfg(test)]
mod dip_tests {
    use super::*;

    fn dip_geometry() -> (Skeleton, people::BrideDip, Skeleton, Vec3, Vec3, Vec3) {
        let (feet, back) = (Vec3::new(10.0, 2.0, 30.0), Vec3::new(-1.0, 0.0, 0.0));
        let side = UP.cross(&back).normalize();
        let bride = people::bride_skeleton(feet, back, side);
        let points = people::bride_dip_points(&bride);
        let (zombie, _) = dip_zombie(&bride, &points, feet, side);
        (bride, points, zombie, feet, back, side)
    }

    #[test]
    fn the_bride_faces_the_nave_and_the_zombie_bites_her_neck() {
        let (bride, points, zombie, feet, _, side) = dip_geometry();
        assert!(bride.face.dot(&-side) > 0.7, "la cara mira hacia la nave (-side)");
        let mouth = anatomy::face(&zombie).mouth;
        assert!((mouth - points.bite).magnitude() < 0.05 * M, "la boca del zombi está en la mordida");
        let hips = (zombie.pelvis.y - feet.y) / M;
        println!("torso del zombi a {:.0}° de la vertical", zombie.up.dot(&UP).acos().to_degrees());
        println!("pelvis del zombi a {hips:.2} m, rodillas a {:.2} / {:.2} m", (zombie.knees[0].y - feet.y) / M, (zombie.knees[1].y - feet.y) / M);
        assert!(hips > 0.8, "de pie, no agachado: la pelvis a {hips:.2} m");
        assert!(zombie.head.dot(&side) > bride.head.dot(&side), "la cabeza del zombi queda detrás de la de ella, no delante de su cara");
    }

    #[test]
    fn the_zombie_stands_upright_behind_her_and_shows_above_her() {
        let (bride, _, zombie, feet, _, side) = dip_geometry();
        for ankle in zombie.ankles {
            assert!((ankle.y - feet.y - 0.075 * zombie.k).abs() < 0.03 * M, "el tobillo a la altura de un pie apoyado");
        }
        let lean = zombie.up.dot(&UP).acos().to_degrees();
        println!("torso a {lean:.0}°, pelvis a {:.2} m, cuello a {:.2} m (cabeza de ella a {:.2} m)", (zombie.pelvis.y - feet.y) / M, (zombie.neck.y - feet.y) / M, (bride.head.y - feet.y) / M);
        assert!(zombie.pelvis.y - feet.y > 0.78 * M, "de pie, no agachado");
        assert!((zombie.pelvis - bride.head).dot(&side) > 0.3 * M, "parado detrás de ella, del lado opuesto a la nave");
        assert!(zombie.neck.y > bride.head.y && lean < 40.0, "erguido: el cuello le queda por encima de la cabeza de ella");
    }
}

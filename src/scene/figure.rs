//! El esqueleto de cápsulas de la invasión (`zombies.rs`): una pose dice dónde está la
//! pelvis y hacia dónde va cada tramo, y de ahí salen las articulaciones y las cápsulas del
//! cuerpo, todas de carne podrida. Las figuras de la boda y los padres de los novios usan
//! en cambio la anatomía esculpida (`anatomy.rs`), que toma la misma `Pose` como punto de
//! partida.
//!
//! Todo en coordenadas de mundo (1 unidad = un vóxel de 20 cm de la catedral).

use crate::rng::Rng;
use crate::scene::canvas::Canvas;
use crate::scene::skeleton::DETAIL;
use nalgebra_glm::Vec3;

/// Metros → unidades de mundo.
pub const M: f32 = 5.0;
pub const UP: Vec3 = Vec3::new(0.0, 1.0, 0.0);

/// Una pose: la pelvis, hacia dónde va el torso (`up`, de la pelvis al cuello), hacia
/// dónde mira el pecho (`front`), la dirección de cada tramo: (brazo, antebrazo) y
/// (muslo, pierna), izquierda y derecha, y hacia dónde mira la cara (`look`). El índice
/// 0 es el lado `-side` y el 1 el `+side`, con `side = up × front`.
pub struct Pose {
    pub pelvis: Vec3,
    pub up: Vec3,
    pub front: Vec3,
    pub arms: [(Vec3, Vec3); 2],
    pub legs: [(Vec3, Vec3); 2],
    pub look: Vec3,
}

/// De qué parte del cuerpo es cada cápsula.
#[derive(Clone, Copy, PartialEq)]
pub enum Part {
    Torso,
    Head,
    UpperArm,
    Forearm,
    Hand,
    Thigh,
    Shin,
    Eye,
    Jaw,
}

/// Las articulaciones de una pose: cuello, y por lado (0 = `-side`, 1 = `+side`) hombro,
/// codo, mano, cadera, rodilla y pie.
pub struct Joints {
    pub neck: Vec3,
    pub shoulders: [Vec3; 2],
    pub elbows: [Vec3; 2],
    pub hands: [Vec3; 2],
    pub hips: [Vec3; 2],
    pub knees: [Vec3; 2],
    pub feet: [Vec3; 2],
}

pub fn joints(pose: &Pose) -> Joints {
    let side = pose.up.cross(&pose.front).normalize();
    let neck = pose.pelvis + pose.up * (0.55 * M);
    let per_side = |i: usize| {
        let s = if i == 0 { -1.0 } else { 1.0 };
        let shoulder = neck + side * (s * 0.2 * M) - pose.up * (0.04 * M);
        let (d1, d2) = pose.arms[i];
        let elbow = shoulder + d1 * (0.3 * M);
        let hand = elbow + d2 * (0.28 * M);
        let hip = pose.pelvis + side * (s * 0.1 * M);
        let (d3, d4) = pose.legs[i];
        let knee = hip + d3 * (0.42 * M);
        (shoulder, elbow, hand, hip, knee, knee + d4 * (0.42 * M))
    };
    let (l, r) = (per_side(0), per_side(1));
    Joints {
        neck,
        shoulders: [l.0, r.0],
        elbows: [l.1, r.1],
        hands: [l.2, r.2],
        hips: [l.3, r.3],
        knees: [l.4, r.4],
        feet: [l.5, r.5],
    }
}

pub fn random_dir(rng: &mut Rng) -> Vec3 {
    loop {
        let v = Vec3::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
        let len = v.magnitude();
        if (0.1..=1.0).contains(&len) {
            return v / len;
        }
    }
}

/// `up` y `front` perpendiculares, con `front` lo más parecido posible a `front_hint`.
pub fn frame(up: Vec3, front_hint: Vec3) -> (Vec3, Vec3) {
    let up = up.normalize();
    let mut front = front_hint - up * front_hint.dot(&up);
    if front.magnitude() < 1e-3 {
        front = up.cross(&Vec3::new(0.3, 0.1, 0.9)); // por si quedaron paralelos
    }
    (up, front.normalize())
}

/// Direcciones (muslo, pierna) de una pierna de dos tramos de `len` que va de `hip` a
/// `foot`, con la rodilla doblada hacia `bend`. Si el pie queda lejos, la pierna va
/// estirada hacia él. Sirve igual para un brazo.
pub fn leg_to(hip: Vec3, foot: Vec3, len: f32, bend: Vec3) -> (Vec3, Vec3) {
    let reach = foot - hip;
    let d = reach.magnitude();
    let axis = reach / d;
    if d >= 2.0 * len {
        return (axis, axis);
    }
    let out = (bend - axis * bend.dot(&axis)).normalize();
    let knee = hip + reach / 2.0 + out * (len * len - d * d / 4.0).sqrt();
    ((knee - hip).normalize(), (foot - knee).normalize())
}

/// Dónde va la cabeza y hacia dónde mira: el centro, la cara (`look`), la coronilla y el
/// eje de oreja a oreja.
pub struct HeadFrame {
    pub head: Vec3,
    pub face: Vec3,
    pub crown: Vec3,
    pub across: Vec3,
}

pub fn head_frame(pose: &Pose) -> HeadFrame {
    let neck = pose.pelvis + pose.up * (0.55 * M);
    let head = neck + (pose.up * 0.8 + pose.front * 0.35).normalize() * (0.17 * M);
    let face = pose.look.normalize();
    let crown = pose.up - face * pose.up.dot(&face);
    let crown = if crown.magnitude() > 1e-3 { crown.normalize() } else { pose.front };
    HeadFrame { head, face, crown, across: crown.cross(&face).normalize() }
}

/// Las cápsulas del cuerpo en la pose dada: `(desde, hasta, radio, parte)`. La cara mira
/// hacia `look`: cuencas oscuras y una mandíbula colgando (las personas no la usan).
pub fn capsules(pose: &Pose) -> Vec<(Vec3, Vec3, f32, Part)> {
    let j = joints(pose);
    let HeadFrame { head, face, crown, across } = head_frame(pose);
    let jaw = head + face * (0.09 * M) - crown * (0.07 * M);
    let mut parts = vec![
        (pose.pelvis, j.neck, 0.13 * M, Part::Torso),
        (head, head, 0.12 * M, Part::Head),
        (jaw, jaw + face * (0.03 * M) - crown * (0.07 * M), 0.04 * M, Part::Jaw),
    ];
    for i in 0..2 {
        let s = if i == 0 { -1.0 } else { 1.0 };
        let eye = head + face * (0.1 * M) + across * (s * 0.045 * M) + crown * (0.02 * M);
        parts.push((eye, eye, 0.035 * M, Part::Eye));
        parts.push((j.shoulders[i], j.elbows[i], 0.05 * M, Part::UpperArm));
        parts.push((j.elbows[i], j.hands[i], 0.045 * M, Part::Forearm));
        parts.push((j.hands[i], j.hands[i], 0.055 * M, Part::Hand));
        parts.push((j.hips[i], j.knees[i], 0.065 * M, Part::Thigh));
        parts.push((j.knees[i], j.feet[i], 0.055 * M, Part::Shin));
    }
    parts
}

/// Grid ajustado a las cápsulas de un conjunto de poses y a otras cápsulas sueltas
/// (`extra`: extremos con su radio). Cuanto más ajustado, menos celdas vacías recorre cada
/// rayo que lo cruza.
pub fn canvas_for(poses: &[Pose], extra: &[(Vec3, f32)]) -> Canvas {
    let (min, max) = canvas_bounds(poses, extra);
    Canvas::new(min, max, DETAIL)
}

pub fn canvas_bounds(poses: &[Pose], extra: &[(Vec3, f32)]) -> (Vec3, Vec3) {
    let points = poses
        .iter()
        .flat_map(|pose| capsules(pose).into_iter().flat_map(|(a, b, r, _)| [(a, r), (b, r)]))
        .chain(extra.iter().copied());
    let (min, max) = points.fold((Vec3::repeat(f32::INFINITY), Vec3::repeat(f32::NEG_INFINITY)), |(lo, hi), (p, r)| {
        (lo.inf(&(p - Vec3::repeat(r))), hi.sup(&(p + Vec3::repeat(r))))
    });
    (min - Vec3::repeat(DETAIL), max + Vec3::repeat(DETAIL))
}

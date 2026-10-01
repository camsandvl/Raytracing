//! Anatomía esculpida para las cuatro figuras de la boda: un esqueleto con proporciones
//! reales (un adulto de 1.80 m mide unas 7.5 cabezas; la pelvis queda a la mitad de la
//! altura, el codo a la altura de la cintura) y, sobre él, las masas del cuerpo como
//! arcilla (`sculpt::Shape`): pelvis y glúteos, abdomen, caja torácica, pectorales o
//! busto, trapecios y clavículas, deltoides, bíceps, antebrazos que se afinan hacia la
//! muñeca, muslos, rodillas, pantorrillas, pies con talón, manos con palma, dedos y
//! pulgar, y una cabeza con cráneo, mandíbula, pómulos, arco de las cejas, nariz, orejas,
//! cuencas y la boca abierta.
//!
//! Tres complexiones: hombre, mujer y cadáver (el zombi de la mordida: costillas y
//! columna a la vista, el vientre hundido, miembros flacos con las articulaciones
//! nudosas y una cara de calavera — sin nariz, sin labios, los dientes al aire).
//!
//! Las medidas van en metros de una persona de 1.80 m; `Skeleton::k` las pasa a
//! unidades de mundo y a la altura de cada figura.

use crate::scene::figure::{Pose, M, UP};
use crate::scene::sculpt::{along, perpendicular, Shape};
use nalgebra_glm::Vec3;

#[derive(Clone, Copy, PartialEq)]
pub enum Build {
    Man,
    Woman,
    Corpse,
}

/// Las articulaciones y la orientación de una figura. Índice 0 = lado `-side`.
pub struct Skeleton {
    pub build: Build,
    /// Metros (de una persona de 1.80 m) → unidades de mundo, a la altura de la figura.
    pub k: f32,
    pub pelvis: Vec3,
    pub up: Vec3,
    pub front: Vec3,
    pub side: Vec3,
    /// La base del cuello (entre las clavículas) y el arranque de la cabeza.
    pub neck: Vec3,
    pub neck_top: Vec3,
    /// Centro de la cabeza (a la altura de las orejas) y sus ejes: la cara, la coronilla
    /// y de oreja a oreja.
    pub head: Vec3,
    pub face: Vec3,
    pub crown: Vec3,
    pub across: Vec3,
    pub shoulders: [Vec3; 2],
    pub elbows: [Vec3; 2],
    pub wrists: [Vec3; 2],
    pub hips: [Vec3; 2],
    pub knees: [Vec3; 2],
    pub ankles: [Vec3; 2],
    /// Hacia dónde mira la palma de cada mano.
    pub palms: [Vec3; 2],
    /// Hacia dónde apunta cada pie. Si apunta en la línea de la pierna (hacia donde sigue
    /// la pantorrilla), el pie queda estirado: arrodillado, o boca abajo en el piso.
    pub toes: [Vec3; 2],
    /// Cuánto más grande que lo normal es la cabeza (1 = proporción real), y cuánto más
    /// gruesos los brazos: para que una figura se lea mejor de lejos sin cambiarle la pose
    /// (el zombi de la mordida).
    pub head_scale: f32,
    pub arm_scale: f32,
}

impl Skeleton {
    /// El esqueleto de `pose` para una figura de `height` metros. La cabeza mira hacia
    /// `pose.look`, con la coronilla del lado de `pose.up`.
    pub fn new(pose: &Pose, height: f32, build: Build) -> Self {
        let k = M * height / 1.8;
        let (up, front) = (pose.up.normalize(), pose.front.normalize());
        let side = up.cross(&front).normalize();
        let woman = build == Build::Woman;
        let neck = pose.pelvis + up * ((if woman { 0.52 } else { 0.53 }) * k);
        let shoulder_width = if woman { 0.172 } else { 0.19 };
        let hip_width = if woman { 0.092 } else { 0.088 };
        let per_side = |i: usize| {
            let s = if i == 0 { -1.0 } else { 1.0 };
            let shoulder = neck + side * (s * shoulder_width * k) - up * (0.03 * k);
            let (d1, d2) = pose.arms[i];
            let elbow = shoulder + d1.normalize() * (0.30 * k);
            let wrist = elbow + d2.normalize() * (0.255 * k);
            let hip = pose.pelvis + side * (s * hip_width * k);
            let (d3, d4) = pose.legs[i];
            let knee = hip + d3.normalize() * (0.45 * k);
            let ankle = knee + d4.normalize() * (0.43 * k);
            // La palma, de entrada, mira hacia el cuerpo (hacia el otro lado).
            let palm = perpendicular(-side * s, d2.normalize(), front);
            (shoulder, elbow, wrist, hip, knee, ankle, palm)
        };
        let (l, r) = (per_side(0), per_side(1));
        let toe = perpendicular(front, UP, front);
        let mut skeleton = Skeleton {
            build,
            k,
            pelvis: pose.pelvis,
            up,
            front,
            side,
            neck,
            neck_top: neck,
            head: neck,
            face: front,
            crown: up,
            across: side,
            shoulders: [l.0, r.0],
            elbows: [l.1, r.1],
            wrists: [l.2, r.2],
            hips: [l.3, r.3],
            knees: [l.4, r.4],
            ankles: [l.5, r.5],
            palms: [l.6, r.6],
            toes: [toe, toe],
            head_scale: 1.0,
            arm_scale: 1.0,
        };
        skeleton.turn_head(pose.look, up);
        skeleton
    }

    /// Gira la cabeza: la cara hacia `face`, la coronilla lo más hacia `crown` posible.
    /// El cuello se inclina hacia la coronilla.
    pub fn turn_head(&mut self, face: Vec3, crown: Vec3) {
        let face = face.normalize();
        let crown = perpendicular(crown, face, self.up);
        let k = self.k;
        self.neck_top = self.neck + (self.up * 0.6 + crown * 0.4 + face * 0.1).normalize() * (0.085 * k);
        self.head = self.neck_top + (crown * 0.075 - face * 0.005) * (k * self.head_scale);
        self.face = face;
        self.crown = crown;
        self.across = crown.cross(&face).normalize();
    }

    /// Lleva la muñeca `i` a `wrist` (o lo más cerca que llegue el brazo), con el codo
    /// doblado hacia `bend`.
    pub fn reach_arm(&mut self, i: usize, wrist: Vec3, bend: Vec3) {
        let (upper, fore) = (0.30 * self.k, 0.255 * self.k);
        let (d1, d2) = reach(self.shoulders[i], wrist, upper, fore, bend);
        self.elbows[i] = self.shoulders[i] + d1 * upper;
        self.wrists[i] = self.elbows[i] + d2 * fore;
    }

    /// Lleva el tobillo `i` a `ankle`, con la rodilla doblada hacia `bend`.
    pub fn reach_leg(&mut self, i: usize, ankle: Vec3, bend: Vec3) {
        let (thigh, shin) = (0.45 * self.k, 0.43 * self.k);
        let (d1, d2) = reach(self.hips[i], ankle, thigh, shin, bend);
        self.knees[i] = self.hips[i] + d1 * thigh;
        self.ankles[i] = self.knees[i] + d2 * shin;
    }

    /// Mueve toda la figura `by`.
    pub fn shift(&mut self, by: Vec3) {
        for p in [&mut self.pelvis, &mut self.neck, &mut self.neck_top, &mut self.head] {
            *p += by;
        }
        for joints in [&mut self.shoulders, &mut self.elbows, &mut self.wrists, &mut self.hips, &mut self.knees, &mut self.ankles] {
            for p in joints.iter_mut() {
                *p += by;
            }
        }
    }

    /// Un punto del tronco: `h` por encima de la pelvis, `fwd` hacia el pecho y `sd` hacia
    /// `side` (en metros).
    pub fn at(&self, h: f32, fwd: f32, sd: f32) -> Vec3 {
        self.pelvis + (self.up * h + self.front * fwd + self.side * sd) * self.k
    }

    /// Un punto de la cabeza: `u` hacia la coronilla, `f` hacia la cara y `a` de oreja a
    /// oreja, desde el centro de la cabeza (en metros).
    pub fn head_at(&self, u: f32, f: f32, a: f32) -> Vec3 {
        self.head + (self.crown * u + self.face * f + self.across * a) * (self.k * self.head_scale)
    }

    pub fn torso_axes(&self) -> [Vec3; 3] {
        [self.side, self.front, self.up]
    }

    pub fn head_axes(&self) -> [Vec3; 3] {
        [self.across, self.face, self.crown]
    }

    fn limb(&self) -> f32 {
        match self.build {
            Build::Man => 1.0,
            Build::Woman => 0.84,
            Build::Corpse => 0.7,
        }
    }
}

/// Las direcciones (primer tramo, segundo tramo) de un miembro de dos tramos `l1`, `l2`
/// que va de `from` a `to`, con la articulación del medio doblada hacia `bend`. Si `to`
/// queda fuera de alcance, el miembro se estira hacia él.
pub fn reach(from: Vec3, to: Vec3, l1: f32, l2: f32, bend: Vec3) -> (Vec3, Vec3) {
    let r = to - from;
    let axis = r.normalize();
    let d = r.magnitude().clamp((l1 - l2).abs() + 1e-3, l1 + l2 - 1e-3);
    let out = perpendicular(bend, axis, along(axis)[0]);
    let a = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
    let joint = from + axis * a + out * (l1 * l1 - a * a).max(0.0).sqrt();
    ((joint - from).normalize(), (from + axis * d - joint).normalize())
}

/// El cuerpo por partes, para poder vestir cada una (el saco sobre el tronco y los
/// brazos, el pantalón sobre las piernas...).
pub struct Body {
    /// Pelvis y glúteos (la cadera, de donde cuelga una falda).
    pub hips: Vec<Shape>,
    pub torso: Vec<Shape>,
    pub neck: Vec<Shape>,
    pub upper_arms: [Vec<Shape>; 2],
    pub forearms: [Vec<Shape>; 2],
    pub hands: [Vec<Shape>; 2],
    pub thighs: [Vec<Shape>; 2],
    pub shins: [Vec<Shape>; 2],
    pub feet: [Vec<Shape>; 2],
    pub head: Vec<Shape>,
    /// Lo que se talla en la cabeza: cuencas, la boca abierta, (en el cadáver) mejillas
    /// hundidas y el hueco de la nariz.
    pub carve: Vec<Shape>,
}

impl Body {
    pub fn all(&self) -> Vec<Shape> {
        let mut all = Vec::new();
        all.extend(&self.hips);
        all.extend(&self.torso);
        all.extend(&self.neck);
        for i in 0..2 {
            all.extend(&self.upper_arms[i]);
            all.extend(&self.forearms[i]);
            all.extend(&self.hands[i]);
            all.extend(&self.thighs[i]);
            all.extend(&self.shins[i]);
            all.extend(&self.feet[i]);
        }
        all.extend(&self.head);
        all
    }

    /// La unión suave que conviene para la piel de esta figura.
    pub fn blend(skeleton: &Skeleton) -> f32 {
        0.014 * skeleton.k
    }
}

pub fn body(sk: &Skeleton) -> Body {
    let k = sk.k;
    let t = sk.limb();
    let axes = sk.torso_axes();
    let e = |center: Vec3, axes: [Vec3; 3], r: [f32; 3]| Shape::ellipsoid(center, axes, Vec3::new(r[0], r[1], r[2]) * k);
    let sphere = |center: Vec3, r: f32| Shape::sphere(center, r * k);

    let (mut hips, mut torso) = (Vec::new(), Vec::new());
    match sk.build {
        Build::Man => {
            hips.push(e(sk.at(0.03, 0.0, 0.0), axes, [0.148, 0.1, 0.11]));
            torso.push(e(sk.at(0.17, 0.01, 0.0), axes, [0.128, 0.094, 0.13]));
            torso.push(e(sk.at(0.35, 0.0, 0.0), axes, [0.148, 0.103, 0.165]));
            for s in [-1.0f32, 1.0] {
                hips.push(e(sk.at(-0.03, -0.05, s * 0.064), axes, [0.068, 0.064, 0.09]));
                torso.push(e(sk.at(0.405, 0.052, s * 0.062), axes, [0.074, 0.05, 0.062]));
                torso.push(e(sk.at(0.34, -0.04, s * 0.08), axes, [0.064, 0.054, 0.13]));
            }
        }
        Build::Woman => {
            hips.push(e(sk.at(0.02, 0.0, 0.0), axes, [0.172, 0.104, 0.115]));
            torso.push(e(sk.at(0.17, 0.004, 0.0), axes, [0.11, 0.083, 0.11]));
            torso.push(e(sk.at(0.33, 0.0, 0.0), axes, [0.13, 0.094, 0.15]));
            for s in [-1.0f32, 1.0] {
                hips.push(e(sk.at(-0.04, -0.055, s * 0.078), axes, [0.085, 0.075, 0.1]));
                torso.push(e(sk.at(0.355, 0.07, s * 0.056), axes, [0.06, 0.056, 0.06]));
            }
        }
        Build::Corpse => {
            hips.push(e(sk.at(0.03, 0.0, 0.0), axes, [0.148, 0.095, 0.1]));
            torso.push(e(sk.at(0.17, -0.015, 0.0), axes, [0.112, 0.072, 0.12]));
            torso.push(e(sk.at(0.35, 0.005, 0.0), axes, [0.148, 0.1, 0.165]));
            // El esternón, las costillas marcadas bajo la piel, la columna y los omóplatos.
            torso.push(Shape::capsule(sk.at(0.27, 0.098, 0.0), sk.at(0.48, 0.092, 0.0), 0.013 * k));
            for rib in 0..7 {
                let h = 0.25 + 0.036 * rib as f32;
                for s in [-1.0f32, 1.0] {
                    let point = |angle: f32| {
                        let (sin, cos) = angle.to_radians().sin_cos();
                        sk.at(h - 0.03 * cos, 0.103 * cos, s * 0.15 * sin)
                    };
                    for step in 0..5 {
                        let (a0, a1) = (15.0 + 22.0 * step as f32, 37.0 + 22.0 * step as f32);
                        torso.push(Shape::capsule(point(a0), point(a1), 0.011 * k));
                    }
                }
            }
            for v in 0..12 {
                torso.push(sphere(sk.at(0.02 + 0.045 * v as f32, -0.088, 0.0), 0.018));
            }
            for s in [-1.0f32, 1.0] {
                hips.push(e(sk.at(-0.03, -0.042, s * 0.065), axes, [0.06, 0.055, 0.08]));
                hips.push(sphere(sk.at(0.1, 0.035, s * 0.118), 0.03)); // las crestas de la cadera
                torso.push(Shape::block(sk.at(0.4, -0.078, s * 0.078), axes, Vec3::new(0.055, 0.012, 0.075) * k, 0.01 * k));
            }
        }
    }
    // Clavículas y trapecios: del cuello a cada hombro.
    let trap = match sk.build {
        Build::Woman => 0.034,
        _ => 0.044,
    };
    for i in 0..2 {
        let s = if i == 0 { -1.0 } else { 1.0 };
        torso.push(Shape::cone(sk.at(0.52, 0.065, s * 0.02), sk.shoulders[i] + sk.front * (0.03 * k), 0.013 * k, 0.015 * k));
        torso.push(Shape::cone(sk.neck + sk.up * (0.04 * k) - sk.front * (0.02 * k), sk.shoulders[i] - sk.front * (0.01 * k), trap * k, trap * 0.85 * k));
    }

    let neck_r = match sk.build {
        Build::Man => 0.058,
        Build::Woman => 0.047,
        Build::Corpse => 0.042,
    };
    let mut neck = vec![Shape::cone(sk.neck - sk.up * (0.02 * k), sk.neck_top, neck_r * k, neck_r * 0.88 * k)];
    if sk.build == Build::Corpse {
        // Los tendones del cuello marcados.
        for s in [-1.0f32, 1.0] {
            neck.push(Shape::capsule(sk.neck + sk.side * (s * 0.02 * k) + sk.front * (0.03 * k), sk.head_at(-0.07, 0.0, s * 0.045), 0.012 * k));
        }
    }

    let mut upper_arms: [Vec<Shape>; 2] = Default::default();
    let mut forearms: [Vec<Shape>; 2] = Default::default();
    let mut hands: [Vec<Shape>; 2] = Default::default();
    let mut thighs: [Vec<Shape>; 2] = Default::default();
    let mut shins: [Vec<Shape>; 2] = Default::default();
    let mut feet: [Vec<Shape>; 2] = Default::default();
    for i in 0..2 {
        let (shoulder, elbow, wrist) = (sk.shoulders[i], sk.elbows[i], sk.wrists[i]);
        let (d1, d2) = ((elbow - shoulder).normalize(), (wrist - elbow).normalize());
        let out = perpendicular(shoulder - sk.neck, d1, sk.side);
        let t = t * sk.arm_scale; // los brazos (piernas aparte)
        upper_arms[i] = vec![
            e(shoulder + d1 * (0.035 * k) + out * (0.008 * k), along(d1), [0.058 * t.max(0.85), 0.055 * t.max(0.85), 0.08]),
            Shape::cone(shoulder, elbow, 0.046 * t * k, 0.036 * t * k),
            e(shoulder + d1 * (0.15 * k), along(d1), [0.045 * t, 0.043 * t, 0.1]),
        ];
        let knob = if sk.build == Build::Corpse { 0.034 * sk.arm_scale } else { 0.036 * t };
        forearms[i] = vec![
            sphere(elbow, knob),
            Shape::cone(elbow, wrist, 0.042 * t * k, 0.025 * t.max(0.8) * k),
            e(elbow + d2 * (0.075 * k), along(d2), [0.043 * t, 0.038 * t, 0.085]),
        ];
        hands[i] = hand(sk, wrist, d2, sk.palms[i], i);
        let t = sk.limb();

        let (hip, knee, ankle) = (sk.hips[i], sk.knees[i], sk.ankles[i]);
        let (d3, d4) = ((knee - hip).normalize(), (ankle - knee).normalize());
        let toe = perpendicular(sk.toes[i], d4, sk.front);
        thighs[i] = vec![
            Shape::cone(hip, knee, 0.085 * t * k, 0.05 * t.max(0.8) * k),
            e(hip + d3 * (0.19 * k) + perpendicular(sk.front, d3, sk.front) * (0.012 * k), along(d3), [0.074 * t, 0.07 * t, 0.19]),
        ];
        let knee_r = if sk.build == Build::Corpse { 0.046 } else { 0.05 * t.max(0.85) };
        shins[i] = vec![
            sphere(knee, knee_r),
            Shape::cone(knee, ankle, 0.046 * t * k, 0.03 * t.max(0.85) * k),
            e(knee + d4 * (0.12 * k) - toe * (0.025 * k), along(d4), [0.05 * t, 0.055 * t, 0.12]),
        ];
        let f = if sk.build == Build::Woman { 0.9 } else { 1.0 };
        feet[i] = if sk.toes[i].dot(&d4) > 0.7 {
            // El pie estirado en la línea de la pierna (arrodillado, o boca abajo en el
            // piso): el empeine apoyado y la punta hacia atrás.
            let flat = perpendicular(UP, d4, sk.up);
            let axes = [d4, flat.cross(&d4).normalize(), flat];
            vec![sphere(ankle, 0.034 * f), Shape::block(ankle + d4 * (0.085 * k), axes, Vec3::new(0.105 * f, 0.042 * f, 0.022) * k, 0.016 * k)]
        } else {
            let sole = -d4; // del pie hacia el tobillo
            let foot_axes = [toe, sole.cross(&toe).normalize(), sole];
            vec![
                sphere(ankle - sole * (0.035 * k) - toe * (0.025 * k), 0.034 * f),
                Shape::block(ankle - sole * (0.05 * k) + toe * (0.07 * k), foot_axes, Vec3::new(0.105 * f, 0.042 * f, 0.022) * k, 0.016 * k),
            ]
        };
    }

    let (head, carve) = head(sk);
    Body { hips, torso, neck, upper_arms, forearms, hands, thighs, shins, feet, head, carve }
}

/// Una mano: la palma, los dedos juntos (un poco curvados hacia la palma; en garra en el
/// cadáver) y el pulgar, desde la muñeca hacia `dir`.
fn hand(sk: &Skeleton, wrist: Vec3, dir: Vec3, palm: Vec3, i: usize) -> Vec<Shape> {
    let k = sk.k;
    let s = match sk.build {
        Build::Woman => 0.88,
        _ => 1.0,
    } * (1.0 + (sk.arm_scale - 1.0) * 0.4);
    let palm = perpendicular(palm, dir, sk.front);
    let thumb_side = if i == 0 { 1.0 } else { -1.0 };
    let across = dir.cross(&palm).normalize() * thumb_side;
    let (curl, finger_len) = if sk.build == Build::Corpse { (0.9, 0.05) } else { (0.35, 0.042) };
    let fingers = (dir + palm * curl).normalize();
    let knuckles = wrist + dir * (0.085 * s * k);
    vec![
        Shape::block(wrist + dir * (0.045 * s * k), [dir, across, palm], Vec3::new(0.043, 0.04, 0.014) * s * k, 0.01 * k),
        Shape::block(knuckles + fingers * (finger_len * s * k), [fingers, across, perpendicular(palm, fingers, palm)], Vec3::new(finger_len, 0.036, 0.011) * s * k, 0.009 * k),
        Shape::cone(wrist + across * (0.03 * s * k) + dir * (0.02 * s * k), wrist + across * (0.05 * s * k) + dir * (0.085 * s * k) + palm * (0.02 * s * k), 0.013 * s * k, 0.01 * s * k),
    ]
}

/// La cabeza y lo que se le talla.
fn head(sk: &Skeleton) -> (Vec<Shape>, Vec<Shape>) {
    let k = sk.k * sk.head_scale;
    let axes = sk.head_axes();
    let e = |c: Vec3, r: [f32; 3]| Shape::ellipsoid(c, axes, Vec3::new(r[0], r[1], r[2]) * k);
    let sphere = |c: Vec3, r: f32| Shape::sphere(c, r * k);
    let h = |u: f32, f: f32, a: f32| sk.head_at(u, f, a);
    let w = if sk.build == Build::Woman { 0.94 } else { 1.0 };
    let mut add = vec![
        e(h(0.035, -0.012, 0.0), [0.074 * w, 0.096 * w, 0.092 * w]), // cráneo
        e(h(-0.045, 0.03, 0.0), [0.06 * w, 0.058 * w, 0.072 * w]),   // cara
    ];
    let mut carve = Vec::new();
    for s in [-1.0f32, 1.0] {
        add.push(e(h(-0.012, -0.008, s * 0.076 * w), [0.012, 0.026, 0.032])); // orejas
    }
    match sk.build {
        Build::Man | Build::Woman => {
            let jaw = if sk.build == Build::Man { 0.028 } else { 0.023 };
            for s in [-1.0f32, 1.0] {
                add.push(sphere(h(-0.075, 0.0, s * 0.05 * w), jaw)); // ángulos de la mandíbula
                add.push(sphere(h(-0.012, 0.058, s * 0.045 * w), 0.024)); // pómulos
                carve.push(sphere(h(0.004, 0.09 * w, s * 0.033 * w), 0.019)); // cuencas
            }
            add.push(sphere(h(-0.108 * w, 0.06 * w, 0.0), if sk.build == Build::Man { 0.025 } else { 0.02 })); // mentón
            add.push(Shape::capsule(h(0.03, 0.082 * w, -0.042), h(0.03, 0.082 * w, 0.042), (if sk.build == Build::Man { 0.016 } else { 0.012 }) * k)); // cejas
            add.push(Shape::cone(h(0.016, 0.09 * w, 0.0), h(-0.034, 0.112 * w, 0.0), 0.009 * k, 0.016 * k)); // nariz
            // La boca abierta en un grito.
            carve.push(e(h(-0.076 * w, 0.095 * w, 0.0), [0.022, 0.03, 0.03]));
        }
        Build::Corpse => {
            for s in [-1.0f32, 1.0] {
                add.push(sphere(h(-0.012, 0.06, s * 0.05), 0.022)); // pómulos salientes
                add.push(sphere(h(-0.08, 0.0, s * 0.048), 0.022));
                add.push(Shape::capsule(h(-0.08, 0.0, s * 0.048), h(-0.135, 0.045, s * 0.012), 0.016 * k)); // la mandíbula, abierta
                carve.push(sphere(h(0.0, 0.088, s * 0.035), 0.024)); // cuencas hondas
                carve.push(sphere(h(-0.05, 0.078, s * 0.048), 0.02)); // mejillas hundidas
            }
            add.push(sphere(h(-0.138, 0.05, 0.0), 0.018));
            add.push(Shape::capsule(h(0.03, 0.084, -0.045), h(0.03, 0.084, 0.045), 0.017 * k));
            carve.push(sphere(h(-0.022, 0.1, 0.0), 0.015)); // el hueco de la nariz
            carve.push(e(h(-0.092, 0.085, 0.0), [0.03, 0.042, 0.045])); // la boca abierta
        }
    }
    (add, carve)
}

/// Dónde van los ojos (centro del globo), la boca (el hueco) y su fondo.
pub struct Face {
    pub eyes: [Vec3; 2],
    pub mouth: Vec3,
    pub mouth_back: Vec3,
}

pub fn face(sk: &Skeleton) -> Face {
    let w = if sk.build == Build::Woman { 0.94 } else { 1.0 };
    let (depth, mouth_u) = if sk.build == Build::Corpse { (0.068, -0.095) } else { (0.07 * w, -0.076 * w) };
    Face {
        eyes: [-1.0f32, 1.0].map(|s| sk.head_at(0.004, depth, s * 0.033 * w)),
        mouth: sk.head_at(mouth_u, 0.1 * w, 0.0),
        mouth_back: sk.head_at(mouth_u, if sk.build == Build::Corpse { 0.05 } else { 0.055 * w }, 0.0),
    }
}

/// ¿Está `p` dentro de la zona del pelo? Por encima de una línea que pasa sobre la frente,
/// baja por las sienes por encima de las orejas y llega a la nuca. `front` y `back` son
/// las alturas (en metros, sobre el centro de la cabeza) de la línea en la frente y en la
/// nuca.
pub fn hairline(sk: &Skeleton, p: Vec3, front: f32, back: f32) -> bool {
    let d = (p - sk.head) / sk.k;
    let (u, f) = (d.dot(&sk.crown), d.dot(&sk.face));
    let blend = ((f + 0.06) / 0.12).clamp(0.0, 1.0); // 0 en la nuca, 1 en la frente
    u > back + (front - back) * blend * blend
}

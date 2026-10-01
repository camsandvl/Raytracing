//! Los novios y el oficiante, como personas, en el instante del ataque: un zombi tiene a
//! la novia inclinada hacia atrás, como el novio que la inclina para besarla, pero
//! mordiéndole el cuello; el novio, con las manos a los costados de la cabeza, mira sin
//! poder creerlo; el oficiante levanta las manos del espanto y se le cae el libro.
//!
//! Las cuatro figuras de la boda (novios, oficiante y la mordida) están esculpidas
//! (`anatomy.rs`, `sculpt.rs`): cuerpos con proporciones y masas reales, la ropa en capas
//! sobre el cuerpo y la tela que cae por gravedad, a 2.5 cm (`zombies::SCULPT_DETAIL`).
//! Los padres de los novios, atacados a lo largo de la nave (las escenas las arma
//! `zombies::feast`), también están esculpidos, vestidos a la moda de una boda de los 80:
//! `Suit`/`suited_father` y `Dress`/`eighties_mother`.

use crate::scene::canvas::Canvas;
use crate::materials::MaterialSet;
use crate::ray_intersect::Material;
use crate::scene::anatomy::{body, face, hairline, Body, Build, Skeleton};
use crate::scene::figure::{frame, Pose, M, UP};
use crate::scene::sculpt::{along, perpendicular, stamp, Drape, Folds, Layer, Shape};
use crate::scene::zombies::SCULPT_DETAIL;
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::Vec3;

/// La piel de una figura esculpida, con los labios (`lips`) alrededor de la boca abierta.
pub fn skin_layer<'a>(sk: &'a Skeleton, body: &Body, skin: Material, lips: Material) -> Layer<'a> {
    let f = face(sk);
    let k = sk.k;
    // Las cuencas, en piel más oscura: a esta escala la sombra sola no alcanza para que se
    // lean los ojos bajo la luz de frente.
    let socket = Material { diffuse: skin.diffuse * 0.62, glow: skin.glow * 0.5, ..skin };
    Layer::new(body.all(), move |p| {
        let front = (p - sk.head).dot(&sk.face) > 0.07 * k;
        if front && (p - f.mouth).magnitude() < 0.034 * k {
            return Some(lips);
        }
        if front && f.eyes.iter().any(|&e| (p - e).magnitude() < 0.032 * k) {
            return Some(socket);
        }
        Some(skin)
    })
    .blend(Body::blend(sk))
    .carve(body.carve.clone(), 0.006 * k)
}

/// Los ojos (el blanco y el iris), el fondo oscuro de la boca, los dientes de arriba y
/// las cejas (si `brows`), sobre la cara de `sk`.
pub fn face_layers<'a>(m: &'a MaterialSet, sk: &'a Skeleton, brows: Option<Material>) -> Vec<Layer<'a>> {
    let f = face(sk);
    let k = sk.k;
    let mut layers = vec![
        Layer::solid(vec![Shape::sphere(f.mouth_back, 0.028 * k)], m.mouth),
        Layer::solid(vec![Shape::block(f.mouth_back + sk.crown * (0.024 * k) + sk.face * (0.03 * k), sk.head_axes(), Vec3::new(0.02, 0.008, 0.008) * k, 0.0)], m.wax),
    ];
    for eye in f.eyes {
        layers.push(Layer::new(vec![Shape::sphere(eye, 0.019 * k)], move |p| {
            let d = p - eye;
            Some(if d.dot(&sk.face) > 0.004 * k && d.dot(&sk.across).abs() < 0.011 * k { m.iris } else { m.wax })
        }));
    }
    if let Some(hair) = brows {
        for s in [-1.0f32, 1.0] {
            layers.push(Layer::solid(vec![Shape::capsule(sk.head_at(0.03, 0.088, s * 0.016), sk.head_at(0.036, 0.08, s * 0.05), 0.011 * k)], hair));
        }
    }
    layers
}

/// Una tira de tela plana (estola, cinta) de `a` a `b`, de `width` de ancho hacia
/// `across` y `thick` de espesor.
fn strap(a: Vec3, b: Vec3, across: Vec3, width: f32, thick: f32) -> Shape {
    let dir = (b - a).normalize();
    let across = perpendicular(across, dir, along(dir)[0]);
    let normal = dir.cross(&across).normalize();
    Shape::block((a + b) / 2.0, [across, normal, dir], Vec3::new(width / 2.0, thick / 2.0, (b - a).magnitude() / 2.0), thick / 3.0)
}

/// El esqueleto de la novia (1.68 m) inclinada hacia atrás sobre un pie, hacia `back`,
/// con los pies en `feet`: la cadera adelantada y el torso a 50° de la vertical (como en el
/// paso de baile, a la altura justa para que el zombi la alcance de pie, inclinado sobre
/// ella), la rodilla de adelante
/// levantada bajo la falda, la cabeza echada hacia atrás y girada hacia `-side` (hacia
/// quien llega por la nave), gritando: no cuelga del todo, para que la cara no quede
/// acostada de costado para quien la mira. El brazo del lado de `side` cae suelto hacia el
/// piso; el otro (`BRIDE_GRIP`) lo ubica quien arma la mordida, aferrado al zombi.
/// En el esqueleto, el índice 0 es el lado de `side`.
pub fn bride_skeleton(feet: Vec3, back: Vec3, side: Vec3) -> Skeleton {
    let k = M * 1.68 / 1.8;
    let pelvis = feet + UP * (0.85 * M) - back * (0.02 * M); // la cadera adelantada, hacia él
    let up = (UP * 0.64 + back * 0.77).normalize();
    let front = perpendicular(UP, up, -back);
    let raised = ((-back * 0.8 - UP * 0.15).normalize(), (-UP * 0.9 + back * 0.25).normalize());
    let pose = Pose { pelvis, up, front, arms: [(-UP, -UP); 2], legs: [(-UP, -UP), raised], look: UP };
    let mut sk = Skeleton::new(&pose, 1.68, Build::Woman);
    sk.turn_head(UP * 0.45 - side * 0.85, back + UP * 0.4);
    sk.reach_leg(0, feet + side * (0.05 * M) + UP * (0.075 * k), -back);
    let limp = sk.shoulders[0] - UP * (0.5 * k) + back * (0.08 * k) + side * (0.04 * k);
    sk.reach_arm(0, limp, -back + UP * 0.2);
    sk.toes = [-back, (-back - UP * 0.4).normalize()];
    sk
}

/// El brazo de la novia que se aferra al zombi (el del lado de `-side`).
pub const BRIDE_GRIP: usize = 1;

/// Dónde muerde el zombi: el costado del cuello que la cabeza girada deja al aire (arriba
/// y del lado contrario a la nave), y el frente de la cintura, adonde baja la sangre por el corpiño.
pub struct BrideDip {
    pub bite: Vec3,
    pub waist: Vec3,
}

pub fn bride_dip_points(sk: &Skeleton) -> BrideDip {
    let axis = (sk.neck_top - sk.neck).normalize();
    let out = perpendicular(UP * 0.6 - sk.side * 0.8, axis, sk.front); // `sk.side` apunta hacia la nave
    BrideDip { bite: (sk.neck + sk.neck_top) / 2.0 + out * (0.05 * sk.k), waist: sk.at(0.13, 0.1, 0.0) }
}

/// Estampa a la novia del esqueleto `sk` (ver `bride_skeleton`): el cuerpo, el corpiño
/// strapless de raso, la falda que cae por gravedad de la cadera al piso — apoyada sobre
/// la rodilla levantada, abriéndose en pliegues y juntándose en el piso — y la cola en
/// abanico hacia `back`, el pelo recogido con mechones sueltos que cuelgan, la cara
/// (ojos, boca abierta con labios pintados) y el velo. `pressed` son las piernas de quien
/// la sostiene: la falda y la cola se abren donde la empujan.
pub fn dipped_bride(c: &mut Canvas, m: &MaterialSet, sk: &Skeleton, feet: Vec3, back: Vec3, pressed: &[Shape]) {
    let k = sk.k;
    let b = body(sk);
    let grow = |shapes: &[Shape], t: f32| shapes.iter().map(|s| s.grow(t * k)).collect::<Vec<_>>();

    // Corpiño: escote corazón (más bajo al centro), la espalda un poco más baja.
    let bodice = {
        let mut shapes = grow(&b.torso, 0.012);
        shapes.extend(grow(&b.hips, 0.014));
        Layer::new(shapes, move |p| {
            let d = (p - sk.pelvis) / k;
            let (h, fwd, sd) = (d.dot(&sk.up), d.dot(&sk.front), d.dot(&sk.side));
            let neckline = if fwd > 0.0 { 0.392 - 0.03 * (1.0 - (sd.abs() / 0.07).min(1.0)) } else { 0.37 };
            (h < neckline).then_some(m.satin)
        })
        .blend(Body::blend(sk))
    };
    let ribbon = Layer::solid(vec![Shape::ellipsoid(sk.at(0.15, 0.004, 0.0), sk.torso_axes(), Vec3::new(0.128, 0.1, 0.022) * k)], m.wax);

    // La falda: cuelga de la cadera, de los dos muslos y de la rodilla levantada.
    let mut holders = grow(&b.hips, 0.035);
    for i in 0..2 {
        holders.extend(grow(&b.thighs[i], 0.03));
    }
    holders.push(b.shins[1][0].grow(0.035 * k)); // la rodilla
    let skirt = Layer::solid(holders, m.satin).blend(0.03 * k).drape(Drape {
        floor: feet.y,
        length: 0.95 * M,
        flare: 0.34,
        axis: feet + back * (0.05 * M),
        folds: 13.0,
        depth: 0.03 * M,
        pool: 0.06 * M,
        pool_height: 0.14 * M,
    })
    .carve(pressed.to_vec(), 0.02 * k);
    let side = sk.side;
    let train = Layer::solid(vec![Shape::ellipsoid(feet + back * (0.55 * M) + UP * (0.012 * M), [back, side, UP], Vec3::new(0.62, 0.4, 0.035) * M)], m.satin)
        .folds(Folds { axis: feet, count: 17.0, depth: 0.018 * M })
        .carve(pressed.to_vec(), 0.02 * k);

    // El pelo: recogido (con el rodete en la nuca) y unos mechones sueltos que cuelgan.
    let hair = Layer::new(vec![b.head[0].grow(0.016 * k)], move |p| hairline(sk, p, 0.05, -0.045).then_some(m.hair));
    let bun = sk.head_at(0.02, -0.105, 0.0);
    let mut locks = vec![Shape::sphere(bun, 0.048 * k)];
    for (u, f, a) in [(-0.02, 0.02, 0.07), (-0.02, 0.02, -0.07), (0.0, -0.11, 0.03)] {
        let root = sk.head_at(u, f, a);
        let tip = root - UP * (0.16 * k) + back * (0.02 * k);
        locks.push(Shape::cone(root, tip, 0.014 * k, 0.012 * k));
    }
    let locks = Layer::solid(locks, m.hair).blend(0.01 * k);

    let mut layers = vec![skin_layer(sk, &b, m.skin, m.rouge), bodice, ribbon, skirt, train, hair, locks];
    layers.extend(face_layers(m, sk, Some(m.hair)));
    stamp(c, &layers);

    // El velo: cae medio metro desde el rodete, abierto a lo ancho de la cabeza. Tela
    // semitransparente: no se esculpe ni se sombrea.
    let spread = perpendicular(back, UP, sk.across);
    for i in -2..=2 {
        let top = bun + spread * (i as f32 * 0.07 * M) + UP * (0.02 * M);
        c.capsule(top, top - UP * (0.5 * M) + back * (0.12 * M), 0.04 * M, &|_| m.veil);
    }
}

/// Un sombrero de ala ancha tirado en el piso en `spot`, con su cinta.
pub fn fallen_hat(c: &mut Canvas, spot: Vec3, felt: Material, ribbon: Material) {
    let paint = |material: Material| move |_: Vec3| material;
    c.cone(spot + UP * (0.01 * M), spot + UP * (0.035 * M), 0.2 * M, 0.2 * M, &paint(felt));
    c.cone(spot + UP * (0.03 * M), spot + UP * (0.12 * M), 0.1 * M, 0.09 * M, &paint(felt));
    c.cone(spot + UP * (0.04 * M), spot + UP * (0.065 * M), 0.105 * M, 0.105 * M, &paint(ribbon));
}

/// Un libro abierto tirado en el piso en `spot`, hacia `front`: tapas de cuero y las hojas
/// encima, con una hoja suelta al lado.
fn fallen_book(c: &mut Canvas, spot: Vec3, front: Vec3, cover: Material, pages: Material) {
    let side = UP.cross(&front).normalize();
    let paint = |material: Material| move |_: Vec3| material;
    c.cuboid(spot + UP * (0.01 * M), [side, UP, front], Vec3::new(0.2, 0.01, 0.16) * M, &paint(cover));
    c.cuboid(spot + UP * (0.025 * M), [side, UP, front], Vec3::new(0.17, 0.005, 0.13) * M, &paint(pages));
    let loose = spot + front * (0.3 * M) + side * (0.15 * M);
    c.cuboid(loose + UP * (0.005 * M), [side, UP, front], Vec3::new(0.09, 0.005, 0.12) * M, &paint(pages));
}

/// El novio (1.80 m, como el oficiante, de cadera angosta), de pie, con un pie atrás y las manos
/// sobre la cabeza, agarrándosela del espanto, con los codos abiertos (la cara queda a la
/// vista entre los brazos). El cuerpo mira hacia `front` (horizontal) y la cara hacia
/// `look`. Saco negro de lana con solapas de raso, abierto sobre la pechera blanca y
/// cortado a la cintura (sin colas: entre las piernas se veían raras), moño, boutonnière
/// roja, pantalón que cae recto y zapatos de charol.
pub fn groom(m: &MaterialSet, feet: Vec3, front: Vec3, look: Vec3) -> VoxelGrid {
    let (up, front) = frame(UP, front);
    let side = up.cross(&front).normalize();
    let legs = [
        ((-up * 0.92 - front * 0.3 - side * 0.12).normalize(), (-up * 0.97 - front * 0.12 - side * 0.04).normalize()), // el que queda atrás
        ((-up * 0.98 + front * 0.1 + side * 0.14).normalize(), (-up + side * 0.04).normalize()),                       // el que queda plantado
    ];
    let height = 1.8; // como el oficiante (1.78 m)
    let pose = Pose { pelvis: feet + up * (0.95 * M * height / 1.8), up, front, arms: [(-up, -up); 2], legs, look };
    let mut sk = Skeleton::new(&pose, height, Build::Man);
    for i in 0..2 {
        let s = if i == 0 { -1.0 } else { 1.0 };
        // Las manos sobre la cabeza, detrás de la cara; los codos abiertos a los costados
        // (un poco hacia adelante), así los brazos enmarcan la cara en vez de taparla.
        let wrist = sk.head_at(0.02, -0.035, s * 0.115);
        sk.reach_arm(i, wrist, front * 0.35 + side * (s * 0.85) - up * 0.25);
        sk.palms[i] = sk.head - sk.wrists[i];
    }
    sk.toes = [(front - side * 0.25).normalize(), (front + side * 0.2).normalize()];
    let k = sk.k;
    let b = body(&sk);
    let grow = |shapes: &[Shape], t: f32| shapes.iter().map(|s| s.grow(t * k)).collect::<Vec<_>>();

    // Pantalón: la cadera y los muslos, y de la rodilla al zapato un caño recto.
    let mut trousers = grow(&b.hips, 0.01);
    for i in 0..2 {
        trousers.extend(grow(&b.thighs[i], 0.01));
        trousers.push(Shape::cone(sk.knees[i], sk.ankles[i] - (sk.ankles[i] - sk.knees[i]).normalize() * (0.02 * k), 0.056 * k, 0.052 * k));
    }
    let trousers = Layer::solid(trousers, m.tux).blend(0.01 * k);
    let shoes = Layer::solid(b.feet.iter().flat_map(|f| grow(f, 0.012)).collect(), m.lapel).blend(0.01 * k);

    // La camisa (se ve en la pechera, el cuello y los puños).
    let mut shirt = grow(&b.torso, 0.008);
    for i in 0..2 {
        shirt.extend(grow(&b.forearms[i], 0.01));
    }
    shirt.push(Shape::cone(sk.neck - up * (0.01 * k), sk.neck + (sk.neck_top - sk.neck) * 0.55, 0.066 * k, 0.06 * k));
    let skr = &sk;
    let shirt = Layer::new(shirt, move |p| {
        let below = -(p - skr.neck).dot(&skr.up) / k;
        Some(if below > 0.38 { m.lapel } else { m.satin }) // la faja negra a la cintura
    })
    .blend(Body::blend(&sk));

    // El saco: abierto en V sobre la pechera, con solapas; cortado por delante a la
    // altura de la cintura (el frac); las mangas dejan ver los puños.
    let mut coat = grow(&b.torso, 0.018);
    coat.extend(grow(&b.hips, 0.014));
    for i in 0..2 {
        coat.extend(grow(&b.upper_arms[i], 0.018));
        coat.extend(grow(&b.forearms[i], 0.016));
    }
    let (wrists, elbows) = (sk.wrists, sk.elbows);
    let coat = Layer::new(coat, move |p| {
        let d = (p - skr.neck) / k;
        let (below, fwd, sd) = (-d.dot(&skr.up), d.dot(&skr.front), d.dot(&skr.side));
        let opening = 0.09 * (1.0 - below / 0.34);
        for i in 0..2 {
            let dir = (wrists[i] - elbows[i]).normalize();
            if (p - wrists[i]).dot(&dir) > -0.04 * k && (p - wrists[i]).magnitude() < 0.09 * k {
                return None; // el puño de la camisa
            }
        }
        if fwd > 0.02 && below > 0.43 {
            return None; // el frac, cortado por delante
        }
        if fwd > -0.05 && below > 0.5 {
            return None; // ni a los costados de la cadera: solo cubre el asiento, por detrás
        }
        if fwd > 0.02 && below > -0.03 && below < 0.34 && sd.abs() < opening {
            return None; // la pechera
        }
        if fwd > 0.02 && below < 0.37 && sd.abs() < opening.max(0.0) + 0.04 {
            return Some(m.lapel);
        }
        Some(m.tux)
    })
    .blend(Body::blend(&sk));
    let bow = sk.neck + up * (0.045 * k) + front * (0.062 * k);
    let tie = Layer::solid(vec![Shape::block(bow, sk.torso_axes(), Vec3::new(0.05, 0.012, 0.02) * k, 0.008 * k)], m.lapel);
    let flower = Layer::solid(vec![Shape::sphere(sk.at(0.45, 0.13, -0.1), 0.024 * k)], m.petal);

    let hair = Layer::new(vec![b.head[0].grow(0.014 * k), Shape::ellipsoid(sk.head_at(0.07, 0.01, 0.0), sk.head_axes(), Vec3::new(0.07, 0.085, 0.045) * k)], move |p| {
        hairline(skr, p, 0.056, -0.05).then_some(m.hair)
    })
    .blend(0.02 * k);

    let mut layers = vec![skin_layer(&sk, &b, m.skin, m.lips), trousers, shoes, shirt, coat, tie, flower, hair];
    layers.extend(face_layers(m, &sk, Some(m.hair)));
    let radius = 0.75 * M;
    let mut c = Canvas::new(feet - Vec3::new(radius, 0.0, radius), feet + Vec3::new(radius, 2.1 * M, radius), SCULPT_DETAIL);
    stamp(&mut c, &layers);
    sculpted(c)
}

/// El oficiante (1.78 m), de pie, con las dos manos levantadas del espanto (las palmas
/// hacia adelante) y la cara hacia arriba, y el libro abierto tirado en el piso delante
/// de él. De alba de lino crudo, que cae de la cadera al piso en pliegues, con las mangas
/// anchas que se le deslizan hacia los hombros (los antebrazos quedan al aire), y la
/// estola bordó que cuelga del cuello. Canoso y pelado arriba.
pub fn officiant(m: &MaterialSet, feet: Vec3, front: Vec3) -> VoxelGrid {
    let (up, front) = frame(UP, front);
    let side = up.cross(&front).normalize();
    let legs = [-1.0f32, 1.0].map(|s| ((-up + side * (s * 0.04)).normalize(), -up));
    let arms = [-1.0f32, 1.0].map(|s| ((up * 0.85 + side * (s * 0.32) + front * 0.1).normalize(), (up * 0.95 + side * (s * 0.12) + front * 0.12).normalize()));
    let look = (up * 0.55 + front * 0.45).normalize();
    let pose = Pose { pelvis: feet + up * (0.94 * M), up, front, arms, legs, look };
    let mut sk = Skeleton::new(&pose, 1.78, Build::Man);
    sk.palms = [front, front];
    let k = sk.k;
    let b = body(&sk);
    let grow = |shapes: &[Shape], t: f32| shapes.iter().map(|s| s.grow(t * k)).collect::<Vec<_>>();
    let shoes = Layer::solid(b.feet.iter().flat_map(|f| grow(f, 0.012)).collect(), m.lapel).blend(0.01 * k);

    let mut alb = grow(&b.torso, 0.028);
    alb.push(Shape::cone(sk.neck - up * (0.01 * k), sk.neck + (sk.neck_top - sk.neck) * 0.4, 0.07 * k, 0.065 * k));
    let alb = Layer::solid(alb, m.wax).blend(Body::blend(&sk));
    // Las mangas: cuelgan del brazo y del codo, juntándose sobre el hombro.
    let mut sleeves = Vec::new();
    for i in 0..2 {
        sleeves.extend(grow(&b.upper_arms[i], 0.02));
        let dir = (sk.wrists[i] - sk.elbows[i]).normalize();
        sleeves.push(Shape::cone(sk.elbows[i], sk.elbows[i] + dir * (0.07 * k), 0.058 * k, 0.054 * k));
    }
    let sleeves = Layer::solid(sleeves, m.wax).blend(0.02 * k).drape(Drape {
        floor: feet.y,
        length: 0.08 * M,
        flare: 0.25,
        axis: sk.neck,
        folds: 9.0,
        depth: 0.012 * M,
        pool: 0.0,
        pool_height: 1.0,
    });
    let mut holders = grow(&b.hips, 0.04);
    holders.push(b.torso[0].grow(0.03 * k)); // el abdomen: el alba no se abre en la cintura
    for i in 0..2 {
        holders.extend(grow(&b.thighs[i], 0.035));
    }
    let skirt = Layer::solid(holders, m.wax).blend(0.03 * k).drape(Drape {
        floor: feet.y,
        length: 1.0 * M,
        flare: 0.1,
        axis: sk.pelvis,
        folds: 10.0,
        depth: 0.022 * M,
        pool: 0.03 * M,
        pool_height: 0.1 * M,
    });
    // La estola: sobre los hombros, baja por el pecho y cuelga recta hasta las rodillas.
    let mut stole = Vec::new();
    for s in [-1.0f32, 1.0] {
        let path = [
            sk.at(0.55, -0.03, s * 0.075),
            sk.at(0.44, 0.14, s * 0.082),
            sk.at(0.12, 0.155, s * 0.09),
            sk.at(0.12, 0.155, s * 0.09) - UP * (0.52 * k) + front * (0.11 * k),
        ];
        for w in path.windows(2) {
            stole.push(strap(w[0], w[1], side, 0.075 * k, 0.03 * k));
        }
    }
    let stole = Layer::solid(stole, m.leather).blend(0.02 * k);
    let skr = &sk;
    let hair = Layer::new(vec![b.head[0].grow(0.012 * k)], move |p| {
        let d = (p - skr.head) / k;
        let (u, f) = (d.dot(&skr.crown), d.dot(&skr.face));
        (u < 0.035 && u > -0.06 && f < 0.035).then_some(m.hair_grey)
    });

    let mut layers = vec![skin_layer(&sk, &b, m.skin, m.lips), shoes, alb, sleeves, skirt, stole, hair];
    layers.extend(face_layers(m, &sk, Some(m.hair_grey)));
    let radius = 0.75 * M;
    let reach = Vec3::new(front.x.abs(), 0.0, front.z.abs()) * (0.5 * M);
    let mut c = Canvas::new(feet - Vec3::new(radius, 0.0, radius) - reach, feet + Vec3::new(radius, 2.35 * M, radius) + reach, SCULPT_DETAIL);
    stamp(&mut c, &layers);
    fallen_book(&mut c, feet + front * (0.5 * M), front, m.leather, m.wax);
    sculpted(c)
}

/// El traje de un padre de los novios (a la moda de los 80).
pub struct Suit {
    /// La tela del saco y el pantalón.
    pub cloth: Material,
    /// Corbata (`false`) o moño (`true`), del color `accent`.
    pub bow_tie: bool,
    pub accent: Material,
    /// Esmoquin: saco abierto en V hasta la cintura, pechera con volados y faja (del color
    /// `accent`); si no, saco cerrado con la V angosta y hombreras.
    pub tuxedo: bool,
    pub hair: Material,
}

impl Suit {
    /// El padre del novio: traje azul marino con hombreras y corbata fucsia, pelo castaño.
    pub fn groom_father(m: &MaterialSet) -> Self {
        Suit { cloth: m.suit_navy, bow_tie: false, accent: m.dress_fuchsia, tuxedo: false, hair: m.hair }
    }

    /// El padre de la novia: esmoquin gris perla, pechera con volados, moño y faja bordó,
    /// pelo canoso.
    pub fn bride_father(m: &MaterialSet) -> Self {
        Suit { cloth: m.tux_grey, bow_tie: true, accent: m.leather, tuxedo: true, hair: m.hair_grey }
    }
}

/// Un padre de los novios, esculpido en el esqueleto `sk` (la pose la arma
/// `zombies::feast`), vestido con `suit`: pantalón que cae recto, zapatos negros, la
/// camisa blanca, el saco, la corbata o el moño, pelo corto y bigote, gritando.
pub fn suited_father(c: &mut Canvas, m: &MaterialSet, sk: &Skeleton, suit: &Suit) {
    let k = sk.k;
    let b = body(sk);
    let grow = |shapes: &[Shape], t: f32| shapes.iter().map(|s| s.grow(t * k)).collect::<Vec<_>>();

    let mut trousers = grow(&b.hips, 0.012);
    for i in 0..2 {
        trousers.extend(grow(&b.thighs[i], 0.012));
        trousers.push(Shape::cone(sk.knees[i], sk.ankles[i] - (sk.ankles[i] - sk.knees[i]).normalize() * (0.02 * k), 0.058 * k, 0.054 * k));
    }
    let trousers = Layer::solid(trousers, suit.cloth).blend(0.012 * k);
    let shoes = Layer::solid(b.feet.iter().flat_map(|f| grow(f, 0.012)).collect(), m.lapel).blend(0.01 * k);

    let mut shirt = grow(&b.torso, 0.008);
    for i in 0..2 {
        shirt.extend(grow(&b.forearms[i], 0.01));
    }
    shirt.push(Shape::cone(sk.neck - sk.up * (0.01 * k), sk.neck + (sk.neck_top - sk.neck) * 0.55, 0.066 * k, 0.06 * k));
    let (accent, tuxedo) = (suit.accent, suit.tuxedo);
    let shirt = Layer::new(shirt, move |p| {
        let below = -(p - sk.neck).dot(&sk.up) / k;
        Some(if tuxedo && below > 0.36 { accent } else { m.satin }) // la faja del esmoquin
    })
    .blend(Body::blend(sk));

    // El saco: cerrado con la V angosta y hombreras, o (esmoquin) abierto en V hasta la
    // cintura; las mangas dejan ver los puños de la camisa.
    let mut jacket = grow(&b.torso, 0.02);
    jacket.extend(grow(&b.hips, 0.024));
    for i in 0..2 {
        jacket.extend(grow(&b.upper_arms[i], 0.016));
        jacket.extend(grow(&b.forearms[i], 0.014));
        if !tuxedo {
            jacket.push(Shape::sphere(sk.shoulders[i] + sk.up * (0.015 * k), 0.068 * k)); // hombreras
        }
    }
    let (wrists, elbows, cloth) = (sk.wrists, sk.elbows, suit.cloth);
    let (depth, width) = if tuxedo { (0.36, 0.09) } else { (0.27, 0.07) };
    let jacket = Layer::new(jacket, move |p| {
        let d = (p - sk.neck) / k;
        let (below, fwd, sd) = (-d.dot(&sk.up), d.dot(&sk.front), d.dot(&sk.side));
        for i in 0..2 {
            let dir = (wrists[i] - elbows[i]).normalize();
            if (p - wrists[i]).dot(&dir) > -0.035 * k && (p - wrists[i]).magnitude() < 0.09 * k {
                return None; // el puño de la camisa
            }
        }
        if fwd > 0.02 && below > -0.03 && below < depth && sd.abs() < width * (1.0 - below / depth) {
            return None; // la V
        }
        if tuxedo && fwd > 0.02 && below > 0.43 {
            return None; // el esmoquin abierto deja ver la faja y el pantalón
        }
        Some(cloth)
    })
    .blend(Body::blend(sk));

    let mut extras = Vec::new();
    if suit.bow_tie {
        let bow = sk.neck + sk.up * (0.045 * k) + sk.front * (0.062 * k);
        extras.push(Layer::solid(vec![Shape::block(bow, sk.torso_axes(), Vec3::new(0.05, 0.012, 0.02) * k, 0.008 * k)], suit.accent));
    } else {
        extras.push(Layer::solid(vec![strap(sk.at(0.5, 0.09, 0.0), sk.at(0.29, 0.13, 0.0), sk.side, 0.045 * k, 0.026 * k)], suit.accent));
    }
    if tuxedo {
        // La pechera con volados: unos rulos de tela blanca bajando por el centro.
        let ruffles = (0..5).map(|n| Shape::sphere(sk.at(0.47 - 0.05 * n as f32, 0.112, 0.0), 0.022 * k)).collect();
        extras.push(Layer::solid(ruffles, m.satin).blend(0.01 * k));
    }

    let hair = suit.hair;
    let hair_layer = Layer::new(vec![b.head[0].grow(0.013 * k)], move |p| hairline(sk, p, 0.05, -0.05).then_some(hair)).blend(0.02 * k);
    let mustache = Layer::solid(vec![Shape::block(sk.head_at(-0.05, 0.1, 0.0), sk.head_axes(), Vec3::new(0.03, 0.01, 0.009) * k, 0.004 * k)], hair);

    let mut layers = vec![skin_layer(sk, &b, m.skin, m.lips), trousers, shoes, shirt, jacket];
    layers.extend(extras);
    layers.extend([hair_layer, mustache]);
    layers.extend(face_layers(m, sk, Some(hair)));
    stamp(c, &layers);
}

/// El vestido de una madre de los novios (a la moda de los 80).
pub struct Dress {
    pub satin: Material,
    pub hair: Material,
    /// Mangas abullonadas enormes y los antebrazos al aire (`true`), o mangas largas con
    /// hombreras (`false`).
    pub puffed: bool,
}

impl Dress {
    /// La madre de la novia: raso fucsia con mangas abullonadas, permanente castaña.
    pub fn bride_mother(m: &MaterialSet) -> Self {
        Dress { satin: m.dress_fuchsia, hair: m.hair, puffed: true }
    }

    /// La madre del novio: verde azulado con hombreras y mangas largas, permanente rubia.
    pub fn groom_mother(m: &MaterialSet) -> Self {
        Dress { satin: m.dress_teal, hair: m.hair_blonde, puffed: false }
    }
}

/// Una madre de los novios, esculpida en el esqueleto `sk` (la pose la arma
/// `zombies::feast`), con el piso a la altura `floor`: el vestido `dress` con la falda
/// hasta la rodilla, que cae por su peso hasta el piso y se junta ahí (esté como esté
/// ella: boca abajo, sentada), collar de perlas, permanente, zapatos negros y los labios
/// pintados.
pub fn eighties_mother(c: &mut Canvas, m: &MaterialSet, sk: &Skeleton, floor: f32, dress: &Dress) {
    let k = sk.k;
    let b = body(sk);
    let grow = |shapes: &[Shape], t: f32| shapes.iter().map(|s| s.grow(t * k)).collect::<Vec<_>>();
    let satin = dress.satin;

    let bodice = Layer::new(grow(&b.torso, 0.012), move |p| ((p - sk.pelvis).dot(&sk.up) / k < 0.47).then_some(satin)).blend(Body::blend(sk));
    let mut sleeves = Vec::new();
    for i in 0..2 {
        sleeves.extend(grow(&b.upper_arms[i], 0.014));
        if dress.puffed {
            let along = (sk.elbows[i] - sk.shoulders[i]).normalize();
            sleeves.push(Shape::ellipsoid(sk.shoulders[i] + along * (0.08 * k), crate::scene::sculpt::along(along), Vec3::new(0.075, 0.075, 0.1) * k));
        } else {
            sleeves.extend(grow(&b.forearms[i], 0.012));
            sleeves.push(Shape::sphere(sk.shoulders[i] + sk.up * (0.015 * k), 0.062 * k)); // hombreras
        }
    }
    let sleeves = Layer::solid(sleeves, satin).blend(0.02 * k);
    let mut holders = grow(&b.hips, 0.03);
    for i in 0..2 {
        holders.extend(grow(&b.thighs[i], 0.026));
    }
    let skirt = Layer::solid(holders, satin).blend(0.025 * k).drape(Drape {
        floor,
        length: 0.45 * M,
        flare: 0.3,
        axis: sk.pelvis,
        folds: 9.0,
        depth: 0.02 * M,
        pool: 0.03 * M,
        pool_height: 0.06 * M,
    });
    let shoes = Layer::solid(b.feet.iter().flat_map(|f| grow(f, 0.01)).collect(), m.lapel).blend(0.01 * k);

    let neck_axis = (sk.neck_top - sk.neck).normalize();
    let ring_x = perpendicular(sk.front, neck_axis, sk.side);
    let ring_y = neck_axis.cross(&ring_x);
    let pearls = (0..12)
        .map(|n| {
            let angle = n as f32 / 12.0 * std::f32::consts::TAU;
            Shape::sphere(sk.neck + neck_axis * (0.015 * k) + (ring_x * angle.cos() + ring_y * angle.sin()) * (0.058 * k), 0.013 * k)
        })
        .collect();
    let pearls = Layer::solid(pearls, m.wax);

    let perm = vec![
        b.head[0].grow(0.035 * k),
        Shape::sphere(sk.head_at(-0.03, -0.01, 0.075), 0.05 * k),
        Shape::sphere(sk.head_at(-0.03, -0.01, -0.075), 0.05 * k),
        Shape::sphere(sk.head_at(-0.04, -0.07, 0.0), 0.06 * k),
    ];
    let hair = dress.hair;
    let hair_layer = Layer::new(perm, move |p| hairline(sk, p, 0.045, -0.11).then_some(hair)).blend(0.025 * k);

    let mut layers = vec![skin_layer(sk, &b, m.skin, m.rouge), bodice, sleeves, skirt, shoes, pearls, hair_layer];
    layers.extend(face_layers(m, sk, Some(hair)));
    stamp(c, &layers);
}

/// El grid de una figura esculpida, con la oclusión horneada (los pliegues, las axilas,
/// las cuencas y la boca se oscurecen) y las normales suavizadas (la luz sigue la forma
/// esculpida y no la escalera de cubos).
pub fn sculpted(c: Canvas) -> VoxelGrid {
    let mut grid = c.into_grid();
    grid.bake_occlusion(3, 1.2);
    grid.smooth_normals(4);
    grid
}

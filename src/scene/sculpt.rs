//! Escultura por campos de distancia para las figuras de la boda. En vez de estampar
//! cápsulas sueltas (que se leen como caños pegados), cada figura es "arcilla": formas
//! simples (elipsoides, conos redondeados, bloques) que se funden entre sí con una unión
//! suave, así el hombro se mezcla con el pecho y el muslo con la cadera, y se pueden
//! tallar huecos (las cuencas, la boca abierta) con una resta suave.
//!
//! Una figura se arma en capas (`Layer`), de adentro hacia afuera: la piel, la camisa, el
//! saco... Cada capa pinta sus celdas encima de las anteriores, y su pintura puede dejar
//! celdas sin pintar (`None`): el saco abierto deja ver la camisa.
//!
//! La tela que cuelga (`Drape`) cae por gravedad desde lo que la sostiene: la falda cuelga
//! de la cadera hasta el piso y se apoya sobre la rodilla levantada, abriéndose y
//! formando pliegues a medida que baja, y se junta sobre el piso.
//!
//! Todo en coordenadas de mundo (1 unidad = un vóxel de 20 cm de la catedral).

use crate::ray_intersect::Material;
use crate::scene::canvas::Canvas;
use crate::scene::figure::UP;
use nalgebra_glm::Vec3;

/// Una forma de arcilla.
#[derive(Clone, Copy)]
pub enum Shape {
    /// Elipsoide con sus tres ejes (unitarios, perpendiculares) y sus tres radios.
    Ellipsoid { center: Vec3, axes: [Vec3; 3], radii: Vec3 },
    /// Cono redondeado de `a` (radio `ra`) a `b` (radio `rb`); con los dos radios iguales
    /// es una cápsula, y con `a == b` una esfera.
    Cone { a: Vec3, b: Vec3, ra: f32, rb: f32 },
    /// Caja orientada con las esquinas redondeadas (`round`), mitad de tamaño `half`.
    Block { center: Vec3, axes: [Vec3; 3], half: Vec3, round: f32 },
}

impl Shape {
    pub fn sphere(center: Vec3, r: f32) -> Shape {
        Shape::Cone { a: center, b: center, ra: r, rb: r }
    }

    pub fn capsule(a: Vec3, b: Vec3, r: f32) -> Shape {
        Shape::Cone { a, b, ra: r, rb: r }
    }

    pub fn cone(a: Vec3, b: Vec3, ra: f32, rb: f32) -> Shape {
        Shape::Cone { a, b, ra, rb }
    }

    pub fn ellipsoid(center: Vec3, axes: [Vec3; 3], radii: Vec3) -> Shape {
        Shape::Ellipsoid { center, axes, radii }
    }

    pub fn block(center: Vec3, axes: [Vec3; 3], half: Vec3, round: f32) -> Shape {
        Shape::Block { center, axes, half, round }
    }

    /// Distancia (aproximada, con signo: negativa adentro) de `p` a la superficie.
    pub fn dist(&self, p: Vec3) -> f32 {
        match *self {
            Shape::Ellipsoid { center, axes, radii } => {
                let d = p - center;
                let local = Vec3::new(d.dot(&axes[0]), d.dot(&axes[1]), d.dot(&axes[2]));
                let k0 = local.component_div(&radii).magnitude();
                let k1 = local.component_div(&radii.component_mul(&radii)).magnitude();
                if k1 < 1e-6 {
                    -radii.min()
                } else {
                    k0 * (k0 - 1.0) / k1
                }
            }
            Shape::Cone { a, b, ra, rb } => {
                let ab = b - a;
                let len2 = ab.dot(&ab);
                let t = if len2 < 1e-9 { 0.0 } else { ((p - a).dot(&ab) / len2).clamp(0.0, 1.0) };
                (p - (a + ab * t)).magnitude() - (ra + (rb - ra) * t)
            }
            Shape::Block { center, axes, half, round } => {
                let d = p - center;
                let q = Vec3::new(d.dot(&axes[0]).abs(), d.dot(&axes[1]).abs(), d.dot(&axes[2]).abs()) - half + Vec3::repeat(round);
                q.sup(&Vec3::zeros()).magnitude() + q.x.max(q.y).max(q.z).min(0.0) - round
            }
        }
    }

    /// La misma forma, `t` más gruesa: la ropa sobre el cuerpo.
    pub fn grow(self, t: f32) -> Shape {
        match self {
            Shape::Ellipsoid { center, axes, radii } => Shape::Ellipsoid { center, axes, radii: radii + Vec3::repeat(t) },
            Shape::Cone { a, b, ra, rb } => Shape::Cone { a, b, ra: ra + t, rb: rb + t },
            Shape::Block { center, axes, half, round } => Shape::Block { center, axes, half: half + Vec3::repeat(t), round: round + t },
        }
    }

    fn bounds(&self) -> (Vec3, Vec3) {
        let (lo, hi) = match *self {
            Shape::Ellipsoid { center, radii, .. } => (center - Vec3::repeat(radii.max()), center + Vec3::repeat(radii.max())),
            Shape::Cone { a, b, ra, rb } => {
                let r = Vec3::repeat(ra.max(rb));
                (a.inf(&b) - r, a.sup(&b) + r)
            }
            Shape::Block { center, half, .. } => (center - Vec3::repeat(half.magnitude()), center + Vec3::repeat(half.magnitude())),
        };
        (lo, hi)
    }
}

/// Unión suave: como `min`, pero redondea el encuentro en un radio `k`.
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 || !a.is_finite() || !b.is_finite() {
        return a.min(b);
    }
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

/// Tela que cuelga: hasta `length` por debajo de lo que la sostiene (sin pasar del piso,
/// a la altura `floor`), abriéndose `flare` por cada unidad que baja, con `folds`
/// pliegues de hondura `depth` alrededor del eje vertical que pasa por `axis` (más
/// hondos cuanto más cae), y juntándose en el piso (`pool` más ancha en los últimos
/// `pool_height`).
#[derive(Clone, Copy)]
pub struct Drape {
    pub floor: f32,
    pub length: f32,
    pub flare: f32,
    pub axis: Vec3,
    pub folds: f32,
    pub depth: f32,
    pub pool: f32,
    pub pool_height: f32,
}

/// Pliegues fijos (sin caída) alrededor del eje vertical por `axis`: la cola del vestido
/// sobre el piso, que se abre en abanico.
#[derive(Clone, Copy)]
pub struct Folds {
    pub axis: Vec3,
    pub count: f32,
    pub depth: f32,
}

/// Una capa de la figura: la unión suave de `shapes` menos la de `carve`, o lo que cuelga
/// de ellas si tiene `drape`. `paint` elige el material de cada celda adentro (o la deja
/// como estaba con `None`).
pub struct Layer<'a> {
    shapes: Vec<Shape>,
    blend: f32,
    carve: Vec<Shape>,
    carve_blend: f32,
    drape: Option<Drape>,
    folds: Option<Folds>,
    paint: Box<dyn Fn(Vec3) -> Option<Material> + 'a>,
}

impl<'a> Layer<'a> {
    pub fn new(shapes: Vec<Shape>, paint: impl Fn(Vec3) -> Option<Material> + 'a) -> Self {
        Layer { shapes, blend: 0.0, carve: Vec::new(), carve_blend: 0.0, drape: None, folds: None, paint: Box::new(paint) }
    }

    /// Una capa de un solo material.
    pub fn solid(shapes: Vec<Shape>, material: Material) -> Self {
        Layer::new(shapes, move |_| Some(material))
    }

    pub fn blend(mut self, k: f32) -> Self {
        self.blend = k;
        self
    }

    pub fn carve(mut self, shapes: Vec<Shape>, k: f32) -> Self {
        self.carve = shapes;
        self.carve_blend = k;
        self
    }

    pub fn drape(mut self, drape: Drape) -> Self {
        self.drape = Some(drape);
        self
    }

    pub fn folds(mut self, folds: Folds) -> Self {
        self.folds = Some(folds);
        self
    }

    /// La unión suave de las formas (sin tallar).
    fn union_dist(&self, p: Vec3) -> f32 {
        self.shapes.iter().fold(f32::INFINITY, |d, s| smin(d, s.dist(p), self.blend))
    }

    fn dist(&self, p: Vec3, step: f32) -> f32 {
        let d = self.draped_dist(p, step);
        if self.carve.is_empty() {
            return d;
        }
        // Lo tallado se resta donde termina la forma (o la tela), no donde cuelga: una
        // pierna que empuja la falda la abre ahí mismo.
        let hole = self.carve.iter().fold(f32::INFINITY, |d, s| smin(d, s.dist(p), self.carve_blend));
        -smin(-d, hole, self.carve_blend)
    }

    fn draped_dist(&self, p: Vec3, step: f32) -> f32 {
        let around = |axis: Vec3| {
            let h = p - axis;
            h.z.atan2(h.x)
        };
        let Some(drape) = self.drape else {
            let d = self.union_dist(p);
            return match self.folds {
                Some(f) => d + f.depth * (f.count * around(f.axis)).sin(),
                None => d,
            };
        };
        if p.y < drape.floor {
            return f32::INFINITY;
        }
        // Lo que está a `fall` por encima de `p` sostiene la tela que pasa por `p`.
        let (mut best, mut best_fall) = (f32::INFINITY, 0.0);
        let mut fall = 0.0;
        while fall <= drape.length {
            let d = self.union_dist(p + UP * fall) - drape.flare * fall;
            if d < best {
                (best, best_fall) = (d, fall);
            }
            fall += step;
        }
        let deepening = (best_fall / drape.length).clamp(0.0, 1.0).sqrt();
        let fold = drape.depth * deepening * (drape.folds * around(drape.axis)).sin();
        let pooling = drape.pool * (1.0 - (p.y - drape.floor) / drape.pool_height).clamp(0.0, 1.0);
        best + fold - pooling
    }

    fn bounds(&self) -> (Vec3, Vec3) {
        let (mut lo, mut hi) = self.shapes.iter().map(Shape::bounds).fold(
            (Vec3::repeat(f32::INFINITY), Vec3::repeat(f32::NEG_INFINITY)),
            |(lo, hi), (a, b)| (lo.inf(&a), hi.sup(&b)),
        );
        let pad = self.folds.map_or(0.0, |f| f.depth);
        if let Some(d) = self.drape {
            let spread = d.flare * d.length + d.depth + d.pool;
            lo -= Vec3::new(spread, 0.0, spread);
            hi += Vec3::new(spread, 0.0, spread);
            lo.y = d.floor.max(lo.y - d.length);
        }
        (lo - Vec3::repeat(pad), hi + Vec3::repeat(pad))
    }
}

/// Estampa las capas en orden: cada una pinta encima de las anteriores.
pub fn stamp(c: &mut Canvas, layers: &[Layer]) {
    let step = c.cell_size() * 0.75;
    for layer in layers {
        let (lo, hi) = layer.bounds();
        c.fill(lo, hi, &|p| if layer.dist(p, step) <= 0.0 { (layer.paint)(p) } else { None });
    }
}

/// Tres ejes perpendiculares con el tercero a lo largo de `dir` (un brazo, una pierna).
pub fn along(dir: Vec3) -> [Vec3; 3] {
    let dir = dir.normalize();
    let helper = if dir.y.abs() < 0.9 { UP } else { Vec3::new(1.0, 0.0, 0.0) };
    let a = helper.cross(&dir).normalize();
    [a, dir.cross(&a), dir]
}

/// `v` sin su componente a lo largo de `axis`, normalizado (o `fallback` si queda nulo).
pub fn perpendicular(v: Vec3, axis: Vec3, fallback: Vec3) -> Vec3 {
    let w = v - axis * v.dot(&axis);
    if w.magnitude() < 1e-4 {
        fallback
    } else {
        w.normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_are_negative_inside_and_positive_outside() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let axes = [x, UP, Vec3::new(0.0, 0.0, 1.0)];
        let e = Shape::ellipsoid(Vec3::zeros(), axes, Vec3::new(2.0, 1.0, 1.0));
        assert!(e.dist(Vec3::new(1.5, 0.0, 0.0)) < 0.0);
        assert!(e.dist(Vec3::new(0.0, 1.5, 0.0)) > 0.0);
        let b = Shape::block(Vec3::zeros(), axes, Vec3::new(1.0, 1.0, 1.0), 0.2);
        assert!(b.dist(Vec3::new(0.9, 0.0, 0.0)) < 0.0);
        assert!(b.dist(Vec3::new(0.98, 0.98, 0.98)) > 0.0, "las esquinas están redondeadas");
        assert!(Shape::capsule(Vec3::zeros(), x * 2.0, 0.5).dist(x * 2.4) < 0.0);
    }

    #[test]
    fn smooth_union_fills_the_crease_between_two_shapes() {
        let (a, b) = (Shape::sphere(Vec3::new(-1.0, 0.0, 0.0), 1.0), Shape::sphere(Vec3::new(1.0, 0.0, 0.0), 1.0));
        let p = Vec3::new(0.0, 0.3, 0.0); // en la muesca entre las dos esferas
        assert!(a.dist(p).min(b.dist(p)) > 0.0);
        assert!(smin(a.dist(p), b.dist(p), 0.5) < 0.0);
        assert_eq!(smin(f32::INFINITY, -0.2, 0.5), -0.2, "la primera forma de una capa arranca desde infinito");
    }

    #[test]
    fn cloth_hangs_down_from_its_support_to_the_floor_but_not_upward() {
        let hip = Shape::sphere(UP * 5.0, 1.0);
        let drape = Drape { floor: 0.0, length: 10.0, flare: 0.1, axis: UP * 5.0, folds: 0.0, depth: 0.0, pool: 0.0, pool_height: 1.0 };
        let skirt = Layer::new(vec![hip], |_| None).drape(drape);
        assert!(skirt.dist(UP * 1.0 + Vec3::new(1.2, 0.0, 0.0), 0.1) < 0.0, "debajo de la cadera y abierta hacia afuera");
        assert!(skirt.dist(UP * 7.0, 0.1) > 0.0, "no sube por encima de lo que la sostiene");
        assert!(skirt.dist(UP * -0.5, 0.1) > 0.0, "no atraviesa el piso");
        let leg = Shape::capsule(Vec3::new(1.2, 0.0, 0.0), Vec3::new(1.2, 2.0, 0.0), 0.4);
        let pushed = Layer::new(vec![hip], |_| None).drape(drape).carve(vec![leg], 0.0);
        assert!(pushed.dist(UP * 1.0 + Vec3::new(1.2, 0.0, 0.0), 0.1) > 0.0, "una pierna que empuja la falda la abre ahí");
        assert!(pushed.dist(UP * 1.0 + Vec3::new(-1.2, 0.0, 0.0), 0.1) < 0.0, "del otro lado la falda sigue entera");
    }
}

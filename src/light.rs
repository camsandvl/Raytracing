use crate::color::Color;
use nalgebra_glm::Vec3;

/// El `Light` del curso (`position`, `color`, `intensity`) más un modo direccional.
/// Las velas son puntuales (se apagan con la distancia); la luna es direccional (luz
/// paralela, sin atenuación) — sin ella el exterior de la iglesia quedaría negro de
/// noche, porque las velas están adentro.
pub struct Light {
    /// Posición de la luz, o — si `directional` — la dirección normalizada HACIA ella.
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    pub directional: bool,
    /// Si es un foco: solo ilumina dentro de un cono (ver `Spot`).
    pub spot: Option<Spot>,
    /// Si la luz solo puede entrar por un vano (ver `Aperture`).
    pub aperture: Option<Aperture>,
}

/// El vano rectangular por el que entra una luz de afuera (la luna por el vitral del
/// ábside, los focos por los vitrales de las naves laterales): centro y medio ancho y
/// medio alto como vectores sobre el plano del vano. Un punto cuya línea hacia la luz no
/// cruza el rectángulo no puede recibir esa luz, así que ni se lanza su rayo de sombra:
/// sin esto, todo el piso lanzaba un rayo largo hacia la luna que terminaba en la piedra.
#[derive(Clone, Copy)]
pub struct Aperture {
    pub center: Vec3,
    pub half_width: Vec3,
    pub half_height: Vec3,
}

impl Aperture {
    /// ¿La línea desde `point` hacia la luz (dirección `toward`, hasta `distance`) pasa
    /// por el vano?
    pub fn admits(&self, point: &Vec3, toward: &Vec3, distance: f32) -> bool {
        let normal = self.half_width.cross(&self.half_height);
        let facing = toward.dot(&normal);
        if facing.abs() < 1e-6 {
            return false;
        }
        let t = (self.center - point).dot(&normal) / facing;
        if t <= 0.0 || t >= distance {
            return false;
        }
        let offset = point + toward * t - self.center;
        let within = |half: &Vec3| offset.dot(half).abs() <= half.dot(half);
        within(&self.half_width) && within(&self.half_height)
    }
}

/// El cono de un foco: hacia dónde apunta y los cosenos de los ángulos donde empieza a
/// apagarse (`inner`) y donde ya no ilumina (`outer`), con un borde suave entre medio.
#[derive(Clone, Copy)]
pub struct Spot {
    pub direction: Vec3,
    pub inner: f32,
    pub outer: f32,
}

impl Spot {
    /// Cuánto ilumina el foco en la dirección `from_light` (de la luz hacia el punto):
    /// 1 adentro del cono, 0 afuera, suave en el borde.
    pub fn falloff(&self, from_light: &Vec3) -> f32 {
        let t = ((from_light.dot(&self.direction) - self.outer) / (self.inner - self.outer)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }
}

impl Light {
    pub fn new(position: Vec3, color: Color, intensity: f32) -> Self {
        Light { position, color, intensity, directional: false, spot: None, aperture: None }
    }

    /// Un foco en `position` que apunta a `target`, con el cono de `half_angle` (radianes)
    /// y un borde suave de un 30% más.
    pub fn spot(position: Vec3, target: Vec3, half_angle: f32, color: Color, intensity: f32) -> Self {
        let spot = Spot { direction: (target - position).normalize(), inner: half_angle.cos(), outer: (half_angle * 1.3).cos() };
        Light { position, color, intensity, directional: false, spot: Some(spot), aperture: None }
    }

    pub fn directional(toward_light: Vec3, color: Color, intensity: f32) -> Self {
        Light { position: toward_light.normalize(), color, intensity, directional: true, spot: None, aperture: None }
    }

    /// La misma luz, pero que solo entra por el vano `aperture`.
    pub fn through(mut self, aperture: Aperture) -> Self {
        self.aperture = Some(aperture);
        self
    }
}

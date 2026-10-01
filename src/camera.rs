//! Cámara orbital — mismo diseño que la rama `15-RT-03-ORBIT-CAMERA` /
//! `18-RT-06-REFLECTIONS` del curso (`eye`/`center`/`up`, `orbit(yaw, pitch)`,
//! `basis_change` para pasar un rayo de espacio-cámara a espacio-mundo). El curso
//! todavía no publica una rama con zoom, así que `zoom()` es una adición propia con el
//! mismo estilo: escala la distancia `eye`-`center` a lo largo del mismo eje, sujeta a
//! [min_radius, max_radius] para no atravesar la geometría ni alejarse al infinito.

use nalgebra_glm::Vec3;
use std::f32::consts::PI;

const PITCH_LIMIT: f32 = PI / 2.0 - 0.1;

pub struct Camera {
    pub eye: Vec3,
    pub center: Vec3,
    pub up: Vec3,
    pub min_radius: f32,
    pub max_radius: f32,
    /// El volumen seguro, como la unión de varias cajas (min, max): fuera de todas ellas
    /// no hay nada modelado, así que orbitar o alejar la cámara nunca puede sacarla por
    /// una puerta o ventana rota hacia el cielo de afuera. Una sola caja no alcanza para
    /// una planta en cruz como la de la catedral — el rectángulo que envuelve a toda la
    /// nave MÁS el crucero deja huecos en las esquinas (al costado de la nave, pero
    /// dentro del ancho del crucero) que en realidad son aire vacío de afuera, nunca
    /// tallado — por eso son varias cajas, una por cada sala real (ver
    /// `Scene::camera_bounds`). Vacío = sin límite (los tests y el modo primera persona,
    /// que ya tiene su propia colisión).
    pub bounds: Vec<(Vec3, Vec3)>,
}

impl Camera {
    pub fn new(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let radius = (eye - center).magnitude();
        Camera {
            eye,
            center,
            up,
            // Antes esto era siempre `radius * 0.25`: desde una vista lejana (p. ej.
            // "aerea", a ~166 unidades) el acercamiento mínimo quedaba igual de lejos
            // (~41 unidades) y nunca se podía llegar cerca de nada. Con el `.min(...)`
            // el acercamiento máximo queda fijo sin importar desde qué vista se arrancó.
            min_radius: (radius * 0.25).min(1.5),
            max_radius: radius * 4.0,
            bounds: Vec::new(),
        }
    }

    /// Fija el volumen seguro como la unión de `boxes` (cada una, dos esquinas en
    /// mundo), con un margen hacia adentro en cada una para no quedar pegada a la cara
    /// interior del muro. Con un volumen seguro, el alejamiento ya no tiene tope de radio:
    /// el límite son las paredes (desde el vitral del ábside se puede retroceder hasta la
    /// entrada y ver la iglesia entera).
    pub fn with_bounds(mut self, boxes: &[(Vec3, Vec3)]) -> Self {
        const MARGIN: f32 = 2.0;
        let margin = Vec3::repeat(MARGIN);
        self.bounds = boxes.iter().map(|&(min, max)| (min + margin, max - margin)).collect();
        if !self.bounds.is_empty() {
            self.max_radius = 100_000.0;
        }
        self.clamp_to_bounds();
        self
    }

    /// El punto de `b` más cercano a `p` (por componente, dentro de cada eje).
    fn closest_point(p: Vec3, (min, max): (Vec3, Vec3)) -> Vec3 {
        p.sup(&min).inf(&max)
    }

    /// Si hay un volumen seguro y `eye` ya está dentro de alguna de sus cajas, no toca
    /// nada. Si no, lo manda al punto más cercano de la unión: a la caja cuyo punto más
    /// cercano quede a menos distancia de `eye`. Así orbitar o alejar más allá de una
    /// pared frena justo ahí, deslizando hacia la sala real más próxima, en vez de seguir
    /// hacia el vacío de afuera.
    fn clamp_to_bounds(&mut self) {
        if self.bounds.is_empty() {
            return;
        }
        let inside = |p: Vec3, (min, max): &(Vec3, Vec3)| (min.x..=max.x).contains(&p.x) && (min.y..=max.y).contains(&p.y) && (min.z..=max.z).contains(&p.z);
        if self.bounds.iter().any(|b| inside(self.eye, b)) {
            return;
        }
        self.eye = self
            .bounds
            .iter()
            .map(|&b| Self::closest_point(self.eye, b))
            .min_by(|a, b| (a - self.eye).norm_squared().total_cmp(&(b - self.eye).norm_squared()))
            .expect("bounds no está vacío acá");
    }

    pub fn basis_change(&self, vector: &Vec3) -> Vec3 {
        let forward = (self.center - self.eye).normalize();
        let right = forward.cross(&self.up).normalize();
        let up = right.cross(&forward).normalize();

        let rotated = vector.x * right + vector.y * up - vector.z * forward;

        rotated.normalize()
    }

    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        let radius_vector = self.eye - self.center;
        let radius = radius_vector.magnitude();

        let current_yaw = radius_vector.z.atan2(radius_vector.x);
        let radius_xz = (radius_vector.x * radius_vector.x + radius_vector.z * radius_vector.z).sqrt();
        let current_pitch = (-radius_vector.y).atan2(radius_xz);

        let new_yaw = (current_yaw + delta_yaw) % (2.0 * PI);
        let new_pitch = (current_pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        self.eye = self.center
            + Vec3::new(
                radius * new_yaw.cos() * new_pitch.cos(),
                -radius * new_pitch.sin(),
                radius * new_yaw.sin() * new_pitch.cos(),
            );
        self.clamp_to_bounds();
    }

    pub fn radius(&self) -> f32 {
        (self.eye - self.center).magnitude()
    }

    /// Acerca (`delta` < 0) o aleja (`delta` > 0) la cámara de `center`, en la misma
    /// dirección en la que ya está, sujeto a [min_radius, max_radius] — el rubro de
    /// "20 pts: dejar que la cámara se acerque y aleje" de la rúbrica.
    pub fn zoom(&mut self, delta: f32) {
        let radius_vector = self.eye - self.center;
        let radius = radius_vector.magnitude();
        let new_radius = (radius + delta).clamp(self.min_radius, self.max_radius);
        self.eye = self.center + radius_vector.normalize() * new_radius;
        self.clamp_to_bounds();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dos salas, como la nave (angosta y larga) y el crucero (ancho pero corto) de la
    /// catedral: su unión tiene una forma en cruz, con huecos en las esquinas (al costado
    /// de la nave, pero dentro del ancho del crucero) que no son parte de ninguna sala —
    /// el caso que una sola caja envolvente no puede representar.
    fn cross_shaped_bounds() -> Vec<(Vec3, Vec3)> {
        vec![
            (Vec3::new(24.0, 0.0, 0.0), Vec3::new(116.0, 120.0, 220.0)), // nave: angosta, recorre todo el largo
            (Vec3::new(4.0, 0.0, 90.0), Vec3::new(136.0, 120.0, 130.0)), // crucero: ancho, un tramo corto
        ]
    }

    /// La vista "nave": desde cerca de la puerta (z chico) mirando hacia el ábside (z
    /// grande), con un radio inicial grande — el caso que se veía desde afuera antes de
    /// este arreglo, alejando la cámara en línea recta hacia atrás, a través de la puerta.
    fn nave_like_camera() -> Camera {
        let (eye, target) = (Vec3::new(70.0, 34.0, 8.0), Vec3::new(70.0, 200.0, 26.0));
        Camera::new(eye, target, Vec3::new(0.0, 1.0, 0.0)).with_bounds(&cross_shaped_bounds())
    }

    fn assert_in_some_box(eye: Vec3, boxes: &[(Vec3, Vec3)], msg: &str) {
        let margin = Vec3::repeat(2.0); // el mismo margen que aplica `with_bounds`
        let ok = boxes.iter().any(|&(min, max)| {
            let (min, max) = (min + margin, max - margin);
            (min.x..=max.x).contains(&eye.x) && (min.y..=max.y).contains(&eye.y) && (min.z..=max.z).contains(&eye.z)
        });
        assert!(ok, "{msg}: {eye:?} quedó fuera de las dos salas");
    }

    #[test]
    fn zooming_out_cannot_leave_the_building() {
        let mut camera = nave_like_camera();
        camera.max_radius = 100_000.0; // simula el alejamiento máximo de antes del arreglo
        camera.zoom(99_000.0);
        assert_in_some_box(camera.eye, &cross_shaped_bounds(), "después de zoom");
    }

    #[test]
    fn orbiting_sideways_cannot_land_in_the_gap_beside_the_nave() {
        // Con una sola caja envolvente (el arreglo anterior), girar la vista "nave" hacia
        // el costado dejaba la cámara en el hueco entre el ancho de la nave y el del
        // crucero — adentro de la caja envolvente total, pero fuera de las dos salas
        // reales. Acá se gira hacia los costados (yaw cerca de ±90°) sin tocar el radio.
        for yaw_steps in [-24, -12, 12, 24] {
            let mut camera = nave_like_camera();
            camera.orbit(yaw_steps as f32 * PI / 48.0, 0.0);
            assert_in_some_box(camera.eye, &cross_shaped_bounds(), &format!("yaw {yaw_steps}/48 de vuelta"));
        }
    }

    #[test]
    fn orbiting_a_full_turn_cannot_leave_either_room() {
        let mut camera = nave_like_camera();
        let steps = 48;
        for i in 0..steps {
            camera.orbit(2.0 * PI / steps as f32, 0.0);
            camera.orbit(0.0, 0.3);
            assert_in_some_box(camera.eye, &cross_shaped_bounds(), &format!("paso {i}"));
        }
    }

    #[test]
    fn without_bounds_the_camera_is_unaffected() {
        // El modo primera persona construye su propia `Camera::new` sin `with_bounds`
        // (tiene su propia colisión); acá se verifica que sin límites, nada cambia.
        let mut camera = Camera::new(Vec3::new(0.0, 0.0, -5.0), Vec3::zeros(), Vec3::new(0.0, 1.0, 0.0));
        camera.max_radius = 100_000.0; // aísla el límite de bounds del de radio, que ya existía
        camera.zoom(1000.0);
        assert!(camera.radius() > 100.0, "sin bounds, el zoom no se frena por una caja inexistente");
    }
}

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
}

impl Camera {
    pub fn new(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let radius = (eye - center).magnitude();
        Camera {
            eye,
            center,
            up,
            min_radius: radius * 0.25,
            max_radius: radius * 4.0,
        }
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
    }
}

//! Modo exploración en primera persona — opcional, no es parte de la rúbrica. Todo el
//! comportamiento vive acá: `main.rs` solo lo prende/apaga con F, le pasa las teclas y
//! copia la posición resultante a la `Camera` de siempre, así que el render no cambia.
//!
//! Las medidas están en metros y se convierten con `units_per_meter` de la escena: si
//! la iglesia cambia de escala, este módulo no se toca.

use crate::camera::Camera;
use crate::ray_intersect::RayIntersect;
use nalgebra_glm::Vec3;

const EYE_HEIGHT: f32 = 1.6;
const BODY_HEIGHT: f32 = 1.75;
const RADIUS: f32 = 0.25;
/// Escalones más bajos que esto se suben caminando (el presbiterio, la escalinata).
const STEP_HEIGHT: f32 = 0.3;
const WALK_SPEED: f32 = 2.5;
const RUN_SPEED: f32 = 5.0;
const GRAVITY: f32 = 9.8;
const JUMP_SPEED: f32 = 3.5;
const PITCH_LIMIT: f32 = 1.45;
/// Si cae más que esto por debajo del punto de aparición (se salió del diorama),
/// reaparece.
const FALL_LIMIT: f32 = 30.0;

#[derive(Clone, Copy)]
pub struct Spawn {
    /// Posición de los pies, en unidades de mundo.
    pub feet: Vec3,
    /// 0 = mirando hacia +Z.
    pub yaw: f32,
    pub units_per_meter: f32,
}

#[derive(Default)]
pub struct MoveInput {
    /// -1..1: atrás/adelante.
    pub forward: f32,
    /// -1..1: izquierda/derecha.
    pub right: f32,
    pub run: bool,
    pub jump: bool,
}

pub struct Walker {
    spawn: Spawn,
    feet: Vec3,
    yaw: f32,
    pitch: f32,
    vertical_speed: f32,
    grounded: bool,
}

impl Walker {
    pub fn new(spawn: Spawn) -> Self {
        Walker { spawn, feet: spawn.feet, yaw: spawn.yaw, pitch: 0.0, vertical_speed: 0.0, grounded: false }
    }

    pub fn respawn(&mut self) {
        *self = Walker::new(self.spawn);
    }

    /// Yaw positivo gira a la izquierda; pitch positivo mira hacia arriba.
    pub fn look(&mut self, d_yaw: f32, d_pitch: f32) {
        self.yaw += d_yaw;
        self.pitch = (self.pitch + d_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Avanza la simulación `dt` segundos. Devuelve si la posición cambió de forma
    /// visible (para no re-renderizar por diferencias de punto flotante estando quieto).
    pub fn update(&mut self, dt: f32, input: &MoveInput, objects: &[Box<dyn RayIntersect>]) -> bool {
        let upm = self.spawn.units_per_meter;
        let dt = dt.min(0.25);
        let before = self.feet;

        let (sin, cos) = self.yaw.sin_cos();
        let forward = Vec3::new(sin, 0.0, cos);
        let right = Vec3::new(-cos, 0.0, sin);
        let mut wish = forward * input.forward + right * input.right;
        if wish.magnitude() > 1.0 {
            wish = wish.normalize();
        }
        let speed = if input.run { RUN_SPEED } else { WALK_SPEED } * upm;
        let horizontal = wish * speed * dt;

        if input.jump && self.grounded {
            self.vertical_speed = JUMP_SPEED * upm;
            self.grounded = false;
        }
        self.vertical_speed -= GRAVITY * upm * dt;
        let dy = self.vertical_speed * dt;

        // Sub-pasos de medio radio: con cuadros lentos (el interior a ~5 fps) un solo
        // paso grande podría atravesar una pared delgada.
        let max_step = RADIUS * upm * 0.5;
        let largest = horizontal.x.abs().max(horizontal.z.abs()).max(dy.abs());
        let steps = ((largest / max_step).ceil() as usize).max(1);
        for _ in 0..steps {
            self.move_horizontal(0, horizontal.x / steps as f32, objects);
            self.move_horizontal(2, horizontal.z / steps as f32, objects);
            self.move_vertical(dy / steps as f32, objects);
        }

        if self.feet.y < self.spawn.feet.y - FALL_LIMIT * upm {
            self.respawn();
        }
        (self.feet - before).magnitude() > 1e-3
    }

    pub fn apply_to(&self, camera: &mut Camera) {
        let eye = self.feet + Vec3::new(0.0, EYE_HEIGHT * self.spawn.units_per_meter, 0.0);
        let (sp, cp) = self.pitch.sin_cos();
        let (sy, cy) = self.yaw.sin_cos();
        camera.eye = eye;
        camera.center = eye + Vec3::new(sy * cp, sp, cy * cp);
    }

    fn collides(&self, feet: Vec3, objects: &[Box<dyn RayIntersect>]) -> bool {
        let upm = self.spawn.units_per_meter;
        let (r, h, eps) = (RADIUS * upm, BODY_HEIGHT * upm, 1e-3);
        let min = feet + Vec3::new(-r + eps, eps, -r + eps);
        let max = feet + Vec3::new(r - eps, h - eps, r - eps);
        objects.iter().any(|o| o.overlaps_box(&min, &max))
    }

    fn move_horizontal(&mut self, axis: usize, delta: f32, objects: &[Box<dyn RayIntersect>]) {
        if delta == 0.0 {
            return;
        }
        let step = STEP_HEIGHT * self.spawn.units_per_meter;
        let mut target = self.feet;
        target[axis] += delta;
        if !self.collides(target, objects) {
            self.feet = target;
            if self.grounded {
                self.settle(step, objects); // bajar escalones sin "saltitos"
            }
            return;
        }
        if self.grounded {
            let raised = target + Vec3::new(0.0, step, 0.0);
            if !self.collides(raised, objects) {
                self.feet = raised;
                self.settle(step, objects);
            }
        }
    }

    fn move_vertical(&mut self, dy: f32, objects: &[Box<dyn RayIntersect>]) {
        let target = self.feet + Vec3::new(0.0, dy, 0.0);
        if !self.collides(target, objects) {
            self.feet = target;
            self.grounded = false;
            return;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..12 {
            let mid = (lo + hi) / 2.0;
            if self.collides(self.feet + Vec3::new(0.0, dy * mid, 0.0), objects) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        self.feet.y += dy * lo;
        if dy < 0.0 {
            self.grounded = true;
        }
        self.vertical_speed = 0.0;
    }

    /// Baja hasta `max_drop` mientras no choque — apoya los pies en el piso después de
    /// subir o bajar un escalón.
    fn settle(&mut self, max_drop: f32, objects: &[Box<dyn RayIntersect>]) {
        let down = |d: f32| Vec3::new(0.0, -d, 0.0);
        if !self.collides(self.feet + down(max_drop), objects) {
            // No hay piso cerca: es un borde, que caiga con la gravedad.
            self.grounded = false;
            return;
        }
        let (mut lo, mut hi) = (0.0, max_drop);
        for _ in 0..12 {
            let mid = (lo + hi) / 2.0;
            if self.collides(self.feet + down(mid), objects) {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        self.feet.y -= lo;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::ray_intersect::Material;
    use crate::voxel_grid::VoxelGrid;

    fn stone() -> Material {
        Material::new(Color::new(100, 100, 100), 1.0, [1.0, 0.0, 0.0, 0.0], 1.0, None)
    }

    /// Piso de 40×40 en y=0, a 4 unidades por metro, con el caminante en (20, 1, 5)
    /// mirando hacia +Z.
    fn world(extra: impl Fn(&mut VoxelGrid)) -> (Vec<Box<dyn RayIntersect>>, Walker) {
        let mut grid = VoxelGrid::new((40, 20, 40), 1.0, Vec3::zeros());
        for x in 0..40 {
            for z in 0..40 {
                grid.set(x, 0, z, stone());
            }
        }
        extra(&mut grid);
        let walker = Walker::new(Spawn { feet: Vec3::new(20.0, 1.0, 5.0), yaw: 0.0, units_per_meter: 4.0 });
        (vec![Box::new(grid)], walker)
    }

    fn walk_forward(walker: &mut Walker, objects: &[Box<dyn RayIntersect>], seconds: f32) {
        let input = MoveInput { forward: 1.0, ..Default::default() };
        for _ in 0..(seconds * 30.0) as usize {
            walker.update(1.0 / 30.0, &input, objects);
        }
    }

    #[test]
    fn climbs_a_one_cell_step() {
        let (objects, mut walker) = world(|g| {
            for x in 0..40 {
                for z in 10..40 {
                    g.set(x, 1, z, stone());
                }
            }
        });
        walk_forward(&mut walker, &objects, 1.0);
        assert!(walker.feet.z > 11.0, "avanzó más allá del escalón: z = {}", walker.feet.z);
        assert!((walker.feet.y - 2.0).abs() < 0.01, "quedó parado sobre el escalón: y = {}", walker.feet.y);
    }

    #[test]
    fn wall_blocks_movement() {
        let (objects, mut walker) = world(|g| {
            for x in 0..40 {
                for y in 1..12 {
                    g.set(x, y, 10, stone());
                }
            }
        });
        walk_forward(&mut walker, &objects, 1.0);
        assert!(walker.feet.z < 10.0 - RADIUS * 4.0 + 0.01, "no atraviesa la pared: z = {}", walker.feet.z);
        assert!((walker.feet.y - 1.0).abs() < 0.01, "sigue en el piso: y = {}", walker.feet.y);
    }
}

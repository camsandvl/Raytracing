pub mod anatomy;
pub mod builders;
pub mod canvas;
pub mod cathedral;
pub mod church;
pub mod figure;
pub mod people;
pub mod props;
pub mod sculpt;
pub mod skeleton;
pub mod zombies;

use crate::group::Group;
use crate::light::Light;
use crate::ray_intersect::RayIntersect;
use crate::voxel_grid::VoxelGrid;
use crate::walker::Spawn;
use nalgebra_glm::Vec3;

pub struct CameraPreset {
    pub name: &'static str,
    pub eye: Vec3,
    pub target: Vec3,
}

/// Todo lo que el render necesita de una escena: la geometría (la iglesia más los
/// objetos de detalle fino, como los esqueletos), sus luces (las velas salen de donde
/// se colocan los candeleros, así que las define el generador), y vistas de cámara
/// predefinidas.
pub struct Scene {
    pub grids: Vec<VoxelGrid>,
    /// Props chicos agrupados por cercanía: cada grupo se prueba con una sola caja
    /// envolvente antes de mirar sus props (ver `group.rs`).
    pub groups: Vec<Vec<VoxelGrid>>,
    pub lights: Vec<Light>,
    pub presets: Vec<CameraPreset>,
    /// Dónde aparece el modo primera persona (tecla F).
    pub walk_spawn: Spawn,
    /// El volumen fuera del cual la cámara orbital no puede orbitar ni alejarse, como la
    /// unión de una caja por cada sala real (ver `Camera::with_bounds`): una sola caja
    /// envolvente no alcanza para una planta en cruz, porque sus esquinas (al costado de
    /// la nave, pero dentro del ancho del crucero) son aire vacío de afuera, nunca
    /// tallado. Vacío para la iglesia original, que sí tiene vistas exteriores a propósito
    /// (su grid incluye la plaza alrededor); con salas para la catedral, que es solo
    /// interior — sin esto, alejar la cámara, u orbitarla hacia un costado, podía sacarla
    /// por la puerta o hacia esas esquinas, hacia el cielo de afuera, que no existe.
    pub camera_bounds: Vec<(Vec3, Vec3)>,
}

/// Los grids y grupos como la lista de objetos que recorre el render.
pub fn into_objects(grids: Vec<VoxelGrid>, groups: Vec<Vec<VoxelGrid>>) -> Vec<Box<dyn RayIntersect>> {
    let grids = grids.into_iter().map(|g| Box::new(g) as Box<dyn RayIntersect>);
    let groups = groups.into_iter().map(|g| Box::new(Group::new(g)) as Box<dyn RayIntersect>);
    grids.chain(groups).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::texture::TextureBank;
    use crate::walker::{MoveInput, Walker};

    /// El punto de aparición de primera persona está parado sobre el piso y se puede
    /// caminar hacia adelante desde ahí.
    fn assert_walkable(build: fn(&mut TextureBank) -> Scene) {
        let scene = build(&mut TextureBank::new());
        let spawn = scene.walk_spawn;
        let objects = into_objects(scene.grids, scene.groups);
        let mut walker = Walker::new(spawn);
        let mut camera = Camera::new(Vec3::zeros(), Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 0.0));

        for _ in 0..30 {
            walker.update(1.0 / 30.0, &MoveInput::default(), &objects);
        }
        walker.apply_to(&mut camera);
        let standing = camera.eye;
        let eye_height = 1.6 * spawn.units_per_meter;
        assert!((standing.y - (spawn.feet.y + eye_height)).abs() < 0.05, "parado en el piso: ojo en y = {}", standing.y);

        let forward = MoveInput { forward: 1.0, ..Default::default() };
        for _ in 0..30 {
            walker.update(1.0 / 30.0, &forward, &objects);
        }
        walker.apply_to(&mut camera);
        let walked = (camera.eye - standing).magnitude() / spawn.units_per_meter;
        assert!(walked > 2.0, "avanzó {walked:.2} m en 1 s");
    }

    #[test]
    fn church_spawn_is_walkable() {
        assert_walkable(church::build);
    }

    #[test]
    fn cathedral_spawn_is_walkable() {
        assert_walkable(cathedral::build);
    }
}

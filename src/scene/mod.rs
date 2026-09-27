pub mod builders;
pub mod church;
pub mod skeleton;

use crate::light::Light;
use crate::voxel_grid::VoxelGrid;
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
    pub lights: Vec<Light>,
    pub presets: Vec<CameraPreset>,
}

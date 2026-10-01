//! Piezas genéricas para construir estructuras arquitectónicas en un `VoxelGrid`.
//! Cada función recibe sus medidas como parámetros; `church.rs` es quien las llama con
//! las proporciones concretas de la iglesia.
//!
//! Todas las coordenadas de celda son `isize`: una pieza se puede posicionar con
//! offsets relativos sin aritmética a mano en cada llamado — `set_checked`/
//! `clear_checked` ignoran silenciosamente cualquier celda fuera del grid.

use crate::ray_intersect::Material;
use crate::voxel_grid::VoxelGrid;

/// Qué eje es el "espesor" (a través del cual se atraviesa) de un muro vertical. El
/// otro eje horizontal recorre el ANCHO del muro, y Y siempre es la altura.
#[derive(Clone, Copy, Debug)]
pub enum Axis {
    X,
    Z,
}

/// Hash entero determinista → [0, 1). Toda la "aleatoriedad" de la escena (bordes
/// derruidos, arbustos, hiedra, musgo en los esqueletos) sale de acá, así la escena es
/// idéntica en cada corrida.
pub fn hash(a: isize, b: isize, seed: u32) -> f32 {
    let mut h = (a as u32).wrapping_mul(0x8da6_b343) ^ (b as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

pub fn set_checked(grid: &mut VoxelGrid, x: isize, y: isize, z: isize, material: Material) {
    if x >= 0 && y >= 0 && z >= 0 {
        grid.set(x as usize, y as usize, z as usize, material);
    }
}

pub fn clear_checked(grid: &mut VoxelGrid, x: isize, y: isize, z: isize) {
    if x >= 0 && y >= 0 && z >= 0 {
        grid.clear(x as usize, y as usize, z as usize);
    }
}

/// Rellena una caja rectangular sólida (extremos exclusivos, como un `Range`).
pub fn fill_box(grid: &mut VoxelGrid, x: (isize, isize), y: (isize, isize), z: (isize, isize), material: Material) {
    for xi in x.0..x.1 {
        for yi in y.0..y.1 {
            for zi in z.0..z.1 {
                set_checked(grid, xi, yi, zi, material);
            }
        }
    }
}

fn wall_cell(axis: Axis, t: isize, along: isize, y: isize) -> (isize, isize, isize) {
    match axis {
        Axis::X => (t, y, along),
        Axis::Z => (along, y, t),
    }
}

/// Altura del contorno de un arco ojival ("de dos centros") en la posición horizontal
/// `u` dentro de `[0, width]`, medida desde la base de la abertura. Por debajo de
/// `jamb_height` la abertura es rectangular; por encima, el contorno lo dan dos arcos
/// de radio = `width` centrados cada uno en el arranque OPUESTO — la construcción
/// clásica del arco ojival equilátero.
pub fn pointed_arch_profile(u: f32, width: f32, jamb_height: f32) -> f32 {
    let radius = width;
    let center_x = if u < width / 2.0 { width } else { 0.0 };
    let dx = u - center_x;
    jamb_height + (radius * radius - dx * dx).max(0.0).sqrt()
}

/// Talla un vano ojival a través de todo el espesor de un muro. `fill`: `None` deja
/// aire (puerta, vano abierto); `Some(material)` pone un panel de ese material en la
/// capa CENTRAL del muro (una ventana) — así el vidrio queda retirado dentro del
/// vano, con el derrame de piedra visible a ambos lados, en vez de ser un bloque
/// macizo del espesor completo del muro.
#[allow(clippy::too_many_arguments)]
pub fn carve_pointed_arch_opening(
    grid: &mut VoxelGrid,
    axis: Axis,
    thickness_range: (isize, isize),
    along_start: isize,
    base_y: isize,
    width_cells: isize,
    jamb_height_cells: isize,
    fill: Option<Material>,
) {
    let pane = (thickness_range.0 + thickness_range.1 - 1) / 2;
    for u in 0..width_cells {
        let profile = pointed_arch_profile(u as f32 + 0.5, width_cells as f32, jamb_height_cells as f32);
        for v in 0..profile.round() as isize {
            for t in thickness_range.0..thickness_range.1 {
                let (x, y, z) = wall_cell(axis, t, along_start + u, base_y + v);
                match fill {
                    Some(material) if t == pane => set_checked(grid, x, y, z, material),
                    _ => clear_checked(grid, x, y, z),
                }
            }
        }
    }
}

/// Rosetón inscrito en un círculo de radio `radius` centrado en `(center_along,
/// center_y)` sobre el plano del muro: cubo central, anillo intermedio y aro exterior
/// de piedra, con `petal_count` rayos dividiendo los pétalos de vidrio — la silueta de
/// los renders de referencia simplificada a bloques. La tracería atraviesa todo el
/// espesor del muro; el vidrio (si hay) es un panel en la capa central, y sin vidrio
/// los pétalos quedan abiertos al cielo.
#[allow(clippy::too_many_arguments)]
pub fn place_rose_window(
    grid: &mut VoxelGrid,
    axis: Axis,
    thickness_range: (isize, isize),
    center_along: f32,
    center_y: f32,
    radius: f32,
    petal_count: usize,
    frame: Material,
    glass: Option<Material>,
) {
    let pane = (thickness_range.0 + thickness_range.1 - 1) / 2;
    let bound = radius.ceil() as isize + 1;
    let slice = std::f32::consts::TAU / petal_count as f32;

    for du in -bound..=bound {
        for dv in -bound..=bound {
            let (fu, fv) = (du as f32 + 0.5, dv as f32 + 0.5);
            let r = (fu * fu + fv * fv).sqrt();
            if r > radius {
                continue;
            }
            // Los pétalos del anillo interior quedan desfasados medio gajo respecto
            // de los del exterior, como en un rosetón real.
            let offset = if r < radius * 0.55 { slice * 0.5 } else { 0.0 };
            let within = ((fv.atan2(fu) + offset).rem_euclid(slice)) / slice;

            let hub = r < radius * 0.2;
            let middle_ring = (radius * 0.5..radius * 0.6).contains(&r);
            let rim = r > radius * 0.88;
            let spoke = !(0.14..=0.86).contains(&within);
            let is_frame = hub || middle_ring || rim || spoke;

            let along = (center_along + du as f32).floor() as isize;
            let y = (center_y + dv as f32).floor() as isize;
            for t in thickness_range.0..thickness_range.1 {
                let (x, yy, z) = wall_cell(axis, t, along, y);
                match glass {
                    _ if is_frame => set_checked(grid, x, yy, z, frame),
                    Some(glass) if t == pane => set_checked(grid, x, yy, z, glass),
                    _ => clear_checked(grid, x, yy, z),
                }
            }
        }
    }
}

/// Almenas sobre el perímetro de un rectángulo: merlones de `merlon` celdas
/// separados por huecos de `gap`, empezando siempre en una esquina. `thickness` es el
/// espesor del parapeto hacia adentro del rectángulo.
#[allow(clippy::too_many_arguments)]
pub fn crenellate(
    grid: &mut VoxelGrid,
    x: (isize, isize),
    z: (isize, isize),
    base_y: isize,
    height: isize,
    thickness: isize,
    merlon: isize,
    gap: isize,
    material: Material,
) {
    let is_merlon = |along: isize| along.rem_euclid(merlon + gap) < merlon;
    for xi in x.0..x.1 {
        for zi in z.0..z.1 {
            let (lx, lz) = (xi - x.0, zi - z.0);
            let (w, d) = (x.1 - x.0, z.1 - z.0);
            let on_x_side = lx < thickness || lx >= w - thickness;
            let on_z_side = lz < thickness || lz >= d - thickness;
            let merlon_here = (on_z_side && is_merlon(lx)) || (on_x_side && is_merlon(lz));
            if merlon_here {
                for y in base_y..base_y + height {
                    set_checked(grid, xi, y, zi, material);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use nalgebra_glm::Vec3;

    fn stone() -> Material {
        Material::new(Color::new(200, 200, 200), 10.0, [0.8, 0.1, 0.0, 0.0], 1.0, None)
    }

    #[test]
    fn arch_profile_peaks_at_center_and_is_symmetric() {
        let (width, jamb) = (12.0, 4.0);
        let center = pointed_arch_profile(width / 2.0, width, jamb);
        assert!(center > pointed_arch_profile(0.5, width, jamb));
        let (left, right) = (pointed_arch_profile(2.0, width, jamb), pointed_arch_profile(width - 2.0, width, jamb));
        assert!((left - right).abs() < 1e-4);
    }

    #[test]
    fn window_pane_sits_in_the_middle_of_the_wall() {
        let mut grid = VoxelGrid::new((10, 30, 10), 1.0, Vec3::new(0.0, 0.0, 0.0));
        fill_box(&mut grid, (0, 3), (0, 30), (0, 10), stone());
        let glass = Material::new(Color::new(255, 200, 120), 80.0, [0.0, 0.5, 0.0, 0.9], 1.5, None);
        carve_pointed_arch_opening(&mut grid, Axis::X, (0, 3), 2, 5, 5, 6, Some(glass));
        assert_eq!(grid.get(1, 6, 4), Some(glass), "panel en la capa central (x=1)");
        assert_eq!(grid.get(0, 6, 4), None, "derrame exterior vacío");
        assert_eq!(grid.get(2, 6, 4), None, "derrame interior vacío");
        assert_eq!(grid.get(0, 2, 4), Some(stone()), "debajo del alféizar sigue siendo muro");
    }

    #[test]
    fn crenellation_starts_with_merlons_on_the_corners() {
        let mut grid = VoxelGrid::new((20, 10, 20), 1.0, Vec3::new(0.0, 0.0, 0.0));
        crenellate(&mut grid, (0, 13), (0, 13), 0, 2, 1, 3, 2, stone());
        assert!(grid.get(0, 0, 0).is_some(), "esquina = merlón");
        assert!(grid.get(3, 0, 0).is_none(), "después del merlón viene un hueco");
        assert!(grid.get(6, 0, 12).is_some(), "segundo merlón del lado opuesto");
        assert!(grid.get(6, 0, 6).is_none(), "el centro del rectángulo queda libre");
    }
}

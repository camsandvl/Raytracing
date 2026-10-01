//! Props de detalle de la catedral (sección 8 del spec + candelabros): mobiliario en
//! celdas de 5 cm, como los esqueletos. Cada diseño es una función que arma su propio
//! grid, del tamaño justo del prop, a partir de su esquina mínima en mundo. Grids
//! ajustados: un grid grande compartido por varios props (una fila de candelabros, todo
//! el presbiterio) obliga a los rayos que lo cruzan a recorrer miles de celdas vacías de
//! 5 cm, y el render se triplicaba.
//!
//! Coordenadas locales como las del spec: `x` = ancho, `y` = hacia el este, `z` = arriba,
//! en celdas de 5 cm desde la esquina mínima del prop. Los props con velas devuelven la
//! posición de su luz: justo encima de la llama, nunca adentro (si no, la propia llama le
//! haría sombra a todo).

use crate::materials::MaterialSet;
use crate::ray_intersect::Material;
use crate::scene::skeleton::DETAIL;
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::Vec3;

struct DetailGrid {
    grid: VoxelGrid,
    origin: Vec3,
}

impl DetailGrid {
    /// `origin`: esquina mínima en mundo; `size`: celdas en (x, y hacia el este, z).
    fn new(origin: Vec3, size: (isize, isize, isize)) -> Self {
        DetailGrid { grid: VoxelGrid::new((size.0 as usize, size.2 as usize, size.1 as usize), DETAIL, origin), origin }
    }

    fn into_grid(self) -> VoxelGrid {
        self.grid
    }

    fn fill(&mut self, x: (isize, isize), y: (isize, isize), z: (isize, isize), material: Material) {
        for xi in x.0..x.1 {
            for yi in y.0..y.1 {
                for zi in z.0..z.1 {
                    if xi >= 0 && yi >= 0 && zi >= 0 {
                        self.grid.set(xi as usize, zi as usize, yi as usize, material);
                    }
                }
            }
        }
    }

    fn point(&self, x: f32, y: f32, z: f32) -> Vec3 {
        self.origin + Vec3::new(x, z, y) * DETAIL
    }

    /// Vela de 2×2 celdas con la llama encima; devuelve dónde va su luz.
    fn candle(&mut self, x: isize, y: isize, base: isize, height: isize, m: &MaterialSet) -> Vec3 {
        self.fill((x, x + 2), (y, y + 2), (base, base + height), m.wax);
        self.fill((x, x + 2), (y, y + 2), (base + height, base + height + 1), m.flame);
        self.point(x as f32 + 1.0, y as f32 + 1.0, (base + height + 3) as f32)
    }

    /// Pie de hierro escalonado: base ancha, un escalón y un fuste de 2×2 centrado.
    fn stand(&mut self, size: isize, stem_top: isize, m: &MaterialSet) {
        let c = size / 2;
        self.fill((0, size), (0, size), (0, 2), m.iron);
        self.fill((c - 3, c + 3), (c - 3, c + 3), (2, 4), m.iron);
        self.fill((c - 1, c + 1), (c - 1, c + 1), (4, stem_top), m.iron);
    }
}

/// 1. Altar mayor (1.8 × 1.0 × 1.0 m): basa y tapa de mármol, cuerpo de roble.
pub fn altar(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (36, 20, 20));
    p.fill((0, 36), (0, 20), (0, 2), m.marble);
    p.fill((2, 34), (2, 18), (2, 17), m.wood);
    p.fill((0, 36), (0, 20), (17, 20), m.marble);
    p.into_grid()
}

/// 2. Retablo (1.8 × 0.4 × 2.4 m), escalonado, con paneles hundidos en mármol oscuro
/// y una cruz de hierro arriba.
pub fn retable(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (36, 8, 48));
    p.fill((0, 36), (0, 8), (0, 20), m.marble);
    p.fill((2, 34), (2, 8), (20, 36), m.marble);
    p.fill((8, 28), (4, 8), (36, 42), m.trim);
    for x in [3, 14, 25] {
        p.fill((x, x + 8), (0, 1), (4, 16), m.marble_dark);
    }
    p.fill((17, 19), (5, 7), (42, 48), m.iron);
    p.fill((15, 21), (5, 7), (44, 46), m.iron);
    p.into_grid()
}

/// 3. Ambón (0.8 × 0.8 × 1.6 m): basa de mármol, columna y atril de roble.
pub fn ambo(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (16, 16, 32));
    p.fill((0, 16), (0, 16), (0, 4), m.marble);
    p.fill((4, 12), (4, 12), (4, 24), m.wood);
    p.fill((1, 15), (2, 14), (24, 28), m.wood);
    for z in 28..32 {
        p.fill((1, 15), (2 + (z - 28) * 2, 14), (z, z + 1), m.wood); // atril inclinado hacia el oeste
    }
    p.into_grid()
}

/// 4. Cátedra del obispo (1.2 × 0.8 × 1.8 m), mirando al oeste: roble con el travesaño
/// del asiento en mármol.
pub fn bishop_chair(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (24, 16, 36));
    p.fill((0, 24), (0, 12), (0, 8), m.wood);
    p.fill((0, 24), (0, 2), (8, 11), m.marble);
    p.fill((0, 24), (2, 12), (8, 11), m.wood);
    p.fill((0, 24), (12, 16), (0, 36), m.wood);
    for x in [0, 21] {
        p.fill((x, x + 3), (0, 12), (11, 18), m.wood);
    }
    p.into_grid()
}

/// 5. Púlpito (1.2 × 1.2 × 2.4 m): fuste y tambor octogonal de mármol, atril de roble.
pub fn pulpit(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (24, 24, 48));
    p.fill((8, 16), (8, 16), (0, 24), m.marble);
    for x in 0..24isize {
        for y in 0..24isize {
            let (dx, dy) = ((2 * x - 23).abs(), (2 * y - 23).abs());
            if dx + dy <= 34 {
                p.fill((x, x + 1), (y, y + 1), (24, 40), m.marble);
            }
        }
    }
    p.fill((4, 20), (4, 20), (40, 42), m.wood);
    p.into_grid()
}

/// 6–7. Reclinatorio (0.6 × 0.6 × 0.7 m) de roble: se arrodilla del lado oeste.
pub fn kneeler(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (12, 12, 14));
    p.fill((0, 12), (0, 6), (0, 4), m.wood);
    for x in [0, 10] {
        p.fill((x, x + 2), (6, 12), (0, 10), m.wood);
    }
    p.fill((0, 12), (8, 12), (10, 14), m.wood);
    p.into_grid()
}

/// 8. Cruz procesional (0.6 × 0.2 × 1.8 m) de hierro, de brazos gruesos.
pub fn processional_cross(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (12, 4, 36));
    p.fill((0, 12), (0, 4), (0, 3), m.iron);
    p.fill((5, 7), (1, 3), (3, 36), m.iron);
    p.fill((0, 12), (1, 3), (26, 29), m.iron);
    p.into_grid()
}

/// Altar lateral de los brazos del crucero (0.8 × 1.8 × 1.0 m, más angosto que largo a
/// lo largo del muro): basa y tapa de mármol, cuerpo de roble.
pub fn side_altar(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (16, 36, 20));
    p.fill((0, 16), (0, 36), (0, 2), m.marble);
    p.fill((2, 14), (2, 34), (2, 17), m.wood);
    p.fill((0, 16), (0, 36), (17, 20), m.marble);
    p.into_grid()
}

/// Soporte de velas votivas (0.6 × 1.2 × 1.0 m): pirámide escalonada de hierro con filas
/// de velitas, simétrica para servir en cualquier brazo. Una sola luz para todo el
/// soporte.
pub fn votive_rack(m: &MaterialSet, origin: Vec3) -> (VoxelGrid, Vec3) {
    let mut p = DetailGrid::new(origin, (12, 24, 22));
    for (x, y) in [(0, 0), (11, 0), (0, 23), (11, 23)] {
        p.fill((x, x + 1), (y, y + 1), (0, 10), m.iron);
    }
    // (grada, x de sus filas de velas)
    for (tier, rows) in [((0, 12, 10), [1, 10]), ((3, 9, 13), [4, 7]), ((5, 7, 16), [5, 6])] {
        let (x0, x1, z) = tier;
        p.fill((x0, x1), (0, 24), (z, z + 1), m.iron);
        for x in rows {
            for y in (2..22).step_by(3) {
                p.fill((x, x + 1), (y, y + 1), (z + 1, z + 3), m.wax);
                p.fill((x, x + 1), (y, y + 1), (z + 3, z + 4), m.flame);
            }
        }
    }
    let light = p.point(6.0, 12.0, 23.0);
    (p.into_grid(), light)
}

/// 9. Relicario (1.2 × 0.6 × 0.8 m): cofre de mármol con un panel de hueso y musgo.
pub fn reliquary(m: &MaterialSet, origin: Vec3) -> VoxelGrid {
    let mut p = DetailGrid::new(origin, (24, 12, 16));
    p.fill((0, 24), (0, 12), (0, 12), m.marble);
    p.fill((1, 23), (1, 11), (12, 16), m.trim);
    p.fill((4, 20), (0, 1), (3, 10), m.bone_moss);
    p.into_grid()
}

/// 10. Cirio pascual (0.6 × 0.6 × 2.4 m).
pub fn paschal_candle(m: &MaterialSet, origin: Vec3) -> (VoxelGrid, Vec3) {
    let mut p = DetailGrid::new(origin, (12, 12, 48));
    p.stand(12, 38, m);
    p.fill((3, 9), (3, 9), (38, 40), m.iron);
    let light = p.candle(5, 5, 40, 7, m);
    (p.into_grid(), light)
}

/// 11–12. Candelero del altar (0.4 × 0.4 × 1.6 m).
pub fn altar_stand(m: &MaterialSet, origin: Vec3) -> (VoxelGrid, Vec3) {
    let mut p = DetailGrid::new(origin, (8, 8, 32));
    p.stand(8, 24, m);
    p.fill((2, 6), (2, 6), (24, 25), m.iron);
    let light = p.candle(3, 3, 25, 6, m);
    (p.into_grid(), light)
}

/// Candelabro de pie (0.8 × 0.8 × 2.4 m): tres velas sobre un travesaño, la del centro
/// más alta — el de las imágenes de referencia, a lo largo de la nave.
pub fn candelabrum(m: &MaterialSet, origin: Vec3) -> (VoxelGrid, Vec3) {
    let mut p = DetailGrid::new(origin, (16, 16, 48));
    p.stand(16, 38, m);
    p.fill((2, 14), (7, 9), (36, 38), m.iron);
    for x in [2, 12] {
        p.candle(x, 7, 38, 6, m);
    }
    let light = p.candle(7, 7, 38, 8, m);
    (p.into_grid(), light)
}

/// Lámpara colgante (1.6 × 1.6 × 0.8 m): aro de hierro con ocho velas y cuatro varillas a
/// un collar central, del que sube el tirante hasta la bóveda (ese va en el grid de la
/// catedral). La luz queda en el hueco del aro.
pub fn chandelier(m: &MaterialSet, origin: Vec3) -> (VoxelGrid, Vec3) {
    let mut p = DetailGrid::new(origin, (32, 32, 16));
    let (c, radius) = (16.0, 14.5);
    for x in 0..32 {
        for y in 0..32 {
            let r = ((x as f32 + 0.5 - c).powi(2) + (y as f32 + 0.5 - c).powi(2)).sqrt();
            if (13.0..16.0).contains(&r) {
                p.fill((x, x + 1), (y, y + 1), (0, 2), m.iron);
            }
        }
    }
    for k in 0..8 {
        let angle = k as f32 * std::f32::consts::FRAC_PI_4;
        let (x, y) = ((c + radius * angle.cos()) as isize - 1, (c + radius * angle.sin()) as isize - 1);
        p.candle(x, y, 2, 5, m);
    }
    for k in 0..4 {
        let angle = k as f32 * std::f32::consts::FRAC_PI_2;
        for step in 0..=24 {
            let t = step as f32 / 24.0;
            let r = radius * (1.0 - t);
            let (x, y, z) = ((c + r * angle.cos()) as isize, (c + r * angle.sin()) as isize, (1.0 + 14.0 * t) as isize);
            p.fill((x - 1, x + 1), (y - 1, y + 1), (z, z + 1), m.iron);
        }
    }
    p.fill((14, 18), (14, 18), (12, 16), m.iron);
    let light = p.point(c, c, 8.0);
    (p.into_grid(), light)
}

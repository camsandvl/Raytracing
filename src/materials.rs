//! Los materiales de la escena, en un solo lugar.
//!
//! Los 5 que cuentan para la rúbrica (cada uno con textura propia y parámetros propios
//! de albedo/specular/transparencia/reflectividad, ver skill de rúbrica): Piedra,
//! Madera, Vidrio (refracción), Metal (reflexión, textura PNG) y Hueso musgoso.
//!
//! El resto son decorativos — dan variedad visual (y suman al criterio subjetivo de
//! complejidad), pero no se presentan como parte de los 5.

use crate::color::Color;
use crate::ray_intersect::Material;
use crate::texture::{Texture, TextureBank};

pub struct MaterialSet {
    // Los 5 de la rúbrica.
    pub stone: Material,
    pub wood: Material,
    pub glass: Material,
    pub metal: Material,
    pub bone_moss: Material,

    // Decorativos.
    pub cobble: Material,
    pub earth: Material,
    pub roof: Material,
    pub floor: Material,
    pub wax: Material,
    pub flame: Material,
    pub leaves: Material,
    pub soot: Material,
    pub leather: Material,
    pub veil: Material,
    pub petal: Material,
}

const MOSS: Color = Color { r: 62, g: 88, b: 46 };

pub fn build(textures: &mut TextureBank) -> MaterialSet {
    let white = Color::new(255, 255, 255);

    let stone_tex = textures.add(Texture::block(12, 4, Color::new(108, 108, 114), 0.1, 0.45, MOSS, 0.05, 1001));
    let wood_tex = textures.add(Texture::grained(32, Color::new(84, 54, 32), 0.25, 2002));
    let glass_tex = textures.add(Texture::leaded_glass(16, 3003));
    let bone_tex = textures.add(Texture::moss(32, Color::new(214, 204, 178), Color::new(70, 112, 52), 0.3, 4004));

    // Metal es el único cargado de un PNG real (la mitad "cargada de archivo" de la
    // estrategia mixta de texturas) — generado una vez con
    // `tools/gen_metal_texture.py` y commiteado como asset.
    let metal_tex = Texture::load_png("assets/textures/metal_ornamental.png", 1.0)
        .map(|tex| textures.add(tex))
        .expect("falta assets/textures/metal_ornamental.png (correr tools/gen_metal_texture.py)");

    let cobble_tex = textures.add(Texture::block(10, 4, Color::new(84, 86, 82), 0.14, 0.5, MOSS, 0.2, 5005));
    let earth_tex = textures.add(Texture::block(8, 4, Color::new(70, 52, 38), 0.2, 0.2, MOSS, 0.03, 6006));
    let roof_tex = textures.add(Texture::block(12, 4, Color::new(60, 64, 78), 0.1, 0.5, MOSS, 0.1, 7007));
    let floor_tex = textures.add(Texture::block(12, 4, Color::new(96, 92, 86), 0.08, 0.35, MOSS, 0.02, 8008));
    let leaves_tex = textures.add(Texture::block(6, 4, Color::new(50, 76, 38), 0.3, 0.3, Color::new(84, 112, 50), 0.3, 9009));
    let veil_tex = textures.add(Texture::moss(16, Color::new(242, 240, 232), Color::new(170, 168, 160), 0.25, 9109));

    MaterialSet {
        stone: Material::new(white, 6.0, [0.85, 0.08, 0.0, 0.0], 1.0, Some(stone_tex)),
        wood: Material::new(white, 12.0, [0.85, 0.12, 0.0, 0.0], 1.0, Some(wood_tex)),
        glass: Material::new(Color::new(255, 196, 120), 90.0, [0.03, 0.6, 0.0, 0.9], 1.5, Some(glass_tex)),
        metal: Material::new(Color::new(225, 180, 90), 150.0, [0.35, 0.8, 0.55, 0.0], 1.0, Some(metal_tex)),
        bone_moss: Material::new(white, 6.0, [0.9, 0.05, 0.0, 0.0], 1.0, Some(bone_tex)),

        cobble: Material::new(white, 4.0, [0.85, 0.05, 0.0, 0.0], 1.0, Some(cobble_tex)),
        earth: Material::new(white, 2.0, [0.9, 0.02, 0.0, 0.0], 1.0, Some(earth_tex)),
        roof: Material::new(white, 20.0, [0.8, 0.15, 0.0, 0.0], 1.0, Some(roof_tex)),
        floor: Material::new(white, 16.0, [0.85, 0.12, 0.0, 0.0], 1.0, Some(floor_tex)),
        wax: Material::new(Color::new(236, 224, 192), 20.0, [0.9, 0.2, 0.0, 0.0], 1.0, None),
        flame: Material::new(Color::new(255, 150, 50), 1.0, [0.0, 0.0, 0.0, 0.0], 1.0, None).with_emission(1.8),
        leaves: Material::new(white, 4.0, [0.9, 0.03, 0.0, 0.0], 1.0, Some(leaves_tex)),
        soot: Material::new(Color::new(14, 12, 12), 2.0, [0.6, 0.02, 0.0, 0.0], 1.0, None),
        leather: Material::new(Color::new(92, 28, 34), 18.0, [0.85, 0.15, 0.0, 0.0], 1.0, None),
        // El velo de la novia: tela gastada, semitransparente (casi no desvía la luz).
        veil: Material::new(white, 12.0, [0.3, 0.1, 0.0, 0.72], 1.03, Some(veil_tex)),
        petal: Material::new(Color::new(128, 24, 38), 10.0, [0.9, 0.1, 0.0, 0.0], 1.0, None),
    }
}

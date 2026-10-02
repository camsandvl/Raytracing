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

    // Los novios y el oficiante (personas): piel (con un brillo propio muy tenue, para que
    // no se pierdan en la iglesia oscura), pelo y tela blanca (vestido de novia, camisa,
    // alba).
    pub skin: Material,
    pub hair: Material,

    // Los padres de los novios, vestidos a la moda de una boda de los 80: esmoquin gris
    // perla y traje azul marino para ellos, vestidos de raso fucsia y verde azulado para
    // ellas, pelo canoso y permanente rubia.
    pub tux_grey: Material,
    pub suit_navy: Material,
    pub dress_fuchsia: Material,
    pub dress_teal: Material,
    pub hair_grey: Material,
    pub hair_blonde: Material,

    // La invasión: carne podrida amarillo verdosa con un resplandor radiactivo tenue, y
    // sangre húmeda, con algo de brillo y un resplandor casi imperceptible.
    pub zombie_skin: Material,
    pub blood: Material,

    // Las figuras esculpidas de la boda (`anatomy.rs`): raso liso para el vestido (la
    // tela con textura se leía como tejido al crochet a esta escala), lana negra y solapas
    // de raso para el frac, labios, el fondo de la boca, el iris, y la carne más podrida
    // (oscura) del zombi de la mordida.
    pub satin: Material,
    pub tux: Material,
    pub lapel: Material,
    pub lips: Material,
    pub rouge: Material,
    pub mouth: Material,
    pub iris: Material,
    pub rot: Material,

    // Catedral. `marble` es la Piedra de la rúbrica en la catedral: mármol pálido (M02)
    // con textura propia de vetas. Además: mármol gris azulado (M03) para plementería,
    // nichos y basas, mármol de molduras (M04), el piso en damero pulido, el vitral del
    // ábside (M05–M08) y el hierro de emplomado y herrajes (M09).
    pub marble: Material,
    pub marble_dark: Material,
    pub trim: Material,
    /// Mármol gris azulado veteado ("bardiglio") de los fustes y los paneles de las
    /// pilastras: más oscuro que los muros pálidos, gótico y sobrio.
    pub marble_grey: Material,
    pub floor_pale: Material,
    pub floor_dark: Material,
    pub floor_beige: Material,
    /// Mármol amarillo ("giallo antico") pulido: el sol del crucero y los acentos dorados.
    pub floor_gold: Material,
    pub glass_cobalt: Material,
    pub glass_ruby: Material,
    pub glass_amber: Material,
    pub glass_pale: Material,
    pub iron: Material,
    /// Plata: la plementería de las bóvedas, las cuencas oscuras, los paneles de las
    /// capillas y del sol y la luna, y los plintos de las columnas. Antes eran de mármol
    /// oscuro y las bóvedas se confundían con ventanas. Con la textura ornamental del
    /// metal, un poco de reflejo que atrapa las velas y un brillo propio muy tenue: en la
    /// iglesia oscura, el metal sin luz se veía gris oscuro, no plateado.
    pub silver: Material,
}

const MOSS: Color = Color { r: 62, g: 88, b: 46 };

pub fn build(textures: &mut TextureBank) -> MaterialSet {
    let white = Color::new(255, 255, 255);

    let stone_tex = textures.add(Texture::block(12, 4, Color::new(108, 108, 114), 0.1, 0.0, MOSS, 0.05, 1001));
    let wood_tex = textures.add(Texture::grained(32, Color::new(84, 54, 32), 0.25, 2002));
    let glass_tex = textures.add(Texture::leaded_glass(16, 3003));
    let bone_tex = textures.add(Texture::moss(32, Color::new(214, 204, 178), Color::new(70, 112, 52), 0.3, 4004));

    // Metal es el único cargado de un PNG real (la mitad "cargada de archivo" de la
    // estrategia mixta de texturas) — generado una vez con
    // `tools/gen_metal_texture.py` y commiteado como asset.
    let metal_tex = Texture::load_png("assets/textures/metal_ornamental.png", 1.0)
        .map(|tex| textures.add(tex))
        .expect("falta assets/textures/metal_ornamental.png (correr tools/gen_metal_texture.py)");

    let cobble_tex = textures.add(Texture::block(10, 4, Color::new(84, 86, 82), 0.14, 0.0, MOSS, 0.2, 5005));
    let earth_tex = textures.add(Texture::block(8, 4, Color::new(70, 52, 38), 0.2, 0.0, MOSS, 0.03, 6006));
    let roof_tex = textures.add(Texture::block(12, 4, Color::new(60, 64, 78), 0.1, 0.0, MOSS, 0.1, 7007));
    let floor_tex = textures.add(Texture::block(12, 4, Color::new(96, 92, 86), 0.08, 0.0, MOSS, 0.02, 8008));
    let leaves_tex = textures.add(Texture::block(6, 4, Color::new(50, 76, 38), 0.3, 0.0, Color::new(84, 112, 50), 0.3, 9009));
    let veil_tex = textures.add(Texture::moss(16, Color::new(242, 240, 232), Color::new(170, 168, 160), 0.25, 9109));
    // Vitral: vidrio levemente moteado, sin líneas de plomo por celda (el emplomado son
    // vóxeles de hierro propios). El color de cada vidrio lo da `diffuse`.
    let stained_tex = textures.add(Texture::moss(16, white, Color::new(196, 202, 214), 0.35, 3103));
    let stained = |color| Material::new(color, 90.0, [0.03, 0.6, 0.0, 0.72], 1.5, Some(stained_tex));
    let rot_tex = textures.add(Texture::moss(16, Color::new(172, 168, 84), Color::new(118, 116, 56), 0.35, 6606));
    let marble_tex = textures.add(Texture::marble(64, 8.0, Color::new(216, 211, 201), Color::new(146, 144, 146), 0.55, 1201));
    let marble_dark_tex = textures.add(Texture::marble(64, 8.0, Color::new(69, 75, 81), Color::new(38, 42, 47), 0.5, 1202));
    let trim_tex = textures.add(Texture::marble(64, 8.0, Color::new(183, 173, 158), Color::new(138, 130, 118), 0.35, 1203));
    let gold_tex = textures.add(Texture::marble(64, 8.0, Color::new(204, 160, 82), Color::new(150, 108, 52), 0.45, 1204));
    let grey_tex = textures.add(Texture::marble(64, 8.0, Color::new(128, 133, 140), Color::new(84, 88, 96), 0.5, 1205));
    // El piso: mismo mármol, pero pulido — un poco de reflejo para las velas.
    let polished = |texture| Material::new(white, 60.0, [0.8, 0.3, 0.12, 0.0], 1.0, Some(texture));

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

        zombie_skin: Material::new(white, 6.0, [0.9, 0.05, 0.0, 0.0], 1.0, Some(rot_tex)).with_glow(0.16),
        blood: Material::new(Color::new(98, 10, 14), 50.0, [0.8, 0.35, 0.06, 0.0], 1.0, None).with_glow(0.1),
        satin: Material::new(Color::new(240, 236, 226), 40.0, [0.88, 0.22, 0.0, 0.0], 1.0, None),
        tux: Material::new(Color::new(30, 30, 36), 14.0, [0.9, 0.1, 0.0, 0.0], 1.0, None),
        lapel: Material::new(Color::new(22, 22, 28), 80.0, [0.8, 0.4, 0.0, 0.0], 1.0, None),
        lips: Material::new(Color::new(176, 104, 96), 20.0, [0.9, 0.12, 0.0, 0.0], 1.0, None).with_glow(0.08),
        rouge: Material::new(Color::new(150, 28, 40), 40.0, [0.9, 0.2, 0.0, 0.0], 1.0, None).with_glow(0.06),
        mouth: Material::new(Color::new(44, 8, 10), 30.0, [0.9, 0.15, 0.0, 0.0], 1.0, None),
        iris: Material::new(Color::new(30, 20, 14), 60.0, [0.8, 0.3, 0.0, 0.0], 1.0, None),
        rot: Material::new(Color::new(150, 128, 96), 6.0, [0.9, 0.05, 0.0, 0.0], 1.0, Some(rot_tex)).with_glow(0.11),
        skin: Material::new(Color::new(222, 176, 142), 12.0, [0.9, 0.08, 0.0, 0.0], 1.0, None).with_glow(0.1),
        hair: Material::new(Color::new(62, 40, 26), 8.0, [0.9, 0.05, 0.0, 0.0], 1.0, None),
        tux_grey: Material::new(Color::new(188, 190, 198), 20.0, [0.88, 0.15, 0.0, 0.0], 1.0, Some(veil_tex)),
        suit_navy: Material::new(Color::new(38, 46, 82), 16.0, [0.9, 0.12, 0.0, 0.0], 1.0, None),
        // El raso brilla un poco más que la lana.
        dress_fuchsia: Material::new(Color::new(196, 42, 124), 45.0, [0.85, 0.3, 0.0, 0.0], 1.0, None),
        dress_teal: Material::new(Color::new(28, 142, 140), 45.0, [0.85, 0.3, 0.0, 0.0], 1.0, None),
        hair_grey: Material::new(Color::new(168, 168, 172), 8.0, [0.9, 0.05, 0.0, 0.0], 1.0, None),
        hair_blonde: Material::new(Color::new(208, 172, 108), 8.0, [0.9, 0.05, 0.0, 0.0], 1.0, None),
        marble: Material::new(white, 24.0, [0.85, 0.12, 0.0, 0.0], 1.0, Some(marble_tex)),
        marble_dark: Material::new(white, 24.0, [0.85, 0.12, 0.0, 0.0], 1.0, Some(marble_dark_tex)),
        trim: Material::new(white, 16.0, [0.85, 0.1, 0.0, 0.0], 1.0, Some(trim_tex)),
        marble_grey: Material::new(white, 30.0, [0.85, 0.15, 0.0, 0.0], 1.0, Some(grey_tex)),
        floor_pale: polished(marble_tex),
        floor_dark: polished(marble_dark_tex),
        floor_beige: polished(trim_tex),
        floor_gold: polished(gold_tex),
        glass_cobalt: stained(Color::new(23, 71, 160)),
        glass_ruby: stained(Color::new(154, 38, 62)),
        glass_amber: stained(Color::new(230, 168, 59)),
        glass_pale: stained(Color::new(217, 230, 214)),
        iron: Material::new(Color::new(52, 56, 59), 60.0, [0.5, 0.4, 0.3, 0.0], 1.0, Some(metal_tex)),
        silver: Material::new(Color::new(206, 212, 222), 120.0, [0.75, 0.7, 0.2, 0.0], 1.0, Some(metal_tex)).with_glow(0.08),
    }
}

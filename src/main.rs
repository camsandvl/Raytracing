mod camera;
mod group;
mod color;
mod font;
mod intro;
mod light;
mod materials;
mod post;
mod ray_intersect;
mod rng;
mod scene;
mod skybox;
mod texture;
mod voxel_grid;
mod walker;

use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use nalgebra_glm::{dot, Vec3};
use std::f32::consts::PI;
use std::time::{Duration, Instant};

use camera::Camera;
use color::Color;
use light::Light;
use ray_intersect::{Intersect, RayIntersect};
use scene::{CameraPreset, Scene};
use texture::TextureBank;
use walker::{MoveInput, Spawn, Walker};

// Resolución interactiva (mientras la cámara se mueve); al soltarla se re-renderiza a
// la resolución completa de la ventana (`WIDTH * DISPLAY_SCALE`).
const WIDTH: usize = 640;
const HEIGHT: usize = 360;
const DISPLAY_SCALE: usize = 2;
/// Filas por franja del render de calidad completa (ver `render_rows`).
const FULL_BAND: usize = 90;

const FOV: f32 = PI / 3.0;
const MAX_DEPTH: u32 = 3;
const SURFACE_BIAS: f32 = 1e-3;
const MAX_SHADOW_HITS: usize = 8;
/// Un rayo secundario (reflejo o refracción) que aportaría menos que esto al color del
/// píxel no se lanza: un reflejo del piso en el piso (12% × 12%) no se nota y costaba lo
/// mismo que cualquier otro rayo, sombras incluidas.
const MIN_RAY_WEIGHT: f32 = 0.03;
/// Luces cuya contribución (ya atenuada por distancia) queda por debajo de esto no se
/// evalúan — ahorra el rayo de sombra de cada vela lejana.
const LIGHT_CUTOFF: f32 = 0.04;
/// Distancia² mínima para la atenuación de las velas: sin esto, una superficie pegada
/// a una llama recibe una intensidad casi infinita y se quema a blanco.
const LIGHT_MIN_DIST2: f32 = 9.0;
/// Relleno ambiental frío (el cielo nocturno): nada queda negro absoluto, pero lo que
/// no alcanza ninguna vela ni la luna se lee en penumbra azulada.
const AMBIENT: Color = Color { r: 12, g: 14, b: 24 };
/// Exposición del tone mapping: con 1.4 los medios tonos quedan casi lineales y solo
/// las zonas junto a las llamas se comprimen hacia el blanco cálido.
const EXPOSURE: f32 = 1.8;
/// Bruma de profundidad: cada superficie se funde hacia `HAZE` según la distancia que
/// recorrió el rayo (la mitad a ~25 m). Los rayos que salen por una ventana al cielo no
/// se tocan: la luna y las estrellas quedan nítidas en los vanos.
const HAZE: Color = Color { r: 5, g: 7, b: 13 };
const HAZE_DENSITY: f32 = 0.0055; // por unidad de mundo (20 cm)

const ORBIT_SPEED: f32 = 1.5; // rad/s
const MOUSE_ORBIT_SPEED: f32 = 0.005; // rad por pixel arrastrado
const ZOOM_SPEED: f32 = 0.8; // fracción del radio por segundo (teclado)
const WHEEL_ZOOM: f32 = 0.08; // fracción del radio por "click" de scroll
const WALK_TURN_SPEED: f32 = 1.8; // rad/s con las flechas, en primera persona

pub fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - normal * (2.0 * dot(incident, normal))
}

/// Ley de Snell. `ior` es el índice de refracción del material asumiendo que el rayo
/// viene del aire (n = 1.0) de un lado y del material del otro. `None` = reflexión
/// interna total (el ángulo es demasiado rasante para que la luz "escape" del medio
/// más denso — toda la energía se refleja en cambio).
pub fn refract(incident: &Vec3, normal: &Vec3, ior: f32) -> Option<Vec3> {
    let mut cosi = dot(incident, normal).clamp(-1.0, 1.0);
    let (n_ratio, n);
    if cosi < 0.0 {
        cosi = -cosi;
        n_ratio = 1.0 / ior;
        n = *normal;
    } else {
        n_ratio = ior;
        n = -normal;
    }
    let k = 1.0 - n_ratio * n_ratio * (1.0 - cosi * cosi);
    if k < 0.0 {
        None
    } else {
        Some(incident * n_ratio + n * (n_ratio * cosi - k.sqrt()))
    }
}

/// Aproximación de Schlick a Fresnel: qué fracción de la luz se refleja (el resto
/// refracta) según el ángulo de incidencia — a ángulos rasantes casi todo se refleja,
/// de frente casi todo refracta.
pub fn fresnel_schlick(incident: &Vec3, normal: &Vec3, ior: f32) -> f32 {
    let cosi = dot(incident, normal).clamp(-1.0, 1.0);
    let (n1, n2) = if cosi > 0.0 { (ior, 1.0) } else { (1.0, ior) };
    let cosi = cosi.abs();
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cosi).powi(5)
}

/// Origen de un rayo secundario, corrido un poquito del lado de la superficie hacia
/// el que viaja: un rayo que entra al vidrio arranca apenas adentro, uno que sale (o
/// se refleja) arranca apenas afuera. Si se corriera siempre hacia el mismo lado, el
/// rayo que sale del vidrio arrancaría todavía adentro y se volvería a encontrar con
/// la misma cara de salida una y otra vez.
fn offset_origin(point: &Vec3, normal: &Vec3, direction: &Vec3) -> Vec3 {
    if dot(direction, normal) >= 0.0 {
        point + normal * SURFACE_BIAS
    } else {
        point - normal * SURFACE_BIAS
    }
}

fn closest_hit(origin: &Vec3, direction: &Vec3, objects: &[Box<dyn RayIntersect>]) -> Option<Intersect> {
    ray_intersect::nearest_hit(objects, origin, direction, f32::INFINITY)
}

/// Cuánta luz llega desde `point` hasta la luz, por canal (0 = sombra total, 1 = sin
/// obstáculos). Un objeto opaco en el camino bloquea todo; uno transparente (el vidrio)
/// deja pasar su fracción de transparencia TEÑIDA de su color y el rayo sigue — así la
/// luna que entra por un vitral pinta el piso de azul, rojo y ámbar.
fn shadow_transmission(point: &Vec3, normal: &Vec3, light_dir: &Vec3, light_distance: f32, objects: &[Box<dyn RayIntersect>]) -> Vec3 {
    let mut origin = offset_origin(point, normal, light_dir);
    let mut remaining = light_distance;
    let mut transmission = Vec3::new(1.0, 1.0, 1.0);

    for _ in 0..MAX_SHADOW_HITS {
        // No hace falta el impacto más cercano de todos: en cuanto un objeto reporta algo
        // opaco antes de la luz, la sombra es total, sin mirar el resto de los objetos (la
        // catedral va primero y es la que tapa casi siempre).
        let mut nearest_glass: Option<Intersect> = None;
        for object in objects {
            if object.entry_distance(&origin, light_dir).is_none_or(|t| t >= remaining) {
                continue;
            }
            if let Some(hit) = object.ray_intersect_within(&origin, light_dir, remaining) {
                if hit.material.albedo[3] <= 0.0 {
                    return Vec3::zeros();
                }
                if nearest_glass.is_none_or(|g| hit.distance < g.distance) {
                    nearest_glass = Some(hit);
                }
            }
        }
        let Some(hit) = nearest_glass else {
            return transmission;
        };
        let transparency = hit.material.albedo[3];
        // Cada panel de vidrio son dos cruces (entrada y salida): se atenúa solo al entrar.
        if dot(light_dir, &hit.normal) < 0.0 {
            transmission = transmission.component_mul(&(linear(hit.material.diffuse) * transparency));
        }
        remaining -= hit.distance;
        origin = offset_origin(&hit.point, &hit.normal, light_dir);
    }
    transmission
}

/// Color de 8 bits → color lineal en punto flotante. La iluminación se acumula en
/// flotante (sin tope en 1.0) y recién al final se comprime con `tonemap` — si se
/// acumulara en `u8`, cada suma saturaría y todo lo cercano a una vela se quemaría a
/// blanco puro, además de producir bandas en los degradados oscuros de la noche.
fn linear(color: Color) -> Vec3 {
    Vec3::new(color.r as f32, color.g as f32, color.b as f32) / 255.0
}

/// Tone mapping exponencial por canal: casi lineal en los medios tonos, comprime las
/// altas luces suavemente hacia el blanco en vez de recortarlas.
fn tonemap(color: &Vec3) -> u32 {
    let map = |c: f32| ((1.0 - (-c.max(0.0) * EXPOSURE).exp()) * 255.0).round() as u32;
    (map(color.x) << 16) | (map(color.y) << 8) | map(color.z)
}

/// Iluminación local de un punto — el Phong del curso (difuso + especular por luz,
/// con sombras), con el difuso teñido por el color de cada luz (las velas iluminan
/// cálido, la luna frío). Devuelve `(superficie, contraluz)`: el contraluz solo existe
/// en materiales transparentes, y es la luz que llega desde ATRÁS del panel y lo
/// atraviesa dispersándose hacia quien mira — lo que hace brillar un vitral de noche
/// cuando hay velas del otro lado.
pub fn shade(intersect: &Intersect, ray_origin: &Vec3, lights: &[Light], objects: &[Box<dyn RayIntersect>], textures: &TextureBank) -> (Vec3, Vec3) {
    let material = &intersect.material;
    let mut base = linear(material.diffuse);
    if let Some(tex_id) = material.texture {
        base = base.component_mul(&linear(textures.get(tex_id).sample(intersect.u, intersect.v)));
    }

    if material.emission > 0.0 {
        return (base * material.emission, Vec3::zeros());
    }

    let view_direction = (ray_origin - intersect.point).normalize();
    let transparency = material.albedo[3];
    let mut surface = base.component_mul(&linear(AMBIENT)) + base * material.glow;
    let mut backlight = Vec3::zeros();

    for light in lights {
        let (light_direction, light_distance, attenuation) = if light.directional {
            (light.position, f32::INFINITY, 1.0)
        } else {
            let to_light = light.position - intersect.point;
            let distance = to_light.magnitude();
            // Inverso del cuadrado de la distancia — cada vela ilumina un charco
            // cercano y se apaga rápido, que es el look "iluminado solo por velas".
            let direction = to_light / distance;
            let cone = light.spot.map_or(1.0, |spot| spot.falloff(&-direction));
            (direction, distance, cone / (distance * distance).max(LIGHT_MIN_DIST2))
        };

        // Se resta el umbral en vez de solo cortar: así cada vela se apaga suave hasta 0
        // justo donde deja de evaluarse, sin un borde visible en muros y piso.
        let intensity = light.intensity * attenuation - LIGHT_CUTOFF;
        let facing = dot(&intersect.normal, &light_direction);
        if intensity <= 0.0 || (facing <= 0.0 && transparency <= 0.0) {
            continue;
        }

        if light.aperture.is_some_and(|a| !a.admits(&intersect.point, &light_direction, light_distance)) {
            continue;
        }
        let transmission = shadow_transmission(&intersect.point, &intersect.normal, &light_direction, light_distance, objects);
        if transmission.max() <= 0.0 {
            continue;
        }

        let light_color = linear(light.color).component_mul(&transmission) * intensity;
        if facing > 0.0 {
            surface += base.component_mul(&light_color) * (facing * material.albedo[0]);
            let reflect_dir = reflect(&-light_direction, &intersect.normal);
            let specular = dot(&view_direction, &reflect_dir).max(0.0).powf(material.specular);
            surface += light_color * (specular * material.albedo[1]);
        } else {
            backlight += base.component_mul(&light_color) * (-facing * transparency);
        }
    }

    (surface, backlight)
}

pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Box<dyn RayIntersect>],
    lights: &[Light],
    textures: &TextureBank,
    depth: u32,
    weight: f32,
) -> Vec3 {
    if depth > MAX_DEPTH {
        return linear(skybox::sample(ray_direction));
    }

    let Some(intersect) = closest_hit(ray_origin, ray_direction, objects) else {
        return linear(skybox::sample(ray_direction));
    };
    let color = hit_color(&intersect, ray_origin, ray_direction, objects, lights, textures, depth, weight);
    let fade = (-intersect.distance * HAZE_DENSITY).exp();
    color * fade + linear(HAZE) * (1.0 - fade)
}

/// El color de lo que el rayo encontró: su iluminación, y el reflejo o la refracción si
/// el material los tiene. `weight` es cuánto aporta este rayo al píxel; los rayos
/// secundarios que aportarían menos que `MIN_RAY_WEIGHT` no se lanzan.
#[allow(clippy::too_many_arguments)]
fn hit_color(
    intersect: &Intersect,
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Box<dyn RayIntersect>],
    lights: &[Light],
    textures: &TextureBank,
    depth: u32,
    weight: f32,
) -> Vec3 {
    let (local_color, backlight) = shade(intersect, ray_origin, lights, objects, textures);
    let material = &intersect.material;
    let (reflectivity, transparency) = (material.albedo[2], material.albedo[3]);
    let normal = intersect.normal;
    let trace = |direction: Vec3, share: f32| {
        let origin = offset_origin(&intersect.point, &normal, &direction);
        cast_ray(&origin, &direction, objects, lights, textures, depth + 1, share)
    };

    if transparency > 0.0 {
        let refract_dir = refract(ray_direction, &normal, material.refractive_index).map(|d| d.normalize());
        // Sin refracción posible (reflexión interna total) toda la energía se refleja.
        let fresnel = if refract_dir.is_some() { fresnel_schlick(ray_direction, &normal, material.refractive_index) } else { 1.0 };
        let share = weight * transparency;
        let reflect_color = (share * fresnel >= MIN_RAY_WEIGHT).then(|| trace(reflect(ray_direction, &normal).normalize(), share * fresnel));
        let refract_color = refract_dir.filter(|_| share * (1.0 - fresnel) >= MIN_RAY_WEIGHT).map(|d| trace(d, share * (1.0 - fresnel)));

        // Lo que se ve a través (y reflejado en) el vidrio sale filtrado por su color.
        let blended = match (refract_color, reflect_color) {
            (Some(through), Some(mirror)) => through + (mirror - through) * fresnel,
            (Some(through), None) => through,
            (None, Some(mirror)) => mirror,
            (None, None) => local_color,
        };
        let transmitted = blended.component_mul(&linear(material.diffuse));
        return local_color * (1.0 - transparency) + transmitted * transparency + backlight;
    }

    if reflectivity > 0.0 && weight * reflectivity >= MIN_RAY_WEIGHT {
        let reflect_color = trace(reflect(ray_direction, &normal).normalize(), weight * reflectivity);
        return local_color * (1.0 - reflectivity) + reflect_color * reflectivity;
    }

    local_color
}

/// Reparte el trabajo por franjas horizontales de filas entre los núcleos disponibles
/// con `std::thread::scope` — nada de `Arc`/`Mutex` hace falta porque `chunks_mut`
/// entrega franjas del buffer que no se superponen, y la escena solo se LEE durante el
/// render (`Sync` alcanza, ver `RayIntersect`). Las franjas son finas (varias por
/// hilo) para que ninguna quede mucho más cara que el resto: el cielo es barato, la
/// nave con velas no.
fn render(buffer: &mut [u32], width: usize, height: usize, objects: &[Box<dyn RayIntersect>], camera: &Camera, lights: &[Light], textures: &TextureBank) {
    render_rows(buffer, width, height, 0, objects, camera, lights, textures);
}

/// Como `render`, pero solo una franja de filas: `buffer` son las filas desde
/// `first_row` de una imagen de `width` × `height`. La ventana renderiza la calidad
/// completa así, de a franjas, para seguir atendiendo el teclado y el mouse entre una y
/// otra.
#[allow(clippy::too_many_arguments)]
fn render_rows(buffer: &mut [u32], width: usize, height: usize, first_row: usize, objects: &[Box<dyn RayIntersect>], camera: &Camera, lights: &[Light], textures: &TextureBank) {
    let rows = buffer.len() / width;
    let w = width as f32;
    let h = height as f32;
    let aspect_ratio = w / h;
    let perspective_scale = (FOV / 2.0).tan();

    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let rows_per_chunk = rows.div_ceil(threads * 4).max(1);
    let chunks = std::sync::Mutex::new(buffer.chunks_mut(rows_per_chunk * width).enumerate());

    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let next = chunks.lock().unwrap().next();
                let Some((index, chunk)) = next else { break };
                let y_start = first_row + index * rows_per_chunk;
                for (row_offset, row) in chunk.chunks_mut(width).enumerate() {
                    let y = y_start + row_offset;
                    let screen_y = (-(2.0 * y as f32 + 1.0) / h + 1.0) * perspective_scale;
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let screen_x = ((2.0 * x as f32 + 1.0) / w - 1.0) * aspect_ratio * perspective_scale;
                        let ray_direction = camera.basis_change(&Vec3::new(screen_x, screen_y, -1.0).normalize());
                        *pixel = tonemap(&cast_ray(&camera.eye, &ray_direction, objects, lights, textures, 0, 1.0));
                    }
                }
            });
        }
    });
}

/// Render con antialiasing por supersampling: se renderiza a `ss`× la resolución y se
/// promedia cada bloque de `ss`×`ss` píxeles.
fn render_supersampled(width: usize, height: usize, ss: usize, objects: &[Box<dyn RayIntersect>], camera: &Camera, lights: &[Light], textures: &TextureBank) -> Vec<u32> {
    let (big_w, big_h) = (width * ss, height * ss);
    let mut big = vec![0u32; big_w * big_h];
    render(&mut big, big_w, big_h, objects, camera, lights, textures);
    if ss == 1 {
        return big;
    }

    let mut out = vec![0u32; width * height];
    let n = (ss * ss) as u32;
    for y in 0..height {
        for x in 0..width {
            let (mut r, mut g, mut b) = (0u32, 0u32, 0u32);
            for dy in 0..ss {
                for dx in 0..ss {
                    let px = big[(y * ss + dy) * big_w + x * ss + dx];
                    r += (px >> 16) & 0xFF;
                    g += (px >> 8) & 0xFF;
                    b += px & 0xFF;
                }
            }
            out[y * width + x] = ((r / n) << 16) | ((g / n) << 8) | (b / n);
        }
    }
    out
}

/// `bounds` es `Scene::camera_bounds`: una caja por cada sala real, para las escenas donde
/// no hay nada afuera de ellas (la catedral, solo interior). Fuera de esa unión la cámara
/// orbital no puede orbitar ni alejarse más (ver `Camera::with_bounds`); vacío la deja
/// libre (la iglesia original, que sí tiene vistas exteriores a propósito). El modo
/// primera persona no pasa por acá — tiene su propia colisión en `walker.rs`.
fn camera_from(preset: &CameraPreset, bounds: &[(Vec3, Vec3)]) -> Camera {
    Camera::new(preset.eye, preset.target, Vec3::new(0.0, 1.0, 0.0)).with_bounds(bounds)
}

/// `--snapshot <carpeta> [--width W] [--height H] [--ss N] [--view nombre]`: renderiza
/// las vistas predefinidas de la escena a PNG, sin abrir ventana. Sirve para revisar
/// la escena sin estar frente a la pantalla, y como base para los frames del video.
fn run_snapshot(presets: &[CameraPreset], bounds: &[(Vec3, Vec3)], lights: &[Light], textures: &TextureBank, objects: &[Box<dyn RayIntersect>], args: &[String]) {
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1));
    let dir = flag("--snapshot").expect("falta la carpeta de salida después de --snapshot");
    let width: usize = flag("--width").map_or(1280, |v| v.parse().expect("--width inválido"));
    let height: usize = flag("--height").map_or(720, |v| v.parse().expect("--height inválido"));
    let ss: usize = flag("--ss").map_or(2, |v| v.parse().expect("--ss inválido"));
    let only = flag("--view");

    std::fs::create_dir_all(dir).expect("no se pudo crear la carpeta de salida");
    for preset in presets.iter().filter(|p| only.is_none_or(|v| v == p.name)) {
        let start = Instant::now();
        let mut pixels = render_supersampled(width, height, ss, objects, &camera_from(preset, bounds), lights, textures);
        post::apply(&mut pixels, width, height);
        let image = image::RgbImage::from_fn(width as u32, height as u32, |x, y| {
            let px = pixels[y as usize * width + x as usize];
            image::Rgb([(px >> 16) as u8, (px >> 8) as u8, px as u8])
        });
        let path = std::path::Path::new(dir).join(format!("{}.png", preset.name));
        image.save(&path).expect("no se pudo guardar el PNG");
        println!("{} -> {} ({:.1}s)", preset.name, path.display(), start.elapsed().as_secs_f32());
    }
}

/// Qué le falta a la imagen de la ventana.
#[derive(Clone, Copy)]
enum Pending {
    /// La cámara se movió: vista previa a media resolución.
    Preview,
    /// La cámara quedó quieta: calidad completa, de a franjas desde `row`, fuera de la
    /// pantalla (en `sharp`). Recién cuando está entera, con los retoques, reemplaza a la
    /// vista previa de una sola vez: sin barrido de franjas ni salto al final.
    Full { row: usize, millis: u128 },
    /// Al día.
    Done,
}

#[allow(clippy::too_many_arguments)]
fn run_window(presets: &[CameraPreset], bounds: &[(Vec3, Vec3)], walk_spawn: Spawn, lights: &[Light], textures: &TextureBank, objects: &[Box<dyn RayIntersect>], skip_intro: bool) {
    let (display_w, display_h) = (WIDTH * DISPLAY_SCALE, HEIGHT * DISPLAY_SCALE);
    let mut window = Window::new("Diorama Raytracer", display_w, display_h, WindowOptions::default()).expect("no se pudo abrir la ventana");
    window.set_target_fps(60);

    // La placa de título y la escena de introducción (opcional, ver `intro.rs`); si el
    // usuario cierra la ventana ahí, no hay que seguir.
    let mut from_white = false;
    if !skip_intro {
        match intro::show(&mut window, display_w, display_h) {
            intro::Ending::Closed => return,
            intro::Ending::White => from_white = true,
            intro::Ending::Black => {}
        }
    }
    // Si la intro terminó en el blanco del flash, el diorama aparece desde ese blanco: se
    // aclara a partir del primer cuadro dibujado (el flash queda en pantalla mientras).
    let mut fade_start: Option<Instant> = None;
    let mut faded = vec![0u32; display_w * display_h];

    let mut preset_index = 0;
    let mut camera = camera_from(&presets[preset_index], bounds);
    let mut walker = Walker::new(walk_spawn);
    let mut walking = false;

    let mut interactive = vec![0u32; WIDTH * HEIGHT];
    let mut display = vec![0u32; display_w * display_h];
    let mut sharp = vec![0u32; display_w * display_h];

    let mut last_mouse: Option<(f32, f32)> = None;
    let mut last_frame = Instant::now();
    let mut pending = Pending::Preview;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;
        let mut moved = false;
        let key_axis = |pos: Key, neg: Key| (window.is_key_down(pos) as i32 - window.is_key_down(neg) as i32) as f32;

        // Arrastre del mouse con el botón izquierdo: orbita, o mira en primera persona.
        let mut drag = None;
        if window.get_mouse_down(MouseButton::Left) {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
                if let Some((lx, ly)) = last_mouse {
                    if mx != lx || my != ly {
                        drag = Some((mx - lx, my - ly));
                    }
                }
                last_mouse = Some((mx, my));
            }
        } else {
            last_mouse = None;
        }

        // F: entra/sale de la exploración en primera persona (retoma donde quedó).
        if window.is_key_pressed(Key::F, KeyRepeat::No) {
            walking = !walking;
            if walking {
                walker.apply_to(&mut camera);
            } else {
                camera = camera_from(&presets[preset_index], bounds);
            }
            moved = true;
        }

        if walking {
            // P: sacar una foto — solo el flash de la cámara sobre lo que se está viendo.
            if window.is_key_pressed(Key::P, KeyRepeat::No) {
                from_white = true;
                fade_start = Some(Instant::now());
            }
            if window.is_key_pressed(Key::R, KeyRepeat::No) {
                walker.respawn();
                moved = true;
            }
            let (turn, tilt) = (key_axis(Key::Left, Key::Right), key_axis(Key::Up, Key::Down));
            if turn != 0.0 || tilt != 0.0 {
                walker.look(turn * WALK_TURN_SPEED * dt, tilt * WALK_TURN_SPEED * dt);
                moved = true;
            }
            if let Some((dx, dy)) = drag {
                walker.look(-dx * MOUSE_ORBIT_SPEED, -dy * MOUSE_ORBIT_SPEED);
                moved = true;
            }
            let input = MoveInput {
                forward: key_axis(Key::W, Key::S),
                right: key_axis(Key::D, Key::A),
                run: window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift),
                jump: window.is_key_pressed(Key::Space, KeyRepeat::No),
            };
            moved |= walker.update(dt, &input, objects);
            if moved {
                walker.apply_to(&mut camera);
            }
        } else {
            // C: siguiente vista predefinida (exterior, frente, aérea, ábside, interior).
            if window.is_key_pressed(Key::C, KeyRepeat::No) {
                preset_index = (preset_index + 1) % presets.len();
                camera = camera_from(&presets[preset_index], bounds);
                moved = true;
            }

            // Órbita con teclado (igual que el curso: flechas) como respaldo accesible.
            let keys = [(Key::Left, -1.0, 0.0), (Key::Right, 1.0, 0.0), (Key::Up, 0.0, -1.0), (Key::Down, 0.0, 1.0)];
            for (key, yaw, pitch) in keys {
                if window.is_key_down(key) {
                    camera.orbit(yaw * ORBIT_SPEED * dt, pitch * ORBIT_SPEED * dt);
                    moved = true;
                }
            }
            if window.is_key_down(Key::W) || window.is_key_down(Key::Equal) {
                camera.zoom(-camera.radius() * ZOOM_SPEED * dt);
                moved = true;
            }
            if window.is_key_down(Key::S) || window.is_key_down(Key::Minus) {
                camera.zoom(camera.radius() * ZOOM_SPEED * dt);
                moved = true;
            }
            if let Some((dx, dy)) = drag {
                camera.orbit(dx * MOUSE_ORBIT_SPEED, dy * MOUSE_ORBIT_SPEED);
                moved = true;
            }
            if let Some((_, scroll)) = window.get_scroll_wheel() {
                if scroll != 0.0 {
                    camera.zoom(-scroll.signum() * camera.radius() * WHEEL_ZOOM);
                    moved = true;
                }
            }
        }
        let mode = if walking { "primera persona (F: salir, WASD, flechas/arrastrar: mirar, Shift: correr, Espacio: saltar, R: reaparecer, P: foto)" } else { "orbital (F: primera persona)" };

        if moved {
            pending = Pending::Preview;
        }

        match pending {
            Pending::Preview => {
                let start = Instant::now();
                render(&mut interactive, WIDTH, HEIGHT, objects, &camera, lights, textures);
                post::apply(&mut interactive, WIDTH, HEIGHT);
                for y in 0..display_h {
                    for x in 0..display_w {
                        display[y * display_w + x] = interactive[(y / DISPLAY_SCALE) * WIDTH + x / DISPLAY_SCALE];
                    }
                }
                window.set_title(&format!("Diorama Raytracer - {mode} - {} ms (moviendo)", start.elapsed().as_millis()));
                pending = if moved { Pending::Preview } else { Pending::Full { row: 0, millis: 0 } };
            }
            Pending::Full { row, millis } => {
                // Una franja por vuelta: si la cámara se mueve a mitad de camino, se
                // abandona y vuelve la vista previa sin esperar al resto.
                let start = Instant::now();
                let end = (row + FULL_BAND).min(display_h);
                render_rows(&mut sharp[row * display_w..end * display_w], display_w, display_h, row, objects, &camera, lights, textures);
                let millis = millis + start.elapsed().as_millis();
                pending = if end < display_h {
                    Pending::Full { row: end, millis }
                } else {
                    post::apply(&mut sharp, display_w, display_h);
                    display.copy_from_slice(&sharp);
                    window.set_title(&format!("Diorama Raytracer - {mode} - {millis} ms (calidad completa)"));
                    Pending::Done
                };
            }
            Pending::Done => {}
        }

        if from_white {
            let level = intro::fade_in_level(fade_start.get_or_insert_with(Instant::now).elapsed());
            from_white = level > 0;
            if !from_white {
                fade_start = None; // listo para el próximo flash
            }
            intro::whiten(&display, level, &mut faded);
            window.update_with_buffer(&faded, display_w, display_h).expect("fallo actualizando la ventana");
        } else {
            window.update_with_buffer(&display, display_w, display_h).expect("fallo actualizando la ventana");
        }
        std::thread::sleep(Duration::from_millis(16).saturating_sub(now.elapsed()));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let start = Instant::now();
    let mut textures = TextureBank::new();
    // La catedral; `--church` abre la iglesia original.
    let build = if args.iter().any(|a| a == "--church") { scene::church::build } else { scene::cathedral::build };
    let Scene { grids, groups, lights, presets, walk_spawn, camera_bounds } = build(&mut textures);
    let objects = scene::into_objects(grids, groups);
    println!("escena construida en {:.2}s, {} luces", start.elapsed().as_secs_f32(), lights.len());

    if args.iter().any(|a| a == "--snapshot") {
        run_snapshot(&presets, &camera_bounds, &lights, &textures, &objects, &args);
    } else {
        // `--no-intro` salta la placa de título y la escena de introducción: para iterar
        // rápido durante el desarrollo sin tener que apretar una tecla cada vez.
        let skip_intro = args.iter().any(|a| a == "--no-intro");
        run_window(&presets, &camera_bounds, walk_spawn, &lights, &textures, &objects, skip_intro);
    }
}

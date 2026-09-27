mod camera;
mod color;
mod light;
mod materials;
mod ray_intersect;
mod rng;
mod scene;
mod skybox;
mod texture;
mod voxel_grid;

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

// Resolución interactiva (mientras la cámara se mueve); al soltarla se re-renderiza a
// la resolución completa de la ventana (`WIDTH * DISPLAY_SCALE`).
const WIDTH: usize = 640;
const HEIGHT: usize = 360;
const DISPLAY_SCALE: usize = 2;

const FOV: f32 = PI / 3.0;
const MAX_DEPTH: u32 = 3;
const SURFACE_BIAS: f32 = 1e-3;
const MAX_SHADOW_HITS: usize = 8;
/// Luces cuya contribución (ya atenuada por distancia) queda por debajo de esto no se
/// evalúan — ahorra el rayo de sombra de cada vela lejana.
const LIGHT_CUTOFF: f32 = 0.015;
/// Distancia² mínima para la atenuación de las velas: sin esto, una superficie pegada
/// a una llama recibe una intensidad casi infinita y se quema a blanco.
const LIGHT_MIN_DIST2: f32 = 9.0;
/// Relleno ambiental frío (el cielo nocturno): nada queda negro absoluto, pero lo que
/// no alcanza ninguna vela ni la luna se lee en penumbra azulada.
const AMBIENT: Color = Color { r: 24, g: 28, b: 42 };
/// Exposición del tone mapping: con 1.4 los medios tonos quedan casi lineales y solo
/// las zonas junto a las llamas se comprimen hacia el blanco cálido.
const EXPOSURE: f32 = 1.4;

const ORBIT_SPEED: f32 = 1.5; // rad/s
const MOUSE_ORBIT_SPEED: f32 = 0.005; // rad por pixel arrastrado
const ZOOM_SPEED: f32 = 0.8; // fracción del radio por segundo (teclado)
const WHEEL_ZOOM: f32 = 0.08; // fracción del radio por "click" de scroll

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
    let mut closest: Option<Intersect> = None;
    for object in objects {
        if let Some(hit) = object.ray_intersect(origin, direction) {
            if closest.is_none_or(|current| hit.distance < current.distance) {
                closest = Some(hit);
            }
        }
    }
    closest
}

/// Cuánta luz llega desde `point` hasta la luz (0 = sombra total, 1 = sin obstáculos).
/// Un objeto opaco en el camino bloquea todo; uno transparente (el vidrio) deja pasar
/// su fracción de transparencia y el rayo sigue — así la luz de las velas atraviesa
/// las ventanas y se derrama sobre la plaza de afuera.
fn shadow_transmission(point: &Vec3, normal: &Vec3, light_dir: &Vec3, light_distance: f32, objects: &[Box<dyn RayIntersect>]) -> f32 {
    let mut origin = offset_origin(point, normal, light_dir);
    let mut remaining = light_distance;
    let mut transmission = 1.0;

    for _ in 0..MAX_SHADOW_HITS {
        let Some(hit) = closest_hit(&origin, light_dir, objects) else {
            return transmission;
        };
        if hit.distance >= remaining {
            return transmission;
        }
        let transparency = hit.material.albedo[3];
        if transparency <= 0.0 {
            return 0.0;
        }
        // Cada panel de vidrio son dos cruces (entrada y salida): se atenúa solo al entrar.
        if dot(light_dir, &hit.normal) < 0.0 {
            transmission *= transparency;
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
    let mut surface = base.component_mul(&linear(AMBIENT));
    let mut backlight = Vec3::zeros();

    for light in lights {
        let (light_direction, light_distance, attenuation) = if light.directional {
            (light.position, f32::INFINITY, 1.0)
        } else {
            let to_light = light.position - intersect.point;
            let distance = to_light.magnitude();
            // Inverso del cuadrado de la distancia — cada vela ilumina un charco
            // cercano y se apaga rápido, que es el look "iluminado solo por velas".
            (to_light / distance, distance, 1.0 / (distance * distance).max(LIGHT_MIN_DIST2))
        };

        let intensity = light.intensity * attenuation;
        let facing = dot(&intersect.normal, &light_direction);
        if intensity < LIGHT_CUTOFF || (facing <= 0.0 && transparency <= 0.0) {
            continue;
        }

        let intensity = intensity * shadow_transmission(&intersect.point, &intersect.normal, &light_direction, light_distance, objects);
        if intensity <= 0.0 {
            continue;
        }

        let light_color = linear(light.color);
        if facing > 0.0 {
            surface += base.component_mul(&light_color) * (facing * material.albedo[0] * intensity);
            let reflect_dir = reflect(&-light_direction, &intersect.normal);
            let specular = dot(&view_direction, &reflect_dir).max(0.0).powf(material.specular);
            surface += light_color * (specular * material.albedo[1] * intensity);
        } else {
            backlight += base.component_mul(&light_color) * (-facing * transparency * intensity);
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
) -> Vec3 {
    if depth > MAX_DEPTH {
        return linear(skybox::sample(ray_direction));
    }

    let Some(intersect) = closest_hit(ray_origin, ray_direction, objects) else {
        return linear(skybox::sample(ray_direction));
    };

    let (local_color, backlight) = shade(&intersect, ray_origin, lights, objects, textures);

    let reflectivity = intersect.material.albedo[2];
    let transparency = intersect.material.albedo[3];

    if transparency > 0.0 {
        let normal = intersect.normal;
        let fresnel = fresnel_schlick(ray_direction, &normal, intersect.material.refractive_index);

        let reflect_dir = reflect(ray_direction, &normal).normalize();
        let reflect_origin = offset_origin(&intersect.point, &normal, &reflect_dir);
        let reflect_color = cast_ray(&reflect_origin, &reflect_dir, objects, lights, textures, depth + 1);

        let refract_color = match refract(ray_direction, &normal, intersect.material.refractive_index) {
            Some(refract_dir) => {
                let refract_dir = refract_dir.normalize();
                let refract_origin = offset_origin(&intersect.point, &normal, &refract_dir);
                cast_ray(&refract_origin, &refract_dir, objects, lights, textures, depth + 1)
            }
            // reflexión interna total: no hay refracción posible, toda la energía se refleja
            None => reflect_color,
        };

        // Lo que se ve a través (y reflejado en) el vidrio sale filtrado por su color.
        let blended = refract_color + (reflect_color - refract_color) * fresnel;
        let transmitted = blended.component_mul(&linear(intersect.material.diffuse));
        return local_color * (1.0 - transparency) + transmitted * transparency + backlight;
    }

    if reflectivity > 0.0 {
        let reflect_dir = reflect(ray_direction, &intersect.normal).normalize();
        let reflect_origin = offset_origin(&intersect.point, &intersect.normal, &reflect_dir);
        let reflect_color = cast_ray(&reflect_origin, &reflect_dir, objects, lights, textures, depth + 1);
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
    let w = width as f32;
    let h = height as f32;
    let aspect_ratio = w / h;
    let perspective_scale = (FOV / 2.0).tan();

    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let rows_per_chunk = height.div_ceil(threads * 4).max(1);
    let chunks = std::sync::Mutex::new(buffer.chunks_mut(rows_per_chunk * width).enumerate());

    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let next = chunks.lock().unwrap().next();
                let Some((index, chunk)) = next else { break };
                let y_start = index * rows_per_chunk;
                for (row_offset, row) in chunk.chunks_mut(width).enumerate() {
                    let y = y_start + row_offset;
                    let screen_y = (-(2.0 * y as f32 + 1.0) / h + 1.0) * perspective_scale;
                    for (x, pixel) in row.iter_mut().enumerate() {
                        let screen_x = ((2.0 * x as f32 + 1.0) / w - 1.0) * aspect_ratio * perspective_scale;
                        let ray_direction = camera.basis_change(&Vec3::new(screen_x, screen_y, -1.0).normalize());
                        *pixel = tonemap(&cast_ray(&camera.eye, &ray_direction, objects, lights, textures, 0));
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

fn camera_from(preset: &CameraPreset) -> Camera {
    Camera::new(preset.eye, preset.target, Vec3::new(0.0, 1.0, 0.0))
}

/// `--snapshot <carpeta> [--width W] [--height H] [--ss N] [--view nombre]`: renderiza
/// las vistas predefinidas de la escena a PNG, sin abrir ventana. Sirve para revisar
/// la escena sin estar frente a la pantalla, y como base para los frames del video.
fn run_snapshot(presets: &[CameraPreset], lights: &[Light], textures: &TextureBank, objects: &[Box<dyn RayIntersect>], args: &[String]) {
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1));
    let dir = flag("--snapshot").expect("falta la carpeta de salida después de --snapshot");
    let width: usize = flag("--width").map_or(1280, |v| v.parse().expect("--width inválido"));
    let height: usize = flag("--height").map_or(720, |v| v.parse().expect("--height inválido"));
    let ss: usize = flag("--ss").map_or(2, |v| v.parse().expect("--ss inválido"));
    let only = flag("--view");

    std::fs::create_dir_all(dir).expect("no se pudo crear la carpeta de salida");
    for preset in presets.iter().filter(|p| only.is_none_or(|v| v == p.name)) {
        let start = Instant::now();
        let pixels = render_supersampled(width, height, ss, objects, &camera_from(preset), lights, textures);
        let image = image::RgbImage::from_fn(width as u32, height as u32, |x, y| {
            let px = pixels[y as usize * width + x as usize];
            image::Rgb([(px >> 16) as u8, (px >> 8) as u8, px as u8])
        });
        let path = std::path::Path::new(dir).join(format!("{}.png", preset.name));
        image.save(&path).expect("no se pudo guardar el PNG");
        println!("{} -> {} ({:.1}s)", preset.name, path.display(), start.elapsed().as_secs_f32());
    }
}

fn run_window(presets: &[CameraPreset], lights: &[Light], textures: &TextureBank, objects: &[Box<dyn RayIntersect>]) {
    let (display_w, display_h) = (WIDTH * DISPLAY_SCALE, HEIGHT * DISPLAY_SCALE);
    let mut window = Window::new("Diorama Raytracer", display_w, display_h, WindowOptions::default()).expect("no se pudo abrir la ventana");
    window.set_target_fps(60);

    let mut preset_index = 0;
    let mut camera = camera_from(&presets[preset_index]);

    let mut interactive = vec![0u32; WIDTH * HEIGHT];
    let mut display = vec![0u32; display_w * display_h];

    let mut last_mouse: Option<(f32, f32)> = None;
    let mut last_frame = Instant::now();
    // `Some(false)` = hay que renderizar rápido (la cámara se movió); `Some(true)` = la
    // cámara quedó quieta, falta el render de calidad completa; `None` = al día.
    let mut pending: Option<bool> = Some(false);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;
        let mut moved = false;

        // C: siguiente vista predefinida (exterior, frente, aérea, ábside, interior).
        if window.is_key_pressed(Key::C, KeyRepeat::No) {
            preset_index = (preset_index + 1) % presets.len();
            camera = camera_from(&presets[preset_index]);
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

        // Órbita arrastrando el mouse con el botón izquierdo, zoom con la rueda.
        if window.get_mouse_down(MouseButton::Left) {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Pass) {
                if let Some((lx, ly)) = last_mouse {
                    if mx != lx || my != ly {
                        camera.orbit((mx - lx) * MOUSE_ORBIT_SPEED, (my - ly) * MOUSE_ORBIT_SPEED);
                        moved = true;
                    }
                }
                last_mouse = Some((mx, my));
            }
        } else {
            last_mouse = None;
        }
        if let Some((_, scroll)) = window.get_scroll_wheel() {
            if scroll != 0.0 {
                camera.zoom(-scroll.signum() * camera.radius() * WHEEL_ZOOM);
                moved = true;
            }
        }

        if moved {
            pending = Some(false);
        }

        match pending {
            Some(false) => {
                let start = Instant::now();
                render(&mut interactive, WIDTH, HEIGHT, objects, &camera, lights, textures);
                for y in 0..display_h {
                    for x in 0..display_w {
                        display[y * display_w + x] = interactive[(y / DISPLAY_SCALE) * WIDTH + x / DISPLAY_SCALE];
                    }
                }
                window.set_title(&format!("Diorama Raytracer - {} ms (moviendo)", start.elapsed().as_millis()));
                pending = if moved { Some(false) } else { Some(true) };
            }
            Some(true) => {
                let start = Instant::now();
                render(&mut display, display_w, display_h, objects, &camera, lights, textures);
                window.set_title(&format!("Diorama Raytracer - {} ms (calidad completa)", start.elapsed().as_millis()));
                pending = None;
            }
            None => {}
        }

        window.update_with_buffer(&display, display_w, display_h).expect("fallo actualizando la ventana");
        std::thread::sleep(Duration::from_millis(16).saturating_sub(now.elapsed()));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let start = Instant::now();
    let mut textures = TextureBank::new();
    let Scene { grids, lights, presets } = scene::church::build(&mut textures);
    let objects: Vec<Box<dyn RayIntersect>> = grids.into_iter().map(|g| Box::new(g) as Box<dyn RayIntersect>).collect();
    println!("escena construida en {:.2}s, {} luces", start.elapsed().as_secs_f32(), lights.len());

    if args.iter().any(|a| a == "--snapshot") {
        run_snapshot(&presets, &lights, &textures, &objects, &args);
    } else {
        run_window(&presets, &lights, &textures, &objects);
    }
}

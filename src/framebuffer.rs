/// Igual al `Framebuffer` de la fase de Software Rendering del curso: un buffer de
/// píxeles `u32` (0xRRGGBB) que `minifb` puede mostrar directo con `update_with_buffer`.
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub buffer: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer { width, height, buffer: vec![0; width * height] }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }
}

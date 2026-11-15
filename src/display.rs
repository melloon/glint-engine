//! The "television": turns the console's framebuffer of palette indices into
//! real RGB pixels and puts them in the host window. This is the only video
//! code that talks to macroquad.

use macroquad::prelude::*;

use crate::video::{Framebuffer, MASTER_COLORS, Rgb, SCREEN_H, SCREEN_W, master_palette};

pub struct Display {
    palette: [Rgb; MASTER_COLORS],
    image: Image,
    texture: Texture2D,
}

impl Display {
    pub fn new() -> Self {
        let image = Image::gen_image_color(SCREEN_W as u16, SCREEN_H as u16, BLACK);
        let texture = Texture2D::from_image(&image);
        // Nearest-neighbour keeps pixels as hard squares instead of blurring.
        texture.set_filter(FilterMode::Nearest);
        Self {
            palette: master_palette(),
            image,
            texture,
        }
    }

    /// Look up every pixel in the master palette, upload the result to the
    /// GPU and draw it as large as the window allows.
    pub fn present(&mut self, fb: &Framebuffer) {
        for (rgba, &index) in self.image.bytes.chunks_exact_mut(4).zip(fb.pixels()) {
            let [r, g, b] = self.palette[index as usize];
            rgba.copy_from_slice(&[r, g, b, 255]);
        }
        self.texture.update(&self.image);

        // Whole-number scale only: a fractional scale would make some pixels
        // wider than others and the picture would shimmer when scrolling.
        let scale = (screen_width() / SCREEN_W as f32)
            .min(screen_height() / SCREEN_H as f32)
            .floor()
            .max(1.0);
        let size = vec2(SCREEN_W as f32 * scale, SCREEN_H as f32 * scale);
        let origin = ((vec2(screen_width(), screen_height()) - size) / 2.0).floor();

        clear_background(BLACK);
        draw_texture_ex(
            &self.texture,
            origin.x,
            origin.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                ..Default::default()
            },
        );
    }

    /// Save what is actually in the window right now, scaling included.
    /// (Screen captures come back bottom-up and `export_png` flips them, so
    /// this goes through the screen rather than exporting `image` directly.)
    pub fn save_png(&self, path: &str) {
        get_screen_data().export_png(path);
    }
}

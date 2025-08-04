use spitfire_draw::{
    context::DrawContext,
    pixels::{Pixels, PixelsAccessRgba},
    sprite::Sprite,
    utils::{Drawable, ShaderRef, Vertex},
};
use spitfire_glow::{
    app::{App, AppControl, AppState},
    graphics::{Graphics, Shader},
    renderer::{GlowBlending, GlowTextureFiltering},
};
use vek::Rgba;

fn main() {
    App::<Vertex>::default().run(State::default());
}

#[derive(Default)]
struct State {
    context: DrawContext,
    pixels: Option<Pixels>,
}

impl State {
    fn redraw_pixels(width: usize, height: usize, mut pixels: PixelsAccessRgba) {
        for (index, pixel) in pixels.iter_mut().enumerate() {
            let x = index % width;
            let y = index / width;
            let cx = (x as f64 / width as f64) * 3.5 - 2.5;
            let cy = (y as f64 / height as f64) * 2.0 - 1.0;
            let (r, g, b) = mandelbrot_smooth(cx, cy);
            *pixel = Rgba::new(r, g, b, 255);
        }
    }
}

impl AppState<Vertex> for State {
    fn on_init(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        graphics.state.color = [0.25, 0.25, 0.25, 1.0];

        self.context.shaders.insert(
            "image".into(),
            graphics
                .shader(Shader::TEXTURED_VERTEX_2D, Shader::TEXTURED_FRAGMENT)
                .unwrap(),
        );
    }

    fn on_redraw(&mut self, graphics: &mut Graphics<Vertex>, _: &mut AppControl) {
        let width = graphics.state.main_camera.screen_size.x as usize / 2;
        let height = graphics.state.main_camera.screen_size.y as usize / 2;
        if self
            .pixels
            .as_ref()
            .map(|pixels| pixels.width() != width || pixels.height() != height)
            .unwrap_or(true)
        {
            self.pixels = Some(Pixels::simple(width as u32, height as u32, graphics).unwrap());
            let pixels = self.pixels.as_mut().unwrap();
            Self::redraw_pixels(width, height, pixels.access_rgba());
            pixels.commit();
        }

        self.context.begin_frame(graphics);
        self.context.push_shader(&ShaderRef::name("image"));
        self.context.push_blending(GlowBlending::Alpha);

        Sprite::single(
            self.pixels
                .as_mut()
                .unwrap()
                .sprite_texture("u_image".into(), GlowTextureFiltering::Linear),
        )
        .size(graphics.state.main_camera.screen_size)
        .draw(&mut self.context, graphics);

        self.context.end_frame();
    }
}

fn mandelbrot_smooth(cx: f64, cy: f64) -> (u8, u8, u8) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut iter = 0;
    let max_iter = 50;

    while x * x + y * y <= 4.0 && iter < max_iter {
        let xtemp = x * x - y * y + cx;
        y = 2.0 * x * y + cy;
        x = xtemp;
        iter += 1;
    }

    if iter == max_iter {
        return (0, 0, 0);
    }

    let zn = (x * x + y * y).sqrt();
    let nu = (zn.ln() / 2.0).ln() / std::f64::consts::LN_2;
    let smooth_iter = iter as f64 + 1.0 - nu;

    let t = (smooth_iter / max_iter as f64).sqrt();

    let r = (255.0 * t.powf(2.0)) as u8;
    let g = (255.0 * t.powf(1.5)) as u8;
    let b = (255.0 * t.powf(0.5)) as u8;

    (r, g, b)
}

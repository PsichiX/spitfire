use crate::{
    sprite::SpriteTexture,
    utils::{TextureRef, Vertex},
};
use spitfire_glow::{
    graphics::{Graphics, Texture},
    renderer::{GlowTextureFiltering, GlowTextureFormat},
};
use std::{
    borrow::Cow,
    ops::{Deref, DerefMut, Index, IndexMut},
};
use vek::{Clamp, Rgba};

pub struct Pixels {
    texture: Texture,
    buffer: Vec<u8>,
}

impl Pixels {
    pub fn simple(width: u32, height: u32, graphics: &Graphics<Vertex>) -> Result<Self, String> {
        Ok(Self {
            texture: graphics.texture(width, height, 1, GlowTextureFormat::Rgba, None)?,
            buffer: vec![0; (width * height * 4) as usize],
        })
    }

    pub fn from_screen(graphics: &Graphics<Vertex>) -> Result<Self, String> {
        let width = graphics.state.main_camera.screen_size.x as u32;
        let height = graphics.state.main_camera.screen_size.y as u32;
        Ok(Self {
            texture: graphics.texture(width, height, 1, GlowTextureFormat::Rgba, None)?,
            buffer: vec![0; (width * height * 4) as usize],
        })
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn sprite_texture(
        &self,
        sampler: Cow<'static, str>,
        filtering: GlowTextureFiltering,
    ) -> SpriteTexture {
        SpriteTexture {
            sampler,
            texture: TextureRef::object(self.texture.clone()),
            filtering,
        }
    }

    pub fn width(&self) -> usize {
        self.texture.width() as usize
    }

    pub fn height(&self) -> usize {
        self.texture.height() as usize
    }

    pub fn access_bytes(&'_ mut self) -> PixelsAccessBytes<'_> {
        PixelsAccessBytes {
            width: self.width(),
            height: self.height(),
            stride: 4,
            buffer: &mut self.buffer,
        }
    }

    pub fn access_channels<'a>(&'a mut self) -> PixelsAccessChannels<'a> {
        PixelsAccessChannels {
            width: self.width(),
            height: self.height(),
            buffer: unsafe {
                std::slice::from_raw_parts_mut(
                    self.buffer.as_mut_ptr() as *mut [u8; 4],
                    self.width() * self.height(),
                )
            },
        }
    }

    pub fn access_rgba<'a>(&'a mut self) -> PixelsAccessRgba<'a> {
        PixelsAccessRgba {
            width: self.width(),
            height: self.height(),
            buffer: unsafe {
                std::slice::from_raw_parts_mut(
                    self.buffer.as_mut_ptr() as *mut Rgba<u8>,
                    self.width() * self.height(),
                )
            },
        }
    }

    pub fn commit(&mut self) {
        self.texture.upload(
            self.width() as _,
            self.height() as _,
            1,
            GlowTextureFormat::Rgba,
            Some(&self.buffer),
        );
    }
}

pub struct PixelsAccessBytes<'a> {
    width: usize,
    height: usize,
    stride: usize,
    buffer: &'a mut [u8],
}

impl PixelsAccessBytes<'_> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn stride(&self) -> usize {
        self.stride
    }
}

impl Deref for PixelsAccessBytes<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.buffer
    }
}

impl DerefMut for PixelsAccessBytes<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
    }
}

impl Index<[usize; 2]> for PixelsAccessBytes<'_> {
    type Output = u8;

    fn index(&self, index: [usize; 2]) -> &Self::Output {
        &self.buffer[(index[1] * self.width + index[0]) * self.stride]
    }
}

impl IndexMut<[usize; 2]> for PixelsAccessBytes<'_> {
    fn index_mut(&mut self, index: [usize; 2]) -> &mut Self::Output {
        &mut self.buffer[(index[1] * self.width + index[0]) * self.stride]
    }
}

pub struct PixelsAccessChannels<'a> {
    width: usize,
    height: usize,
    buffer: &'a mut [[u8; 4]],
}

impl PixelsAccessChannels<'_> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

impl Deref for PixelsAccessChannels<'_> {
    type Target = [[u8; 4]];

    fn deref(&self) -> &Self::Target {
        self.buffer
    }
}

impl DerefMut for PixelsAccessChannels<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
    }
}

impl Index<[usize; 2]> for PixelsAccessChannels<'_> {
    type Output = [u8; 4];

    fn index(&self, index: [usize; 2]) -> &Self::Output {
        &self.buffer[index[1] * self.width() + index[0]]
    }
}

impl IndexMut<[usize; 2]> for PixelsAccessChannels<'_> {
    fn index_mut(&mut self, index: [usize; 2]) -> &mut Self::Output {
        &mut self.buffer[index[1] * self.width() + index[0]]
    }
}

pub struct PixelsAccessRgba<'a> {
    width: usize,
    height: usize,
    buffer: &'a mut [Rgba<u8>],
}

impl<'a> PixelsAccessRgba<'a> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn blend<F: Fn(Rgba<f32>, Rgba<f32>) -> Rgba<f32>>(
        self,
        blend: F,
    ) -> PixelsAccessRgbaBlend<'a, F> {
        PixelsAccessRgbaBlend {
            access: self,
            blend,
        }
    }
}

impl Deref for PixelsAccessRgba<'_> {
    type Target = [Rgba<u8>];

    fn deref(&self) -> &Self::Target {
        self.buffer
    }
}

impl DerefMut for PixelsAccessRgba<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
    }
}

impl Index<[usize; 2]> for PixelsAccessRgba<'_> {
    type Output = Rgba<u8>;

    fn index(&self, index: [usize; 2]) -> &Self::Output {
        &self.buffer[index[1] * self.width() + index[0]]
    }
}

impl IndexMut<[usize; 2]> for PixelsAccessRgba<'_> {
    fn index_mut(&mut self, index: [usize; 2]) -> &mut Self::Output {
        &mut self.buffer[index[1] * self.width() + index[0]]
    }
}

pub struct PixelsAccessRgbaBlend<'a, F: Fn(Rgba<f32>, Rgba<f32>) -> Rgba<f32>> {
    access: PixelsAccessRgba<'a>,
    blend: F,
}

impl<'a, F: Fn(Rgba<f32>, Rgba<f32>) -> Rgba<f32>> PixelsAccessRgbaBlend<'a, F> {
    pub fn into_inner(self) -> PixelsAccessRgba<'a> {
        self.access
    }

    pub fn width(&self) -> usize {
        self.access.width
    }

    pub fn height(&self) -> usize {
        self.access.height
    }

    pub fn get(&self, index: [usize; 2]) -> Rgba<f32> {
        self.access[index].numcast().unwrap() / 255.0
    }

    pub fn blend(&mut self, index: [usize; 2], color: Rgba<f32>) {
        let rgba = &mut self.access[index];
        let color = (self.blend)(rgba.numcast().unwrap() / 255.0, color)
            .clamped(Rgba::<f32>::zero(), Rgba::<f32>::one());
        *rgba = (color * 255.0).numcast().unwrap();
    }
}

pub fn blend_overwrite(_: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    new
}

pub fn blend_additive(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    old + new
}

pub fn blend_subtractive(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    old - new
}

pub fn blend_multiply(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    old * new
}

pub fn blend_alpha(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    old * (1.0 - new.a) + new * new.a
}

pub fn blend_screen(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    Rgba::new(
        1.0 - (1.0 - old.r) * (1.0 - new.r),
        1.0 - (1.0 - old.g) * (1.0 - new.g),
        1.0 - (1.0 - old.b) * (1.0 - new.b),
        new.a,
    )
}

pub fn blend_overlay(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    let r = if old.r < 0.5 {
        2.0 * old.r * new.r
    } else {
        1.0 - 2.0 * (1.0 - old.r) * (1.0 - new.r)
    };
    let g = if old.g < 0.5 {
        2.0 * old.g * new.g
    } else {
        1.0 - 2.0 * (1.0 - old.g) * (1.0 - new.g)
    };
    let b = if old.b < 0.5 {
        2.0 * old.b * new.b
    } else {
        1.0 - 2.0 * (1.0 - old.b) * (1.0 - new.b)
    };
    Rgba::new(r, g, b, new.a)
}

pub fn blend_darken(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    Rgba::new(old.r.min(new.r), old.g.min(new.g), old.b.min(new.b), new.a)
}

pub fn blend_lighten(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    Rgba::new(old.r.max(new.r), old.g.max(new.g), old.b.max(new.b), new.a)
}

pub fn blend_difference(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    Rgba::new(
        (old.r - new.r).abs(),
        (old.g - new.g).abs(),
        (old.b - new.b).abs(),
        new.a,
    )
}

pub fn blend_exclusion(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    Rgba::new(
        old.r + new.r - 2.0 * old.r * new.r,
        old.g + new.g - 2.0 * old.g * new.g,
        old.b + new.b - 2.0 * old.b * new.b,
        new.a,
    )
}

pub fn blend_color_dodge(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    let r = if new.r == 0.0 {
        0.0
    } else {
        (old.r / (1.0 - new.r)).min(1.0)
    };
    let g = if new.g == 0.0 {
        0.0
    } else {
        (old.g / (1.0 - new.g)).min(1.0)
    };
    let b = if new.b == 0.0 {
        0.0
    } else {
        (old.b / (1.0 - new.b)).min(1.0)
    };
    Rgba::new(r, g, b, new.a)
}

pub fn blend_color_burn(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    let r = if new.r == 1.0 {
        1.0
    } else {
        (1.0 - (1.0 - old.r) / new.r).max(0.0)
    };
    let g = if new.g == 1.0 {
        1.0
    } else {
        (1.0 - (1.0 - old.g) / new.g).max(0.0)
    };
    let b = if new.b == 1.0 {
        1.0
    } else {
        (1.0 - (1.0 - old.b) / new.b).max(0.0)
    };
    Rgba::new(r, g, b, new.a)
}

pub fn blend_hard_light(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    let r = if new.r < 0.5 {
        2.0 * old.r * new.r
    } else {
        1.0 - 2.0 * (1.0 - old.r) * (1.0 - new.r)
    };
    let g = if new.g < 0.5 {
        2.0 * old.g * new.g
    } else {
        1.0 - 2.0 * (1.0 - old.g) * (1.0 - new.g)
    };
    let b = if new.b < 0.5 {
        2.0 * old.b * new.b
    } else {
        1.0 - 2.0 * (1.0 - old.b) * (1.0 - new.b)
    };
    Rgba::new(r, g, b, new.a)
}

pub fn blend_soft_light(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    let r = old.r + (new.r * (1.0 - old.r));
    let g = old.g + (new.g * (1.0 - old.g));
    let b = old.b + (new.b * (1.0 - old.b));
    Rgba::new(r, g, b, new.a)
}

pub fn blend_linear_dodge(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    Rgba::new(
        (old.r + new.r).min(1.0),
        (old.g + new.g).min(1.0),
        (old.b + new.b).min(1.0),
        new.a,
    )
}

pub fn blend_linear_burn(old: Rgba<f32>, new: Rgba<f32>) -> Rgba<f32> {
    Rgba::new(
        (old.r + new.r - 1.0).max(0.0),
        (old.g + new.g - 1.0).max(0.0),
        (old.b + new.b - 1.0).max(0.0),
        new.a,
    )
}

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
use vek::Rgba;

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
        let width = graphics.main_camera.screen_size.x as u32;
        let height = graphics.main_camera.screen_size.y as u32;
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

    pub fn access_bytes(&mut self) -> PixelsAccessBytes {
        PixelsAccessBytes {
            width: self.width(),
            height: self.height(),
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
    buffer: &'a mut [u8],
}

impl PixelsAccessBytes<'_> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
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
        &self.buffer[index[1] * self.width() + index[0]]
    }
}

impl IndexMut<[usize; 2]> for PixelsAccessBytes<'_> {
    fn index_mut(&mut self, index: [usize; 2]) -> &mut Self::Output {
        &mut self.buffer[index[1] * self.width() + index[0]]
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

impl PixelsAccessRgba<'_> {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
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

use bytemuck::{Pod, Zeroable};
use std::{ops::Range, vec::Drain};

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Triangle {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl Default for Triangle {
    fn default() -> Self {
        Self { a: 0, b: 1, c: 2 }
    }
}

impl Triangle {
    pub fn offset(mut self, offset: usize) -> Self {
        self.a += offset as u32;
        self.b += offset as u32;
        self.c += offset as u32;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VertexStreamToken {
    vertices: usize,
    triangles: usize,
    batches: usize,
}

pub struct VertexStream<V: Pod, B> {
    vertices: Vec<V>,
    triangles: Vec<Triangle>,
    batches: Vec<(B, Range<usize>)>,
    resize_count: usize,
}

impl<V: Pod, B> Default for VertexStream<V, B> {
    fn default() -> Self {
        Self {
            vertices: Vec::with_capacity(1024),
            triangles: Vec::with_capacity(1024),
            batches: Vec::with_capacity(1024),
            resize_count: 1024,
        }
    }
}

impl<V: Pod, B: Clone> Clone for VertexStream<V, B> {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            triangles: self.triangles.clone(),
            batches: self.batches.clone(),
            resize_count: self.resize_count,
        }
    }
}

impl<V: Pod, B> VertexStream<V, B> {
    pub fn new(resize_count: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(resize_count),
            triangles: Vec::with_capacity(resize_count),
            batches: Vec::with_capacity(resize_count),
            resize_count,
        }
    }

    pub fn fork(&self) -> Self {
        Self::new(self.resize_count)
    }

    pub fn token(&self) -> VertexStreamToken {
        VertexStreamToken {
            vertices: self.vertices.len(),
            triangles: self.triangles.len(),
            batches: self.batches.len(),
        }
    }

    /// # Safety
    /// By extracting part of stream, you might make this and new stream invalid!
    pub unsafe fn extract(&mut self, token: VertexStreamToken) -> Self {
        let VertexStreamToken {
            vertices,
            triangles,
            batches,
        } = token;
        let mut result = self.fork();
        unsafe {
            result.extend_vertices(self.vertices.drain(vertices..));
            result.extend_triangles(
                false,
                self.triangles.drain(triangles..).map(|mut triangle| {
                    triangle.a -= vertices as u32;
                    triangle.b -= vertices as u32;
                    triangle.c -= vertices as u32;
                    triangle
                }),
            );
            result.extend_batches(self.batches.drain(batches..).map(|(batch, mut range)| {
                range.start -= triangles;
                range.end -= triangles;
                (batch, range)
            }));
        }
        result
    }

    pub fn transformed(
        &mut self,
        mut f: impl FnMut(&mut Self),
        mut t: impl FnMut(&mut V),
    ) -> &mut Self {
        let start = self.vertices.len();
        f(self);
        let end = self.vertices.len();
        for vertex in &mut self.vertices[start..end] {
            t(vertex);
        }
        self
    }

    pub fn triangle(&mut self, vertices: [V; 3]) -> &mut Self {
        self.ensure_capacity();
        let offset = self.vertices.len();
        self.vertices.extend(vertices);
        self.triangles.push(Triangle::default().offset(offset));
        self
    }

    pub fn triangle_fan(&mut self, vertices: impl IntoIterator<Item = V>) -> &mut Self {
        self.ensure_capacity();
        let start = self.vertices.len() as u32;
        self.vertices.extend(vertices);
        let end = self.vertices.len() as u32;
        let count = (end - start).saturating_sub(2);
        let mut offset = start + 1;
        for _ in 0..count {
            self.triangles.push(Triangle {
                a: start,
                b: offset,
                c: offset + 1,
            });
            offset += 1;
        }
        self
    }

    pub fn triangle_strip(&mut self, vertices: impl IntoIterator<Item = V>) -> &mut Self {
        self.ensure_capacity();
        let start = self.vertices.len() as u32;
        self.vertices.extend(vertices);
        let end = self.vertices.len() as u32;
        let count = (end - start).saturating_sub(2);
        let mut offset = start;
        let mut flip = false;
        for _ in 0..count {
            self.triangles.push(if flip {
                Triangle {
                    a: offset + 1,
                    b: offset,
                    c: offset + 2,
                }
            } else {
                Triangle {
                    a: offset,
                    b: offset + 1,
                    c: offset + 2,
                }
            });
            offset += 1;
            flip = !flip;
        }
        self
    }

    pub fn quad(&mut self, vertices: [V; 4]) -> &mut Self {
        self.ensure_capacity();
        let offset = self.vertices.len();
        self.vertices.extend(vertices);
        self.triangles
            .push(Triangle { a: 0, b: 1, c: 2 }.offset(offset));
        self.triangles
            .push(Triangle { a: 2, b: 3, c: 0 }.offset(offset));
        self
    }

    pub fn extend(
        &mut self,
        vertices: impl IntoIterator<Item = V>,
        triangles: impl IntoIterator<Item = Triangle>,
    ) -> &mut Self {
        self.ensure_capacity();
        let offset = self.vertices.len();
        self.vertices.extend(vertices);
        self.triangles.extend(
            triangles
                .into_iter()
                .map(|triangle| triangle.offset(offset)),
        );
        self
    }

    /// # Safety
    /// By writing raw vertices you might produce invalid renderables!
    pub unsafe fn extend_vertices(&mut self, iter: impl IntoIterator<Item = V>) -> &Self {
        self.vertices.extend(iter);
        self
    }

    /// # Safety
    /// By writing raw triangles you might produce invalid renderables!
    pub unsafe fn extend_triangles(
        &mut self,
        relative: bool,
        iter: impl IntoIterator<Item = Triangle>,
    ) -> &Self {
        if relative {
            let offset = self.vertices.len();
            self.triangles
                .extend(iter.into_iter().map(|triangle| triangle.offset(offset)));
        } else {
            self.triangles.extend(iter);
        }
        self
    }

    /// # Safety
    /// By writing raw batches you might produce invalid renderables!
    pub unsafe fn extend_batches(
        &mut self,
        iter: impl IntoIterator<Item = (B, Range<usize>)>,
    ) -> &Self {
        self.batches.extend(iter);
        self
    }

    pub fn append(&mut self, other: &mut Self) {
        let offset = self.triangles.len();
        self.extend(other.vertices.drain(..), other.triangles.drain(..));
        self.batches.extend(
            other
                .batches
                .drain(..)
                .map(|(data, range)| (data, (range.start + offset)..(range.end + offset))),
        );
    }

    pub fn append_cloned(&mut self, other: &Self)
    where
        B: Clone,
    {
        let offset = self.triangles.len();
        self.extend(
            other.vertices.iter().copied(),
            other.triangles.iter().copied(),
        );
        self.batches.extend(
            other
                .batches
                .iter()
                .map(|(data, range)| (data.clone(), (range.start + offset)..(range.end + offset))),
        );
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.triangles.clear();
        self.batches.clear();
    }

    pub fn batch(&mut self, data: B) {
        if self.batches.len() == self.batches.capacity() {
            self.batches.reserve_exact(self.resize_count);
        }
        self.batch_end();
        let start = self.triangles.len();
        self.batches.push((data, start..start))
    }

    pub fn batch_optimized(&mut self, data: B)
    where
        B: PartialEq,
    {
        if let Some(last) = self.batches.last_mut()
            && last.0 == data
        {
            last.1.end = self.triangles.len();
            return;
        }
        self.batch(data);
    }

    pub fn batch_end(&mut self) {
        if let Some(last) = self.batches.last_mut() {
            last.1.end = self.triangles.len();
        }
    }

    pub fn render<R: VertexStreamRenderer<V, B>>(
        &mut self,
        renderer: &mut R,
    ) -> Result<(), R::Error> {
        self.batch_end();
        renderer.render(self)
    }

    pub fn vertices(&self) -> &[V] {
        &self.vertices
    }

    pub fn triangles(&self) -> &[Triangle] {
        &self.triangles
    }

    pub fn batches(&self) -> &[(B, Range<usize>)] {
        &self.batches
    }

    #[allow(clippy::type_complexity)]
    pub fn drain(
        &'_ mut self,
    ) -> (
        Drain<'_, V>,
        Drain<'_, Triangle>,
        Drain<'_, (B, Range<usize>)>,
    ) {
        self.batch_end();
        (
            self.vertices.drain(..),
            self.triangles.drain(..),
            self.batches.drain(..),
        )
    }

    fn ensure_capacity(&mut self) {
        if self.vertices.len() == self.vertices.capacity() {
            self.vertices.reserve_exact(self.resize_count);
        }
        if self.triangles.len() == self.triangles.capacity() {
            self.triangles.reserve_exact(self.resize_count);
        }
    }
}

pub trait VertexStreamRenderer<V: Pod, B> {
    type Error;

    fn render(&mut self, stream: &mut VertexStream<V, B>) -> Result<(), Self::Error>;
}

impl<V: Pod, B> VertexStreamRenderer<V, B> for () {
    type Error = ();

    fn render(&mut self, _: &mut VertexStream<V, B>) -> Result<(), Self::Error> {
        Ok(())
    }
}

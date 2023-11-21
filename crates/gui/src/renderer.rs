use fontdue::layout::{HorizontalAlign, VerticalAlign};
use raui_core::prelude::*;
use spitfire_draw::prelude::*;
use spitfire_glow::prelude::*;
use vek::{Rgba, Vec2};

pub struct GuiRenderer<'a> {
    pub texture_filtering: GlowTextureFiltering,
    pub draw: &'a mut DrawContext,
    pub graphics: &'a mut Graphics<Vertex>,
    pub colored_shader: &'a ShaderRef,
    pub textured_shader: &'a ShaderRef,
    pub text_shader: &'a ShaderRef,
}

impl<'a> GuiRenderer<'a> {
    fn draw_node(&mut self, node: &WidgetUnit, mapping: &CoordsMapping, layout: &Layout) {
        match node {
            WidgetUnit::None | WidgetUnit::PortalBox(_) => {}
            WidgetUnit::AreaBox(node) => {
                self.draw_node(&node.slot, mapping, layout);
            }
            WidgetUnit::ContentBox(node) => {
                for item in &node.items {
                    self.draw_node(&item.slot, mapping, layout);
                }
            }
            WidgetUnit::FlexBox(node) => {
                for item in &node.items {
                    self.draw_node(&item.slot, mapping, layout);
                }
            }
            WidgetUnit::GridBox(node) => {
                for item in &node.items {
                    self.draw_node(&item.slot, mapping, layout);
                }
            }
            WidgetUnit::SizeBox(node) => {
                self.draw_node(&node.slot, mapping, layout);
            }
            WidgetUnit::ImageBox(node) => {
                if let Some(layout) = layout.items.get(&node.id) {
                    let rect = mapping.virtual_to_real_rect(layout.ui_space, false);
                    match &node.material {
                        ImageBoxMaterial::Color(color) => {
                            let tint = Rgba {
                                r: color.color.r,
                                g: color.color.g,
                                b: color.color.b,
                                a: color.color.a,
                            };
                            let mut size = Vec2::new(rect.width(), rect.height());
                            let mut position = Vec2::new(rect.left, rect.top);
                            match &color.scaling {
                                ImageBoxImageScaling::Stretch => {
                                    Sprite::default()
                                        .shader(self.colored_shader.clone())
                                        .tint(tint)
                                        .size(size)
                                        .position(position)
                                        .blending(GlowBlending::Alpha)
                                        .screen_space(true)
                                        .draw(self.draw, self.graphics);
                                }
                                ImageBoxImageScaling::Frame(frame) => {
                                    position += size * 0.5;
                                    if frame.frame_keep_aspect_ratio {
                                        let source_aspect =
                                            frame.source.width() / frame.source.height();
                                        let size_aspect = size.x / size.y;
                                        if source_aspect >= size_aspect {
                                            size.y /= source_aspect;
                                        } else {
                                            size.x *= source_aspect;
                                        }
                                    }
                                    NineSliceSprite::default()
                                        .shader(self.colored_shader.clone())
                                        .tint(tint)
                                        .size(size)
                                        .position(position)
                                        .pivot(0.5.into())
                                        .blending(GlowBlending::Alpha)
                                        .margins_source(NineSliceMargins {
                                            left: frame.source.left,
                                            right: frame.source.right,
                                            top: frame.source.top,
                                            bottom: frame.source.bottom,
                                        })
                                        .margins_target(NineSliceMargins {
                                            left: frame.destination.left,
                                            right: frame.destination.right,
                                            top: frame.destination.top,
                                            bottom: frame.destination.bottom,
                                        })
                                        .frame_only(frame.frame_only)
                                        .screen_space(true)
                                        .draw(self.draw, self.graphics);
                                }
                            }
                        }
                        ImageBoxMaterial::Image(image) => {
                            let texture = TextureRef::name(image.id.to_owned());
                            let rect = if let Some(aspect) = node.content_keep_aspect_ratio {
                                let size = self
                                    .draw
                                    .texture(Some(&texture))
                                    .map(|texture| {
                                        Vec2::new(texture.width() as f32, texture.height() as f32)
                                    })
                                    .unwrap_or(Vec2::one());
                                let ox = rect.left;
                                let oy = rect.top;
                                let iw = rect.width();
                                let ih = rect.height();
                                let ra = size.x / size.y;
                                let ia = iw / ih;
                                let scale = if (ra >= ia) != aspect.outside {
                                    iw / size.x
                                } else {
                                    ih / size.y
                                };
                                let w = size.x * scale;
                                let h = size.y * scale;
                                let ow = lerp(0.0, iw - w, aspect.horizontal_alignment);
                                let oh = lerp(0.0, ih - h, aspect.vertical_alignment);
                                Rect {
                                    left: ox + ow,
                                    right: ox + ow + w,
                                    top: oy + oh,
                                    bottom: oy + oh + h,
                                }
                            } else {
                                rect
                            };
                            let tint = Rgba {
                                r: image.tint.r,
                                g: image.tint.g,
                                b: image.tint.b,
                                a: image.tint.a,
                            };
                            let mut size = Vec2::new(rect.width(), rect.height());
                            let mut position = Vec2::new(rect.left, rect.top);
                            match &image.scaling {
                                ImageBoxImageScaling::Stretch => {
                                    Sprite::single(SpriteTexture {
                                        sampler: "u_image".into(),
                                        texture,
                                        filtering: self.texture_filtering,
                                    })
                                    .shader(self.textured_shader.clone())
                                    .region_page(
                                        image
                                            .source_rect
                                            .map(|rect| vek::Rect {
                                                x: rect.left,
                                                y: rect.top,
                                                w: rect.width(),
                                                h: rect.height(),
                                            })
                                            .unwrap_or_else(|| vek::Rect {
                                                x: 0.0,
                                                y: 0.0,
                                                w: 1.0,
                                                h: 1.0,
                                            }),
                                        0.0,
                                    )
                                    .tint(tint)
                                    .size(size)
                                    .position(position)
                                    .blending(GlowBlending::Alpha)
                                    .screen_space(true)
                                    .draw(self.draw, self.graphics);
                                }
                                ImageBoxImageScaling::Frame(frame) => {
                                    position += size * 0.5;
                                    if frame.frame_keep_aspect_ratio {
                                        let source_aspect =
                                            frame.source.width() / frame.source.height();
                                        let size_aspect = size.x / size.y;
                                        if source_aspect >= size_aspect {
                                            size.y /= source_aspect;
                                        } else {
                                            size.x *= source_aspect;
                                        }
                                    }
                                    NineSliceSprite::single(SpriteTexture {
                                        sampler: "u_image".into(),
                                        texture: TextureRef::name(image.id.to_owned()),
                                        filtering: self.texture_filtering,
                                    })
                                    .shader(self.textured_shader.clone())
                                    .tint(tint)
                                    .size(size)
                                    .position(position)
                                    .pivot(0.5.into())
                                    .blending(GlowBlending::Alpha)
                                    .margins_source(NineSliceMargins {
                                        left: frame.source.left,
                                        right: frame.source.right,
                                        top: frame.source.top,
                                        bottom: frame.source.bottom,
                                    })
                                    .margins_target(NineSliceMargins {
                                        left: frame.destination.left,
                                        right: frame.destination.right,
                                        top: frame.destination.top,
                                        bottom: frame.destination.bottom,
                                    })
                                    .frame_only(frame.frame_only)
                                    .screen_space(true)
                                    .draw(self.draw, self.graphics);
                                }
                            }
                        }
                        ImageBoxMaterial::Procedural(_) => {
                            unimplemented!(
                                "Procedural images are not yet implemented in this version!"
                            );
                        }
                    }
                }
            }
            WidgetUnit::TextBox(node) => {
                if let Some(layout) = layout.items.get(node.id()) {
                    let rect = mapping.virtual_to_real_rect(layout.ui_space, false);
                    Text::default()
                        .shader(self.text_shader.clone())
                        .font(node.font.name.to_owned())
                        .size(node.font.size * mapping.scalar_scale(false))
                        .text(node.text.to_owned())
                        .tint(Rgba {
                            r: node.color.r,
                            g: node.color.g,
                            b: node.color.b,
                            a: node.color.a,
                        })
                        .horizontal_align(match node.horizontal_align {
                            TextBoxHorizontalAlign::Left => HorizontalAlign::Left,
                            TextBoxHorizontalAlign::Center => HorizontalAlign::Center,
                            TextBoxHorizontalAlign::Right => HorizontalAlign::Right,
                        })
                        .vertical_align(match node.vertical_align {
                            TextBoxVerticalAlign::Top => VerticalAlign::Top,
                            TextBoxVerticalAlign::Middle => VerticalAlign::Middle,
                            TextBoxVerticalAlign::Bottom => VerticalAlign::Bottom,
                        })
                        .position(Vec2::new(rect.left, rect.top))
                        .width(rect.width())
                        .height(rect.height())
                        .screen_space(true)
                        .draw(self.draw, self.graphics);
                }
            }
        }
    }
}

impl<'a> Renderer<(), ()> for GuiRenderer<'a> {
    fn render(
        &mut self,
        tree: &WidgetUnit,
        mapping: &CoordsMapping,
        layout: &Layout,
    ) -> Result<(), ()> {
        self.draw_node(tree, mapping, layout);
        Ok(())
    }
}

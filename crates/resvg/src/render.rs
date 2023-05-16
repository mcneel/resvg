// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use usvg::FuzzyEq;

use crate::geom::{ScreenRect, ScreenSize, UsvgRectExt};
use crate::tree::{ConvTransform, Group, Node, OptionLog, Tree};

pub struct Context {
    pub root_transform: usvg::Transform,
    pub target_size: ScreenSize,
    pub max_filter_region: ScreenRect,
}

impl Tree {
    /// Renders an SVG tree onto the pixmap.
    ///
    /// `transform` will be used as a root transform.
    /// Can be used to position SVG inside the `pixmap`.
    pub fn render(&self, transform: tiny_skia::Transform, pixmap: &mut tiny_skia::PixmapMut) {
        let target_size = ScreenSize::new(pixmap.width(), pixmap.height()).unwrap();

        let max_filter_region = ScreenRect::new(
            -(target_size.width() as i32),
            -(target_size.height() as i32),
            target_size.width() * 2,
            target_size.height() * 2,
        )
        .unwrap();

        let ts =
            usvg::utils::view_box_to_transform(self.view_box.rect, self.view_box.aspect, self.size);

        let root_transform = transform.pre_concat(ts.to_native());

        let ctx = Context {
            root_transform: usvg::Transform::from_native(root_transform),
            target_size,
            max_filter_region,
        };

        render_nodes(&self.children, &ctx, (0, 0), root_transform, pixmap);
    }
}

pub fn render_nodes(
    children: &[Node],
    ctx: &Context,
    parent_offset: (i32, i32),
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    for node in children {
        render_node(node, ctx, parent_offset, transform, pixmap);
    }
}

fn render_node(
    node: &Node,
    ctx: &Context,
    parent_offset: (i32, i32),
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) {
    match node {
        Node::Group(ref group) => {
            render_group(group, ctx, parent_offset, transform, pixmap);
        }
        Node::FillPath(ref path) => {
            crate::path::render_fill_path(
                path,
                tiny_skia::BlendMode::SourceOver,
                ctx,
                transform,
                pixmap,
            );
        }
        Node::StrokePath(ref path) => {
            crate::path::render_stroke_path(
                path,
                tiny_skia::BlendMode::SourceOver,
                ctx,
                transform,
                pixmap,
            );
        }
        Node::Image(ref image) => {
            crate::image::render_image(image, transform, pixmap);
        }
    }
}

fn render_group(
    group: &Group,
    ctx: &Context,
    parent_offset: (i32, i32), // TODO: test
    transform: tiny_skia::Transform,
    pixmap: &mut tiny_skia::PixmapMut,
) -> Option<()> {
    if group.bbox.fuzzy_eq(&usvg::PathBbox::new_bbox()) {
        log::warn!("Invalid group layer bbox detected.");
        return None;
    }

    let bbox = group.bbox.transform(&ctx.root_transform)?;

    let mut ibbox = if group.filters.is_empty() {
        // Convert group bbox into an integer one, expanding each side outwards by 2px
        // to make sure that anti-aliased pixels would not be clipped.
        ScreenRect::new(
            bbox.x().floor() as i32 - 2,
            bbox.y().floor() as i32 - 2,
            bbox.width().ceil() as u32 + 4,
            bbox.height().ceil() as u32 + 4,
        )?
    } else {
        // The bounding box for groups with filters is special and should not be expanded by 2px,
        // because it's already acting as a clipping region.
        let bbox = bbox.to_rect()?.to_screen_rect_round_out();
        // Make sure our filter region is not bigger than 2x the canvas size.
        // This is required mainly to prevent huge filter regions that would tank the performance.
        // It should not affect the final result in any way.
        bbox.fit_to_rect(ctx.max_filter_region)
    };

    let original_ibbox = ibbox;

    ibbox = ibbox.translate(-parent_offset.0, -parent_offset.1);

    // Make sure that our bbox is not bigger than the canvas.
    // There is no point in rendering anything outside the canvas,
    // since it will be clipped anyway.
    if group.filters.is_empty() {
        ibbox = ibbox.fit_to_rect(ctx.target_size.to_screen_rect());
    }

    let shift_ts = {
        // Original shift.
        let mut dx = bbox.x() as f32;
        let mut dy = bbox.y() as f32;

        // Account for subpixel positioned layers.
        dx -= bbox.x() as f32 - original_ibbox.x() as f32;
        dy -= bbox.y() as f32 - original_ibbox.y() as f32;

        // Include `parent_offset` and fit to canvas offsets.
        dx -= (original_ibbox.x() - ibbox.x()) as f32;
        dy -= (original_ibbox.y() - ibbox.y()) as f32;

        tiny_skia::Transform::from_translate(-dx, -dy)
    };

    let transform = shift_ts.pre_concat(transform);

    let mut sub_pixmap = tiny_skia::Pixmap::new(ibbox.width(), ibbox.height())
        .log_none(|| log::warn!("Failed to allocate a group layer for: {:?}.", ibbox))?;

    render_nodes(
        &group.children,
        ctx,
        (parent_offset.0 + ibbox.x(), parent_offset.1 + ibbox.y()),
        transform,
        &mut sub_pixmap.as_mut(),
    );

    if !group.filters.is_empty() {
        let fill_paint = prepare_filter_paint(group.filter_fill.as_ref(), ctx, &sub_pixmap);
        let stroke_paint = prepare_filter_paint(group.filter_stroke.as_ref(), ctx, &sub_pixmap);
        for filter in &group.filters {
            crate::filter::apply(
                filter,
                original_ibbox,
                &ctx.root_transform,
                fill_paint.as_ref(),
                stroke_paint.as_ref(),
                &mut sub_pixmap,
            );
        }
    }

    if let Some(ref clip_path) = group.clip_path {
        crate::clip::apply(clip_path, transform, &mut sub_pixmap);
    }

    if let Some(ref mask) = group.mask {
        crate::mask::apply(
            mask,
            ctx,
            (ibbox.x(), ibbox.y()),
            transform,
            &mut sub_pixmap,
        );
    }

    let paint = tiny_skia::PixmapPaint {
        opacity: group.opacity,
        blend_mode: group.blend_mode,
        quality: tiny_skia::FilterQuality::Nearest,
    };

    pixmap.draw_pixmap(
        ibbox.x(),
        ibbox.y(),
        sub_pixmap.as_ref(),
        &paint,
        tiny_skia::Transform::identity(),
        None,
    );

    Some(())
}

/// Renders an image used by `FillPaint`/`StrokePaint` filter input.
///
/// FillPaint/StrokePaint is mostly an undefined behavior and will produce different results
/// in every application.
/// And since there are no expected behaviour, we will simply fill the filter region.
///
/// https://github.com/w3c/fxtf-drafts/issues/323
fn prepare_filter_paint(
    paint: Option<&crate::paint_server::Paint>,
    ctx: &Context,
    pixmap: &tiny_skia::Pixmap,
) -> Option<tiny_skia::Pixmap> {
    use std::rc::Rc;

    let paint = paint?;
    let mut sub_pixmap = tiny_skia::Pixmap::new(pixmap.width(), pixmap.height()).unwrap();

    let rect = tiny_skia::Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32)?;
    let path = tiny_skia::PathBuilder::from_rect(rect);

    let path = crate::path::FillPath {
        transform: tiny_skia::Transform::default(),
        paint: paint.clone(), // TODO: remove clone
        rule: tiny_skia::FillRule::Winding,
        anti_alias: true,
        path: Rc::new(path),
    };

    crate::path::render_fill_path(
        &path,
        tiny_skia::BlendMode::SourceOver,
        ctx,
        tiny_skia::Transform::default(),
        &mut sub_pixmap.as_mut(),
    );

    Some(sub_pixmap)
}

pub trait TinySkiaPixmapMutExt {
    fn create_rect_mask(
        &self,
        transform: tiny_skia::Transform,
        rect: tiny_skia::Rect,
    ) -> Option<tiny_skia::Mask>;
}

impl TinySkiaPixmapMutExt for tiny_skia::PixmapMut<'_> {
    fn create_rect_mask(
        &self,
        transform: tiny_skia::Transform,
        rect: tiny_skia::Rect,
    ) -> Option<tiny_skia::Mask> {
        let path = tiny_skia::PathBuilder::from_rect(rect);

        let mut mask = tiny_skia::Mask::new(self.width(), self.height())?;
        mask.fill_path(&path, tiny_skia::FillRule::Winding, true, transform);

        Some(mask)
    }
}

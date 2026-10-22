//! CPU rasterization of timeline graphics into a transparent RGBA overlay
//! (Phase 5 slice 5: the export path).
//!
//! The live preview composites graphics as DOM boxes; this module is the
//! raster twin used by the `saturngraphicsblend` GStreamer element during
//! export. Geometry parity with the preview lives at the box level: transform
//! boxes are project pixels, text uses the same font size/leading inputs,
//! vertical text flows right-to-left like CSS `vertical-rl`, and rotation
//! pivots on the box centre like a CSS `rotate`.
//!
//! Pixels are straight (non-premultiplied) alpha throughout.

use saturn_core::project::{Graphic, GraphicKind, GraphicShadow, GraphicStroke};
use saturn_core::time::Tick;
use saturn_text::{TextAlignment, TextContext, TextStyle};

/// Renders every visible graphic active at `playhead` into a transparent
/// straight-alpha RGBA buffer of `width * height * 4` bytes. `scale`
/// multiplies project pixels (1.0 at native export resolution). Layers render
/// bottom-most first, matching the preview's `zIndex` ordering.
pub fn render_graphics_overlay(
    graphics: &[Graphic],
    playhead: Tick,
    width: u32,
    height: u32,
    scale: f64,
    fonts: &mut TextContext,
) -> Vec<u8> {
    let mut overlay = vec![0_u8; (width as usize) * (height as usize) * 4];
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };

    let mut active: Vec<&Graphic> = graphics
        .iter()
        .filter(|graphic| graphic.visible && graphic_covers(graphic, playhead))
        .collect();
    active.sort_by_key(|graphic| (graphic.layer, graphic.id));

    for graphic in active {
        let Some(layer) = render_graphic(graphic, scale, fonts) else {
            continue;
        };
        // Shadow first: it sits above lower graphics/media but under this
        // graphic's own ink (CSS paint order).
        if let Some(shadow) = &graphic.shadow {
            if shadow.blur > 0.0 || shadow.offset_x != 0.0 || shadow.offset_y != 0.0 {
                if let Some(shadow_layer) = shadow_layer_from(&layer, shadow, scale) {
                    blit_straight_over(
                        &mut overlay,
                        width,
                        height,
                        &shadow_layer.pixels,
                        shadow_layer.width,
                        shadow_layer.height,
                        shadow_layer.offset_x,
                        shadow_layer.offset_y,
                        graphic.transform.opacity.clamp(0.0, 1.0) as f32,
                    );
                }
            }
        }
        blit_straight_over(
            &mut overlay,
            width,
            height,
            &layer.pixels,
            layer.width,
            layer.height,
            layer.offset_x,
            layer.offset_y,
            graphic.transform.opacity.clamp(0.0, 1.0) as f32,
        );
    }
    overlay
}

fn graphic_covers(graphic: &Graphic, playhead: Tick) -> bool {
    let start = graphic.start_ticks.0;
    let end = start.saturating_add(graphic.duration_ticks.0);
    playhead.0 >= start && playhead.0 < end
}

/// A rasterized graphic: tightly-cropped RGBA content plus the top-left
/// offset it occupies inside the frame, in scaled project pixels.
pub struct GraphicLayer {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub offset_x: i64,
    pub offset_y: i64,
}

fn render_graphic(graphic: &Graphic, scale: f64, fonts: &mut TextContext) -> Option<GraphicLayer> {
    let transform = &graphic.transform;
    let box_w = (transform.width * scale).round().max(1.0) as u32;
    let box_h = (transform.height * scale).round().max(1.0) as u32;
    let origin_x = (transform.x * scale).round() as i64;
    let origin_y = (transform.y * scale).round() as i64;

    if graphic.kind == GraphicKind::Adjustment {
        // Adjustment layers grade the layers below; they carry no ink here.
        return None;
    }

    // Text may overflow its box in the preview, so the content canvas grows
    // to fit the shaped text; shapes use the box exactly.
    let shaped = if graphic.kind == GraphicKind::Text && graphic.text.is_some() {
        Some(shape_text_layer(graphic, fonts, scale))
    } else {
        None
    };
    let (content_w, content_h) = match &shaped {
        Some(shaped) => (box_w.max(shaped.width), box_h.max(shaped.height)),
        None => (box_w, box_h),
    };

    let mut pixels = vec![0_u8; (content_w as usize) * (content_h as usize) * 4];

    if let Some(shaped) = &shaped {
        let text = graphic.text.as_ref().expect("checked above");
        // Alignment mirrors the DOM box: left/center/right inside the box.
        let offset_x = match text.align.as_str() {
            "center" => (content_w as i64 - shaped.width as i64) / 2,
            "right" => content_w as i64 - shaped.width as i64,
            _ => 0,
        }
        .max(0);
        blit_straight_over(
            &mut pixels,
            content_w,
            content_h,
            &shaped.pixels,
            shaped.width,
            shaped.height,
            offset_x,
            0,
            1.0,
        );
    } else if graphic.kind == GraphicKind::Rectangle
        || graphic.kind == GraphicKind::Ellipse
        || graphic.kind == GraphicKind::Polygon
    {
        paint_shape(graphic, &mut pixels, content_w, content_h, scale);
    }

    // Shadow is composited at the frame level in render_graphics_overlay (a
    // blurred tinted copy of this layer's alpha, blitted under the ink), so
    // offsets may extend beyond the box without being cropped here.

    // Rotate around the box centre (CSS default transform-origin) when set;
    // the rotation helper returns the canvas offset in content coordinates,
    // so the frame origin is added on top.
    let (mut pixels, mut width, mut height, mut offset_x, mut offset_y) =
        if transform.rotation.abs() > f64::EPSILON {
            let (pixels, width, height, dx, dy) = rotate_around_box_centre(
                pixels,
                content_w,
                content_h,
                box_w,
                box_h,
                transform.rotation,
            );
            (pixels, width, height, origin_x + dx, origin_y + dy)
        } else {
            (pixels, content_w, content_h, origin_x, origin_y)
        };
    if let Some((cropped, crop_w, crop_h, dx, dy)) = crop_to_ink(&pixels, width, height) {
        pixels = cropped;
        width = crop_w;
        height = crop_h;
        offset_x += dx;
        offset_y += dy;
    }
    if pixels.is_empty() || width == 0 || height == 0 {
        return None;
    }
    Some(GraphicLayer {
        pixels,
        width,
        height,
        offset_x,
        offset_y,
    })
}

/// Shapes the graphic's text payload through saturn-text, honouring fill
/// colour/opacity, alignment, leading and vertical mode.
fn shape_text_layer(
    graphic: &Graphic,
    fonts: &mut TextContext,
    scale: f64,
) -> saturn_text::RgbaLayer {
    let text = graphic.text.as_ref().expect("caller checks text presence");
    let fill = graphic.fill.clone().unwrap_or_default();
    let [r, g, b, a] = parse_hex_color(&fill.color);
    let style = TextStyle {
        font_family: text.font_family.clone(),
        font_size: text.font_size.max(1.0) as f32,
        tracking: text.tracking.max(0.0) as f32,
        leading: if text.leading > 0.0 {
            text.leading
        } else {
            1.2
        } as f32,
        fill: [
            r,
            g,
            b,
            (a as f32 * fill.opacity.clamp(0.0, 1.0) as f32).round() as u8,
        ],
        alignment: match text.align.as_str() {
            "center" => TextAlignment::Center,
            "right" => TextAlignment::Right,
            _ => TextAlignment::Left,
        },
        vertical: text.vertical,
    };
    fonts.shape_rgba(&text.content, &style, scale)
}

/// Paints rect / ellipse / polygon ink (fill, then stroke over it) into the
/// box. Polygon strokes fall back to no ring in v1 (rect/ellipse rings use
/// the signed distance to their edges; polygon outlines need a path stroke).
fn paint_shape(graphic: &Graphic, pixels: &mut [u8], width: u32, height: u32, scale: f64) {
    if width == 0 || height == 0 {
        return;
    }
    let stroke: Option<&GraphicStroke> = graphic
        .stroke
        .as_ref()
        .filter(|s| s.width > 0.0 && graphic.kind != GraphicKind::Polygon);
    // CSS `border` draws fully inside the shape, hard-edged; the band spans
    // `width * scale` pixels from the edge inward with 1px anti-aliasing.
    let stroke_band = stroke.map(|s| s.width * scale).unwrap_or(0.0);

    let fill = graphic.fill.as_ref();
    let fill_alpha = fill.map(|f| (f.opacity.clamp(0.0, 1.0) * 255.0) as u8);
    let fill_rgb = fill.map(|f| parse_hex_color(&f.color));
    let stroke_rgb = stroke.map(|s| parse_hex_color(&s.color));

    let is_polygon = graphic.kind == GraphicKind::Polygon;
    let inside_polygon = |x: f64, y: f64| -> bool {
        let Some(points) = &graphic.points else {
            return false;
        };
        if points.len() < 3 {
            return false;
        }
        let nx = x / width as f64;
        let ny = y / height as f64;
        let mut inside = false;
        let mut j = points.len() - 1;
        for i in 0..points.len() {
            let (xi, yi) = (points[i][0], points[i][1]);
            let (xj, yj) = (points[j][0], points[j][1]);
            if (yi > ny) != (yj > ny)
                && nx < (xj - xi) * (ny - yi) / (yj - yi + f64::EPSILON.copysign(yj - yi)) + xi
            {
                inside = !inside;
            }
            j = i;
        }
        inside
    };

    // Signed distance to the shape boundary in output pixels (positive in).
    let edge_distance = |x: f64, y: f64| -> f64 {
        match graphic.kind {
            GraphicKind::Ellipse => {
                let rx = (width as f64 / 2.0).max(1.0);
                let ry = (height as f64 / 2.0).max(1.0);
                let dxn = (x - width as f64 / 2.0) / rx;
                let dyn_ = (y - height as f64 / 2.0) / ry;
                (1.0 - (dxn * dxn + dyn_ * dyn_).sqrt()) * rx.min(ry)
            }
            _ => {
                let dx = x.min(width as f64 - x);
                let dy = y.min(height as f64 - y);
                dx.min(dy)
            }
        }
    };

    const SS: u32 = 4; // supersampling grid per axis for edge anti-aliasing
    let step = 1.0 / SS as f64;
    for y in 0..height {
        for x in 0..width {
            let mut fill_cov = 0.0_f32;
            let mut stroke_cov = 0.0_f32;
            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f64 + (sx as f64 + 0.5) * step;
                    let py = y as f64 + (sy as f64 + 0.5) * step;
                    let inside = if is_polygon {
                        inside_polygon(px, py)
                    } else {
                        edge_distance(px, py) > 0.0
                    };
                    if inside {
                        fill_cov += 1.0;
                    }
                    if stroke_band > 0.0 {
                        let d = edge_distance(px, py);
                        // Band d ∈ [0, stroke_band], hard edges with 1px AA.
                        let outer = (d + 0.5).clamp(0.0, 1.0);
                        let inner = (stroke_band - d + 0.5).clamp(0.0, 1.0);
                        stroke_cov += outer.min(inner) as f32;
                    }
                }
            }
            let total = (SS * SS) as f32;
            fill_cov /= total;
            stroke_cov /= total;

            let mut color: Option<[u8; 4]> = None;
            if let (Some(rgb), Some(alpha)) = (fill_rgb, fill_alpha) {
                if fill_cov > 0.0 {
                    color = Some([
                        rgb[0],
                        rgb[1],
                        rgb[2],
                        (fill_cov * alpha as f32).round() as u8,
                    ]);
                }
            }
            if let Some(rgb) = stroke_rgb {
                if stroke_cov > 0.0 {
                    let a = (stroke_cov * 255.0).round() as u8;
                    color = match color {
                        None => Some([rgb[0], rgb[1], rgb[2], a]),
                        Some(existing) => {
                            // Stroke over fill (straight alpha).
                            let sa = a as f32 / 255.0;
                            let fa = existing[3] as f32 / 255.0;
                            let out_a = sa + fa * (1.0 - sa);
                            let mix = |s: u8, f: u8| -> u8 {
                                ((s as f32 * sa + f as f32 * fa * (1.0 - sa)) / out_a.max(1e-6))
                                    .round() as u8
                            };
                            Some([
                                mix(rgb[0], existing[0]),
                                mix(rgb[1], existing[1]),
                                mix(rgb[2], existing[2]),
                                (out_a * 255.0).round() as u8,
                            ])
                        }
                    };
                }
            }
            if let Some([r, g, b, a]) = color {
                if a > 0 {
                    write_pixel(pixels, width, x, y, r, g, b, a);
                }
            }
        }
    }
}

/// Builds the shadow layer for a rendered ink layer: a blurred, offset,
/// tinted copy of the ink's alpha. The 0.75 multiplier mirrors the preview's
/// `withAlpha(color, 0.75)` CSS shadow. The blur spreads the silhouette, so
/// the canvas is padded by the blur radius before filtering and the layer is
/// cropped to the final ink afterwards (offsets may extend past the box).
fn shadow_layer_from(
    ink: &GraphicLayer,
    shadow: &GraphicShadow,
    scale: f64,
) -> Option<GraphicLayer> {
    if ink.width == 0 || ink.height == 0 {
        return None;
    }
    let blur = (shadow.blur * scale).round().clamp(0.0, 500.0) as u32;
    let dx = (shadow.offset_x * scale).round() as i64;
    let dy = (shadow.offset_y * scale).round() as i64;
    let [sr, sg, sb, sa] = parse_hex_color(&shadow.color);
    let shadow_alpha = ((sa as f32) * 0.75).round() as u8;
    if shadow_alpha == 0 {
        return None;
    }

    let pad = blur as usize;
    let canvas_w = ink.width as usize + pad * 2;
    let canvas_h = ink.height as usize + pad * 2;
    let mut alpha_map = vec![0_u8; canvas_w * canvas_h];
    for y in 0..ink.height as usize {
        for x in 0..ink.width as usize {
            let source = ((y * ink.width as usize) + x) * 4 + 3;
            alpha_map[(y + pad) * canvas_w + (x + pad)] = ink.pixels[source];
        }
    }
    let blurred = box_blur(&alpha_map, canvas_w as u32, canvas_h as u32, blur);

    let mut shadow_pixels = vec![0_u8; canvas_w * canvas_h * 4];
    for (index, coverage) in blurred.iter().enumerate() {
        if *coverage == 0 {
            continue;
        }
        let slot = &mut shadow_pixels[index * 4..index * 4 + 4];
        slot[0] = sr;
        slot[1] = sg;
        slot[2] = sb;
        slot[3] = ((*coverage as u32) * (shadow_alpha as u32) / 255).min(255) as u8;
    }
    let Some((pixels, width, height, crop_dx, crop_dy)) =
        crop_to_ink(&shadow_pixels, canvas_w as u32, canvas_h as u32)
    else {
        return None;
    };
    Some(GraphicLayer {
        pixels,
        width,
        height,
        offset_x: ink.offset_x - pad as i64 + dx + crop_dx,
        offset_y: ink.offset_y - pad as i64 + dy + crop_dy,
    })
}

/// Separable box blur over an alpha map (horizontal then vertical pass).
fn box_blur(alpha: &[u8], width: u32, height: u32, radius: u32) -> Vec<u8> {
    if radius == 0 {
        return alpha.to_vec();
    }
    let radius = radius.min(250).min(width.max(1) / 2).max(1) as usize;
    let (w, h) = (width as usize, height as usize);
    let mut horizontal = vec![0_u16; w * h];
    for y in 0..h {
        for x in 0..w {
            let from = x.saturating_sub(radius);
            let to = (x + radius).min(w - 1);
            let sum: u32 = alpha[y * w + from..y * w + to + 1]
                .iter()
                .map(|v| *v as u32)
                .sum();
            horizontal[y * w + x] = (sum / (to - from + 1) as u32) as u16;
        }
    }
    let mut out = vec![0_u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let from = y.saturating_sub(radius);
            let to = (y + radius).min(h - 1);
            let sum: u32 = (from..=to).map(|yy| horizontal[yy * w + x] as u32).sum();
            out[y * w + x] = (sum / (to - from + 1) as u32) as u8;
        }
    }
    out
}

/// Rotates RGBA content around the transform-box centre (CSS semantics) into
/// the axis-aligned bounding box of the rotated content. Returns the rotated
/// canvas plus the offset of its top-left in original content coordinates
/// (pre-rotation space), so callers can translate to frame space.
fn rotate_around_box_centre(
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    box_w: u32,
    box_h: u32,
    degrees: f64,
) -> (Vec<u8>, u32, u32, i64, i64) {
    let angle = degrees.to_radians();
    let (sin, cos) = angle.sin_cos();
    let pivot = (box_w as f64 / 2.0, box_h as f64 / 2.0);

    // Content corners in pivot-relative rotated space.
    let corners = [
        (0.0_f64, 0.0_f64),
        (width as f64, 0.0),
        (0.0, height as f64),
        (width as f64, height as f64),
    ]
    .map(|(x, y)| {
        let (dx, dy) = (x - pivot.0, y - pivot.1);
        (dx * cos - dy * sin, dx * sin + dy * cos)
    });
    let min_x = corners.iter().map(|c| c.0).fold(f64::INFINITY, f64::min);
    let min_y = corners.iter().map(|c| c.1).fold(f64::INFINITY, f64::min);
    let max_x = corners
        .iter()
        .map(|c| c.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_y = corners
        .iter()
        .map(|c| c.1)
        .fold(f64::NEG_INFINITY, f64::max);

    let out_w = ((max_x - min_x).ceil() as u32).max(1);
    let out_h = ((max_y - min_y).ceil() as u32).max(1);
    let mut out = vec![0_u8; out_w as usize * out_h as usize * 4];

    for y in 0..out_h {
        for x in 0..out_w {
            // Output pixel (x, y) maps to pivot-relative local' at
            // (x + min_x, y + min_y); invert the rotation into content space.
            let dx = x as f64 + 0.5 + min_x;
            let dy = y as f64 + 0.5 + min_y;
            let sx = dx * cos + dy * sin + pivot.0;
            let sy = -dx * sin + dy * cos + pivot.1;
            if sx < 0.0 || sy < 0.0 || sx >= width as f64 || sy >= height as f64 {
                continue;
            }
            let src = ((sy as u32) * width + sx as u32) as usize * 4;
            let dst = ((y as usize) * out_w as usize + x as usize) * 4;
            out[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
        }
    }

    // The canvas's top-left corner sits at local' (min_x, min_y); translated
    // back to content coordinates the pivot must stay fixed, so the offset is
    // simply pivot + min (each corner (x,y) shows at origin + pivot +
    // (x + min_x, y + min_y), independent of the rotation itself).
    (
        out,
        out_w,
        out_h,
        (pivot.0 + min_x).round() as i64,
        (pivot.1 + min_y).round() as i64,
    )
}

/// Crops fully transparent margins; returns pixels + dims + top-left offset.
fn crop_to_ink(pixels: &[u8], width: u32, height: u32) -> Option<(Vec<u8>, u32, u32, i64, i64)> {
    if width == 0 || height == 0 {
        return None;
    }
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;
    let mut any = false;
    for y in 0..height {
        for x in 0..width {
            if pixels[((y * width + x) * 4 + 3) as usize] > 0 {
                any = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if !any {
        return None;
    }
    let out_w = max_x - min_x + 1;
    let out_h = max_y - min_y + 1;
    let mut out = vec![0_u8; out_w as usize * out_h as usize * 4];
    for y in 0..out_h {
        let src_row = (((y + min_y) * width + min_x) * 4) as usize;
        let dst_row = (y * out_w * 4) as usize;
        let span = (out_w * 4) as usize;
        out[dst_row..dst_row + span].copy_from_slice(&pixels[src_row..src_row + span]);
    }
    Some((out, out_w, out_h, min_x as i64, min_y as i64))
}

/// Straight-alpha blit of `src` into `dst` at `(x, y)` with a global opacity.
#[allow(clippy::too_many_arguments)]
fn blit_straight_over(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    src: &[u8],
    src_w: u32,
    src_h: u32,
    x: i64,
    y: i64,
    opacity: f32,
) {
    if src_w == 0 || src_h == 0 {
        return;
    }
    let opacity = opacity.clamp(0.0, 1.0);
    for row in 0..src_h as i64 {
        let dst_y = y + row;
        if dst_y < 0 || dst_y >= dst_h as i64 {
            continue;
        }
        for column in 0..src_w as i64 {
            let dst_x = x + column;
            if dst_x < 0 || dst_x >= dst_w as i64 {
                continue;
            }
            let src_index = ((row * src_w as i64 + column) * 4) as usize;
            let sa = (src[src_index + 3] as f32 / 255.0) * opacity;
            if sa <= 0.0 {
                continue;
            }
            let dst_index = ((dst_y as u32 * dst_w + dst_x as u32) * 4) as usize;
            let slot = &mut dst[dst_index..dst_index + 4];
            let da = slot[3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            if out_a <= 0.0 {
                continue;
            }
            let mix = |s: u8, d: u8| -> u8 {
                ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            slot[0] = mix(src[src_index], slot[0]);
            slot[1] = mix(src[src_index + 1], slot[1]);
            slot[2] = mix(src[src_index + 2], slot[2]);
            slot[3] = (out_a * 255.0).round() as u8;
        }
    }
}

fn write_pixel(pixels: &mut [u8], width: u32, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
    let index = ((y * width + x) * 4) as usize;
    pixels[index] = r;
    pixels[index + 1] = g;
    pixels[index + 2] = b;
    pixels[index + 3] = a;
}

/// Parses `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA` into straight RGBA bytes;
/// anything unparsable falls back to opaque white (schema default).
pub fn parse_hex_color(color: &str) -> [u8; 4] {
    let hex = color.trim().trim_start_matches('#');
    let pair = |chars: &str| -> Option<u8> { u8::from_str_radix(chars, 16).ok() };
    let nibble = |c: char| -> Option<u8> { c.to_digit(16).map(|d| (d * 17) as u8) };
    let parsed: Option<[u8; 4]> = match hex.len() {
        3 | 4 => {
            let chars: Vec<char> = hex.chars().collect();
            if chars.len() < 3 {
                None
            } else {
                let r = nibble(chars[0]);
                let g = nibble(chars[1]);
                let b = nibble(chars[2]);
                let a = if chars.len() == 4 {
                    nibble(chars[3])
                } else {
                    Some(255)
                };
                match (r, g, b, a) {
                    (Some(r), Some(g), Some(b), Some(a)) => Some([r, g, b, a]),
                    _ => None,
                }
            }
        }
        6 | 8 => {
            let r = pair(&hex[0..2]);
            let g = pair(&hex[2..4]);
            let b = pair(&hex[4..6]);
            let a = if hex.len() == 8 {
                pair(&hex[6..8])
            } else {
                Some(255)
            };
            match (r, g, b, a) {
                (Some(r), Some(g), Some(b), Some(a)) => Some([r, g, b, a]),
                _ => None,
            }
        }
        _ => None,
    };
    parsed.unwrap_or([255, 255, 255, 255])
}

#[cfg(test)]
mod tests {
    use super::*;
    use saturn_core::project::{GraphicFill, GraphicTransform};

    fn solid_rect(id: u64, x: f64, y: f64, w: f64, h: f64, color: &str) -> Graphic {
        Graphic {
            id,
            kind: GraphicKind::Rectangle,
            transform: GraphicTransform {
                x,
                y,
                width: w,
                height: h,
                ..Default::default()
            },
            fill: Some(GraphicFill {
                color: color.into(),
                opacity: 1.0,
            }),
            ..Default::default()
        }
    }

    fn pixel(buffer: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let index = ((y * width + x) * 4) as usize;
        [
            buffer[index],
            buffer[index + 1],
            buffer[index + 2],
            buffer[index + 3],
        ]
    }

    fn ink_bbox(layer: &GraphicLayer) -> (u32, u32, u32, u32, usize) {
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut count = 0;
        for y in 0..layer.height {
            for x in 0..layer.width {
                if layer.pixels[((y * layer.width + x) * 4 + 3) as usize] > 0 {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    count += 1;
                }
            }
        }
        (min_x, min_y, max_x, max_y, count)
    }

    #[test]
    fn hex_colors_parse_all_common_lengths() {
        assert_eq!(parse_hex_color("#fff"), [255, 255, 255, 255]);
        assert_eq!(parse_hex_color("#ff00"), [255, 255, 0, 0]);
        assert_eq!(parse_hex_color("#102030"), [16, 32, 48, 255]);
        assert_eq!(parse_hex_color("#10203040"), [16, 32, 48, 64]);
        assert_eq!(parse_hex_color("nope"), [255, 255, 255, 255]);
    }

    #[test]
    fn rectangle_rasterizes_solid_fill_at_its_box() {
        let fonts = &mut TextContext::new();
        let rect = solid_rect(1, 10.0, 20.0, 100.0, 50.0, "#ff0000");
        let overlay = render_graphics_overlay(&[rect], Tick(0), 200, 200, 1.0, fonts);
        // Centre of the box is red.
        let centre = pixel(&overlay, 200, 60, 45);
        assert_eq!(centre, [255, 0, 0, 255]);
        // Outside the box stays transparent.
        assert_eq!(pixel(&overlay, 200, 5, 5), [0, 0, 0, 0]);
        assert_eq!(pixel(&overlay, 200, 115, 75), [0, 0, 0, 0]);
    }

    #[test]
    fn inactive_or_hidden_graphics_render_nothing() {
        let fonts = &mut TextContext::new();
        let mut rect = solid_rect(1, 0.0, 0.0, 100.0, 100.0, "#00ff00");
        let at_end = render_graphics_overlay(
            &[rect.clone()],
            Tick(5 * saturn_core::time::TICKS_PER_SECOND),
            120,
            120,
            1.0,
            fonts,
        );
        assert!(
            at_end.iter().all(|&b| b == 0),
            "past the end nothing renders"
        );
        rect.visible = false;
        let hidden = render_graphics_overlay(&[rect], Tick(0), 120, 120, 1.0, fonts);
        assert!(
            hidden.iter().all(|&b| b == 0),
            "hidden graphics do not render"
        );
    }

    #[test]
    fn layer_order_lets_upper_graphics_cover_lower_ones() {
        let fonts = &mut TextContext::new();
        let lower = Graphic {
            layer: 0,
            ..solid_rect(1, 0.0, 0.0, 100.0, 100.0, "#ff0000")
        };
        let upper = Graphic {
            layer: 1,
            ..solid_rect(2, 20.0, 20.0, 60.0, 60.0, "#0000ff")
        };
        let overlay = render_graphics_overlay(&[lower, upper], Tick(0), 120, 120, 1.0, fonts);
        assert_eq!(
            pixel(&overlay, 120, 50, 50),
            [0, 0, 255, 255],
            "upper wins at overlap"
        );
        assert_eq!(
            pixel(&overlay, 120, 5, 5),
            [255, 0, 0, 255],
            "lower shows elsewhere"
        );
    }

    #[test]
    fn ellipse_rasterizes_inside_its_box() {
        let fonts = &mut TextContext::new();
        let ellipse = Graphic {
            kind: GraphicKind::Ellipse,
            transform: GraphicTransform {
                width: 100.0,
                height: 60.0,
                ..Default::default()
            },
            fill: Some(GraphicFill {
                color: "#00ff00".into(),
                opacity: 1.0,
            }),
            ..Default::default()
        };
        let overlay = render_graphics_overlay(&[ellipse], Tick(0), 120, 120, 1.0, fonts);
        assert_eq!(pixel(&overlay, 120, 60, 30)[1], 255, "ellipse centre green");
        // Corners of the bounding box stay outside the ellipse.
        assert_eq!(pixel(&overlay, 120, 2, 2), [0, 0, 0, 0]);
    }

    #[test]
    fn stroke_draws_a_ring_around_the_rectangle() {
        let fonts = &mut TextContext::new();
        let rect = Graphic {
            kind: GraphicKind::Rectangle,
            transform: GraphicTransform {
                width: 40.0,
                height: 40.0,
                ..Default::default()
            },
            fill: None,
            stroke: Some(GraphicStroke {
                color: "#0000ff".into(),
                width: 4.0,
            }),
            ..Default::default()
        };
        let overlay = render_graphics_overlay(&[rect], Tick(0), 80, 80, 1.0, fonts);
        let edge = pixel(&overlay, 80, 1, 20);
        assert_eq!(edge, [0, 0, 255, 255], "stroke ring at the inner edge");
        let centre = pixel(&overlay, 80, 20, 20);
        assert_eq!(centre, [0, 0, 0, 0], "no fill, ring only");
    }

    #[test]
    fn text_rasterizes_through_saturn_text() {
        let fonts = &mut TextContext::new();
        let mut graphic = Graphic::default();
        graphic.kind = GraphicKind::Text;
        graphic.transform.width = 400.0;
        graphic.transform.height = 120.0;
        graphic.fill = Some(GraphicFill {
            color: "#ffffff".into(),
            opacity: 1.0,
        });
        graphic.text = Some(saturn_core::project::GraphicText {
            content: "Hello".into(),
            ..Default::default()
        });
        let layer = render_graphic(&graphic, 1.0, fonts).expect("text ink expected");
        let bbox = ink_bbox(&layer);
        assert!(bbox.4 > 10, "expected rendered glyph ink, got {bbox:?}");
        // Nudge the transform and the ink must follow.
        graphic.transform.x = 37.0;
        let moved = render_graphic(&graphic, 1.0, fonts).expect("text ink expected");
        assert!(moved.offset_x >= 37, "text layer honours transform.x");
    }

    #[test]
    fn shadow_offsets_tinted_ink_under_the_shape() {
        let fonts = &mut TextContext::new();
        let rect = Graphic {
            kind: GraphicKind::Rectangle,
            transform: GraphicTransform {
                width: 40.0,
                height: 40.0,
                ..Default::default()
            },
            fill: Some(GraphicFill {
                color: "#ffffff".into(),
                opacity: 1.0,
            }),
            shadow: Some(GraphicShadow {
                color: "#ff0000".into(),
                blur: 0.0,
                offset_x: 80.0,
                offset_y: 0.0,
            }),
            ..Default::default()
        };
        // Frame wide enough to hold box + its fully-offset shadow.
        let overlay = render_graphics_overlay(&[rect], Tick(0), 160, 60, 1.0, fonts);
        let in_box = pixel(&overlay, 160, 20, 20);
        assert_eq!(in_box, [255, 255, 255, 255], "shape ink unchanged in box");
        let shadow_area = pixel(&overlay, 160, 100, 20);
        assert_eq!(
            shadow_area,
            [255, 0, 0, 191],
            "shadow of a #ff0000 ink at 0.75 alpha (CSS quantization)"
        );
    }

    #[test]
    fn rotation_pivots_around_the_box_centre() {
        let fonts = &mut TextContext::new();
        let mut rect = solid_rect(1, 0.0, 0.0, 100.0, 20.0, "#ff0000");
        rect.transform.rotation = 90.0;
        let layer = render_graphic(&rect, 1.0, fonts).expect("rotated ink expected");
        // A 100x20 box rotated 90° is a 20x100 upright column, centred on the
        // same pivot (50, 10).
        assert!(layer.height > layer.width, "rotated column expected");
        let bbox = ink_bbox(&layer);
        assert!(
            bbox.2 - bbox.0 <= 21,
            "tight crop keeps the 20px width: {bbox:?}"
        );
        // Pivot (50,10) in the box maps to the centre of the rotated layer.
        // Ink must exist near the layer centre.
        let cx = layer.width / 2;
        let cy = layer.height / 2;
        assert!(
            pixel(&layer.pixels, layer.width, cx, cy)[3] > 0,
            "pivot inked"
        );
    }

    #[test]
    fn opacity_blends_the_layer_over_the_frame() {
        let fonts = &mut TextContext::new();
        let mut rect = solid_rect(1, 0.0, 0.0, 10.0, 10.0, "#ff0000");
        rect.transform.opacity = 0.5;
        let overlay = render_graphics_overlay(&[rect], Tick(0), 20, 20, 1.0, fonts);
        let centre = pixel(&overlay, 20, 5, 5);
        assert_eq!(centre, [255, 0, 0, 128], "half opacity straight alpha");
    }
}

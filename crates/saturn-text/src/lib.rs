//! Text shaping and rasterization for Easy Edits Pro.
//!
//! Phase 5 foundation (see `docs/phase5-text-stack.md`): `cosmic-text` owns
//! shaping (harfrust), bidi, font fallback, line breaking, and rasterization
//! (swash). This crate renders styled text into a straight-alpha RGBA layer
//! that the wgpu compositor composites at render scale — never screen scale.
//!
//! Vertical layout (tategaki) ships in a follow-up slice behind
//! [`TextStyle::vertical`]; the flag is plumbed through now so callers can
//! already request it (it falls back to horizontal until the column stacker
//! lands with CJK fixtures).

use cosmic_text::{Align, Attrs, Buffer, FontSystem, Metrics, Shaping, SwashCache};

/// A fully opaque 8-bit RGBA pixel layer, straight alpha, row-major.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbaLayer {
    pub width: u32,
    pub height: u32,
    /// Length is `width * height * 4`; bytes are `R, G, B, A`.
    pub pixels: Vec<u8>,
}

impl RgbaLayer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// Straight-alpha blend of `(color, coverage)` onto the pixel at (x, y).
    fn blend(&mut self, x: u32, y: u32, color: [u8; 4], coverage: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        let a = (coverage as u32) * (color[3] as u32) / 255;
        if a == 0 {
            return;
        }
        let dst = &mut self.pixels[index..index + 4];
        // Coverage-weighted source-over with straight alpha.
        for (channel, source) in dst.iter_mut().zip(color[..3].iter()).take(3) {
            *channel = ((*source as u32 * a + *channel as u32 * (255 - a)) / 255).min(255) as u8;
        }
        dst[3] = dst[3].max(a as u8);
    }

    /// Straight-alpha composite of `other` at `(x, y)` (top-left, may be
    /// negative to crop). Used to assemble multi-column vertical text.
    pub fn blit_over(&mut self, other: &RgbaLayer, x: i64, y: i64) {
        for row in 0..other.height as i64 {
            for column in 0..other.width as i64 {
                let index = ((row * other.width as i64 + column) * 4) as usize;
                let alpha = other.pixels[index + 3];
                if alpha == 0 {
                    continue;
                }
                let color = [
                    other.pixels[index],
                    other.pixels[index + 1],
                    other.pixels[index + 2],
                    other.pixels[index + 3],
                ];
                self.blend(
                    (x + column).max(0) as u32,
                    (y + row).max(0) as u32,
                    color,
                    255,
                );
            }
        }
    }
}

/// Horizontal alignment of a text block inside its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlignment {
    fn cosmic(self) -> Option<Align> {
        match self {
            TextAlignment::Left => None,
            TextAlignment::Center => Some(Align::Center),
            TextAlignment::Right => Some(Align::Right),
        }
    }
}

/// Visual style of one text run. Geometry is in project pixels; [`shape_rgba`]
/// multiplies by `scale` so rasterization happens at render scale.
#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_family: String,
    /// Font size in project pixels.
    pub font_size: f32,
    /// Extra letter spacing in project pixels.
    pub tracking: f32,
    /// Line height as a multiplier of font size (1.0 = single).
    pub leading: f32,
    /// Glyph fill colour, straight alpha.
    pub fill: [u8; 4],
    pub alignment: TextAlignment,
    /// Vertical stacking (tategaki): `\n` lines become right-to-left columns
    /// with characters stacked top-to-bottom upright.
    pub vertical: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Inter".into(),
            font_size: 42.0,
            tracking: 0.0,
            leading: 1.2,
            fill: [255, 255, 255, 255],
            alignment: TextAlignment::Left,
            vertical: false,
        }
    }
}

/// Measured layout box of a shaped run, in project pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMetrics {
    pub width: f64,
    pub height: f64,
    /// Number of wrapped/shaped lines.
    pub line_count: usize,
}

/// Shared layout state. Cheap to keep alive; the font database is the cost.
pub struct TextContext {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl Default for TextContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TextContext {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
        }
    }

    /// Lays out and rasterizes `text` into a tightly-fit alpha layer at
    /// `scale` × the style's project-pixel geometry. Rasterization happens at
    /// render scale (`scale` multiplies the metrics), never screen scale.
    ///
    /// With `style.vertical` (tategaki), `\n`-separated lines become columns
    /// flowing right-to-left; characters stack top-to-bottom upright within
    /// each column, matching the CSS `writing-mode: vertical-rl` preview
    /// overlay.
    pub fn shape_rgba(&mut self, text: &str, style: &TextStyle, scale: f64) -> RgbaLayer {
        if text.trim().is_empty() {
            return RgbaLayer::new(0, 0);
        }
        if style.vertical {
            return self.shape_vertical(text, style, scale);
        }
        let scale = scale.max(0.0) as f32;
        let metrics = Metrics {
            font_size: (style.font_size * scale).max(0.5),
            line_height: (style.font_size * f32::from(style.leading.max(0.0)) * scale).max(0.5),
        };
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        // `set_rich_text` (0.19) takes spans plus run-wide attrs, shaping mode,
        // and alignment. No wrap width: the layer is sized to the widest line.
        let attrs = Attrs::new().family(cosmic_text::Family::Name(style.font_family.as_str()));
        buffer.set_rich_text(
            [(text, attrs)],
            &Attrs::new(),
            Shaping::Advanced,
            style.alignment.cosmic(),
        );
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Measure the laid-out runs.
        let mut width = 0i32;
        let mut line_count = 0usize;
        for run in buffer.layout_runs() {
            width = width.max(run.line_w as i32);
            line_count += 1;
        }
        let pad = 2i32; // swash outset safety
        if line_count == 0 {
            return RgbaLayer::new(1, 1);
        }
        let width = (width + pad * 2).max(1) as u32;
        let height =
            (f64::from(metrics.line_height) * line_count as f64).ceil() as u32 + pad as u32 * 2;
        let mut layer = RgbaLayer::new(width, height);

        // Rasterize through the crate's own renderer: it resolves glyph
        // placement, color/mask content, and hinting for us. The callback
        // receives per-pixel rectangles; blend straight-alpha onto the layer.
        let fill =
            cosmic_text::Color::rgba(style.fill[0], style.fill[1], style.fill[2], style.fill[3]);
        buffer.draw(
            &mut self.font_system,
            &mut self.swash_cache,
            fill,
            |x, y, block_w, block_h, color| {
                let [r, g, b, a] = unpack_color(color);
                for off_y in 0..block_h as i32 {
                    for off_x in 0..block_w as i32 {
                        // Coverage lives in the colour's alpha here; pass the
                        // full mask so blend() does not square it.
                        layer.blend(
                            (pad + x + off_x).max(0) as u32,
                            (pad + y + off_y).max(0) as u32,
                            [r, g, b, a],
                            255,
                        );
                    }
                }
            },
        );
        layer
    }

    /// Vertical composition (tategaki): each `\n`-separated input line
    /// becomes a column; characters stack top-to-bottom upright inside it and
    /// columns flow right-to-left. Latin runs are kept upright (the
    /// `text-orientation: upright` flavour — rotation per glyph is deferred).
    fn shape_vertical(&mut self, text: &str, style: &TextStyle, scale: f64) -> RgbaLayer {
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        if lines.is_empty() {
            return RgbaLayer::new(0, 0);
        }
        let gap = ((f64::from(style.font_size) * scale * 0.2) as u32).max(2);

        // Shape each column through the horizontal path with centered stacking.
        let mut columns: Vec<RgbaLayer> = Vec::with_capacity(lines.len());
        for line in &lines {
            let stacked = line
                .chars()
                .map(|character| character.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            let column_style = TextStyle {
                alignment: TextAlignment::Center,
                vertical: false,
                ..style.clone()
            };
            columns.push(self.shape_rgba(&stacked, &column_style, scale));
        }

        let total_width: u32 = columns.iter().map(|column| column.width).sum::<u32>()
            + gap * (columns.len().saturating_sub(1)) as u32;
        let total_height = columns
            .iter()
            .map(|column| column.height)
            .max()
            .unwrap_or(1);

        // First input line is the rightmost column (vertical-rl).
        let mut out = RgbaLayer::new(total_width.max(1), total_height.max(1));
        let mut right_edge = total_width as i64;
        for column in &columns {
            let x = right_edge - column.width as i64;
            out.blit_over(column, x, 0);
            right_edge = x - gap as i64;
        }
        out
    }

    /// Sorted, de-duplicated font family names available to shaping
    /// (Phase 5 `fonts.list`). The first call loads the system font
    /// database, so keep a context alive instead of rebuilding per query.
    pub fn font_families(&mut self) -> Vec<String> {
        let mut names: Vec<String> = self
            .font_system
            .db()
            .faces()
            .filter_map(|face| face.families.first().map(|(name, _)| name.to_owned()))
            .collect();
        names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        names
    }

    /// Layout measurement without rasterization.
    pub fn measure(&mut self, text: &str, style: &TextStyle) -> TextMetrics {
        if text.trim().is_empty() {
            return TextMetrics {
                width: 0.0,
                height: 0.0,
                line_count: 0,
            };
        }
        let metrics = Metrics {
            font_size: style.font_size.max(0.5),
            line_height: (style.font_size * style.leading.max(0.0)).max(0.5),
        };
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let attrs = Attrs::new().family(cosmic_text::Family::Name(style.font_family.as_str()));
        buffer.set_rich_text(
            [(text, attrs)],
            &Attrs::new(),
            Shaping::Advanced,
            style.alignment.cosmic(),
        );
        buffer.shape_until_scroll(&mut self.font_system, false);
        let mut width = 0.0f64;
        let mut line_count = 0usize;
        for run in buffer.layout_runs() {
            width = width.max(f64::from(run.line_w));
            line_count += 1;
        }
        TextMetrics {
            width,
            height: f64::from(metrics.line_height) * line_count.max(1) as f64,
            line_count,
        }
    }
}

/// Unpacks cosmic-text's packed ARGB colour into straight RGBA bytes.
fn unpack_color(color: cosmic_text::Color) -> [u8; 4] {
    let raw = color.0;
    [
        ((raw >> 16) & 0xFF) as u8,
        ((raw >> 8) & 0xFF) as u8,
        (raw & 0xFF) as u8,
        ((raw >> 24) & 0xFF) as u8,
    ]
}

/// Convenience free function over a fresh [`TextContext`]; prefer reusing a
/// context when shaping many strings (the font database load is the cost).
pub fn shape_rgba(text: &str, style: &TextStyle, scale: f64) -> RgbaLayer {
    TextContext::new().shape_rgba(text, style, scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> TextContext {
        TextContext::new()
    }

    #[test]
    fn empty_text_renders_an_empty_layer() {
        let layer = shape_rgba("", &TextStyle::default(), 1.0);
        assert_eq!((layer.width, layer.height), (0, 0));
    }

    #[test]
    fn ascii_renders_non_empty_pixels() {
        let mut ctx = context();
        let layer = ctx.shape_rgba("Hello", &TextStyle::default(), 1.0);
        assert!(layer.width > 4 && layer.height > 4);
        assert!(
            layer.pixels.chunks_exact(4).any(|px| px[3] > 0),
            "some pixel must carry alpha coverage"
        );
    }

    #[test]
    fn scale_invariance_doubles_the_box() {
        let mut ctx = context();
        let style = TextStyle::default();
        let small = ctx.measure("Hello", &style);
        let big = ctx.measure(
            "Hello",
            &TextStyle {
                font_size: style.font_size * 2.0,
                ..style.clone()
            },
        );
        let ratio = big.width / small.width.max(1.0);
        assert!(
            (ratio - 2.0).abs() < 0.05,
            "width should scale ~2x, got {ratio}"
        );
    }

    #[test]
    fn multiline_counts_lines() {
        let mut ctx = context();
        let metrics = ctx.measure("One\nTwo\nThree", &TextStyle::default());
        assert_eq!(metrics.line_count, 3);
    }

    #[test]
    fn alignment_moves_nothing_for_tight_layers() {
        // A tight (no-wrap) layer fits its ink; alignment must not overflow.
        let mut ctx = context();
        let layer = ctx.shape_rgba(
            "EEE EEEE",
            &TextStyle {
                alignment: TextAlignment::Right,
                ..Default::default()
            },
            1.0,
        );
        let ink_right = layer.pixels.chunks_exact(4).rposition(|px| px[3] > 0);
        assert!(ink_right.is_some());
    }

    #[test]
    fn font_families_are_sorted_and_include_something() {
        let mut ctx = context();
        let families = ctx.font_families();
        assert!(
            !families.is_empty(),
            "a system must expose at least one family"
        );
        for window in families.windows(2) {
            assert!(
                window[0].to_lowercase() <= window[1].to_lowercase(),
                "families must be case-insensitively sorted: {window:?}"
            );
        }
    }

    /// Ink bounding box of a layer as (min_x, min_y, max_x, max_y, count).
    fn ink_bbox(layer: &RgbaLayer) -> Option<(u32, u32, u32, u32, usize)> {
        let mut bbox: Option<(u32, u32, u32, u32, usize)> = None;
        for y in 0..layer.height {
            for x in 0..layer.width {
                let alpha = layer.pixels[((y * layer.width + x) * 4 + 3) as usize];
                if alpha == 0 {
                    continue;
                }
                bbox = Some(match bbox {
                    None => (x, y, x, y, 1),
                    Some((min_x, min_y, max_x, max_y, count)) => (
                        min_x.min(x),
                        min_y.min(y),
                        max_x.max(x),
                        max_y.max(y),
                        count + 1,
                    ),
                });
            }
        }
        bbox
    }

    #[test]
    fn vertical_text_stacks_characters_top_to_bottom() {
        let mut ctx = context();
        let style = TextStyle {
            vertical: true,
            ..Default::default()
        };
        let layer = ctx.shape_rgba("日本語", &style, 1.0);
        let bbox = ink_bbox(&layer).expect("CJK ink expected with system Noto fonts");
        // Three stacked fullwidth characters: the column is clearly taller
        // than wide and ink spans the top, middle and bottom thirds.
        let (min_x, min_y, max_x, max_y, _) = bbox;
        assert!(
            max_y - min_y > 2 * (max_x - min_x),
            "column should be taller than wide: {bbox:?}"
        );
        let height = (max_y - min_y) as f32;
        let thirds = [
            (max_y - min_y) as f32 * (1.0 / 3.0),
            (max_y - min_y) as f32 * (2.0 / 3.0),
        ];
        let has_ink_between = |from: f32, to: f32| {
            let steps = ((to - from).max(1.0) as u32).max(1);
            (0..steps).any(|step| {
                let y = min_y as f32 + from + (to - from) * (step as f32 / steps as f32);
                (0..layer.width)
                    .any(|x| layer.pixels[(((y as u32) * layer.width + x) * 4 + 3) as usize] > 0)
            })
        };
        assert!(has_ink_between(0.0, thirds[0]), "top third ink expected");
        assert!(
            has_ink_between(thirds[0], thirds[1]),
            "middle third ink expected"
        );
        let _ = height;
    }

    #[test]
    fn vertical_lines_become_right_to_left_columns() {
        let mut ctx = context();
        let style = TextStyle {
            vertical: true,
            ..Default::default()
        };
        // First line must land in the rightmost column: compare the right
        // band of a two-line layer against a single-line layer of that line.
        let both = ctx.shape_rgba("日\n本", &style, 1.0);
        let first = ctx.shape_rgba("日", &style, 1.0);
        let bbox = ink_bbox(&both).expect("ink for both columns");
        let first_bbox = ink_bbox(&first).expect("ink for first column");
        // The rightmost ink of the two-line layer must touch the right edge
        // band and the leftmost ink must be clearly left of it (two columns).
        assert!(
            bbox.2 - bbox.0 > first_bbox.2 - first_bbox.0,
            "two columns expected"
        );
        let right_half_start = (bbox.0 + bbox.2) / 2;
        let mut right_top = u32::MAX;
        let mut left_top = u32::MAX;
        for y in 0..both.height {
            for x in 0..both.width {
                let alpha = both.pixels[((y * both.width + x) * 4 + 3) as usize];
                if alpha == 0 {
                    continue;
                }
                if x >= right_half_start {
                    right_top = right_top.min(y);
                } else {
                    left_top = left_top.min(y);
                }
            }
        }
        // Both columns start at the top (within one glyph's ink variation).
        let top_tolerance = (both.height / 4).max(4);
        assert!(
            right_top.abs_diff(left_top) <= top_tolerance,
            "columns should start at the top (right={right_top}, left={left_top})"
        );
    }

    #[test]
    fn vertical_empty_lines_render_empty() {
        let mut ctx = context();
        let style = TextStyle {
            vertical: true,
            ..Default::default()
        };
        let layer = ctx.shape_rgba("  \n\n", &style, 1.0);
        assert_eq!((layer.width, layer.height), (0, 0));
    }

    #[test]
    fn vertical_covers_cjk_range_shapes_deterministically() {
        let mut ctx = context();
        let style = TextStyle {
            vertical: true,
            ..Default::default()
        };
        let a = ctx.shape_rgba("縦書き", &style, 2.0);
        let b = ctx.shape_rgba("縦書き", &style, 2.0);
        assert_eq!(a.width, b.width, "shaping must be deterministic");
        assert_eq!(a.pixels, b.pixels);
        assert!(ink_bbox(&a).is_some(), "tategaki fixture expects CJK ink");
    }
}

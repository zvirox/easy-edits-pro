// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! WGSL compute program + dispatch pipeline for the Packet 3.2 colour-node
//! chain. The program mirrors [`crate::color_nodes_eval`] (the documented
//! semantic reference) op-for-op on f32; `tests/color_pipeline.rs` asserts
//! GPU-vs-CPU agreement per node kind.
//!
//! Hardening (plan §10.2.3): the source is pre-validated with naga before any
//! wgpu pipeline object is created, and both pipeline creation and dispatch
//! run inside `wgpu` validation error scopes so every failure surfaces as a
//! typed [`ColorNodesError`] — never a panic or a silent fallback.

use std::borrow::Cow;

use crate::color_nodes_eval::eval_reference;
use crate::color_nodes_gpu::{
    pack_blob, ColorNodesError, EvalNode, FilmPlan, MAX_NODES,
};
use crate::film_emulation::gate_weave_offset;
use crate::gpu_context::{u32_words_as_bytes, GpuContext};

/// The chain program. One invocation per output pixel: gate-weaved source
/// fetch → node loop (windows gate each node) → halation spill add.
const CHAIN_WGSL: &str = r#"
// Blob accessors: data section words are f32 bit-packed (see pack_blob docs).
fn f(w: u32) -> f32 { return bitcast<f32>(w); }

// CPU-mirrored helpers -------------------------------------------------------
fn bandsmooth(e0: f32, e1: f32, x: f32) -> f32 {
  if (e0 == e1) { return select(1.0, 0.0, x < e0); }
  let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
  return t * t * (3.0 - 2.0 * t);
}

fn band_weight(dist: f32, half_w: f32, soft: f32) -> f32 {
  if (soft <= 0.0) { return select(0.0, 1.0, dist <= half_w); }
  let plateau = max(half_w * (1.0 - soft), 0.0);
  return 1.0 - bandsmooth(plateau, half_w, dist);
}

fn luma709(c: vec3<f32>) -> f32 {
  return 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
}

// Returns (hue, sat, lightness); hue in turns 0..1.
fn rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
  let mx = max(c.r, max(c.g, c.b));
  let mn = min(c.r, min(c.g, c.b));
  let l = (mx + mn) * 0.5;
  let d = mx - mn;
  if (d <= 0.0) { return vec3<f32>(0.0, 0.0, l); }
  let s = d / (1.0 - abs(2.0 * l - 1.0));
  var hue: f32;
  if (mx == c.r) { hue = ((c.g - c.b) / d) % 6.0; }
  else if (mx == c.g) { hue = (c.b - c.r) / d + 2.0; }
  else { hue = (c.r - c.g) / d + 4.0; }
  hue = hue / 6.0;
  hue = hue - floor(hue);
  return vec3<f32>(hue, clamp(s, 0.0, 1.0), l);
}

fn hue_to_rgb(p: f32, q: f32, t_in: f32) -> f32 {
  var t = t_in;
  if (t < 0.0) { t = t + 1.0; }
  if (t > 1.0) { t = t - 1.0; }
  if (t < 1.0 / 6.0) { return p + (q - p) * 6.0 * t; }
  if (t < 0.5) { return q; }
  if (t < 2.0 / 3.0) { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
  return p;
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> vec3<f32> {
  if (s <= 0.0) { return vec3<f32>(l, l, l); }
  let q = select(l + s - l * s, l * (1.0 + s), l < 0.5);
  let p = 2.0 * l - q;
  return vec3<f32>(
    hue_to_rgb(p, q, h + 1.0 / 3.0),
    hue_to_rgb(p, q, h),
    hue_to_rgb(p, q, h - 1.0 / 3.0)
  );
}

// Shortest signed hue distance in turns, -0.5..0.5.
fn hue_dist(a: f32, b: f32) -> f32 {
  var d = (a - b) % 1.0;
  if (d < 0.0) { d = d + 1.0; }
  if (d > 0.5) { d = d - 1.0; }
  return d;
}

// Integer hash identical to the Rust reference by construction (u32 wrap).
fn grain_noise(px: u32, py: u32, frame: u32) -> f32 {
  var h = px * 0x9E3779B1u ^ py * 0x85EBCA6Bu ^ frame * 0xC2B2AE35u;
  h = h ^ (h >> 16u);
  h = h * 0x7FE00001u;
  h = h ^ (h >> 15u);
  h = h * 0x9E3779B1u;
  h = h ^ (h >> 16u);
  let unit = f32(h >> 8u) / 16777216.0;
  return unit * 2.0 - 1.0;
}

// Node ops (word orders match the packer docs) -------------------------------
fn apply_wheels(doff: u32, c: vec3<f32>, gate: f32) -> vec3<f32> {
  let exposure = f(blob[doff]) * gate;
  var v = c * exp2(exposure);
  for (var ch = 0u; ch < 3u; ch++) {
    let g = max(1.0 + (f(blob[doff + 2u + ch]) + f(blob[doff + 5u])) * gate, 0.0);
    v[ch] = v[ch] * g;
  }
  let p = exp2(-f(blob[doff + 1u]) * gate);
  for (var ch = 0u; ch < 3u; ch++) {
    v[ch] = pow(max(v[ch], 0.0), p);
  }
  for (var ch = 0u; ch < 3u; ch++) {
    v[ch] = v[ch] + (f(blob[doff + 6u + ch]) + f(blob[doff + 9u])) * gate;
  }
  let shade = max(1.0 - luma709(v), 0.0) * gate;
  let lift = vec3<f32>(f(blob[doff + 10u]), f(blob[doff + 11u]), f(blob[doff + 12u]));
  return v + lift * shade;
}

fn apply_zones(doff: u32, c: vec3<f32>, gate: f32) -> vec3<f32> {
  let l = log2(max(luma709(c), 1e-6));
  var acc = vec3<f32>(0.0, 0.0, 0.0);
  for (var z = 0u; z < 6u; z++) {
    let base = doff + z * 6u;
    let w = band_weight(abs(l - f(blob[base])), f(blob[base + 1u]) * 0.5, 0.0) * gate;
    acc = acc + vec3<f32>(
      (f(blob[base + 2u]) + f(blob[base + 5u])) * w,
      (f(blob[base + 3u]) + f(blob[base + 5u])) * w,
      (f(blob[base + 4u]) + f(blob[base + 5u])) * w
    );
  }
  return c + acc;
}

const CURVE_STRIDE: u32 = 1u + 8u * 2u;
const MAX_CURVE_POINTS: u32 = 8u;

fn curve_count(doff: u32, curve: u32) -> u32 {
  return min(u32(f(blob[doff + curve * CURVE_STRIDE])), MAX_CURVE_POINTS);
}

fn curve_point(doff: u32, curve: u32, k: u32) -> vec2<f32> {
  let o = doff + curve * CURVE_STRIDE + 1u + k * 2u;
  return vec2<f32>(f(blob[o]), f(blob[o + 1u]));
}

// Monotone cubic (Fritsch-Carlson) — direct mirror of monotone_spline_f64.
fn monotone_spline(doff: u32, curve: u32, x: f32) -> f32 {
  let n = curve_count(doff, curve);
  let p0 = curve_point(doff, curve, 0u);
  let plast = curve_point(doff, curve, n - 1u);
  if (x <= p0.x) { return p0.y; }
  if (x >= plast.x) { return plast.y; }
  var seg = 0u;
  for (var k = 0u; k < n - 1u; k++) {
    let a = curve_point(doff, curve, k);
    let b = curve_point(doff, curve, k + 1u);
    if (x >= a.x && x < b.x) { seg = k; break; }
  }
  let a = curve_point(doff, curve, seg);
  let b = curve_point(doff, curve, seg + 1u);
  let dx = max(b.x - a.x, 1e-9);
  var slope0: f32;
  if (seg == 0u) {
    slope0 = (curve_point(doff, curve, 1u).y - p0.y) / dx;
  } else {
    let pm = curve_point(doff, curve, seg - 1u);
    slope0 = (b.y - pm.y) / (b.x - pm.x);
  }
  var slope1: f32;
  if (seg + 2u == n) {
    slope1 = (b.y - a.y) / dx;
  } else {
    let pn = curve_point(doff, curve, seg + 2u);
    slope1 = (pn.y - a.y) / (pn.x - a.x);
  }
  let sec = (b.y - a.y) / dx;
  if (sec <= 0.0) {
    slope0 = 0.0;
    slope1 = 0.0;
  } else {
    let limit = 3.0 * sec;
    slope0 = clamp(slope0, -limit, limit);
    slope1 = clamp(slope1, -limit, limit);
  }
  let t = (x - a.x) / dx;
  let t2 = t * t;
  let t3 = t2 * t;
  let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
  let h10 = t3 - 2.0 * t2 + t;
  let h01 = -2.0 * t3 + 3.0 * t2;
  let h11 = t3 - t2;
  return h00 * a.y + h10 * dx * slope0 + h01 * b.y + h11 * dx * slope1;
}

fn apply_curves(doff: u32, c: vec3<f32>, gate: f32) -> vec3<f32> {
  var curved = c;
  for (var ch = 0u; ch < 3u; ch++) {
    if (curve_count(doff, 0u) >= 2u) {
      curved[ch] = monotone_spline(doff, 0u, clamp(c[ch], 0.0, 1.0));
    }
  }
  for (var ch = 0u; ch < 3u; ch++) {
    if (curve_count(doff, 1u + ch) >= 2u) {
      curved[ch] = monotone_spline(doff, 1u + ch, clamp(curved[ch], 0.0, 1.0));
    }
  }
  return c + (curved - c) * gate;
}

fn lut_corner(doff: u32, size: u32, p: vec3<u32>) -> vec3<f32> {
  let maxc = size - 1u;
  let q = min(p, vec3<u32>(maxc, maxc, maxc));
  let k = doff + 1u + (q.z * size * size + q.y * size + q.x) * 3u;
  return vec3<f32>(f(blob[k]), f(blob[k + 1u]), f(blob[k + 2u]));
}

fn apply_lut(doff: u32, c: vec3<f32>, gate: f32) -> vec3<f32> {
  if (gate <= 0.0) { return c; }
  let size = u32(f(blob[doff]));
  let scale = f32(size - 1u);
  let cs = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)) * vec3<f32>(scale, scale, scale);
  let i = vec3<u32>(floor(cs));
  let fr = cs - vec3<f32>(i);
  // Sort the three fractional parts descending; the tetrahedron steps one
  // axis at a time in that order (continuous across ties).
  var ax1 = 0u;
  var ax2 = 1u;
  var ax3 = 2u;
  if (fr.x >= fr.y && fr.y >= fr.z) { ax1 = 0u; ax2 = 1u; ax3 = 2u; }
  else if (fr.x >= fr.z && fr.z >= fr.y) { ax1 = 0u; ax2 = 2u; ax3 = 1u; }
  else if (fr.y >= fr.x && fr.x >= fr.z) { ax1 = 1u; ax2 = 0u; ax3 = 2u; }
  else if (fr.y >= fr.z && fr.z >= fr.x) { ax1 = 1u; ax2 = 2u; ax3 = 0u; }
  else if (fr.z >= fr.x && fr.x >= fr.y) { ax1 = 2u; ax2 = 0u; ax3 = 1u; }
  else { ax1 = 2u; ax2 = 1u; ax3 = 0u; }
  let wmax = max(fr.x, max(fr.y, fr.z));
  let wmin = min(fr.x, min(fr.y, fr.z));
  let wmid = fr.x + fr.y + fr.z - wmax - wmin;
  var result = lut_corner(doff, size, i);
  var prev = result;
  var steps = vec3<u32>(0u, 0u, 0u);
  for (var k = 0u; k < 3u; k++) {
    var axis = ax1;
    var weight = wmax;
    if (k == 1u) { axis = ax2; weight = wmid; }
    else if (k == 2u) { axis = ax3; weight = wmin; }
    // Cumulative steps: C1 steps one axis, C2 the first two, C3 all three.
    steps[axis] = steps[axis] + 1u;
    let next = lut_corner(doff, size, i + steps);
    result = result + weight * (next - prev);
    prev = next;
  }
  return c + (result - c) * gate;
}

fn apply_hsl(doff: u32, c: vec3<f32>, gate: f32) -> vec3<f32> {
  if (f(blob[doff]) <= 0.0) { return c; }
  let hsl = rgb_to_hsl(c);
  let hue = hsl.x;
  let sat = hsl.y;
  let l = hsl.z;
  let hue_w = band_weight(abs(hue_dist(hue, f(blob[doff + 1u]))), f(blob[doff + 2u]) * 0.5, f(blob[doff + 3u]));
  let sat_w = band_weight(abs(sat - f(blob[doff + 4u])), f(blob[doff + 5u]) * 0.5, f(blob[doff + 6u]));
  let luma_w = band_weight(abs(clamp(luma709(c), 0.0, 1.0) - f(blob[doff + 7u])), f(blob[doff + 8u]) * 0.5, f(blob[doff + 9u]));
  var w = hue_w * sat_w * luma_w * gate;
  if (f(blob[doff + 13u]) > 0.5) { w = 1.0 - w; }
  if (w <= 0.0) { return c; }
  var hue2 = hue + f(blob[doff + 10u]) * w;
  hue2 = hue2 - floor(hue2);
  let sat2 = clamp(sat * (1.0 + (f(blob[doff + 11u]) - 1.0) * w), 0.0, 1.0);
  var rgb2 = hsl_to_rgb(hue2, sat2, l);
  rgb2 = rgb2 * exp2(f(blob[doff + 12u]) * w);
  return rgb2;
}

fn apply_warper(doff: u32, c: vec3<f32>, gate: f32) -> vec3<f32> {
  if (gate <= 0.0) { return c; }
  let count = u32(f(blob[doff]));
  if (count == 0u) { return c; }
  let hsl = rgb_to_hsl(c);
  var wsum = 0.0;
  var dh = 0.0;
  var ds = 0.0;
  for (var p = 0u; p < count; p++) {
    let base = doff + 1u + p * 4u;
    let ph = f(blob[base]);
    let ps = f(blob[base + 1u]);
    let d = hue_dist(hsl.x, ph);
    let dist_sq = d * d + (hsl.y - ps) * (hsl.y - ps);
    let w = 1.0 / max(dist_sq, 1e-4);
    wsum = wsum + w;
    dh = dh + w * f(blob[base + 2u]);
    ds = ds + w * f(blob[base + 3u]);
  }
  var hue2 = hsl.x + dh / wsum;
  hue2 = hue2 - floor(hue2);
  let sat2 = clamp(hsl.y + ds / wsum, 0.0, 1.0);
  let rgb2 = hsl_to_rgb(hue2, sat2, hsl.z);
  return c + (rgb2 - c) * gate;
}

fn apply_grain(doff: u32, c: vec3<f32>, gate: f32, px: vec2<u32>, frame: u32) -> vec3<f32> {
  let strength = f(blob[doff]) * gate;
  if (strength <= 0.0) { return c; }
  let n = grain_noise(px.x, px.y, frame);
  let amp = strength * 0.10 * sqrt(max(luma709(c), 0.01));
  return c + vec3<f32>(n, n, n) * amp;
}

// Power windows ---------------------------------------------------------------
// Normalised polygon distance: inside → 1 - frac, outside → 1 + frac, with
// frac the nearest-edge distance normalised by the mean edge length.
fn polygon_dist(rec: u32, uv: vec2<f32>) -> f32 {
  let n = blob[rec + 13u];
  let pts_off = blob[rec + 12u];
  var inside = false;
  var nearest = 1e30;
  var scale = 0.0;
  for (var k = 0u; k < n; k++) {
    let ko = pts_off + k * 2u;
    let kn = pts_off + ((k + 1u) % n) * 2u;
    let a = vec2<f32>(f(blob[ko]), f(blob[ko + 1u]));
    let b = vec2<f32>(f(blob[kn]), f(blob[kn + 1u]));
    if ((a.y > uv.y) != (b.y > uv.y)) {
      let xint = a.x + (uv.y - a.y) / (b.y - a.y) * (b.x - a.x);
      if (uv.x < xint) { inside = !inside; }
    }
    let ab = b - a;
    let ap = uv - a;
    let ab2 = dot(ab, ab);
    var t = 0.0;
    if (ab2 > 0.0) { t = clamp(dot(ap, ab) / ab2, 0.0, 1.0); }
    nearest = min(nearest, length(uv - (a + ab * t)));
    scale = scale + length(b - a);
  }
  scale = scale / f32(n);
  let frac = min(nearest / max(scale, 1e-6), 1.0);
  return select(1.0 + frac, 1.0 - frac, inside);
}

fn window_weight(rec: u32, uv: vec2<f32>) -> f32 {
  let shape = blob[rec];
  let inverted = blob[rec + 1u] != 0u;
  let opacity = f(blob[rec + 2u]);
  let softness = f(blob[rec + 3u]);
  let center = vec2<f32>(f(blob[rec + 4u]), f(blob[rec + 5u]));
  let half_size = vec2<f32>(f(blob[rec + 6u]), f(blob[rec + 7u]));
  let pivot = vec2<f32>(f(blob[rec + 8u]), f(blob[rec + 9u]));
  let scale = f(blob[rec + 10u]);
  let rot = f(blob[rec + 11u]) * 0.017453292519943295;
  // Static transform around the pivot (clockwise on the y-down screen):
  // p' = pivot + R(theta) * (uv - pivot) / scale.
  let rel = (uv - pivot) / vec2<f32>(scale, scale);
  let s = sin(rot);
  let cth = cos(rot);
  let mapped = pivot + vec2<f32>(rel.x * cth - rel.y * s, rel.x * s + rel.y * cth);
  var dist: f32;
  if (shape == 0u) {
    let d = abs((mapped - center) / half_size);
    dist = max(d.x, d.y);
  } else if (shape == 1u) {
    dist = length((mapped - center) / half_size);
  } else {
    dist = polygon_dist(rec, mapped);
  }
  if (inverted) { dist = -dist; }
  var weight: f32;
  if (softness <= 0.0) {
    weight = select(0.0, 1.0, dist <= 1.0);
  } else {
    weight = 1.0 - bandsmooth(1.0 - softness, 1.0, dist);
  }
  return weight * opacity;
}

// Entry ------------------------------------------------------------------------
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var hal_tex: texture_2d<f32>;
@group(0) @binding(2) var out_tex: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var<storage, read> blob: array<u32>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let width = blob[4u];
  let height = blob[5u];
  if (gid.x >= width || gid.y >= height) {
    return;
  }
  let frame = blob[1u];
  // Gate weave: the print plane moves relative to the gate, so the source is
  // fetched at the weaved (clamped) integer pixel offset.
  let weave = vec2<f32>(f(blob[2u]), f(blob[3u]));
  let off_px = vec2<i32>(round(weave * vec2<f32>(f32(width), f32(height))));
  let sample_px = clamp(
    vec2<i32>(gid.xy) + off_px,
    vec2<i32>(0, 0),
    vec2<i32>(i32(width) - 1, i32(height) - 1)
  );
  var c = textureLoad(src_tex, vec2<u32>(sample_px), 0).rgb;
  let uv = (vec2<f32>(gid.xy) + vec2<f32>(0.5, 0.5)) / vec2<f32>(f32(width), f32(height));
  let node_count = blob[0u];
  for (var n = 0u; n < node_count; n++) {
    let rec = 8u + n * 8u;
    if (blob[rec + 1u] == 0u) {
      continue; // disabled node
    }
    let kind = blob[rec];
    let doff = blob[rec + 2u];
    let woff = blob[rec + 4u];
    let wcount = blob[rec + 5u];
    var gate = 1.0;
    for (var wi = 0u; wi < wcount; wi++) {
      gate = gate * window_weight(woff + wi * 16u, uv);
      if (gate <= 0.0) {
        break;
      }
    }
    if (gate > 0.0) {
      if (kind == 0u) { c = apply_wheels(doff, c, gate); }
      else if (kind == 1u) { c = apply_zones(doff, c, gate); }
      else if (kind == 2u) { c = apply_curves(doff, c, gate); }
      else if (kind == 3u) { c = apply_lut(doff, c, gate); }
      else if (kind == 4u) { c = apply_hsl(doff, c, gate); }
      else if (kind == 5u) { c = apply_warper(doff, c, gate); }
      else if (kind == 6u) { c = apply_grain(doff, c, gate, gid.xy, frame); }
    }
  }
  // Halation spill arrives pre-blurred; the header carries the strength.
  let hal = textureLoad(hal_tex, gid.xy, 0).rgb;
  c = c + f(blob[6u]) * hal;
  textureStore(out_tex, gid.xy, vec4<f32>(c, 1.0));
}
"#;

/// Pre-validates the WGSL with naga (parse + full validation) so a shader
/// defect is a typed error at pipeline construction, not a device error.
fn validate_wgsl(source: &str) -> Result<(), ColorNodesError> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| ColorNodesError::WgslInvalid(e.emit_to_string(source)))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .map_err(|e| ColorNodesError::WgslInvalid(e.to_string()))?;
    Ok(())
}

pub struct ColorNodesPipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl ColorNodesPipeline {
    pub fn new(ctx: &GpuContext) -> Result<Self, ColorNodesError> {
        let source = CHAIN_WGSL;
        validate_wgsl(source)?;
        let guard = ctx.device().push_error_scope(wgpu::ErrorFilter::Validation);
        let module = ctx.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("saturn-render color nodes chain"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
        });
        let bind_group_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("saturn-render color nodes layout"),
                    entries: &[
                        // 0: source texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // 1: halation spill texture (blurred bright pass)
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // 2: output storage texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba32Float,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        // 3: node blob (all u32 words)
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: Some(std::num::NonZeroU64::new(4).unwrap()),
                            },
                            count: None,
                        },
                    ],
                });
        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("saturn-render color nodes pipeline layout"),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });
        let pipeline =
            ctx.device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("saturn-render color nodes pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &module,
                    entry_point: Some("main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        ctx.device()
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| ColorNodesError::Pipeline(e.to_string()))?;
        if let Some(error) = pollster::block_on(guard.pop()) {
            return Err(ColorNodesError::Pipeline(error.to_string()));
        }
        Ok(Self {
            pipeline,
            bind_group_layout,
        })
    }

    /// Runs the chain over RGBA f32 pixels and returns the graded result.
    /// `halation` is the pre-pass spill texture (same size as the frame);
    /// when absent a 1x1 black texture keeps one pipeline shape.
    pub fn transform_rgba(
        &self,
        ctx: &GpuContext,
        width: u32,
        height: u32,
        pixels: &[f32],
        nodes: &[EvalNode],
        film: &FilmPlan,
        frame: u32,
        halation: Option<&wgpu::Texture>,
    ) -> Result<Vec<f32>, ColorNodesError> {
        if nodes.len() > MAX_NODES {
            return Err(ColorNodesError::TooManyNodes(nodes.len()));
        }
        let weave_uv = gate_weave_offset(film.weave_mm, frame);
        let blob = pack_blob(nodes, film, frame, weave_uv, width, height)?;
        let input = ctx.create_rgba32float_texture(
            width,
            height,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        ctx.upload_rgba32float(&input, width, height, pixels);
        let output = ctx.create_rgba32float_texture(
            width,
            height,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        );
        let black = halation.is_none().then(|| {
            let texture = ctx.create_rgba32float_texture(1, 1, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
            ctx.upload_rgba32float(&texture, 1, 1, &[0.0, 0.0, 0.0, 1.0]);
            texture
        });
        let halation_ref = match halation {
            Some(texture) => texture,
            None => black.as_ref().expect("fallback texture just created"),
        };
        {
            let guard = ctx.device().push_error_scope(wgpu::ErrorFilter::Validation);
            let blob_buffer = ctx.device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("saturn-render color nodes blob"),
                size: (blob.len() * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            ctx.queue()
                .write_buffer(&blob_buffer, 0, u32_words_as_bytes(&blob));
            let input_view = input.create_view(&wgpu::TextureViewDescriptor::default());
            let hal_view = halation_ref.create_view(&wgpu::TextureViewDescriptor::default());
            let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
            let bind_group = ctx.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("saturn-render color nodes bind group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&input_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&hal_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&output_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: blob_buffer.as_entire_binding(),
                    },
                ],
            });
            let mut encoder = ctx
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("saturn-render color nodes dispatch"),
                });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("saturn-render color nodes pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
            }
            ctx.queue().submit(Some(encoder.finish()));
            ctx.device()
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|e| ColorNodesError::Dispatch(e.to_string()))?;
            if let Some(error) = pollster::block_on(guard.pop()) {
                return Err(ColorNodesError::Dispatch(error.to_string()));
            }
        }
        ctx.readback_rgba32float(&output, width, height)
            .map_err(Into::into)
    }

    /// Convenience wrapper: builds the eval plan from the stored schema chain
    /// and runs the GPU path.
    #[allow(clippy::too_many_arguments)]
    pub fn transform_clip_chain(
        &self,
        ctx: &GpuContext,
        width: u32,
        height: u32,
        pixels: &[f32],
        chain: &[saturn_core::project::ColorNode],
        frame: u32,
        halation: Option<&wgpu::Texture>,
    ) -> Result<Vec<f32>, ColorNodesError> {
        let (nodes, film) = crate::color_nodes_gpu::build_plan(chain)?;
        self.transform_rgba(ctx, width, height, pixels, &nodes, &film, frame, halation)
    }
}

/// CPU reference over the whole dispatch (source sampling included), used by
/// the parity tests: mirrors the WGSL entry exactly — weaved + clamped source
/// fetch, then `eval_reference` on that sample.
pub fn reference_dispatch(
    width: u32,
    height: u32,
    pixels: &[f32],
    nodes: &[EvalNode],
    film: &FilmPlan,
    frame: u32,
) -> Vec<f32> {
    let weave_uv = gate_weave_offset(film.weave_mm, frame);
    let off_x = (weave_uv[0] as f32 * width as f32).round() as i32;
    let off_y = (weave_uv[1] as f32 * height as f32).round() as i32;
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let sx = (x as i32 + off_x).clamp(0, width as i32 - 1) as usize;
            let sy = (y as i32 + off_y).clamp(0, height as i32 - 1) as usize;
            let o = (sy * width as usize + sx) * 4;
            let source = [
                pixels[o] as f64,
                pixels[o + 1] as f64,
                pixels[o + 2] as f64,
            ];
            let graded = eval_reference(
                source,
                [
                    (x as f64 + 0.5) / f64::from(width),
                    (y as f64 + 0.5) / f64::from(height),
                ],
                [x as i64, y as i64],
                nodes,
                film,
                frame,
            );
            out.extend_from_slice(&[graded[0] as f32, graded[1] as f32, graded[2] as f32, 1.0]);
        }
    }
    out
}

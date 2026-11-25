//! Real-time 3D renderer (OpenGL through egui_glow): textured city chunks built from the
//! OpenStreetMap data, sun/moon lighting, fog, lit windows at night, 3D characters and vehicles.

use crate::game::{Driver, Game, VKind};
use crate::map::{Map, Rng};
use crate::net::Remote;
use eframe::egui::{self as egui};
use eframe::egui_glow;
use eframe::glow::{self, HasContext};
use egui::{Color32, Pos2};
use glam::{Mat4, Vec3};
use std::collections::{HashMap, HashSet};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};
use std::sync::{Arc, Mutex};

pub const FLOATS: usize = 13;
const CHUNK: f32 = 400.0;
const TEX: usize = 256;
const LAYERS: usize = 10;
// texture layers
pub const L_GROUND: f32 = 0.0;
pub const L_ASPHALT: f32 = 1.0;
pub const L_LATERITE: f32 = 2.0;
pub const L_WALL: f32 = 3.0;
pub const L_TIN: f32 = 4.0;
pub const L_CONCRETE: f32 = 5.0;
pub const L_WATER: f32 = 6.0;
pub const L_GRASS: f32 = 7.0;
pub const L_PLAIN: f32 = 8.0;
pub const L_LEAVES: f32 = 9.0;
pub const L_EMISSIVE: f32 = 10.0;
pub const L_SHADOW: f32 = 11.0;

pub fn rgba(c: Color32, a: f32) -> [f32; 4] {
    [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, a]
}
fn rgb(r: u8, g: u8, b: u8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}
fn v3(p: Pos2, y: f32) -> Vec3 {
    Vec3::new(p.x, y, p.y)
}

#[derive(Default)]
pub struct MeshBuilder {
    pub v: Vec<f32>,
}
impl MeshBuilder {
    pub fn vert(&mut self, p: Vec3, n: Vec3, c: [f32; 4], uv: [f32; 2], layer: f32) {
        self.v.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, c[0], c[1], c[2], c[3], uv[0], uv[1], layer]);
    }
    pub fn quad(&mut self, p: [Vec3; 4], n: Vec3, c: [f32; 4], uv: [[f32; 2]; 4], layer: f32) {
        for i in [0, 1, 2, 0, 2, 3] {
            self.vert(p[i], n, c, uv[i], layer);
        }
    }
    pub fn tri(&mut self, p: [Vec3; 3], c: [f32; 4], uvs: [[f32; 2]; 3], layer: f32) {
        for i in 0..3 {
            self.vert(p[i], Vec3::Y, c, uvs[i], layer);
        }
    }
    /// Box of half size `h` in the local frame of `m`.
    pub fn cuboid(&mut self, m: &Mat4, h: Vec3, c: [f32; 4], layer: f32) {
        let faces: [(Vec3, Vec3, Vec3); 6] = [
            (Vec3::X, Vec3::Z, Vec3::Y),
            (-Vec3::X, -Vec3::Z, Vec3::Y),
            (Vec3::Y, Vec3::X, Vec3::Z),
            (-Vec3::Y, Vec3::X, -Vec3::Z),
            (Vec3::Z, -Vec3::X, Vec3::Y),
            (-Vec3::Z, Vec3::X, Vec3::Y),
        ];
        for (n, u, w) in faces {
            let c0 = n * h;
            let du = u * h;
            let dw = w * h;
            let pts = [c0 - du - dw, c0 + du - dw, c0 + du + dw, c0 - du + dw].map(|p| m.transform_point3(p));
            let nn = m.transform_vector3(n).normalize_or_zero();
            self.quad(pts, nn, c, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], layer);
        }
    }
    /// Flat rectangle on the ground (for shadows and markings).
    pub fn ground_rect(&mut self, c: Pos2, y: f32, ang: f32, hl: f32, hw: f32, col: [f32; 4], layer: f32) {
        let f = egui::vec2(ang.cos(), ang.sin());
        let r = egui::vec2(-f.y, f.x);
        let p = [c - f * hl - r * hw, c + f * hl - r * hw, c + f * hl + r * hw, c - f * hl + r * hw].map(|q| v3(q, y));
        self.quad(p, Vec3::Y, col, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], layer);
    }
    pub fn prism(&mut self, c: Pos2, r: f32, y0: f32, y1: f32, col: [f32; 4], layer: f32) {
        let n = 16;
        for i in 0..n {
            let a0 = i as f32 / n as f32 * TAU;
            let a1 = (i + 1) as f32 / n as f32 * TAU;
            let p0 = Vec3::new(c.x + a0.cos() * r, 0.0, c.y + a0.sin() * r);
            let p1 = Vec3::new(c.x + a1.cos() * r, 0.0, c.y + a1.sin() * r);
            let nrm = Vec3::new(((a0 + a1) * 0.5).cos(), 0.0, ((a0 + a1) * 0.5).sin());
            self.quad(
                [p0 + Vec3::Y * y0, p1 + Vec3::Y * y0, p1 + Vec3::Y * y1, p0 + Vec3::Y * y1],
                nrm,
                col,
                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                layer,
            );
        }
    }
}

// ---------------------------------------------------------------- textures

fn hash2(x: i32, y: i32, s: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263) ^ s.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}
fn vnoise(x: f32, y: f32, period: i32, s: u32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let sm = |t: f32| t * t * (3.0 - 2.0 * t);
    let w = |a: i32| a.rem_euclid(period);
    let a = hash2(w(xi), w(yi), s);
    let b = hash2(w(xi + 1), w(yi), s);
    let c = hash2(w(xi), w(yi + 1), s);
    let d = hash2(w(xi + 1), w(yi + 1), s);
    let (u, v) = (sm(fx), sm(fy));
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}
/// Tileable fractal noise in 0..1.
fn fbm(u: f32, v: f32, base: i32, s: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut f = base;
    for o in 0..5 {
        sum += vnoise(u * f as f32, v * f as f32, f, s + o) * amp;
        amp *= 0.5;
        f *= 2;
    }
    sum / 0.97
}

fn gen_textures() -> Vec<u8> {
    let mut out = vec![0u8; TEX * TEX * 4 * LAYERS];
    for layer in 0..LAYERS {
        for y in 0..TEX {
            for x in 0..TEX {
                let u = x as f32 / TEX as f32;
                let v = y as f32 / TEX as f32;
                let n = fbm(u, v, 4, layer as u32 * 17);
                let fine = hash2(x as i32, y as i32, layer as u32 + 99);
                let (mut r, mut g, mut b, mut a) = (1.0f32, 1.0f32, 1.0f32, 1.0f32);
                match layer {
                    0 => {
                        // sandy laterite ground with pebbles and dry tufts
                        let k = 0.78 + n * 0.35 + (fine - 0.5) * 0.08;
                        r = 0.80 * k;
                        g = 0.63 * k;
                        b = 0.45 * k;
                        if fine > 0.985 {
                            r *= 0.7;
                            g *= 0.7;
                            b *= 0.7;
                        }
                        let tuft = fbm(u, v, 16, 5);
                        if tuft > 0.68 {
                            r *= 0.85;
                            g *= 0.9;
                            b *= 0.7;
                        }
                    }
                    1 => {
                        // asphalt
                        let k = 0.85 + n * 0.25 + (fine - 0.5) * 0.18;
                        r = 0.30 * k;
                        g = 0.30 * k;
                        b = 0.32 * k;
                        let crack = fbm(u, v, 8, 77);
                        if (crack - 0.5).abs() < 0.008 {
                            r *= 0.6;
                            g *= 0.6;
                            b *= 0.6;
                        }
                    }
                    2 => {
                        // red laterite street with tyre ruts
                        let k = 0.8 + n * 0.35 + (fine - 0.5) * 0.12;
                        let rut = ((u * TAU * 2.0).sin() * 0.5 + 0.5).powf(6.0) * 0.12;
                        r = 0.70 * k * (1.0 - rut);
                        g = 0.38 * k * (1.0 - rut);
                        b = 0.22 * k * (1.0 - rut);
                    }
                    3 => {
                        // rendered wall, one window per tile (u: 4 m, v: one floor)
                        let k = 0.88 + n * 0.18 + (fine - 0.5) * 0.06;
                        r = k;
                        g = k;
                        b = k;
                        let vv = 1.0 - v; // texture v grows downward
                        let in_win = (0.3..0.7).contains(&u) && (0.35..0.78).contains(&vv);
                        let frame = (0.27..0.73).contains(&u) && (0.32..0.81).contains(&vv);
                        if in_win {
                            let refl = 0.25 + (u + vv) * 0.15;
                            r = 0.18 + refl * 0.3;
                            g = 0.22 + refl * 0.35;
                            b = 0.28 + refl * 0.4;
                            a = 0.0;
                            if ((u - 0.5).abs() < 0.008) || ((vv - 0.56).abs() < 0.008) {
                                r = 0.35;
                                g = 0.33;
                                b = 0.3;
                            }
                        } else if frame {
                            r = 0.55;
                            g = 0.5;
                            b = 0.45;
                        }
                        if vv < 0.06 {
                            r *= 0.7;
                            g *= 0.62;
                            b *= 0.55; // red dust at the foot of the walls
                        }
                        if vv > 0.97 {
                            r *= 0.8;
                            g *= 0.8;
                            b *= 0.8;
                        }
                    }
                    4 => {
                        // corrugated tin with rust
                        let wave = (u * TAU * 16.0).sin() * 0.5 + 0.5;
                        let k = 0.7 + wave * 0.3;
                        let rust = (fbm(u, v, 4, 31) - 0.55).max(0.0) * 3.0;
                        r = k * (0.75 + rust * 0.3);
                        g = k * (0.76 - rust * 0.15);
                        b = k * (0.78 - rust * 0.35);
                    }
                    5 => {
                        let k = 0.78 + n * 0.3 + (fine - 0.5) * 0.05;
                        r = k;
                        g = k * 0.98;
                        b = k * 0.95;
                    }
                    6 => {
                        let w = fbm(u, v, 8, 11);
                        r = 0.22 + w * 0.15;
                        g = 0.38 + w * 0.15;
                        b = 0.42 + w * 0.12;
                    }
                    7 => {
                        let k = 0.75 + n * 0.4 + (fine - 0.5) * 0.15;
                        r = 0.50 * k;
                        g = 0.55 * k;
                        b = 0.25 * k;
                    }
                    8 => {
                        let k = 0.96 + (fine - 0.5) * 0.04;
                        r = k;
                        g = k;
                        b = k;
                    }
                    _ => {
                        let leaf = fbm(u, v, 16, 3);
                        let k = 0.55 + leaf * 0.7;
                        r = 0.26 * k;
                        g = 0.42 * k;
                        b = 0.18 * k;
                    }
                }
                let i = ((layer * TEX + y) * TEX + x) * 4;
                out[i] = (r.clamp(0.0, 1.0) * 255.0) as u8;
                out[i + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
                out[i + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
                out[i + 3] = (a * 255.0) as u8;
            }
        }
    }
    out
}

/// Save the generated textures as PNG files (used for the build's assets folder).
pub fn export_textures(dir: &std::path::Path) {
    let data = gen_textures();
    let _ = std::fs::create_dir_all(dir);
    let names = ["sol", "asphalte", "laterite", "mur", "tole", "beton", "eau", "herbe", "neutre", "feuillage"];
    for (l, name) in names.iter().enumerate() {
        let s = l * TEX * TEX * 4;
        let _ = image::save_buffer(dir.join(format!("{name}.png")), &data[s..s + TEX * TEX * 4], TEX as u32, TEX as u32, image::ColorType::Rgba8);
    }
}

// ---------------------------------------------------------------- city chunks

fn wall_tint(seed: u32) -> [f32; 4] {
    const T: [(u8, u8, u8); 7] = [
        (236, 222, 196),
        (222, 178, 120),
        (242, 240, 232),
        (228, 186, 172),
        (240, 222, 158),
        (186, 124, 84),
        (206, 214, 200),
    ];
    let (r, g, b) = T[(seed as usize) % T.len()];
    rgb(r, g, b)
}

fn road_y(kind: u8) -> f32 {
    match kind {
        0 => 0.10,
        1 => 0.09,
        2 => 0.08,
        3 => 0.07,
        _ => 0.05,
    }
}

pub fn build_chunk(map: &Map, cx: i32, cz: i32) -> Vec<f32> {
    let mut mb = MeshBuilder::default();
    let x0 = cx as f32 * CHUNK;
    let z0 = cz as f32 * CHUNK;
    let inside = |p: Pos2| p.x >= x0 && p.x < x0 + CHUNK && p.y >= z0 && p.y < z0 + CHUNK;
    let lo = Pos2::new(x0, z0);
    let hi = Pos2::new(x0 + CHUNK, z0 + CHUNK);
    let white = [1.0, 1.0, 1.0, 1.0];

    // ground
    let g = [Vec3::new(x0, 0.0, z0), Vec3::new(x0 + CHUNK, 0.0, z0), Vec3::new(x0 + CHUNK, 0.0, z0 + CHUNK), Vec3::new(x0, 0.0, z0 + CHUNK)];
    let uvs = g.map(|p| [p.x / 7.0, p.z / 7.0]);
    mb.quad(g, Vec3::Y, white, uvs, L_GROUND);

    // lakes, parks, airport
    for a in &map.areas {
        if a.max.x < lo.x || a.min.x > hi.x || a.max.y < lo.y || a.min.y > hi.y {
            continue;
        }
        let (y, layer, col) = match a.kind {
            0 => (0.03, L_WATER, white),
            1 => (0.025, L_GRASS, white),
            _ => (0.02, L_CONCRETE, rgb(200, 190, 160)),
        };
        for t in &a.tris {
            let c = Pos2::new((t[0].x + t[1].x + t[2].x) / 3.0, (t[0].y + t[1].y + t[2].y) / 3.0);
            if inside(c) {
                mb.tri(t.map(|p| v3(p, y)), col, t.map(|p| [p.x / 12.0, p.y / 12.0]), layer);
            }
        }
    }

    // streets
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    map.road_grid.query(lo, hi, |id| {
        if seen.insert(id) {
            ids.push(id)
        }
    });
    for id in ids {
        let (ri, si) = map.segs[id as usize];
        let road = &map.roads[ri as usize];
        let (a, b) = (road.pts[si as usize], road.pts[si as usize + 1]);
        let mid = a + (b - a) * 0.5;
        if !inside(mid) {
            continue;
        }
        let hw = road.half_width();
        let len = (b - a).length();
        if len < 0.01 {
            continue;
        }
        let d = (b - a) / len;
        let n = egui::vec2(-d.y, d.x);
        let y = road_y(road.kind);
        let layer = if road.kind == 4 { L_LATERITE } else { L_ASPHALT };
        let s0: f32 = road.pts.windows(2).take(si as usize).map(|w| (w[1] - w[0]).length()).sum();
        let (v0, v1) = (s0 / 8.0, (s0 + len) / 8.0);
        let u = hw / 8.0;
        mb.quad(
            [v3(a - n * hw, y), v3(b - n * hw, y), v3(b + n * hw, y), v3(a + n * hw, y)],
            Vec3::Y,
            white,
            [[-u, v0], [-u, v1], [u, v1], [u, v0]],
            layer,
        );
        // rounded joint at the end vertex
        let joints: &[Pos2] = if si == 0 { &[a, b] } else { &[b] };
        for &c in joints {
            for k in 0..8 {
                let a0 = k as f32 / 8.0 * TAU;
                let a1 = (k + 1) as f32 / 8.0 * TAU;
                let p0 = c + egui::vec2(a0.cos(), a0.sin()) * hw;
                let p1 = c + egui::vec2(a1.cos(), a1.sin()) * hw;
                mb.tri([v3(c, y - 0.004), v3(p0, y - 0.004), v3(p1, y - 0.004)], white, [[c.x / 8.0, c.y / 8.0], [p0.x / 8.0, p0.y / 8.0], [p1.x / 8.0, p1.y / 8.0]], layer);
            }
        }
        if road.kind <= 3 {
            // kerbs / sidewalk strips
            for side in [-1.0f32, 1.0] {
                let o0 = n * side * (hw - 0.35);
                let o1 = n * side * (hw + 0.6);
                mb.quad([v3(a + o0, y + 0.03), v3(b + o0, y + 0.03), v3(b + o1, y + 0.03), v3(a + o1, y + 0.03)], Vec3::Y, rgb(176, 168, 152), [[0.0, 0.0]; 4], L_CONCRETE);
            }
        }
        if road.kind <= 2 {
            // dashed centre line
            let mut s = (6.0 - s0 % 6.0) % 6.0;
            while s + 3.0 < len {
                let p0 = a + d * s;
                let p1 = a + d * (s + 3.0);
                let w = n * 0.12;
                mb.quad([v3(p0 - w, y + 0.01), v3(p1 - w, y + 0.01), v3(p1 + w, y + 0.01), v3(p0 + w, y + 0.01)], Vec3::Y, rgb(235, 225, 170), [[0.0, 0.0]; 4], L_PLAIN);
                s += 6.0;
            }
        }
    }

    // buildings
    let mut blds = Vec::new();
    map.bld_grid.query(lo, hi, |id| blds.push(id));
    for id in blds {
        let b = &map.buildings[id as usize];
        if !inside(b.c) {
            continue;
        }
        let h = b.floors as f32 * 3.2;
        let cs = b.corners();
        let tint = wall_tint(id.wrapping_mul(2654435761) >> 7);
        for i in 0..4 {
            let (p0, p1) = (cs[i], cs[(i + 1) % 4]);
            let mid = p0 + (p1 - p0) * 0.5;
            let mut nrm = egui::vec2(-(p1 - p0).y, (p1 - p0).x).normalized();
            if nrm.dot(mid - b.c) < 0.0 {
                nrm = -nrm;
            }
            let lu = ((p1 - p0).length() / 4.0).round().max(1.0);
            let fl = b.floors as f32;
            mb.quad(
                [v3(p0, 0.0), v3(p1, 0.0), v3(p1, h), v3(p0, h)],
                Vec3::new(nrm.x, 0.0, nrm.y),
                tint,
                [[0.0, fl], [lu, fl], [lu, 0.0], [0.0, 0.0]],
                L_WALL,
            );
        }
        let (rl, rc) = match b.roof {
            0 => (L_TIN, rgb(220, 222, 226)),
            1 => (L_CONCRETE, rgb(230, 226, 218)),
            _ => (L_TIN, rgb(205, 120, 80)),
        };
        let roof_y = h + if b.roof == 1 { 0.0 } else { 0.15 };
        let top = cs.map(|p| v3(p, roof_y));
        let ruv = cs.map(|p| {
            let l = b.to_local(p);
            [l.x / 4.0, l.y / 4.0]
        });
        mb.quad(top, Vec3::Y, rc, ruv, rl);
        if b.roof == 1 {
            // low parapet around a terrace roof
            for i in 0..4 {
                let (p0, p1) = (cs[i], cs[(i + 1) % 4]);
                let c = p0 + (p1 - p0) * 0.5;
                let ang = (p1 - p0).y.atan2((p1 - p0).x);
                let m = Mat4::from_translation(v3(c, h + 0.4)) * Mat4::from_rotation_y(-ang);
                mb.cuboid(&m, Vec3::new((p1 - p0).length() * 0.5, 0.4, 0.12), tint, L_PLAIN);
            }
        }
    }

    // trees (neem, karité)
    let mut trees = Vec::new();
    map.tree_grid.query(lo, hi, |id| trees.push(id));
    for id in trees {
        let t = &map.trees[id as usize];
        if !inside(t.p) {
            continue;
        }
        let trunk_h = 1.6 + t.r * 0.25;
        let m = Mat4::from_translation(v3(t.p, trunk_h * 0.5));
        mb.cuboid(&m, Vec3::new(0.16, trunk_h * 0.5, 0.16), rgb(92, 66, 44), L_PLAIN);
        let leafc = if id % 3 == 0 { rgb(200, 215, 170) } else { rgb(235, 240, 220) };
        for (k, (s, y)) in [(1.0, 0.0), (0.72, 0.55), (0.45, 1.0)].iter().enumerate() {
            let r = t.r * s;
            let m = Mat4::from_translation(v3(t.p, trunk_h + y * t.r * 0.8 + r * 0.35)) * Mat4::from_rotation_y(k as f32 * 0.6 + id as f32);
            mb.cuboid(&m, Vec3::new(r * 0.7, r * 0.38, r * 0.7), leafc, L_LEAVES);
            let m2 = m * Mat4::from_rotation_y(FRAC_PI_4);
            mb.cuboid(&m2, Vec3::new(r * 0.7, r * 0.34, r * 0.7), leafc, L_LEAVES);
        }
    }
    mb.v
}

// ---------------------------------------------------------------- characters and vehicles

#[derive(Clone, Copy)]
pub struct Look {
    pub skin: [f32; 4],
    pub shirt: [f32; 4],
    pub stripe: [f32; 4],
    pub pants: [f32; 4],
    pub cap: Option<[f32; 4]>,
}

pub fn bila_look() -> Look {
    // Faso Dan Fani-style striped shirt, jeans, cap
    Look { skin: rgb(92, 58, 38), shirt: rgb(170, 30, 36), stripe: rgb(30, 120, 60), pants: rgb(48, 62, 96), cap: Some(rgb(235, 190, 40)) }
}

pub fn ped_look(c: Color32, seed: u32) -> Look {
    let skins = [rgb(70, 44, 30), rgb(92, 58, 38), rgb(110, 72, 48), rgb(60, 38, 26)];
    let pants = [rgb(40, 40, 46), rgb(60, 70, 110), rgb(120, 100, 70), rgb(230, 225, 210)];
    Look {
        skin: skins[seed as usize % 4],
        shirt: rgba(c, 1.0),
        stripe: rgba(c, 1.0).map(|x| x * 0.75),
        pants: pants[(seed / 4) as usize % 4],
        cap: if seed % 5 == 0 { Some(rgb(240, 240, 235)) } else { None },
    }
}

pub fn humanoid(mb: &mut MeshBuilder, base: Mat4, look: &Look, phase: f32, moving: f32, seated: bool) {
    let hip_y = 0.95;
    for s in [-1.0f32, 1.0] {
        let swing = if seated { 1.35 } else { phase.sin() * 0.6 * moving * s };
        let m = base * Mat4::from_translation(Vec3::new(0.0, hip_y, s * 0.1)) * Mat4::from_rotation_z(swing) * Mat4::from_translation(Vec3::new(0.0, -0.45, 0.0));
        mb.cuboid(&m, Vec3::new(0.085, 0.45, 0.085), look.pants, L_PLAIN);
        let knee = if seated { -1.3 } else { 0.0 };
        let f = m * Mat4::from_translation(Vec3::new(0.0, -0.43, 0.0)) * Mat4::from_rotation_z(knee) * Mat4::from_translation(Vec3::new(0.06, -0.02, 0.0));
        mb.cuboid(&f, Vec3::new(0.13, 0.045, 0.075), rgb(35, 30, 28), L_PLAIN);
        let arm = if seated { 1.15 } else { -phase.sin() * 0.5 * moving * s };
        let a = base * Mat4::from_translation(Vec3::new(0.0, 1.5, s * 0.28)) * Mat4::from_rotation_z(arm) * Mat4::from_translation(Vec3::new(0.0, -0.3, 0.0));
        mb.cuboid(&a, Vec3::new(0.065, 0.3, 0.065), look.shirt, L_PLAIN);
        let hand = a * Mat4::from_translation(Vec3::new(0.0, -0.34, 0.0));
        mb.cuboid(&hand, Vec3::new(0.05, 0.06, 0.05), look.skin, L_PLAIN);
    }
    let t = base * Mat4::from_translation(Vec3::new(0.0, 1.27, 0.0));
    mb.cuboid(&t, Vec3::new(0.12, 0.32, 0.21), look.shirt, L_PLAIN);
    for y in [1.12, 1.3, 1.46] {
        let st = base * Mat4::from_translation(Vec3::new(0.0, y, 0.0));
        mb.cuboid(&st, Vec3::new(0.125, 0.035, 0.215), look.stripe, L_PLAIN);
    }
    let neck = base * Mat4::from_translation(Vec3::new(0.0, 1.62, 0.0));
    mb.cuboid(&neck, Vec3::new(0.05, 0.05, 0.05), look.skin, L_PLAIN);
    let head = base * Mat4::from_translation(Vec3::new(0.0, 1.76, 0.0));
    mb.cuboid(&head, Vec3::new(0.1, 0.12, 0.09), look.skin, L_PLAIN);
    let hair = base * Mat4::from_translation(Vec3::new(-0.01, 1.87, 0.0));
    mb.cuboid(&hair, Vec3::new(0.1, 0.025, 0.092), rgb(20, 16, 14), L_PLAIN);
    for s in [-1.0f32, 1.0] {
        let eye = base * Mat4::from_translation(Vec3::new(0.1, 1.78, s * 0.04));
        mb.cuboid(&eye, Vec3::new(0.004, 0.012, 0.016), rgb(240, 240, 235), L_PLAIN);
    }
    if let Some(c) = look.cap {
        let cap = base * Mat4::from_translation(Vec3::new(0.0, 1.9, 0.0));
        mb.cuboid(&cap, Vec3::new(0.11, 0.035, 0.1), c, L_PLAIN);
        let visor = base * Mat4::from_translation(Vec3::new(0.15, 1.875, 0.0));
        mb.cuboid(&visor, Vec3::new(0.07, 0.01, 0.09), c, L_PLAIN);
    }
}

fn darker(c: [f32; 4], f: f32) -> [f32; 4] {
    [c[0] * f, c[1] * f, c[2] * f, c[3]]
}

#[allow(clippy::too_many_arguments)]
pub fn vehicle(mb: &mut MeshBuilder, pos: Pos2, ang: f32, kind: VKind, color: Color32, dead: bool, flash: bool, rider: Option<&Look>) {
    let base = Mat4::from_translation(v3(pos, 0.0)) * Mat4::from_rotation_y(-ang);
    let t = |x: f32, y: f32, z: f32| base * Mat4::from_translation(Vec3::new(x, y, z));
    let (hl, hw, ..) = kind.spec();
    let mut col = rgba(color, 1.0);
    if dead {
        col = rgb(34, 30, 28);
    }
    let black = rgb(22, 22, 24);
    let glass = if dead { rgb(15, 15, 15) } else { rgb(40, 55, 70) };
    let front_light = if dead { black } else { rgb(255, 248, 225) };
    let rear_light = if dead { black } else { rgb(220, 30, 30) };
    let ll = if dead { L_PLAIN } else { L_EMISSIVE };
    if kind == VKind::Moto {
        for x in [0.62, -0.62] {
            mb.cuboid(&t(x, 0.31, 0.0), Vec3::new(0.31, 0.31, 0.055), black, L_PLAIN);
        }
        mb.cuboid(&t(0.0, 0.55, 0.0), Vec3::new(0.55, 0.11, 0.08), col, L_PLAIN);
        mb.cuboid(&t(0.22, 0.76, 0.0), Vec3::new(0.24, 0.11, 0.14), col, L_PLAIN);
        mb.cuboid(&t(-0.28, 0.8, 0.0), Vec3::new(0.32, 0.06, 0.13), black, L_PLAIN);
        mb.cuboid(&t(0.55, 0.95, 0.0), Vec3::new(0.04, 0.2, 0.04), rgb(120, 120, 125), L_PLAIN);
        mb.cuboid(&t(0.52, 1.13, 0.0), Vec3::new(0.03, 0.03, 0.32), rgb(60, 60, 64), L_PLAIN);
        mb.cuboid(&t(0.66, 0.9, 0.0), Vec3::new(0.04, 0.07, 0.07), front_light, ll);
        mb.cuboid(&t(-0.66, 0.78, 0.0), Vec3::new(0.03, 0.04, 0.06), rear_light, ll);
        if let Some(look) = rider {
            humanoid(mb, t(-0.22, -0.12, 0.0), look, 0.0, 0.0, true);
        }
        return;
    }
    let wheel_z = hw - 0.05;
    for x in [hl * 0.62, -hl * 0.62] {
        for z in [wheel_z, -wheel_z] {
            mb.cuboid(&t(x, 0.34, z), Vec3::new(0.34, 0.34, 0.11), black, L_PLAIN);
        }
    }
    match kind {
        VKind::Minibus => {
            mb.cuboid(&t(0.0, 1.2, 0.0), Vec3::new(hl, 0.85, hw), col, L_PLAIN);
            mb.cuboid(&t(0.05, 1.58, 0.0), Vec3::new(hl * 0.97, 0.26, hw + 0.01), glass, L_PLAIN);
            mb.cuboid(&t(0.0, 0.92, 0.0), Vec3::new(hl + 0.01, 0.08, hw + 0.01), if dead { black } else { rgb(30, 130, 70) }, L_PLAIN);
            mb.cuboid(&t(0.0, 2.1, 0.0), Vec3::new(hl * 0.8, 0.06, hw * 0.8), darker(col, 0.8), L_PLAIN);
        }
        _ => {
            mb.cuboid(&t(0.0, 0.66, 0.0), Vec3::new(hl, 0.33, hw), col, L_PLAIN);
            mb.cuboid(&t(-0.15, 1.18, 0.0), Vec3::new(hl * 0.52, 0.22, hw * 0.86), glass, L_PLAIN);
            mb.cuboid(&t(-0.15, 1.42, 0.0), Vec3::new(hl * 0.48, 0.03, hw * 0.88), col, L_PLAIN);
            for z in [hw * 0.86, -hw * 0.86] {
                mb.cuboid(&t(-0.15, 1.18, z), Vec3::new(0.05, 0.22, 0.02), col, L_PLAIN);
            }
            mb.cuboid(&t(hl + 0.02, 0.45, 0.0), Vec3::new(0.05, 0.08, hw * 0.95), rgb(60, 60, 62), L_PLAIN);
        }
    }
    for z in [hw * 0.6, -hw * 0.6] {
        let fy = if kind == VKind::Minibus { 0.85 } else { 0.75 };
        mb.cuboid(&t(hl + 0.01, fy, z), Vec3::new(0.02, 0.07, 0.17), front_light, ll);
        mb.cuboid(&t(-hl - 0.01, fy, z), Vec3::new(0.02, 0.06, 0.15), rear_light, ll);
    }
    if kind == VKind::Taxi && !dead {
        mb.cuboid(&t(-0.15, 1.52, 0.0), Vec3::new(0.18, 0.07, 0.35), rgb(250, 220, 60), L_EMISSIVE);
    }
    if kind == VKind::Police && !dead {
        mb.cuboid(&t(0.0, 0.7, 0.0), Vec3::new(hl * 0.45, 0.3, hw + 0.01), rgb(235, 235, 235), L_PLAIN);
        let (a, b) = if flash { (rgb(255, 40, 40), rgb(40, 80, 255)) } else { (rgb(60, 10, 10), rgb(10, 20, 60)) };
        let (a, b) = if flash { (a, darker(b, 0.3)) } else { (darker(rgb(255, 40, 40), 0.3), rgb(40, 80, 255)) };
        let _ = (a, b);
        mb.cuboid(&t(-0.15, 1.52, 0.35), Vec3::new(0.12, 0.07, 0.3), a, L_EMISSIVE);
        mb.cuboid(&t(-0.15, 1.52, -0.35), Vec3::new(0.12, 0.07, 0.3), b, L_EMISSIVE);
    }
}

// ---------------------------------------------------------------- per-frame scene

pub struct FrameData {
    pub view_proj: Mat4,
    pub cam: Vec3,
    pub sun: Vec3,
    pub sun_col: Vec3,
    pub ambient: Vec3,
    pub sky: Vec3,
    pub fog: f32,
    pub night: f32,
    pub opaque: Vec<f32>,
    pub translucent: Vec<f32>,
}

/// Lighting for the time of day: (sun dir, sun colour, ambient, sky, fog distance).
pub fn lighting(hours: f32, night: f32) -> (Vec3, Vec3, Vec3, Vec3, f32) {
    let a = (hours - 6.0) / 12.0 * PI;
    let el = a.sin();
    let day = 1.0 - night;
    let low = (1.0 - el.max(0.0)).powf(4.0) * day;
    let sun = if el > 0.02 {
        Vec3::new(a.cos(), el.max(0.12), 0.35).normalize()
    } else {
        Vec3::new(-0.3, 0.8, 0.4).normalize() // moon
    };
    let sun_col = Vec3::new(1.0, 0.93, 0.82).lerp(Vec3::new(1.0, 0.6, 0.35), low) * day + Vec3::new(0.18, 0.22, 0.35) * night;
    let ambient = Vec3::new(0.46, 0.44, 0.46) * day + Vec3::new(0.10, 0.11, 0.18) * night;
    // dusty harmattan blue by day, orange at dusk, deep blue at night
    let sky = Vec3::new(0.62, 0.74, 0.86).lerp(Vec3::new(0.92, 0.6, 0.38), low) * day + Vec3::new(0.03, 0.04, 0.09) * night;
    let fog = 700.0 * day + 330.0 * night;
    (sun, sun_col, ambient, sky, fog)
}

#[allow(clippy::too_many_arguments)]
pub fn build_scene(g: &Game, remotes: &[Remote], cam: Pos2, story_target: Option<Pos2>, chase_tag: Option<u8>) -> (Vec<f32>, Vec<f32>) {
    let mut op = MeshBuilder::default();
    let mut tr = MeshBuilder::default();
    let near = |p: Pos2| (p - cam).length() < 380.0;
    let flash = (g.time * 6.0) as i32 % 2 == 0;
    let shadow = [0.0, 0.0, 0.0, 0.38];
    let bila = bila_look();
    for (i, v) in g.vehicles.iter().enumerate() {
        if !near(v.pos) {
            continue;
        }
        let (hl, hw, ..) = v.kind.spec();
        tr.ground_rect(v.pos, 0.13, v.angle, hl + 0.25, hw + 0.25, shadow, L_SHADOW);
        let rider_look;
        let rider = if v.kind == VKind::Moto && v.driver != Driver::None && !v.dead {
            rider_look = if g.player.vehicle == Some(i) { bila } else { ped_look(Color32::from_rgb(200, 200, 190), i as u32 * 7 + 3) };
            Some(&rider_look)
        } else {
            None
        };
        vehicle(&mut op, v.pos, v.angle, v.kind, v.color, v.dead, flash, rider);
        if v.dead {
            let fl = (g.time * 13.0 + i as f32).sin() * 0.3 + 1.0;
            tr.cuboid(&Mat4::from_translation(v3(v.pos, 1.6 * fl)), Vec3::new(0.5, 0.6 * fl, 0.5), [1.0, 0.5, 0.1, 0.6], L_EMISSIVE);
        }
        if chase_tag.is_some() && v.tag == chase_tag.unwrap() {
            let bob = (g.time * 3.0).sin() * 0.3;
            let m = Mat4::from_translation(v3(v.pos, 4.0 + bob)) * Mat4::from_rotation_y(g.time * 2.0) * Mat4::from_rotation_z(FRAC_PI_4);
            op.cuboid(&m, Vec3::new(0.45, 0.45, 0.45), [1.0, 0.25, 0.2, 1.0], L_EMISSIVE);
        }
    }
    for (i, p) in g.peds.iter().enumerate() {
        if !near(p.pos) {
            continue;
        }
        let look = ped_look(p.color, i as u32 * 13 + (p.color.r() as u32));
        if p.down > 0.0 {
            let base = Mat4::from_translation(v3(p.pos, 0.12)) * Mat4::from_rotation_y(-p.angle) * Mat4::from_rotation_z(-FRAC_PI_2);
            humanoid(&mut op, base, &look, 0.0, 0.0, false);
            continue;
        }
        tr.ground_rect(p.pos, 0.13, p.angle, 0.35, 0.35, shadow, L_SHADOW);
        let moving = if p.flee > 0.0 { 1.0 } else { 0.7 };
        let phase = g.time * if p.flee > 0.0 { 11.0 } else { 6.0 } + i as f32;
        humanoid(&mut op, Mat4::from_translation(v3(p.pos, 0.0)) * Mat4::from_rotation_y(-p.angle), &look, phase, moving, false);
    }
    for r in remotes {
        let pos = Pos2::new(r.x, r.y);
        if !near(pos) {
            continue;
        }
        let c = Color32::from_rgb(r.color[0], r.color[1], r.color[2]);
        let look = Look { shirt: rgba(c, 1.0), stripe: darker(rgba(c, 1.0), 0.7), ..bila };
        if r.kind == 0 {
            humanoid(&mut op, Mat4::from_translation(v3(pos, 0.0)) * Mat4::from_rotation_y(-r.angle), &look, g.time * 6.0, 0.6, false);
        } else {
            vehicle(&mut op, pos, r.angle, VKind::from_net(r.kind), c, false, flash, Some(&look));
        }
        op.cuboid(&Mat4::from_translation(v3(pos, 3.2)), Vec3::new(0.2, 0.2, 0.2), rgba(c, 1.0), L_EMISSIVE);
    }
    if g.player.vehicle.is_none() {
        let p = g.player.pos;
        tr.ground_rect(p, 0.13, g.player.angle, 0.35, 0.35, shadow, L_SHADOW);
        humanoid(&mut op, Mat4::from_translation(v3(p, 0.0)) * Mat4::from_rotation_y(-g.player.angle), &bila, g.player.walk * 2.2, 1.0, false);
    }
    // objective markers
    let pulse = (g.time * 3.0).sin() * 0.5 + 0.5;
    if let Some(t) = story_target {
        tr.prism(t, 4.0 + pulse * 0.4, 0.0, 5.0, [0.25, 0.8, 1.0, 0.33], L_EMISSIVE);
    }
    if let Some(m) = &g.mission {
        tr.prism(m.to, 4.0 + pulse * 0.4, 0.0, 5.0, [0.25, 0.8, 1.0, 0.33], L_EMISSIVE);
    }
    if g.mission.is_none() && !g.story {
        for s in &g.spots {
            if near(*s) {
                tr.prism(*s, 6.0, 0.0, 3.0, [1.0, 0.82, 0.1, 0.3], L_EMISSIVE);
            }
        }
    }
    // headlight pools on the ground at night are added by the shader through `night`
    let _ = Rng(1);
    (op.v, tr.v)
}

// ---------------------------------------------------------------- GL side

struct Gpu {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    count: i32,
}

pub struct R3D {
    prog: glow::Program,
    tex: glow::Texture,
    chunks: HashMap<(i32, i32), Gpu>,
    dynamic: Gpu,
    trans: Gpu,
    map: Arc<Map>,
}

const VS: &str = r#"
layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_nrm;
layout(location=2) in vec4 a_col;
layout(location=3) in vec2 a_uv;
layout(location=4) in float a_layer;
uniform mat4 u_vp;
out vec3 v_pos; out vec3 v_nrm; out vec4 v_col; out vec2 v_uv; out float v_layer;
void main() {
    v_pos = a_pos; v_nrm = a_nrm; v_col = a_col; v_uv = a_uv; v_layer = a_layer;
    gl_Position = u_vp * vec4(a_pos, 1.0);
}
"#;

const FS: &str = r#"
precision highp float;
precision highp sampler2DArray;
uniform sampler2DArray u_tex;
uniform vec3 u_cam; uniform vec3 u_sun; uniform vec3 u_sun_col; uniform vec3 u_amb; uniform vec3 u_sky;
uniform float u_fog; uniform float u_night;
in vec3 v_pos; in vec3 v_nrm; in vec4 v_col; in vec2 v_uv; in float v_layer;
out vec4 frag;
vec3 fogit(vec3 c) {
    float d = length(v_pos - u_cam);
    float f = clamp((d - u_fog * 0.3) / (u_fog * 0.7), 0.0, 1.0);
    return mix(c, u_sky, f);
}
void main() {
    float L = floor(v_layer + 0.5);
    if (L > 10.5) { frag = vec4(0.0, 0.0, 0.0, v_col.a * (1.0 - u_night * 0.5)); return; }
    if (L > 9.5) { frag = vec4(fogit(v_col.rgb), v_col.a); return; }
    vec4 t = texture(u_tex, vec3(v_uv, L));
    vec3 n = normalize(v_nrm);
    float diff = max(dot(n, u_sun), 0.0);
    // cheap ambient occlusion: darker near the ground on walls
    float ao = (abs(n.y) < 0.5) ? mix(0.72, 1.0, clamp(v_pos.y / 2.5, 0.0, 1.0)) : 1.0;
    vec3 col = v_col.rgb * t.rgb * (u_amb * ao + u_sun_col * diff);
    if (L > 2.5 && L < 3.5 && t.a < 0.5) {
        float h = fract(sin(dot(floor(v_uv) + floor(v_pos.xz * 0.05), vec2(12.9898, 78.233))) * 43758.5453);
        if (h < 0.5) col += vec3(1.0, 0.78, 0.42) * u_night * 0.85;
        // sky reflection in the glass by day
        col += u_sky * 0.12 * (1.0 - u_night);
    }
    frag = vec4(fogit(col), v_col.a);
}
"#;

unsafe fn compile(gl: &glow::Context) -> Result<glow::Program, String> {
    let mut last = String::new();
    for header in ["#version 330 core\n", "#version 300 es\n", "#version 140\n"] {
        let prog = gl.create_program()?;
        let mut ok = true;
        let mut shaders = Vec::new();
        for (ty, src) in [(glow::VERTEX_SHADER, VS), (glow::FRAGMENT_SHADER, FS)] {
            let s = gl.create_shader(ty)?;
            let mut text = header.to_string();
            if header.contains("140") {
                text.push_str("#extension GL_ARB_explicit_attrib_location : enable\n");
            }
            text.push_str(src);
            gl.shader_source(s, &text);
            gl.compile_shader(s);
            if !gl.get_shader_compile_status(s) {
                last = gl.get_shader_info_log(s);
                ok = false;
            }
            gl.attach_shader(prog, s);
            shaders.push(s);
        }
        if ok {
            gl.link_program(prog);
            ok = gl.get_program_link_status(prog);
            if !ok {
                last = gl.get_program_info_log(prog);
            }
        }
        for s in shaders {
            gl.detach_shader(prog, s);
            gl.delete_shader(s);
        }
        if ok {
            return Ok(prog);
        }
        gl.delete_program(prog);
    }
    Err(last)
}

unsafe fn make_gpu(gl: &glow::Context) -> Gpu {
    let vao = gl.create_vertex_array().expect("vao");
    let vbo = gl.create_buffer().expect("vbo");
    gl.bind_vertex_array(Some(vao));
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
    let stride = (FLOATS * 4) as i32;
    let layout = [(0u32, 3i32, 0i32), (1, 3, 3), (2, 4, 6), (3, 2, 10), (4, 1, 12)];
    for (loc, size, off) in layout {
        gl.enable_vertex_attrib_array(loc);
        gl.vertex_attrib_pointer_f32(loc, size, glow::FLOAT, false, stride, off * 4);
    }
    gl.bind_vertex_array(None);
    Gpu { vao, vbo, count: 0 }
}

unsafe fn upload(gl: &glow::Context, g: &mut Gpu, data: &[f32], usage: u32) {
    gl.bind_buffer(glow::ARRAY_BUFFER, Some(g.vbo));
    let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data));
    gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, usage);
    g.count = (data.len() / FLOATS) as i32;
}

impl R3D {
    pub fn new(gl: &glow::Context, map: Arc<Map>) -> Result<R3D, String> {
        unsafe {
            let prog = compile(gl)?;
            let tex = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(tex));
            let data = gen_textures();
            gl.tex_image_3d(
                glow::TEXTURE_2D_ARRAY,
                0,
                glow::RGBA8 as i32,
                TEX as i32,
                TEX as i32,
                LAYERS as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(&data)),
            );
            gl.generate_mipmap(glow::TEXTURE_2D_ARRAY);
            gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, None);
            Ok(R3D { prog, tex, chunks: HashMap::new(), dynamic: make_gpu(gl), trans: make_gpu(gl), map })
        }
    }

    pub fn paint(&mut self, gl: &glow::Context, info: &egui::PaintCallbackInfo, fd: &FrameData) {
        unsafe {
            let vp = info.viewport_in_pixels();
            let clip = info.clip_rect_in_pixels();
            gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
            gl.clear_color(fd.sky.x, fd.sky.y, fd.sky.z, 1.0);
            gl.depth_mask(true);
            gl.clear_depth_f32(1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::BLEND);

            // stream in nearby chunks, drop far ones
            let ccx = (fd.cam.x / CHUNK).floor() as i32;
            let ccz = (fd.cam.z / CHUNK).floor() as i32;
            let mut built = 0;
            for dz in -2..=2 {
                for dx in -2..=2 {
                    let key = (ccx + dx, ccz + dz);
                    if !self.chunks.contains_key(&key) && (built < 2 || (dx.abs() <= 1 && dz.abs() <= 1)) {
                        let mesh = build_chunk(&self.map, key.0, key.1);
                        let mut gpu = make_gpu(gl);
                        upload(gl, &mut gpu, &mesh, glow::STATIC_DRAW);
                        self.chunks.insert(key, gpu);
                        built += 1;
                    }
                }
            }
            let far: Vec<(i32, i32)> = self.chunks.keys().filter(|k| (k.0 - ccx).abs() > 3 || (k.1 - ccz).abs() > 3).copied().collect();
            for k in far {
                if let Some(g) = self.chunks.remove(&k) {
                    gl.delete_vertex_array(g.vao);
                    gl.delete_buffer(g.vbo);
                }
            }

            gl.use_program(Some(self.prog));
            let loc = |n: &str| gl.get_uniform_location(self.prog, n);
            gl.uniform_matrix_4_f32_slice(loc("u_vp").as_ref(), false, &fd.view_proj.to_cols_array());
            gl.uniform_3_f32(loc("u_cam").as_ref(), fd.cam.x, fd.cam.y, fd.cam.z);
            gl.uniform_3_f32(loc("u_sun").as_ref(), fd.sun.x, fd.sun.y, fd.sun.z);
            gl.uniform_3_f32(loc("u_sun_col").as_ref(), fd.sun_col.x, fd.sun_col.y, fd.sun_col.z);
            gl.uniform_3_f32(loc("u_amb").as_ref(), fd.ambient.x, fd.ambient.y, fd.ambient.z);
            gl.uniform_3_f32(loc("u_sky").as_ref(), fd.sky.x, fd.sky.y, fd.sky.z);
            gl.uniform_1_f32(loc("u_fog").as_ref(), fd.fog);
            gl.uniform_1_f32(loc("u_night").as_ref(), fd.night);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(self.tex));
            gl.uniform_1_i32(loc("u_tex").as_ref(), 0);

            for (k, c) in &self.chunks {
                if (k.0 - ccx).abs() <= 2 && (k.1 - ccz).abs() <= 2 && c.count > 0 {
                    gl.bind_vertex_array(Some(c.vao));
                    gl.draw_arrays(glow::TRIANGLES, 0, c.count);
                }
            }
            upload(gl, &mut self.dynamic, &fd.opaque, glow::STREAM_DRAW);
            gl.bind_vertex_array(Some(self.dynamic.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, self.dynamic.count);

            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.depth_mask(false);
            upload(gl, &mut self.trans, &fd.translucent, glow::STREAM_DRAW);
            gl.bind_vertex_array(Some(self.trans.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, self.trans.count);
            gl.depth_mask(true);

            gl.bind_vertex_array(None);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, None);
            gl.use_program(None);
            gl.disable(glow::DEPTH_TEST);
        }
    }
}

pub type Shared = Arc<Mutex<Option<Result<R3D, String>>>>;

pub fn callback(rect: egui::Rect, shared: Shared, map: Arc<Map>, fd: FrameData) -> egui::PaintCallback {
    let fd = Arc::new(fd);
    egui::PaintCallback {
        rect,
        callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            let mut guard = shared.lock().unwrap();
            let r = guard.get_or_insert_with(|| R3D::new(painter.gl(), map.clone()));
            if let Ok(r) = r {
                r.paint(painter.gl(), &info, &fd);
            }
        })),
    }
}

//! Real-time 3D renderer (OpenGL through egui_glow).
//! - City chunks from the real OpenStreetMap footprints, streets, lakes and parks.
//! - Photo textures and normal maps (ambientCG, CC0), sky panoramas (Poly Haven, CC0).
//! - Sun / moon lighting with shadow mapping, fog, lit windows at night.
//! - Monuments of Ouagadougou, mosques and churches, trees and baobabs.
//! - Characters in African clothing (wax prints, boubous, pagnes) and vehicles.

use crate::game::{Driver, Game, VKind};
use crate::map::{triangulate, Map};
use crate::game::ActKind;
use crate::models::{self, Inst, Models, M_MOTO};
use crate::skinned::{self, Anim, CInst, Skinned, C_BILA, C_JEUNE};
use crate::statics;
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
const TEX: usize = 512;
const LAYERS: usize = 38;
const SHADOW_SIZE: i32 = 2048;
const SHADOW_HALF: f32 = 150.0;

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
pub const L_PAVING: f32 = 10.0;
pub const L_GLASS: f32 = 11.0;
pub const L_BARK: f32 = 12.0;
pub const L_THATCH: f32 = 13.0;
pub const L_WAX1: f32 = 14.0;
pub const L_WAX2: f32 = 15.0;
pub const L_EMISSIVE: f32 = 40.0;
pub const L_SHADOW: f32 = 41.0;
/// façade photos (from the person's images): ochre, pink, banco, tiles; courtyard wall; signs
pub const L_OCRE: f32 = 21.0;
pub const L_ROSE: f32 = 22.0;
pub const L_BANCO: f32 = 23.0;
pub const L_CARREAUX: f32 = 24.0;
pub const L_CLOTURE: f32 = 25.0;
pub const L_SIGN0: f32 = 30.0;
/// plaster without painted windows (the windows are 3D kit pieces near the camera)
pub const L_PLASTER: f32 = 20.0;

pub fn rgba(c: Color32, a: f32) -> [f32; 4] {
    [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, a]
}
pub fn rgb(r: u8, g: u8, b: u8) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
}
fn v3(p: Pos2, y: f32) -> Vec3 {
    Vec3::new(p.x, y, p.y)
}
fn darker(c: [f32; 4], f: f32) -> [f32; 4] {
    [c[0] * f, c[1] * f, c[2] * f, c[3]]
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
    pub fn tri_n(&mut self, p: [Vec3; 3], n: Vec3, c: [f32; 4], uvs: [[f32; 2]; 3], layer: f32) {
        for i in 0..3 {
            self.vert(p[i], n, c, uvs[i], layer);
        }
    }
    pub fn tri(&mut self, p: [Vec3; 3], c: [f32; 4], uvs: [[f32; 2]; 3], layer: f32) {
        self.tri_n(p, Vec3::Y, c, uvs, layer)
    }
    /// Box of half size `h` in the local frame of `m`; uv in metres / `uvs` (0: one tile per face).
    pub fn cuboid_uv(&mut self, m: &Mat4, h: Vec3, c: [f32; 4], layer: f32, uvs: f32) {
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
            let (su, sw) = if uvs > 0.0 { ((u * h).length() * 2.0 / uvs, (w * h).length() * 2.0 / uvs) } else { (1.0, 1.0) };
            self.quad(pts, nn, c, [[0.0, sw], [su, sw], [su, 0.0], [0.0, 0.0]], layer);
        }
    }
    pub fn cuboid(&mut self, m: &Mat4, h: Vec3, c: [f32; 4], layer: f32) {
        self.cuboid_uv(m, h, c, layer, 0.0)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn ground_rect(&mut self, c: Pos2, y: f32, ang: f32, hl: f32, hw: f32, col: [f32; 4], layer: f32) {
        let f = egui::vec2(ang.cos(), ang.sin());
        let r = egui::vec2(-f.y, f.x);
        let p = [c - f * hl - r * hw, c + f * hl - r * hw, c + f * hl + r * hw, c - f * hl + r * hw].map(|q| v3(q, y));
        self.quad(p, Vec3::Y, col, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], layer);
    }
    /// Vertical prism (cylinder) of height h, optionally tapering, with an optional top cap.
    #[allow(clippy::too_many_arguments)]
    pub fn cylinder(&mut self, c: Vec3, r0: f32, r1: f32, h: f32, sides: usize, col: [f32; 4], layer: f32, cap: bool) {
        for i in 0..sides {
            let a0 = i as f32 / sides as f32 * TAU;
            let a1 = (i + 1) as f32 / sides as f32 * TAU;
            let d0 = Vec3::new(a0.cos(), 0.0, a0.sin());
            let d1 = Vec3::new(a1.cos(), 0.0, a1.sin());
            let n = ((d0 + d1) * 0.5).normalize() + Vec3::Y * ((r0 - r1) / h.max(0.01)) * 0.5;
            let u0 = i as f32 / sides as f32 * r0.max(0.5) * 2.0;
            let u1 = (i + 1) as f32 / sides as f32 * r0.max(0.5) * 2.0;
            self.quad(
                [c + d0 * r0, c + d1 * r0, c + d1 * r1 + Vec3::Y * h, c + d0 * r1 + Vec3::Y * h],
                n.normalize(),
                col,
                [[u0, h / 3.0], [u1, h / 3.0], [u1, 0.0], [u0, 0.0]],
                layer,
            );
            if cap && r1 > 0.0 {
                let t = c + Vec3::Y * h;
                self.tri([t, t + d0 * r1, t + d1 * r1], col, [[0.5, 0.5], [0.5 + a0.cos() * 0.5, 0.5 + a0.sin() * 0.5], [0.5 + a1.cos() * 0.5, 0.5 + a1.sin() * 0.5]], layer);
            }
        }
    }
    /// Dome (hemisphere approximation).
    pub fn dome(&mut self, c: Vec3, r: f32, col: [f32; 4], layer: f32) {
        let rings = 5;
        for k in 0..rings {
            let a0 = k as f32 / rings as f32 * FRAC_PI_2;
            let a1 = (k + 1) as f32 / rings as f32 * FRAC_PI_2;
            self.cylinder(c + Vec3::Y * (a0.sin() * r), a0.cos() * r, a1.cos() * r, (a1.sin() - a0.sin()) * r, 16, col, layer, k == rings - 1);
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn disc(&mut self, c: Pos2, y: f32, r: f32, col: [f32; 4], layer: f32, uvs: f32) {
        let n = 32;
        for i in 0..n {
            let a0 = i as f32 / n as f32 * TAU;
            let a1 = (i + 1) as f32 / n as f32 * TAU;
            let p0 = c + egui::vec2(a0.cos(), a0.sin()) * r;
            let p1 = c + egui::vec2(a1.cos(), a1.sin()) * r;
            self.tri([v3(c, y), v3(p0, y), v3(p1, y)], col, [[c.x / uvs, c.y / uvs], [p0.x / uvs, p0.y / uvs], [p1.x / uvs, p1.y / uvs]], layer);
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

struct Photo {
    color: &'static [u8],
    normal: &'static [u8],
}
macro_rules! photo {
    ($n:literal) => {
        Photo { color: include_bytes!(concat!("../assets/tex/", $n, "_c.jpg")), normal: include_bytes!(concat!("../assets/tex/", $n, "_n.jpg")) }
    };
}

fn decode(bytes: &[u8]) -> Vec<u8> {
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let img = img.to_rgba8();
            let img = if img.width() as usize != TEX || img.height() as usize != TEX {
                image::imageops::resize(&img, TEX as u32, TEX as u32, image::imageops::FilterType::Triangle)
            } else {
                img
            };
            img.into_raw()
        }
        Err(_) => vec![200; TEX * TEX * 4],
    }
}

fn procedural(layer: usize) -> Vec<u8> {
    let mut out = vec![255u8; TEX * TEX * 4];
    for y in 0..TEX {
        for x in 0..TEX {
            let u = x as f32 / TEX as f32;
            let v = y as f32 / TEX as f32;
            let fine = hash2(x as i32, y as i32, layer as u32 + 99);
            let (r, g, b): (f32, f32, f32) = match layer {
                6 => {
                    let w = fbm(u, v, 8, 11);
                    (0.20 + w * 0.12, 0.34 + w * 0.14, 0.38 + w * 0.12)
                }
                9 => {
                    let leaf = fbm(u, v, 16, 3);
                    let k = 0.5 + leaf * 0.8;
                    (0.22 * k, 0.40 * k, 0.15 * k)
                }
                13 => {
                    let n = fbm(u, v, 4, 13 * 17);
                    let s = ((u * 90.0 + n * 6.0).sin() * 0.5 + 0.5) * 0.25;
                    let k = 0.75 + n * 0.3 - s;
                    (0.80 * k, 0.66 * k, 0.40 * k)
                }
                14 | 15 => {
                    // wax print: circles and diamonds in vivid colours
                    let (pu, pv) = ((u * 8.0).fract() - 0.5, (v * 8.0).fract() - 0.5);
                    let cell = ((u * 8.0) as i32 + (v * 8.0) as i32) % 2;
                    let d = (pu * pu + pv * pv).sqrt();
                    let diamond = pu.abs() + pv.abs();
                    let (base, a, bcol) = if layer == 14 {
                        ((0.95, 0.75, 0.15), (0.75, 0.10, 0.12), (0.10, 0.35, 0.65))
                    } else {
                        ((0.10, 0.45, 0.30), (0.95, 0.55, 0.10), (0.95, 0.95, 0.85))
                    };
                    if d < 0.22 || (d > 0.3 && d < 0.36) {
                        a
                    } else if diamond > 0.42 && cell == 0 {
                        bcol
                    } else {
                        base
                    }
                }
                _ => {
                    let k = 0.96 + (fine - 0.5) * 0.04;
                    (k, k, k)
                }
            };
            let i = (y * TEX + x) * 4;
            out[i] = (r.clamp(0.0, 1.0) * 255.0) as u8;
            out[i + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
            out[i + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
            out[i + 3] = 255;
        }
    }
    out
}

/// Plaster wall with one window per 4 m tile and one floor per tile height.
fn add_windows(img: &mut [u8]) {
    for y in 0..TEX {
        for x in 0..TEX {
            let u = x as f32 / TEX as f32;
            let vv = 1.0 - y as f32 / TEX as f32;
            let i = (y * TEX + x) * 4;
            let in_win = (0.3..0.7).contains(&u) && (0.35..0.8).contains(&vv);
            let frame = (0.27..0.73).contains(&u) && (0.32..0.83).contains(&vv);
            let sill = (0.25..0.75).contains(&u) && (0.29..0.33).contains(&vv);
            let mut px = |r: f32, g: f32, b: f32, a: u8| {
                img[i] = (r * 255.0) as u8;
                img[i + 1] = (g * 255.0) as u8;
                img[i + 2] = (b * 255.0) as u8;
                img[i + 3] = a;
            };
            if in_win {
                let refl = 0.25 + (u + vv) * 0.15;
                let bar = (u - 0.5).abs() < 0.008 || (vv - 0.57).abs() < 0.008;
                if bar {
                    px(0.30, 0.28, 0.26, 255);
                } else {
                    px(0.14 + refl * 0.25, 0.18 + refl * 0.3, 0.24 + refl * 0.35, 0);
                }
            } else if frame || sill {
                px(0.62, 0.58, 0.52, 255);
            } else if vv < 0.06 {
                img[i] = (img[i] as f32 * 0.72) as u8;
                img[i + 1] = (img[i + 1] as f32 * 0.6) as u8;
                img[i + 2] = (img[i + 2] as f32 * 0.52) as u8;
            }
        }
    }
}

fn build_textures() -> (Vec<u8>, Vec<u8>) {
    let photos: Vec<(usize, Photo)> = vec![
        (0, photo!("sol")),
        (1, photo!("asphalte")),
        (5, photo!("beton")),
        (7, photo!("herbe")),
        (11, photo!("vitre")),
        (12, photo!("ecorce")),
        (16, photo!("parquet")),
        (17, photo!("carrelage")),
        (18, photo!("tissu")),
        (19, photo!("bois")),
        (2, Photo { color: include_bytes!("../assets/tex/u_sol_laterite.jpg"), normal: include_bytes!("../assets/tex/laterite_n.jpg") }),
        (3, Photo { color: include_bytes!("../assets/tex/u_facade_creme.jpg"), normal: include_bytes!("../assets/tex/mur_n.jpg") }),
        (4, Photo { color: include_bytes!("../assets/tex/u_toit_tole.jpg"), normal: include_bytes!("../assets/tex/tole_n.jpg") }),
        (10, Photo { color: include_bytes!("../assets/tex/u_sol_trottoir.jpg"), normal: &[] }),
        (14, Photo { color: include_bytes!("../assets/tex/u_tissu_wax_1.jpg"), normal: &[] }),
        (15, Photo { color: include_bytes!("../assets/tex/u_tissu_wax_2.jpg"), normal: &[] }),
        (20, Photo { color: include_bytes!("../assets/tex/u_facade_creme.jpg"), normal: include_bytes!("../assets/tex/mur_n.jpg") }),
        (21, Photo { color: include_bytes!("../assets/tex/u_facade_ocre.jpg"), normal: include_bytes!("../assets/tex/mur_n.jpg") }),
        (22, Photo { color: include_bytes!("../assets/tex/u_facade_rose.jpg"), normal: include_bytes!("../assets/tex/mur_n.jpg") }),
        (23, Photo { color: include_bytes!("../assets/tex/u_facade_banco.jpg"), normal: include_bytes!("../assets/tex/mur_n.jpg") }),
        (24, Photo { color: include_bytes!("../assets/tex/u_facade_carreaux.jpg"), normal: &[] }),
        (25, Photo { color: include_bytes!("../assets/tex/u_mur_cloture.jpg"), normal: include_bytes!("../assets/tex/mur_n.jpg") }),
        (26, Photo { color: include_bytes!("../assets/tex/u_tissu_faso_dan_fani_1.jpg"), normal: &[] }),
        (27, Photo { color: include_bytes!("../assets/tex/u_tissu_faso_dan_fani_2.jpg"), normal: &[] }),
        (28, Photo { color: include_bytes!("../assets/tex/u_tissu_bogolan.jpg"), normal: &[] }),
        (29, Photo { color: include_bytes!("../assets/tex/u_sol_marche.jpg"), normal: &[] }),
        (30, Photo { color: include_bytes!("../assets/tex/u_enseigne_alimentation.jpg"), normal: &[] }),
        (31, Photo { color: include_bytes!("../assets/tex/u_enseigne_coiffure.jpg"), normal: &[] }),
        (32, Photo { color: include_bytes!("../assets/tex/u_enseigne_maquis.jpg"), normal: &[] }),
        (33, Photo { color: include_bytes!("../assets/tex/u_enseigne_telephonie.jpg"), normal: &[] }),
        (34, Photo { color: include_bytes!("../assets/tex/u_enseigne_quincaillerie.jpg"), normal: &[] }),
        (35, Photo { color: include_bytes!("../assets/tex/u_enseigne_couture.jpg"), normal: &[] }),
        (36, Photo { color: include_bytes!("../assets/tex/u_enseigne_pharmacie.jpg"), normal: &[] }),
        (37, Photo { color: include_bytes!("../assets/tex/u_enseigne_restaurant.jpg"), normal: &[] }),
    ];
    let mut color = vec![0u8; TEX * TEX * 4 * LAYERS];
    let mut normal = vec![0u8; TEX * TEX * 4 * LAYERS];
    let flat: Vec<u8> = (0..TEX * TEX).flat_map(|_| [128u8, 128, 255, 255]).collect();
    for l in 0..LAYERS {
        let (c, n) = match photos.iter().find(|(i, _)| *i == l) {
            Some((_, p)) => {
                let mut c = decode(p.color);
                if l == 3 {
                    add_windows(&mut c);
                }
                (c, if p.normal.is_empty() { flat.clone() } else { decode(p.normal) })
            }
            None => (procedural(l), flat.clone()),
        };
        let s = l * TEX * TEX * 4;
        color[s..s + TEX * TEX * 4].copy_from_slice(&c);
        normal[s..s + TEX * TEX * 4].copy_from_slice(&n);
    }
    (color, normal)
}

/// Save the textures as PNG (`--export-textures <dir>`).
pub fn export_textures(dir: &std::path::Path) {
    let (data, _) = build_textures();
    let _ = std::fs::create_dir_all(dir);
    for l in 0..LAYERS {
        let s = l * TEX * TEX * 4;
        let _ = image::save_buffer(dir.join(format!("couche{l}.png")), &data[s..s + TEX * TEX * 4], TEX as u32, TEX as u32, image::ColorType::Rgba8);
    }
}

const SKY_FILES: [&[u8]; 3] = [include_bytes!("../assets/sky/jour.jpg"), include_bytes!("../assets/sky/couchant.jpg"), include_bytes!("../assets/sky/nuit.jpg")];
const SKY_W: usize = 2048;
const SKY_H: usize = 1024;

// ---------------------------------------------------------------- city chunks

fn wall_tint(seed: u32, kind: char) -> [f32; 4] {
    const T: [(u8, u8, u8); 9] = [
        (238, 226, 204),
        (226, 186, 134),
        (246, 244, 236),
        (232, 196, 180),
        (242, 226, 168),
        (196, 136, 96),
        (214, 222, 208),
        (226, 170, 120),
        (250, 238, 214),
    ];
    match kind {
        'm' => rgb(244, 236, 214),
        'c' => rgb(236, 224, 200),
        'u' => rgb(176, 112, 74),
        _ => {
            let (r, g, b) = T[(seed as usize) % T.len()];
            rgb(r, g, b)
        }
    }
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

fn centroid(pts: &[Pos2]) -> Pos2 {
    let mut c = egui::Vec2::ZERO;
    for p in pts {
        c += p.to_vec2();
    }
    (c / pts.len() as f32).to_pos2()
}

fn building_mesh(mb: &mut MeshBuilder, b: &crate::map::Building) {
    let n = b.pts.len();
    let h = b.height();
    let tall = b.levels >= 5 && matches!(b.kind, 'o' | 'g' | 'h' | 'p' | 'x');
    let glass = tall && b.seed % 3 != 0;
    let facade = match b.kind {
        'u' => L_BANCO,
        'o' | 'h' | 'k' if b.seed % 2 == 0 => L_CARREAUX,
        _ => [L_WALL, L_WALL, L_OCRE, L_ROSE, L_CARREAUX, L_BANCO, L_OCRE][(b.seed / 3) as usize % 7],
    };
    let facade = if facade == L_BANCO && b.levels > 1 { L_OCRE } else { facade };
    let tint = if glass {
        rgb(200, 214, 226)
    } else if facade == L_WALL {
        let t = wall_tint(b.seed, b.kind);
        [t[0], t[1], t[2], 0.98]
    } else {
        let k = 0.9 + (b.seed % 7) as f32 * 0.02;
        [k, k, k, if facade == L_BANCO && b.kind == 'u' { 1.0 } else { 0.98 }]
    };
    let wall_layer = if glass { L_GLASS } else { facade };
    let fl = if glass { b.levels as f32 / 3.0 } else { b.levels as f32 };
    let area2: f32 = (0..n)
        .map(|i| {
            let (a, c) = (b.pts[i], b.pts[(i + 1) % n]);
            a.x * c.y - c.x * a.y
        })
        .sum();
    let sign = if area2 >= 0.0 { 1.0 } else { -1.0 };
    for i in 0..n {
        let (p0, p1) = (b.pts[i], b.pts[(i + 1) % n]);
        let e = p1 - p0;
        let len = e.length();
        if len < 0.05 {
            continue;
        }
        let nrm = egui::vec2(e.y, -e.x) / len * sign;
        let lu = if glass { len / 6.0 } else if len > 3.0 { (len / 4.0).round().max(1.0) } else { len / 4.0 };
        mb.quad([v3(p0, 0.0), v3(p1, 0.0), v3(p1, h), v3(p0, h)], Vec3::new(nrm.x, 0.0, nrm.y), tint, [[0.0, fl], [lu, fl], [lu, 0.0], [0.0, 0.0]], wall_layer);
        if b.roof == 1 && b.kind != 'u' {
            let pc = { let d = darker(tint, 0.95); [d[0], d[1], d[2], 1.0] };
            mb.quad([v3(p0, h), v3(p1, h), v3(p1, h + 0.6), v3(p0, h + 0.6)], Vec3::new(nrm.x, 0.0, nrm.y), pc, [[0.0, 0.1], [len / 4.0, 0.1], [len / 4.0, 0.0], [0.0, 0.0]], L_CONCRETE);
        }
    }
    let c = centroid(&b.pts);
    match b.roof {
        3 => {
            let apex = v3(c, h + 1.8);
            for i in 0..n {
                let (p0, p1) = (b.pts[i], b.pts[(i + 1) % n]);
                let o0 = p0 + (p0 - c) * 0.25;
                let o1 = p1 + (p1 - c) * 0.25;
                let nn = (v3(o1, h - 0.3) - apex).cross(v3(o0, h - 0.3) - apex).normalize_or_zero();
                mb.tri_n([apex, v3(o0, h - 0.3), v3(o1, h - 0.3)], if nn.y < 0.0 { -nn } else { nn }, rgb(255, 255, 255), [[0.5, 0.0], [0.0, 1.0], [1.0, 1.0]], L_THATCH);
            }
        }
        r => {
            let (rl, rc, ry) = match r {
                0 => (L_TIN, rgb(226, 228, 232), h + 0.12),
                1 => (L_CONCRETE, rgb(232, 228, 220), h + 0.05),
                _ => (L_TIN, rgb(196, 104, 70), h + 0.12),
            };
            let (rl, rc) = if glass { (L_CONCRETE, rgb(200, 200, 205)) } else { (rl, rc) };
            for t in triangulate(&b.pts) {
                mb.tri(t.map(|p| v3(p, ry)), rc, t.map(|p| [p.x / 4.0, p.y / 4.0]), rl);
            }
            if r == 1 && b.seed % 4 == 0 && n >= 4 {
                mb.cylinder(v3(c, h + 0.05), 0.7, 0.7, 1.4, 10, rgb(30, 60, 140), L_PLAIN, true);
            }
        }
    }
    match b.kind {
        'm' => {
            let corner = b.pts[0] + (c - b.pts[0]).normalized() * 2.5;
            mb.cylinder(v3(corner, 0.0), 1.3, 1.1, h + 13.0, 8, tint, L_CONCRETE, false);
            mb.cylinder(v3(corner, h + 11.0), 1.8, 1.8, 0.4, 8, rgb(30, 130, 80), L_PLAIN, true);
            mb.cylinder(v3(corner, h + 13.0), 1.1, 0.0, 2.5, 8, rgb(30, 130, 80), L_PLAIN, false);
            let area = area2.abs() * 0.5;
            if area > 150.0 {
                let r = (area.sqrt() * 0.28).min(9.0);
                mb.cylinder(v3(c, h), r * 0.95, r * 0.95, 1.0, 16, tint, L_CONCRETE, false);
                mb.dome(v3(c, h + 1.0), r, rgb(40, 140, 90), L_PLAIN);
                mb.cylinder(v3(c, h + 1.0 + r), 0.15, 0.05, 1.5, 6, rgb(230, 190, 60), L_PLAIN, false);
            }
        }
        'c' => {
            let corner = b.pts[0] + (c - b.pts[0]).normalized() * 3.0;
            let m = Mat4::from_translation(v3(corner, (h + 9.0) * 0.5));
            mb.cuboid_uv(&m, Vec3::new(2.0, (h + 9.0) * 0.5, 2.0), tint, L_CONCRETE, 4.0);
            mb.cylinder(v3(corner, h + 9.0), 2.6, 0.0, 4.0, 4, rgb(150, 70, 50), L_PLAIN, false);
            mb.cuboid(&Mat4::from_translation(v3(corner, h + 14.0)), Vec3::new(0.12, 1.0, 0.12), rgb(240, 240, 240), L_PLAIN);
            mb.cuboid(&Mat4::from_translation(v3(corner, h + 14.3)), Vec3::new(0.12, 0.12, 0.6), rgb(240, 240, 240), L_PLAIN);
        }
        _ => {}
    }
}

fn plaza(mb: &mut MeshBuilder, c: Pos2, r: f32) {
    mb.disc(c, 0.14, r, rgb(176, 164, 150), L_PAVING, 3.0);
    mb.cylinder(v3(c, 0.0), r, r, 0.14, 32, rgb(210, 200, 185), L_CONCRETE, false);
}

/// The monuments of Ouagadougou, modelled from their general shape (proportions estimated).
fn landmark_mesh(mb: &mut MeshBuilder, l: &crate::map::Landmark) {
    let p = l.pos;
    let gold = rgb(220, 175, 60);
    let white = rgb(240, 236, 226);
    let bronze = rgb(120, 80, 45);
    match l.kind.as_str() {
        "heros" => {
            plaza(mb, p, 45.0);
            for (k, r) in [16.0f32, 13.0, 10.0].iter().enumerate() {
                mb.cylinder(v3(p, 0.14 + k as f32 * 1.2), *r, *r, 1.2, 24, white, L_CONCRETE, true);
            }
            for k in 0..4 {
                let a = k as f32 * FRAC_PI_2 + FRAC_PI_4;
                let q = p + egui::vec2(a.cos(), a.sin()) * 6.5;
                mb.cylinder(v3(q, 3.7), 1.0, 0.6, 26.0, 8, white, L_CONCRETE, false);
            }
            mb.cylinder(v3(p, 3.7), 3.2, 2.4, 44.0, 16, rgb(232, 226, 214), L_CONCRETE, false);
            mb.cylinder(v3(p, 30.0), 7.5, 7.5, 1.0, 24, gold, L_PLAIN, true);
            mb.cylinder(v3(p, 47.7), 3.0, 3.0, 1.2, 16, gold, L_PLAIN, true);
            mb.dome(v3(p, 48.9), 2.4, gold, L_PLAIN);
            mb.cylinder(v3(p, 51.3), 0.3, 0.05, 5.0, 6, gold, L_PLAIN, false);
        }
        "flambeau" => {
            plaza(mb, p, 14.0);
            mb.cylinder(v3(p, 0.14), 3.0, 2.6, 2.0, 12, white, L_CONCRETE, true);
            mb.cylinder(v3(p, 2.1), 1.0, 0.8, 9.0, 12, bronze, L_PLAIN, false);
            mb.cylinder(v3(p, 11.0), 1.6, 1.0, 1.0, 12, gold, L_PLAIN, true);
            mb.cylinder(v3(p, 12.0), 1.2, 0.0, 3.5, 8, [1.0, 0.55, 0.12, 1.0], L_EMISSIVE, false);
        }
        "cineastes" => {
            plaza(mb, p, 16.0);
            mb.cylinder(v3(p, 0.14), 4.0, 4.0, 1.5, 20, white, L_CONCRETE, true);
            for k in 0..6 {
                let y = 1.7 + k as f32 * 2.2;
                mb.cylinder(v3(p, y), 2.4 - k as f32 * 0.15, 2.4 - k as f32 * 0.15, 0.5, 20, rgb(60, 60, 66), L_PLAIN, true);
                mb.cylinder(v3(p, y + 0.5), 0.6, 0.6, 1.7, 10, rgb(180, 180, 186), L_PLAIN, false);
            }
            mb.cuboid(&Mat4::from_translation(v3(p, 16.2)), Vec3::new(1.4, 0.9, 0.8), rgb(40, 40, 44), L_PLAIN);
            let lens = Mat4::from_translation(v3(p + egui::vec2(1.8, 0.0), 16.2)) * Mat4::from_rotation_z(FRAC_PI_2);
            mb.cuboid(&lens, Vec3::new(0.45, 0.5, 0.45), rgb(20, 20, 24), L_PLAIN);
        }
        "nations" => {
            plaza(mb, p, 22.0);
            mb.cylinder(v3(p, 0.14), 12.0, 12.0, 0.6, 32, white, L_CONCRETE, false);
            mb.disc(p, 0.55, 11.6, rgb(255, 255, 255), L_WATER, 6.0);
            mb.cylinder(v3(p, 0.5), 1.5, 0.4, 16.0, 4, white, L_CONCRETE, false);
            mb.cylinder(v3(p, 16.5), 0.4, 0.0, 1.5, 4, gold, L_PLAIN, false);
        }
        "eau" => {
            mb.cylinder(v3(p, 0.0), 1.6, 1.4, 20.0, 12, white, L_CONCRETE, false);
            mb.cylinder(v3(p, 18.0), 3.0, 7.5, 3.0, 20, rgb(230, 230, 235), L_CONCRETE, false);
            mb.cylinder(v3(p, 21.0), 7.5, 7.5, 5.0, 20, rgb(236, 236, 240), L_CONCRETE, false);
            mb.cylinder(v3(p, 26.0), 7.5, 1.0, 1.5, 20, rgb(40, 90, 160), L_PLAIN, true);
        }
        _ => {
            plaza(mb, p, 14.0);
            mb.cylinder(v3(p, 0.14), 2.5, 2.2, 2.4, 12, white, L_CONCRETE, true);
            let statue = Mat4::from_translation(v3(p, 2.5)) * Mat4::from_scale(Vec3::splat(1.8));
            let look = Look { skin: bronze, top: bronze, accent: bronze, bottom: bronze, outfit: Outfit::Boubou, head: Head::None, wax: 0.0 };
            humanoid(mb, statue, &look, 0.0, 0.0, Pose::Stand);
        }
    }
}

fn tree_mesh(mb: &mut MeshBuilder, t: &crate::map::Tree, id: u32) {
    if t.r <= 0.0 {
        return;
    }
    let bark = rgb(150, 128, 110);
    if t.baobab {
        let h = 5.0 + t.r * 0.4;
        mb.cylinder(v3(t.p, 0.0), t.trunk * 1.5, t.trunk * 1.1, h, 12, bark, L_BARK, false);
        for k in 0..6 {
            let a = k as f32 / 6.0 * TAU + id as f32;
            let m = Mat4::from_translation(v3(t.p, h)) * Mat4::from_rotation_y(-a) * Mat4::from_rotation_z(-0.9) * Mat4::from_translation(Vec3::new(0.0, 1.4, 0.0));
            mb.cuboid_uv(&m, Vec3::new(0.25, 1.5, 0.25), bark, L_BARK, 2.0);
            let tip = (m * Mat4::from_translation(Vec3::new(0.0, 1.6, 0.0))).transform_point3(Vec3::ZERO);
            let cm = Mat4::from_translation(tip) * Mat4::from_rotation_y(a);
            mb.cuboid_uv(&cm, Vec3::new(1.4, 0.7, 1.4), rgb(225, 235, 205), L_LEAVES, 2.0);
        }
        return;
    }
    let trunk_h = 1.8 + t.r * 0.3;
    mb.cylinder(v3(t.p, 0.0), t.trunk, t.trunk * 0.7, trunk_h + 0.6, 7, bark, L_BARK, false);
    let leafc = match id % 4 {
        0 => rgb(206, 220, 176),
        1 => rgb(236, 242, 220),
        2 => rgb(222, 232, 196),
        _ => rgb(196, 214, 170),
    };
    let blobs = 7;
    for k in 0..blobs {
        let a = k as f32 / blobs as f32 * TAU + id as f32 * 0.7;
        let off = if k == 0 { egui::Vec2::ZERO } else { egui::vec2(a.cos(), a.sin()) * t.r * 0.55 };
        let y = trunk_h + t.r * 0.35 + if k == 0 { t.r * 0.35 } else { (k % 3) as f32 * 0.4 };
        let s = if k == 0 { t.r * 0.75 } else { t.r * 0.5 };
        let m = Mat4::from_translation(v3(t.p + off, y)) * Mat4::from_rotation_y(a * 1.7) * Mat4::from_rotation_x(0.2 * (k as f32).sin());
        mb.cuboid_uv(&m, Vec3::new(s, s * 0.55, s), leafc, L_LEAVES, 3.0);
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
    let ground_tint = rgb(255, 236, 214);

    let g = [Vec3::new(x0, 0.0, z0), Vec3::new(x0 + CHUNK, 0.0, z0), Vec3::new(x0 + CHUNK, 0.0, z0 + CHUNK), Vec3::new(x0, 0.0, z0 + CHUNK)];
    mb.quad(g, Vec3::Y, ground_tint, g.map(|p| [p.x / 6.0, p.z / 6.0]), L_GROUND);

    for a in &map.areas {
        if a.max.x < lo.x || a.min.x > hi.x || a.max.y < lo.y || a.min.y > hi.y {
            continue;
        }
        let (y, layer, col, s) = match a.kind {
            0 => (0.03, L_WATER, white, 14.0),
            1 => (0.025, L_GRASS, white, 6.0),
            _ => (0.02, L_CONCRETE, rgb(210, 200, 175), 8.0),
        };
        for t in &a.tris {
            let c = Pos2::new((t[0].x + t[1].x + t[2].x) / 3.0, (t[0].y + t[1].y + t[2].y) / 3.0);
            if inside(c) {
                mb.tri(t.map(|p| v3(p, y)), col, t.map(|p| [p.x / s, p.y / s]), layer);
            }
        }
    }

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
        let (layer, tint, sc) = if road.kind == 4 { (L_LATERITE, rgb(168, 150, 142), 5.0) } else { (L_ASPHALT, rgb(118, 116, 120), 6.0) };
        let s0: f32 = road.pts.windows(2).take(si as usize).map(|w| (w[1] - w[0]).length()).sum();
        let (v0, v1) = (s0 / sc, (s0 + len) / sc);
        let u = hw / sc;
        mb.quad([v3(a - n * hw, y), v3(b - n * hw, y), v3(b + n * hw, y), v3(a + n * hw, y)], Vec3::Y, tint, [[-u, v0], [-u, v1], [u, v1], [u, v0]], layer);
        let joints: &[Pos2] = if si == 0 { &[a, b] } else { &[b] };
        for &c in joints {
            for k in 0..10 {
                let a0 = k as f32 / 10.0 * TAU;
                let a1 = (k + 1) as f32 / 10.0 * TAU;
                let p0 = c + egui::vec2(a0.cos(), a0.sin()) * hw;
                let p1 = c + egui::vec2(a1.cos(), a1.sin()) * hw;
                mb.tri([v3(c, y - 0.004), v3(p0, y - 0.004), v3(p1, y - 0.004)], tint, [[c.x / sc, c.y / sc], [p0.x / sc, p0.y / sc], [p1.x / sc, p1.y / sc]], layer);
            }
        }
        // courtyard walls along the residential streets (gates are Blender pieces near the camera)
        for w in crate::kit::walls_for_segment(map, ri as usize, si as usize) {
            let t = w.b - w.a;
            let len = t.length();
            let ang = t.y.atan2(t.x);
            let col = [w.tint[0], w.tint[1], w.tint[2], 1.0];
            let segs: Vec<(f32, f32)> = if w.gate { vec![(0.0, len / 2.0 - 1.75), (len / 2.0 + 1.75, len)] } else { vec![(0.0, len)] };
            for (s0, s1) in segs {
                if s1 - s0 < 0.2 {
                    continue;
                }
                let c = w.a + t / len * ((s0 + s1) / 2.0);
                let m = Mat4::from_translation(v3(c, 1.0)) * Mat4::from_rotation_y(-ang);
                mb.cuboid_uv(&m, Vec3::new((s1 - s0) / 2.0, 1.0, 0.1), [col[0] * 1.05, col[1] * 1.05, col[2] * 1.05, 1.0], L_CLOTURE, 2.0);
                let cap = Mat4::from_translation(v3(c, 2.04)) * Mat4::from_rotation_y(-ang);
                mb.cuboid(&cap, Vec3::new((s1 - s0) / 2.0 + 0.02, 0.05, 0.14), darker(col, 0.8), L_CONCRETE);
                // red laterite dust at the foot of the wall
                let dust = Mat4::from_translation(v3(c + w.out * 0.11, 0.16)) * Mat4::from_rotation_y(-ang);
                mb.cuboid(&dust, Vec3::new((s1 - s0) / 2.0, 0.16, 0.005), rgb(150, 90, 60), L_PLASTER);
            }
        }
        if road.kind <= 3 {
            for side in [-1.0f32, 1.0] {
                let k0 = n * side * (hw - 0.05);
                let k1 = n * side * (hw + 0.25);
                let kerb = rgb(196, 192, 184);
                let ky = y + 0.18;
                mb.quad([v3(a + k0, ky), v3(b + k0, ky), v3(b + k1, ky), v3(a + k1, ky)], Vec3::Y, kerb, [[0.0, 0.0], [len / 2.0, 0.0], [len / 2.0, 0.2], [0.0, 0.2]], L_CONCRETE);
                mb.quad([v3(a + k0, y), v3(b + k0, y), v3(b + k0, ky), v3(a + k0, ky)], Vec3::new(-n.x * side, 0.0, -n.y * side), kerb, [[0.0, 0.0], [len / 2.0, 0.0], [len / 2.0, 0.1], [0.0, 0.1]], L_CONCRETE);
                let w0 = n * side * (hw + 0.25);
                let w1 = n * side * (hw + 2.2);
                mb.quad([v3(a + w0, ky - 0.02), v3(b + w0, ky - 0.02), v3(b + w1, ky - 0.02), v3(a + w1, ky - 0.02)], Vec3::Y, rgb(196, 182, 168), [[0.0, s0 / 3.0], [0.0, (s0 + len) / 3.0], [0.65, (s0 + len) / 3.0], [0.65, s0 / 3.0]], L_PAVING);
            }
        }
        if road.kind <= 2 {
            let mut s = (6.0 - s0 % 6.0) % 6.0;
            while s + 3.0 < len {
                let p0 = a + d * s;
                let p1 = a + d * (s + 3.0);
                let w = n * 0.12;
                mb.quad([v3(p0 - w, y + 0.01), v3(p1 - w, y + 0.01), v3(p1 + w, y + 0.01), v3(p0 + w, y + 0.01)], Vec3::Y, rgb(240, 236, 220), [[0.0, 0.0]; 4], L_PLAIN);
                s += 6.0;
            }
            let mut s = (35.0 - s0 % 35.0) % 35.0;
            while s < len {
                let p = a + d * s + n * (hw + 0.8);
                mb.cylinder(v3(p, y), 0.09, 0.07, 7.5, 6, rgb(90, 92, 96), L_PLAIN, false);
                let arm = Mat4::from_translation(v3(p - n * 0.9, y + 7.4)) * Mat4::from_rotation_y(-(n.y.atan2(n.x)));
                mb.cuboid(&arm, Vec3::new(0.9, 0.05, 0.05), rgb(90, 92, 96), L_PLAIN);
                mb.cuboid(&Mat4::from_translation(v3(p - n * 1.7, y + 7.3)), Vec3::new(0.3, 0.08, 0.18), rgb(255, 236, 190), L_EMISSIVE);
                s += 35.0;
            }
        }
    }

    let mut seen_b = HashSet::new();
    let mut blds = Vec::new();
    map.bld_grid.query(lo, hi, |id| {
        if seen_b.insert(id) {
            blds.push(id)
        }
    });
    for id in blds {
        let b = &map.buildings[id as usize];
        if inside(b.c) {
            building_mesh(&mut mb, b);
        }
    }
    for l in &map.landmarks {
        if inside(l.pos) {
            landmark_mesh(&mut mb, l);
        }
    }
    let mut seen_t = HashSet::new();
    let mut trees = Vec::new();
    map.tree_grid.query(lo, hi, |id| {
        if seen_t.insert(id) {
            trees.push(id)
        }
    });
    for id in trees {
        let t = &map.trees[id as usize];
        if inside(t.p) {
            tree_mesh(&mut mb, t, id);
        }
    }
    mb.v
}

// ---------------------------------------------------------------- characters

#[derive(Clone, Copy, PartialEq)]
pub enum Outfit {
    Shirt,
    Boubou,
    Dress,
    Suit,
}
#[derive(Clone, Copy, PartialEq)]
pub enum Head {
    None,
    Cap([f32; 4]),
    Wrap([f32; 4]),
    Kufi([f32; 4]),
}
#[derive(Clone, Copy, PartialEq)]
pub enum Pose {
    Stand,
    Seated,
}

#[derive(Clone, Copy)]
pub struct Look {
    pub skin: [f32; 4],
    pub top: [f32; 4],
    pub accent: [f32; 4],
    pub bottom: [f32; 4],
    pub outfit: Outfit,
    pub head: Head,
    /// 0: plain cloth, 14/15: wax print texture
    pub wax: f32,
}

const EBONY: [(u8, u8, u8); 6] = [(46, 30, 22), (58, 38, 28), (66, 42, 30), (52, 34, 25), (76, 50, 35), (40, 27, 20)];

/// Bila: a Faso Dan Fani shirt at first, the grey suit of his photo once he is rich.
pub fn bila_look(suit: bool) -> Look {
    if suit {
        Look { skin: rgb(62, 40, 29), top: rgb(92, 98, 108), accent: rgb(236, 236, 232), bottom: rgb(86, 92, 102), outfit: Outfit::Suit, head: Head::None, wax: 0.0 }
    } else {
        Look { skin: rgb(62, 40, 29), top: rgb(170, 30, 36), accent: rgb(30, 120, 60), bottom: rgb(48, 62, 96), outfit: Outfit::Shirt, head: Head::None, wax: 0.0 }
    }
}

pub fn ped_look(seed: u32) -> Look {
    let h = |k: u32| (seed.wrapping_mul(2654435761).wrapping_add(k.wrapping_mul(40503)).wrapping_mul(2246822519) >> 8) as usize;
    let (r, g, b) = EBONY[h(1) % EBONY.len()];
    let vivid = [rgb(230, 160, 20), rgb(200, 40, 50), rgb(30, 110, 180), rgb(20, 140, 90), rgb(245, 240, 230), rgb(140, 50, 150), rgb(240, 110, 30), rgb(40, 40, 50), rgb(250, 210, 60), rgb(90, 160, 210)];
    let top = vivid[h(2) % vivid.len()];
    let accent = vivid[h(3) % vivid.len()];
    let pants = [rgb(36, 36, 42), rgb(54, 64, 100), rgb(140, 116, 80), rgb(232, 226, 210)];
    let kind = h(4) % 100;
    let (outfit, head) = if kind < 32 {
        (Outfit::Dress, Head::Wrap(accent))
    } else if kind < 55 {
        (Outfit::Boubou, if h(5) % 2 == 0 { Head::Kufi(rgb(240, 240, 235)) } else { Head::None })
    } else {
        (Outfit::Shirt, if h(6) % 5 == 0 { Head::Cap(accent) } else { Head::None })
    };
    let wax = if h(7) % 3 == 0 { if h(8) % 2 == 0 { L_WAX1 } else { L_WAX2 } } else { 0.0 };
    Look { skin: rgb(r, g, b), top, accent, bottom: pants[h(9) % 4], outfit, head, wax }
}

fn cloth(look: &Look) -> (f32, [f32; 4]) {
    if look.wax > 0.0 {
        (look.wax, [1.0, 1.0, 1.0, 1.0])
    } else {
        (L_PLAIN, look.top)
    }
}

pub fn humanoid(mb: &mut MeshBuilder, base: Mat4, look: &Look, phase: f32, moving: f32, pose: Pose) {
    let seated = pose == Pose::Seated;
    let t = |x: f32, y: f32, z: f32| Mat4::from_translation(Vec3::new(x, y, z));
    let (cl, ctop) = cloth(look);
    let hip_y = 0.94;
    let shoe = rgb(30, 26, 24);
    let long_robe = matches!(look.outfit, Outfit::Boubou | Outfit::Dress);
    for s in [-1.0f32, 1.0] {
        let swing = if seated { 1.4 } else { phase.sin() * 0.55 * moving * s };
        let knee = if seated { -1.45 } else { (-(phase + s * 1.2).sin()).max(0.0) * -0.7 * moving };
        let thigh = base * t(0.0, hip_y, s * 0.095) * Mat4::from_rotation_z(swing);
        let leg_col = if look.outfit == Outfit::Dress { look.skin } else { look.bottom };
        mb.cuboid(&(thigh * t(0.0, -0.23, 0.0)), Vec3::new(0.085, 0.23, 0.08), leg_col, L_PLAIN);
        let shin = thigh * t(0.0, -0.46, 0.0) * Mat4::from_rotation_z(knee);
        mb.cuboid(&(shin * t(0.0, -0.22, 0.0)), Vec3::new(0.07, 0.22, 0.07), if long_robe { look.skin } else { look.bottom }, L_PLAIN);
        mb.cuboid(&(shin * t(0.06, -0.46, 0.0)), Vec3::new(0.13, 0.04, 0.065), shoe, L_PLAIN);
        let arm = if seated { 1.2 } else { -phase.sin() * 0.45 * moving * s };
        let up = base * t(0.0, 1.45, s * 0.25) * Mat4::from_rotation_x(s * 0.06) * Mat4::from_rotation_z(arm);
        let (sleeve, sleeve_layer) = if look.outfit == Outfit::Suit { (look.top, L_PLAIN) } else { (ctop, cl) };
        let wide = if look.outfit == Outfit::Boubou { 0.1 } else { 0.058 };
        mb.cuboid(&(up * t(0.0, -0.15, 0.0)), Vec3::new(wide, 0.15, wide), sleeve, sleeve_layer);
        let fore = up * t(0.0, -0.3, 0.0) * Mat4::from_rotation_z(if seated { 0.3 } else { 0.25 + moving * 0.2 });
        let covered = matches!(look.outfit, Outfit::Suit | Outfit::Boubou);
        let (fore_col, fore_layer) = if covered { (sleeve, sleeve_layer) } else { (look.skin, L_PLAIN) };
        mb.cuboid(&(fore * t(0.0, -0.13, 0.0)), Vec3::new(0.05, 0.13, 0.05), fore_col, fore_layer);
        mb.cuboid(&(fore * t(0.0, -0.3, 0.0)), Vec3::new(0.045, 0.06, 0.035), look.skin, L_PLAIN);
    }
    let waist_plain = matches!(look.outfit, Outfit::Shirt | Outfit::Suit);
    mb.cuboid(&(base * t(0.0, 0.98, 0.0)), Vec3::new(0.11, 0.08, 0.19), if waist_plain { look.bottom } else { ctop }, if waist_plain { L_PLAIN } else { cl });
    let (torso_col, torso_layer) = if look.outfit == Outfit::Suit { (look.top, L_PLAIN) } else { (ctop, cl) };
    mb.cuboid_uv(&(base * t(0.0, 1.14, 0.0)), Vec3::new(0.105, 0.1, 0.17), torso_col, torso_layer, 0.4);
    mb.cuboid_uv(&(base * t(0.0, 1.34, 0.0)), Vec3::new(0.12, 0.13, 0.215), torso_col, torso_layer, 0.4);
    mb.cuboid(&(base * t(0.0, 1.46, 0.0)), Vec3::new(0.1, 0.03, 0.24), torso_col, torso_layer);
    match look.outfit {
        Outfit::Suit => {
            mb.cuboid(&(base * t(0.115, 1.36, 0.0)), Vec3::new(0.01, 0.11, 0.06), look.accent, L_PLAIN);
            mb.cuboid(&(base * t(0.128, 1.32, 0.0)), Vec3::new(0.006, 0.13, 0.018), rgb(196, 198, 202), L_PLAIN);
            mb.cuboid(&(base * t(0.06, 0.85, 0.0)), Vec3::new(0.075, 0.12, 0.2), look.top, L_PLAIN);
        }
        Outfit::Boubou => {
            mb.cuboid_uv(&(base * t(0.0, 0.6, 0.0)), Vec3::new(0.16, 0.42, 0.24), ctop, cl, 0.5);
        }
        Outfit::Dress => {
            mb.cuboid_uv(&(base * t(0.0, 0.58, 0.0)), Vec3::new(0.15, 0.4, 0.21), ctop, cl, 0.5);
            mb.cuboid(&(base * t(0.0, 1.02, 0.0)), Vec3::new(0.115, 0.03, 0.2), look.accent, L_PLAIN);
        }
        Outfit::Shirt => {
            mb.cuboid(&(base * t(0.0, 1.25, 0.0)), Vec3::new(0.124, 0.025, 0.218), look.accent, L_PLAIN);
            mb.cuboid(&(base * t(0.0, 1.06, 0.0)), Vec3::new(0.124, 0.025, 0.2), look.accent, L_PLAIN);
        }
    }
    mb.cuboid(&(base * t(0.0, 1.53, 0.0)), Vec3::new(0.045, 0.05, 0.045), look.skin, L_PLAIN);
    let head = base * t(0.0, 1.68, 0.0);
    mb.cuboid(&head, Vec3::new(0.1, 0.115, 0.085), look.skin, L_PLAIN);
    mb.cuboid(&(head * t(0.02, -0.02, 0.0)), Vec3::new(0.095, 0.08, 0.092), look.skin, L_PLAIN);
    mb.cuboid(&(head * t(0.1, -0.08, 0.0)), Vec3::new(0.02, 0.035, 0.06), look.skin, L_PLAIN);
    mb.cuboid(&(head * t(0.103, -0.01, 0.0)), Vec3::new(0.012, 0.03, 0.02), darker(look.skin, 0.9), L_PLAIN);
    for s in [-1.0f32, 1.0] {
        mb.cuboid(&(head * t(0.098, 0.025, s * 0.038)), Vec3::new(0.004, 0.012, 0.018), rgb(236, 232, 226), L_PLAIN);
        mb.cuboid(&(head * t(0.101, 0.025, s * 0.038)), Vec3::new(0.003, 0.008, 0.008), rgb(16, 12, 10), L_PLAIN);
        mb.cuboid(&(head * t(0.0, 0.0, s * 0.09)), Vec3::new(0.025, 0.035, 0.01), darker(look.skin, 0.9), L_PLAIN);
    }
    mb.cuboid(&(head * t(0.1, -0.06, 0.0)), Vec3::new(0.004, 0.008, 0.035), rgb(70, 36, 32), L_PLAIN);
    match look.head {
        Head::None => {
            mb.cuboid(&(head * t(-0.01, 0.11, 0.0)), Vec3::new(0.1, 0.02, 0.088), rgb(14, 11, 10), L_PLAIN);
            mb.cuboid(&(head * t(-0.07, 0.04, 0.0)), Vec3::new(0.035, 0.07, 0.088), rgb(14, 11, 10), L_PLAIN);
        }
        Head::Cap(c) => {
            mb.cuboid(&(head * t(0.0, 0.13, 0.0)), Vec3::new(0.105, 0.035, 0.095), c, L_PLAIN);
            mb.cuboid(&(head * t(0.15, 0.105, 0.0)), Vec3::new(0.07, 0.01, 0.085), c, L_PLAIN);
        }
        Head::Wrap(c) => {
            mb.cuboid_uv(&(head * t(-0.01, 0.15, 0.0)), Vec3::new(0.12, 0.07, 0.11), c, if look.wax > 0.0 { look.wax } else { L_PLAIN }, 0.3);
            mb.cuboid(&(head * t(-0.02, 0.23, 0.04)), Vec3::new(0.08, 0.04, 0.06), c, L_PLAIN);
        }
        Head::Kufi(c) => {
            mb.cuboid(&(head * t(-0.005, 0.135, 0.0)), Vec3::new(0.098, 0.04, 0.09), c, L_PLAIN);
        }
    }
}

// ---------------------------------------------------------------- vehicles

#[allow(clippy::too_many_arguments)]
pub fn vehicle(mb: &mut MeshBuilder, pos: Pos2, ang: f32, kind: VKind, color: Color32, dead: bool, flash: bool, rider: Option<&Look>, spin: f32) {
    let base = Mat4::from_translation(v3(pos, 0.0)) * Mat4::from_rotation_y(-ang);
    let t = |x: f32, y: f32, z: f32| base * Mat4::from_translation(Vec3::new(x, y, z));
    let (hl, hw, ..) = kind.spec();
    let col = if dead { rgb(34, 30, 28) } else { rgba(color, 1.0) };
    let black = rgb(20, 20, 22);
    let rim = rgb(170, 172, 178);
    let glass = if dead { rgb(15, 15, 15) } else { rgb(28, 40, 52) };
    let front_light = if dead { black } else { rgb(255, 248, 225) };
    let rear_light = if dead { black } else { rgb(220, 30, 30) };
    let ll = if dead { L_PLAIN } else { L_EMISSIVE };
    let wheel = |mb: &mut MeshBuilder, x: f32, z: f32, r: f32, w: f32| {
        let m = t(x, r, z) * Mat4::from_rotation_z(-spin);
        mb.cuboid(&m, Vec3::new(r * 0.92, r * 0.92, w), black, L_PLAIN);
        mb.cuboid(&(m * Mat4::from_rotation_z(FRAC_PI_4)), Vec3::new(r * 0.92, r * 0.92, w * 0.98), black, L_PLAIN);
        mb.cuboid(&(m * t(0.0, 0.0, w.copysign(z.max(0.001) - 0.0005))), Vec3::new(r * 0.45, r * 0.45, 0.012), rim, L_PLAIN);
    };
    if kind == VKind::Moto {
        wheel(mb, 0.62, 0.0, 0.31, 0.055);
        wheel(mb, -0.62, 0.0, 0.31, 0.055);
        mb.cuboid(&t(0.0, 0.52, 0.0), Vec3::new(0.5, 0.09, 0.07), rgb(60, 60, 64), L_PLAIN);
        mb.cuboid(&t(0.2, 0.74, 0.0), Vec3::new(0.24, 0.12, 0.15), col, L_PLAIN);
        mb.cuboid(&t(-0.55, 0.62, 0.0), Vec3::new(0.18, 0.1, 0.12), col, L_PLAIN);
        mb.cuboid(&t(-0.28, 0.8, 0.0), Vec3::new(0.32, 0.06, 0.14), black, L_PLAIN);
        mb.cuboid(&t(0.56, 0.95, 0.0), Vec3::new(0.035, 0.22, 0.035), rim, L_PLAIN);
        mb.cuboid(&t(0.52, 1.14, 0.0), Vec3::new(0.025, 0.025, 0.33), rgb(60, 60, 64), L_PLAIN);
        mb.cuboid(&t(0.67, 0.92, 0.0), Vec3::new(0.05, 0.08, 0.08), front_light, ll);
        mb.cuboid(&t(-0.74, 0.68, 0.0), Vec3::new(0.02, 0.04, 0.07), rear_light, ll);
        mb.cuboid(&t(-0.3, 0.38, 0.1), Vec3::new(0.3, 0.04, 0.04), rim, L_PLAIN);
        if let Some(look) = rider {
            humanoid(mb, t(-0.2, -0.12, 0.0), look, 0.0, 0.0, Pose::Seated);
        }
        return;
    }
    let wz = hw - 0.08;
    for x in [hl * 0.64, -hl * 0.64] {
        for z in [wz, -wz] {
            wheel(mb, x, z, 0.34, 0.11);
        }
    }
    match kind {
        VKind::Minibus => {
            mb.cuboid(&t(0.0, 1.2, 0.0), Vec3::new(hl, 0.85, hw), col, L_PLAIN);
            mb.cuboid(&t(0.05, 1.58, 0.0), Vec3::new(hl * 0.97, 0.26, hw + 0.01), glass, L_PLAIN);
            mb.cuboid(&t(hl + 0.005, 1.45, 0.0), Vec3::new(0.01, 0.4, hw * 0.9), glass, L_PLAIN);
            mb.cuboid(&t(0.0, 0.92, 0.0), Vec3::new(hl + 0.01, 0.08, hw + 0.01), if dead { black } else { rgb(30, 130, 70) }, L_PLAIN);
            mb.cuboid(&t(0.0, 2.1, 0.0), Vec3::new(hl * 0.8, 0.06, hw * 0.8), darker(col, 0.8), L_PLAIN);
            if !dead {
                mb.cuboid(&t(-0.4, 2.3, 0.0), Vec3::new(1.0, 0.18, 0.6), rgb(150, 90, 50), L_PLAIN);
                mb.cuboid(&t(0.6, 2.28, 0.2), Vec3::new(0.4, 0.16, 0.35), rgb(40, 90, 160), L_PLAIN);
            }
        }
        _ => {
            mb.cuboid(&t(0.0, 0.58, 0.0), Vec3::new(hl, 0.26, hw), col, L_PLAIN);
            mb.cuboid(&t(hl * 0.62, 0.86, 0.0), Vec3::new(hl * 0.38, 0.04, hw * 0.95), col, L_PLAIN);
            mb.cuboid(&t(-hl * 0.75, 0.86, 0.0), Vec3::new(hl * 0.25, 0.04, hw * 0.95), col, L_PLAIN);
            mb.cuboid(&t(-0.15, 1.1, 0.0), Vec3::new(hl * 0.5, 0.2, hw * 0.86), glass, L_PLAIN);
            mb.cuboid(&t(-0.15, 1.33, 0.0), Vec3::new(hl * 0.47, 0.04, hw * 0.88), col, L_PLAIN);
            for z in [hw * 0.86, -hw * 0.86] {
                mb.cuboid(&t(-0.15, 1.1, z), Vec3::new(0.05, 0.2, 0.02), col, L_PLAIN);
            }
            for z in [hw + 0.005, -hw - 0.005] {
                mb.cuboid(&t(0.0, 0.62, z), Vec3::new(hl * 0.9, 0.008, 0.005), darker(col, 0.6), L_PLAIN);
            }
            mb.cuboid(&t(hl + 0.02, 0.45, 0.0), Vec3::new(0.05, 0.08, hw * 0.95), rgb(60, 60, 62), L_PLAIN);
            mb.cuboid(&t(-hl - 0.02, 0.45, 0.0), Vec3::new(0.05, 0.08, hw * 0.95), rgb(60, 60, 62), L_PLAIN);
            mb.cuboid(&t(hl + 0.01, 0.62, 0.0), Vec3::new(0.01, 0.08, hw * 0.4), rgb(40, 40, 44), L_PLAIN);
            for z in [hw + 0.08, -hw - 0.08] {
                mb.cuboid(&t(hl * 0.25, 1.0, z), Vec3::new(0.06, 0.05, 0.08), col, L_PLAIN);
            }
        }
    }
    for z in [hw * 0.62, -hw * 0.62] {
        let fy = if kind == VKind::Minibus { 0.85 } else { 0.7 };
        mb.cuboid(&t(hl + 0.01, fy, z), Vec3::new(0.02, 0.07, 0.17), front_light, ll);
        mb.cuboid(&t(-hl - 0.01, fy, z), Vec3::new(0.02, 0.06, 0.15), rear_light, ll);
    }
    if kind == VKind::Taxi && !dead {
        mb.cuboid(&t(-0.15, 1.43, 0.0), Vec3::new(0.18, 0.07, 0.35), rgb(250, 220, 60), L_EMISSIVE);
    }
    if kind == VKind::Police && !dead {
        mb.cuboid(&t(0.0, 0.66, 0.0), Vec3::new(hl * 0.45, 0.27, hw + 0.01), rgb(235, 235, 235), L_PLAIN);
        let on = |b: bool, c: [f32; 4]| if b { c } else { darker(c, 0.25) };
        mb.cuboid(&t(-0.15, 1.43, 0.35), Vec3::new(0.12, 0.07, 0.3), on(flash, rgb(255, 40, 40)), L_EMISSIVE);
        mb.cuboid(&t(-0.15, 1.43, -0.35), Vec3::new(0.12, 0.07, 0.3), on(!flash, rgb(40, 80, 255)), L_EMISSIVE);
    }
}

// ---------------------------------------------------------------- per-frame scene

pub struct FrameData {
    pub view_proj: Mat4,
    pub light_vp: Mat4,
    pub cam: Vec3,
    pub sun: Vec3,
    pub sun_col: Vec3,
    pub ambient: Vec3,
    pub sky: Vec3,
    pub sky_w: Vec3,
    pub fog: f32,
    pub night: f32,
    pub focus: Vec3,
    pub opaque: Vec<f32>,
    pub translucent: Vec<f32>,
    pub insts: Vec<Inst>,
    pub chars: Vec<CInst>,
    pub cars: Vec<CarInst>,
    pub interior: Option<usize>,
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub struct Light {
    pub sun: Vec3,
    pub sun_col: Vec3,
    pub ambient: Vec3,
    pub fog_col: Vec3,
    pub fog: f32,
    pub sky_w: Vec3,
}

/// Lighting for the time of day.
pub fn lighting(hours: f32) -> Light {
    let a = (hours - 6.0) / 12.0 * PI;
    let el = a.sin();
    let wd = smooth(0.08, 0.4, el);
    let wn = smooth(0.0, -0.2, el);
    let ws = (1.0 - wd - wn).max(0.0);
    let sun = if el > 0.0 { Vec3::new(a.cos(), el.max(0.1), 0.3).normalize() } else { Vec3::new(-0.3, 0.75, 0.45).normalize() };
    let sun_col = Vec3::new(1.05, 0.97, 0.86) * wd + Vec3::new(1.0, 0.55, 0.3) * ws * 0.9 + Vec3::new(0.16, 0.2, 0.32) * wn;
    let ambient = Vec3::new(0.42, 0.42, 0.46) * wd + Vec3::new(0.36, 0.28, 0.3) * ws + Vec3::new(0.08, 0.09, 0.15) * wn;
    let fog_col = Vec3::new(0.80, 0.82, 0.84) * wd + Vec3::new(0.92, 0.62, 0.42) * ws + Vec3::new(0.04, 0.05, 0.09) * wn;
    Light { sun, sun_col, ambient, fog_col, fog: 900.0 * (1.0 - wn) + 380.0 * wn, sky_w: Vec3::new(wd, ws, wn) }
}

pub fn light_matrix(focus: Vec3, sun: Vec3) -> Mat4 {
    let texel = SHADOW_HALF * 2.0 / SHADOW_SIZE as f32;
    let up = if sun.y.abs() > 0.98 { Vec3::Z } else { Vec3::Y };
    let view0 = Mat4::look_at_rh(sun * 400.0, Vec3::ZERO, up);
    let mut f = view0.transform_point3(focus);
    f.x = (f.x / texel).floor() * texel;
    f.y = (f.y / texel).floor() * texel;
    let proj = Mat4::orthographic_rh_gl(f.x - SHADOW_HALF, f.x + SHADOW_HALF, f.y - SHADOW_HALF, f.y + SHADOW_HALF, -f.z - 500.0, -f.z + 500.0);
    proj * view0
}

pub struct SceneInput<'a> {
    pub game: &'a Game,
    pub remotes: &'a [Remote],
    pub cam: Pos2,
    pub marker: Option<Pos2>,
    pub chase_tag: Option<u8>,
    pub bila_suit: bool,
    pub biz_markers: &'a [(Pos2, bool)],
    pub steer: f32,
}

fn person_model(seed: u32) -> usize {
    skinned::PEOPLE[(seed.wrapping_mul(2654435761) >> 13) as usize % skinned::PEOPLE.len()]
}

fn at(p: Pos2, ang: f32) -> Mat4 {
    Mat4::from_translation(v3(p, 0.0)) * Mat4::from_rotation_y(-ang)
}

/// A Blender-modelled car to draw: body + 4 wheels.
#[derive(Clone, Copy)]
pub struct CarInst {
    pub mesh: usize,
    pub mat: Mat4,
    pub paint: [f32; 3],
    pub spin: f32,
    pub steer: f32,
}

pub struct Scene {
    pub opaque: Vec<f32>,
    pub translucent: Vec<f32>,
    pub insts: Vec<Inst>,
    pub chars: Vec<CInst>,
    pub cars: Vec<CarInst>,
}

fn car_mesh(k: VKind) -> usize {
    match k {
        VKind::Taxi => statics::V_TAXI,
        VKind::Police => statics::V_POLICE,
        VKind::Minibus => statics::V_MINIBUS,
        _ => statics::V_SEDAN,
    }
}

/// Bila's animation for the current state.
pub fn bila_anim(g: &Game) -> (Anim, f32) {
    if let Some(a) = &g.activity {
        let anim = match a.kind {
            ActKind::Eat => Anim::Eat,
            ActKind::Sit | ActKind::Tv | ActKind::Sleep => Anim::Sit,
            ActKind::Squat => Anim::Squat,
            ActKind::Punch => Anim::Punch,
            ActKind::Run => Anim::Run,
        };
        let t = if a.kind == ActKind::Punch { a.t % 0.5 } else { a.t };
        return (anim, t);
    }
    if g.talk.is_some() {
        return (Anim::Talk, g.time);
    }
    if g.punch_t > 0.0 {
        return (Anim::Punch, 0.5 - g.punch_t);
    }
    if g.aiming && g.weapon > 0 {
        return (Anim::Aim, g.time);
    }
    if g.player.moving > 1.2 {
        (Anim::Run, g.player.walk / 4.2)
    } else if g.player.moving > 0.1 {
        (Anim::Walk, g.player.walk / 1.45)
    } else {
        (Anim::Idle, g.time)
    }
}

pub fn build_scene(si: &SceneInput) -> Scene {
    let g = si.game;
    let mut op = MeshBuilder::default();
    let mut tr = MeshBuilder::default();
    let mut insts: Vec<Inst> = Vec::new();
    let mut chars: Vec<CInst> = Vec::new();
    let mut cars: Vec<CarInst> = Vec::new();
    let near = |p: Pos2| (p - si.cam).length() < 320.0;
    let white = [1.0, 1.0, 1.0];
    for (i, v) in g.vehicles.iter().enumerate() {
        if !near(v.pos) {
            continue;
        }
        let base = at(v.pos, v.angle);
        let player_in = g.player.vehicle == Some(i) && g.enter_t <= 0.0;
        if v.kind == VKind::Moto {
            insts.push(Inst { model: M_MOTO, mat: base, phase: 0.0, moving: 0.0, seated: false, tint: if v.dead { [0.25, 0.22, 0.2] } else { white }, aim: false });
            if v.driver != Driver::None && !v.dead && (player_in || g.player.vehicle != Some(i)) {
                let who = if g.player.vehicle == Some(i) { C_BILA } else { crate::people::model_of(v.person) };
                chars.push(CInst { model: who, mat: base * Mat4::from_translation(Vec3::new(-0.3, 0.12, 0.0)), anim: Anim::Ride, t: 0.0 });
            }
        } else {
            let paint = if v.dead { [0.08, 0.07, 0.07] } else { let c = rgba(v.color, 1.0); [c[0], c[1], c[2]] };
            let steer = if player_in { si.steer } else { 0.0 };
            cars.push(CarInst { mesh: car_mesh(v.kind), mat: base, paint, spin: g.time * v.speed() / 0.34, steer });
            if v.driver != Driver::None && !v.dead && (player_in || g.player.vehicle != Some(i)) {
                let who = if g.player.vehicle == Some(i) { C_BILA } else { crate::people::model_of(v.person) };
                // seated low behind the wheel so the head stays under the roof
                let seat = if v.kind == VKind::Minibus { Vec3::new(1.85, 0.05, -0.45) } else { Vec3::new(-0.05, -0.5, -0.38) };
                chars.push(CInst { model: who, mat: base * Mat4::from_translation(seat), anim: Anim::Drive, t: 0.0 });
            }
        }
        if v.dead {
            let fl = (g.time * 13.0 + i as f32).sin() * 0.3 + 1.0;
            tr.cuboid(&Mat4::from_translation(v3(v.pos, 1.6 * fl)), Vec3::new(0.5, 0.6 * fl, 0.5), [1.0, 0.5, 0.1, 0.6], L_EMISSIVE);
        }
        if si.chase_tag.is_some() && Some(v.tag) == si.chase_tag {
            let bob = (g.time * 3.0).sin() * 0.3;
            let m = Mat4::from_translation(v3(v.pos, 4.0 + bob)) * Mat4::from_rotation_y(g.time * 2.0) * Mat4::from_rotation_z(FRAC_PI_4);
            op.cuboid(&m, Vec3::new(0.45, 0.45, 0.45), [1.0, 0.25, 0.2, 1.0], L_EMISSIVE);
        }
    }
    for (i, p) in g.peds.iter().enumerate() {
        if !near(p.pos) {
            continue;
        }
        let model = crate::people::model_of(p.person);
        if p.down > 0.0 {
            let m = Mat4::from_translation(v3(p.pos, 0.2)) * Mat4::from_rotation_y(-p.angle) * Mat4::from_rotation_z(FRAC_PI_2);
            chars.push(CInst { model, mat: m, anim: Anim::Idle, t: 0.0 });
            continue;
        }
        let talking = g.talk.as_ref().is_some_and(|t| t.ped == i);
        let (anim, speed) = if talking { (Anim::Talk, 1.0) } else if p.flee > 0.0 { (Anim::Run, 1.0) } else { (Anim::Walk, 1.0) };
        chars.push(CInst { model, mat: at(p.pos, p.angle), anim, t: g.time * speed + i as f32 * 0.37 });
    }
    for r in si.remotes {
        let pos = Pos2::new(r.x, r.y);
        if !near(pos) {
            continue;
        }
        let c = Color32::from_rgb(r.color[0], r.color[1], r.color[2]);
        if r.kind == 0 {
            chars.push(CInst { model: C_JEUNE, mat: at(pos, r.angle), anim: Anim::Walk, t: g.time });
        } else if VKind::from_net(r.kind) == VKind::Moto {
            insts.push(Inst { model: M_MOTO, mat: at(pos, r.angle), phase: 0.0, moving: 0.0, seated: false, tint: white, aim: false });
            chars.push(CInst { model: C_JEUNE, mat: at(pos, r.angle) * Mat4::from_translation(Vec3::new(-0.3, 0.2, 0.0)), anim: Anim::Ride, t: 0.0 });
        } else {
            let k = VKind::from_net(r.kind);
            let cc = rgba(c, 1.0);
            cars.push(CarInst { mesh: car_mesh(k), mat: at(pos, r.angle), paint: [cc[0], cc[1], cc[2]], spin: g.time * 20.0, steer: 0.0 });
        }
        op.cuboid(&Mat4::from_translation(v3(pos, 3.2)), Vec3::new(0.2, 0.2, 0.2), rgba(c, 1.0), L_EMISSIVE);
    }
    // Bila on foot (or getting in / out)
    let on_foot = g.player.vehicle.is_none() || g.enter_t > 0.0;
    if on_foot {
        let (anim, t) = if g.enter_t > 0.0 { (Anim::Walk, g.time) } else { bila_anim(g) };
        let pos = g.player.pos;
        let mut base = at(pos, g.player.angle);
        if let Some(a) = &g.activity {
            if a.kind == ActKind::Sleep {
                base = Mat4::from_translation(v3(pos, 0.72)) * Mat4::from_rotation_y(-g.player.angle) * Mat4::from_rotation_z(FRAC_PI_2);
            } else if matches!(a.kind, ActKind::Eat | ActKind::Sit | ActKind::Tv) {
                base = base * Mat4::from_translation(Vec3::new(0.0, -0.42, 0.0));
            }
        }
        chars.push(CInst { model: C_BILA, mat: base, anim, t });
        // the weapon held in the right hand (bone from the Blender rig)
        if g.weapon > 0 && g.activity.is_none() {
            if let Some((hand, hm)) = skinned::hand_world(C_BILA, base, anim, t, "handR") {
                let metal = rgb(38, 38, 42);
                let len = if g.weapon == 2 { 0.42 } else { 0.11 };
                let fwd = if g.aiming { Vec3::new(g.player.angle.cos(), 0.0, g.player.angle.sin()) } else { hm.transform_vector3(Vec3::new(0.0, -1.0, 0.0)).normalize_or_zero() };
                let yaw = fwd.z.atan2(fwd.x);
                let pitch = fwd.y.asin();
                let m = Mat4::from_translation(hand + fwd * len * 0.5) * Mat4::from_rotation_y(-yaw) * Mat4::from_rotation_z(pitch);
                op.cuboid(&m, Vec3::new(len, 0.035, 0.022), metal, L_PLAIN);
                op.cuboid(&(m * Mat4::from_translation(Vec3::new(-len * 0.6, -0.07, 0.0)) * Mat4::from_rotation_z(-0.25)), Vec3::new(0.03, 0.06, 0.02), metal, L_PLAIN);
                if g.weapon == 2 {
                    op.cuboid(&(m * Mat4::from_translation(Vec3::new(-len * 0.2, -0.08, 0.0)) * Mat4::from_rotation_z(0.3)), Vec3::new(0.025, 0.07, 0.02), rgb(70, 45, 25), L_PLAIN);
                    op.cuboid(&(m * Mat4::from_translation(Vec3::new(-len * 1.1, -0.02, 0.0))), Vec3::new(0.12, 0.045, 0.025), rgb(70, 45, 25), L_PLAIN);
                }
            }
        }
    }
    // food on the table while eating
    if let Some(a) = &g.activity {
        if a.kind == ActKind::Eat {
            let f = Vec3::new(g.player.angle.cos(), 0.0, g.player.angle.sin());
            let p = v3(g.player.pos, 0.78) + f * 0.55;
            op.cylinder(p, 0.15, 0.15, 0.02, 20, rgb(245, 245, 240), L_PLAIN, true);
            op.cylinder(p + Vec3::Y * 0.02, 0.1, 0.06, 0.05, 12, rgb(200, 150, 60), L_PLAIN, true);
        }
    }
    // gunfire effects
    for fx in &g.fx {
        match fx.kind {
            0 => {
                let d = fx.b - fx.a;
                let len = d.length().max(0.1);
                let ang = d.y.atan2(d.x);
                let mid = fx.a + d * 0.5;
                let m = Mat4::from_translation(v3(mid, 1.38)) * Mat4::from_rotation_y(-ang);
                tr.cuboid(&m, Vec3::new(len * 0.5, 0.012, 0.012), [1.0, 0.9, 0.5, 0.75], L_EMISSIVE);
            }
            1 => {
                let d = (fx.b - fx.a).normalized();
                let m = Mat4::from_translation(v3(fx.a + d * 0.75, 1.4)) * Mat4::from_rotation_y(fx.t * 90.0);
                tr.cuboid(&m, Vec3::new(0.14, 0.09, 0.09), [1.0, 0.75, 0.25, 0.9], L_EMISSIVE);
            }
            2 => {
                let m = Mat4::from_translation(v3(fx.a, 1.0)) * Mat4::from_rotation_y(fx.t * 30.0);
                tr.cuboid(&m, Vec3::splat(0.08 + (0.25 - fx.t) * 0.6), [0.75, 0.7, 0.62, 0.55], L_EMISSIVE);
            }
            _ => {
                tr.ground_rect(fx.a, 0.21, 0.6, 0.7, 0.5, [0.32, 0.02, 0.02, 0.8 * (fx.t / 30.0).min(1.0)], L_EMISSIVE);
            }
        }
    }
    let pulse = (g.time * 3.0).sin() * 0.5 + 0.5;
    // gun shops and doors
    for s in &g.shops {
        if near(*s) {
            tr.cylinder(v3(*s, 0.0), 2.5, 2.5, 2.5, 20, [1.0, 0.25, 0.2, 0.25], L_EMISSIVE, false);
            let m = Mat4::from_translation(v3(*s, 3.2 + (g.time * 3.0).sin() * 0.2)) * Mat4::from_rotation_y(g.time);
            op.cuboid(&m, Vec3::new(0.45, 0.12, 0.08), [1.0, 0.3, 0.25, 1.0], L_EMISSIVE);
            op.cuboid(&(m * Mat4::from_translation(Vec3::new(0.25, -0.2, 0.0))), Vec3::new(0.1, 0.2, 0.07), [1.0, 0.3, 0.25, 1.0], L_EMISSIVE);
        }
    }
    if g.inside.is_none() {
        for d in &g.doors {
            if near(d.pos) && (d.pos - si.cam).length() > 2.5 {
                tr.cylinder(v3(d.pos, 0.0), 1.6, 1.6, 2.2, 20, [0.75, 0.45, 1.0, 0.28], L_EMISSIVE, false);
                let m = Mat4::from_translation(v3(d.pos, 2.9 + pulse * 0.2)) * Mat4::from_rotation_y(g.time * 1.5);
                op.cuboid(&m, Vec3::new(0.35, 0.35, 0.06), [0.8, 0.5, 1.0, 1.0], L_EMISSIVE);
            }
        }
    }
    if let Some(t) = si.marker {
        tr.cylinder(v3(t, 0.0), 4.0 + pulse * 0.4, 4.0 + pulse * 0.4, 6.0, 24, [0.25, 0.8, 1.0, 0.3], L_EMISSIVE, false);
    }
    if let Some(m) = &g.mission {
        tr.cylinder(v3(m.to, 0.0), 4.0 + pulse * 0.4, 4.0 + pulse * 0.4, 6.0, 24, [0.25, 0.8, 1.0, 0.3], L_EMISSIVE, false);
    }
    if g.mission.is_none() && !g.story {
        for s in &g.spots {
            if near(*s) {
                tr.cylinder(v3(*s, 0.0), 6.0, 6.0, 3.0, 24, [1.0, 0.82, 0.1, 0.28], L_EMISSIVE, false);
            }
        }
    }
    for (p, owned) in si.biz_markers {
        if near(*p) {
            let c = if *owned { [0.3, 1.0, 0.45, 0.3] } else { [0.3, 1.0, 0.45, 0.2] };
            tr.cylinder(v3(*p, 0.0), 3.0, 3.0, 2.5, 20, c, L_EMISSIVE, false);
            let m = Mat4::from_translation(v3(*p, 3.4 + pulse * 0.3)) * Mat4::from_rotation_y(g.time);
            op.cuboid(&m, Vec3::new(0.5, 0.5, 0.08), [0.3, 1.0, 0.45, 1.0], L_EMISSIVE);
        }
    }
    if g.inside == Some(statics::I_GARAGE) {
        if let Some((k, col)) = g.garage_car {
            let a = statics::anchor(statics::I_GARAGE);
            if k == VKind::Moto {
                insts.push(Inst { model: M_MOTO, mat: at(Pos2::new(a.x + 0.5, a.y), 0.0), phase: 0.0, moving: 0.0, seated: false, tint: white, aim: false });
            } else {
                let c = rgba(col, 1.0);
                cars.push(CarInst { mesh: car_mesh(k), mat: Mat4::from_translation(Vec3::new(a.x + 0.8, 0.02, a.y)), paint: [c[0], c[1], c[2]], spin: 0.0, steer: 0.0 });
            }
        }
    }
    // interaction spots inside
    if let Some(ii) = g.inside {
        let it = &statics::interiors()[ii];
        let a = statics::anchor(ii);
        for s in &it.spots {
            let p = Pos2::new(a.x + s.x, a.y + s.z);
            if (p - g.player.pos).length() > 5.0 || g.activity.is_some() {
                continue;
            }
            tr.cylinder(v3(p, 0.0), 0.45, 0.45, 0.05, 16, [1.0, 0.85, 0.3, 0.55], L_EMISSIVE, false);
        }
    }
    Scene { opaque: op.v, translucent: tr.v, insts, chars, cars }
}

// ---------------------------------------------------------------- GL side

struct Gpu {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    count: i32,
}

pub struct R3D {
    prog: glow::Program,
    shadow_prog: glow::Program,
    sky_prog: glow::Program,
    tex: glow::Texture,
    nrm: glow::Texture,
    sky: glow::Texture,
    shadow_tex: glow::Texture,
    shadow_fb: glow::Framebuffer,
    chunks: HashMap<(i32, i32), Gpu>,
    dynamic: Gpu,
    trans: Gpu,
    map: Arc<Map>,
    models: Models,
    skinned: Skinned,
    vehicles: Vec<Gpu>,
    rooms: Vec<Gpu>,
    detail: Gpu,
    detail_at: Option<Pos2>,
}

const VS: &str = r#"
layout(location=0) in vec3 a_pos;
layout(location=1) in vec3 a_nrm;
layout(location=2) in vec4 a_col;
layout(location=3) in vec2 a_uv;
layout(location=4) in float a_layer;
uniform mat4 u_vp;
uniform mat4 u_lvp;
uniform mat4 u_model;
uniform vec3 u_paint;
out vec3 v_pos; out vec3 v_nrm; out vec4 v_col; out vec2 v_uv; out float v_layer; out vec4 v_sc;
void main() {
    vec4 wp = u_model * vec4(a_pos, 1.0);
    v_pos = wp.xyz; v_nrm = normalize(mat3(u_model) * a_nrm); v_uv = a_uv; v_layer = a_layer;
    bool paint = a_col.r > 0.99 && a_col.g < 0.01 && a_col.b > 0.99;
    v_col = paint ? vec4(u_paint, a_col.a) : a_col;
    v_sc = u_lvp * vec4(wp.xyz + v_nrm * 0.08, 1.0);
    gl_Position = u_vp * wp;
}
"#;

const FS: &str = r#"
precision highp float;
precision highp sampler2DArray;
precision highp sampler2DShadow;
uniform sampler2DArray u_tex;
uniform sampler2DArray u_nrm;
uniform sampler2DShadow u_shadow;
uniform vec3 u_cam; uniform vec3 u_sun; uniform vec3 u_sun_col; uniform vec3 u_amb; uniform vec3 u_fogc;
uniform float u_fog; uniform float u_night; uniform float u_texel;
uniform vec3 u_focus;
in vec3 v_pos; in vec3 v_nrm; in vec4 v_col; in vec2 v_uv; in float v_layer; in vec4 v_sc;
out vec4 frag;
vec3 fogit(vec3 c) {
    float d = length(v_pos - u_cam);
    float f = clamp((d - u_fog * 0.25) / (u_fog * 0.75), 0.0, 1.0);
    f = f * f * (3.0 - 2.0 * f);
    return mix(c, u_fogc, f);
}
float shadowing() {
    vec3 sc = v_sc.xyz / v_sc.w * 0.5 + 0.5;
    if (sc.x < 0.0 || sc.x > 1.0 || sc.y < 0.0 || sc.y > 1.0 || sc.z > 1.0) return 1.0;
    float s = 0.0;
    for (int x = -1; x <= 1; x++)
        for (int y = -1; y <= 1; y++)
            s += texture(u_shadow, vec3(sc.xy + vec2(float(x), float(y)) * u_texel, sc.z - 0.0012));
    return s / 9.0;
}
vec3 perturb(vec3 N, vec3 nm) {
    vec3 dp1 = dFdx(v_pos); vec3 dp2 = dFdy(v_pos);
    vec2 du1 = dFdx(v_uv); vec2 du2 = dFdy(v_uv);
    vec3 dp2perp = cross(dp2, N); vec3 dp1perp = cross(N, dp1);
    vec3 T = dp2perp * du1.x + dp1perp * du2.x;
    vec3 B = dp2perp * du1.y + dp1perp * du2.y;
    float m = max(dot(T, T), dot(B, B));
    if (m < 1e-12) return N;
    float inv = inversesqrt(m);
    return normalize(mat3(T * inv, B * inv, N) * nm);
}
void main() {
    float L = floor(v_layer + 0.5);
    // foliage and trunks between the camera and Bila become see-through (dithered)
    if (L == 9.0 || L == 12.0 || L == 4.0 || L == 19.0) {
        vec3 ab = u_focus - u_cam;
        float k = clamp(dot(v_pos - u_cam, ab) / max(dot(ab, ab), 1e-4), 0.0, 1.0);
        float d = length(u_cam + ab * k - v_pos);
        if (d < 2.6) {
            float dither = fract(sin(dot(floor(gl_FragCoord.xy), vec2(12.9898, 78.233))) * 43758.5453);
            if (dither > (d - 1.2) / 1.4) discard;
        }
    }
    if (L > 40.5) { frag = vec4(0.0, 0.0, 0.0, v_col.a * (1.0 - u_night * 0.6)); return; }
    if (L > 39.5) { frag = vec4(fogit(v_col.rgb * (1.0 + u_night * 0.5)), v_col.a); return; }
    vec4 t = texture(u_tex, vec3(v_uv, L));
    bool windowed = v_col.a > 0.5 && v_col.a < 0.99;
    if (windowed) {
        vec4 w = texture(u_tex, vec3(v_uv, 3.0));
        if (w.a < 0.5) t = vec4(w.rgb, 0.0);
    }
    vec3 N = normalize(v_nrm);
    vec3 nm = texture(u_nrm, vec3(v_uv, L)).xyz * 2.0 - 1.0;
    if (L == 4.0) nm.xy *= 1.6;
    vec3 n = perturb(N, normalize(nm));
    float diff = max(dot(n, u_sun), 0.0);
    float sh = shadowing();
    float ao = (abs(N.y) < 0.5) ? mix(0.62, 1.0, clamp(v_pos.y / 2.2, 0.0, 1.0)) : 1.0;
    vec3 sky_amb = u_amb * (0.8 + 0.25 * n.y) + vec3(0.06, 0.04, 0.02) * (1.0 - n.y) * (1.0 - u_night);
    vec3 albedo = v_col.rgb * t.rgb;
    vec3 col = albedo * (sky_amb * ao + u_sun_col * diff * sh);
    float ks = 0.0; float shin = 32.0;
    if (L == 4.0) { ks = 0.35; shin = 24.0; }
    if (L == 11.0) { ks = 0.6; shin = 80.0; }
    if (L == 6.0) { ks = 0.9; shin = 120.0; }
    if (L == 1.0) { ks = 0.06; shin = 16.0; }
    if (L == 8.0 && v_col.a > 0.995) { ks = 0.0; }
    if (ks > 0.0) {
        vec3 v = normalize(u_cam - v_pos);
        vec3 h = normalize(v + u_sun);
        col += u_sun_col * ks * pow(max(dot(n, h), 0.0), shin) * sh;
    }
    if (((L == 3.0 || windowed) && t.a < 0.5) || (L == 11.0 && t.b > t.r * 1.05)) {
        float r = fract(sin(dot(floor(v_uv) + floor(v_pos.xz * 0.07), vec2(12.9898, 78.233))) * 43758.5453);
        if (r < 0.55) col += vec3(1.0, 0.78, 0.45) * u_night * 0.9;
        col += u_fogc * 0.1 * (1.0 - u_night);
    }
    float lum = dot(col, vec3(0.299, 0.587, 0.114));
    col = mix(vec3(lum), col, 1.12);
    frag = vec4(fogit(col), v_col.a);
}
"#;

const SVS: &str = r#"
layout(location=0) in vec3 a_pos;
layout(location=4) in float a_layer;
uniform mat4 u_lvp;
uniform mat4 u_model;
out float v_layer;
void main() { v_layer = a_layer; gl_Position = u_lvp * u_model * vec4(a_pos, 1.0); }
"#;
const SFS: &str = r#"
precision highp float;
in float v_layer;
out vec4 frag;
void main() { if (v_layer > 39.5) discard; frag = vec4(1.0); }
"#;

const SKY_VS: &str = r#"
out vec2 v_ndc;
void main() {
    vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2)) * 2.0 - 1.0;
    v_ndc = p;
    gl_Position = vec4(p, 0.9999, 1.0);
}
"#;
const SKY_FS: &str = r#"
precision highp float;
precision highp sampler2DArray;
uniform sampler2DArray u_sky;
uniform mat4 u_inv;
uniform vec3 u_cam; uniform vec3 u_w; uniform vec3 u_sun; uniform vec3 u_sun_col; uniform vec3 u_fogc;
in vec2 v_ndc;
out vec4 frag;
void main() {
    vec4 p = u_inv * vec4(v_ndc, 1.0, 1.0);
    vec3 d = normalize(p.xyz / p.w - u_cam);
    float u = atan(d.z, d.x) / 6.2831853 + 0.5;
    float v = acos(clamp(max(d.y, 0.02), -1.0, 1.0)) / 3.14159265;
    vec3 c = texture(u_sky, vec3(u, v, 0.0)).rgb * u_w.x
           + texture(u_sky, vec3(u, v, 1.0)).rgb * u_w.y
           + texture(u_sky, vec3(u, v, 2.0)).rgb * u_w.z;
    float sd = max(dot(d, u_sun), 0.0);
    c += u_sun_col * (pow(sd, 900.0) * 4.0 + pow(sd, 12.0) * 0.18) * (1.0 - u_w.z);
    float h = 1.0 - clamp(d.y * 4.0, 0.0, 1.0);
    c = mix(c, u_fogc, h * 0.75);
    frag = vec4(c, 1.0);
}
"#;

unsafe fn compile(gl: &glow::Context, vs: &str, fs: &str) -> Result<glow::Program, String> {
    let mut last = String::new();
    for header in ["#version 330 core\n", "#version 300 es\n"] {
        let prog = gl.create_program()?;
        let mut ok = true;
        let mut shaders = Vec::new();
        for (ty, src) in [(glow::VERTEX_SHADER, vs), (glow::FRAGMENT_SHADER, fs)] {
            let s = gl.create_shader(ty)?;
            gl.shader_source(s, &format!("{header}{src}"));
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
    for (loc, size, off) in [(0u32, 3i32, 0i32), (1, 3, 3), (2, 4, 6), (3, 2, 10), (4, 1, 12)] {
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

unsafe fn array_texture(gl: &glow::Context, w: usize, h: usize, layers: usize, data: &[u8], repeat_v: bool) -> Result<glow::Texture, String> {
    let tex = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(tex));
    gl.tex_image_3d(glow::TEXTURE_2D_ARRAY, 0, glow::RGBA8 as i32, w as i32, h as i32, layers as i32, 0, glow::RGBA, glow::UNSIGNED_BYTE, glow::PixelUnpackData::Slice(Some(data)));
    gl.generate_mipmap(glow::TEXTURE_2D_ARRAY);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MIN_FILTER, glow::LINEAR_MIPMAP_LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
    gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_T, if repeat_v { glow::REPEAT } else { glow::CLAMP_TO_EDGE } as i32);
    // anisotropic filtering when the driver offers it (GL_TEXTURE_MAX_ANISOTROPY)
    gl.tex_parameter_f32(glow::TEXTURE_2D_ARRAY, 0x84FE, 8.0);
    let _ = gl.get_error();
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, None);
    Ok(tex)
}

impl R3D {
    unsafe fn draw_statics(&self, gl: &glow::Context, prog: glow::Program, fd: &FrameData) {
        let lm = gl.get_uniform_location(prog, "u_model");
        let lp = gl.get_uniform_location(prog, "u_paint");
        if let Some(i) = fd.interior {
            if let Some(g) = self.rooms.get(i) {
                let a = statics::anchor(i);
                gl.uniform_matrix_4_f32_slice(lm.as_ref(), false, &Mat4::from_translation(Vec3::new(a.x, 0.02, a.y)).to_cols_array());
                gl.bind_vertex_array(Some(g.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, g.count);
            }
        }
        for c in &fd.cars {
            let (Some(body), Some(wheel)) = (self.vehicles.get(c.mesh), self.vehicles.get(statics::V_WHEEL)) else { continue };
            gl.uniform_3_f32(lp.as_ref(), c.paint[0], c.paint[1], c.paint[2]);
            gl.uniform_matrix_4_f32_slice(lm.as_ref(), false, &c.mat.to_cols_array());
            gl.bind_vertex_array(Some(body.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, body.count);
            gl.bind_vertex_array(Some(wheel.vao));
            for (k, w) in statics::wheels(c.mesh).iter().enumerate() {
                let steer = if k < 2 { c.steer } else { 0.0 };
                let flip = if w[2] < 0.0 { Mat4::from_rotation_y(std::f32::consts::PI) } else { Mat4::IDENTITY };
                let m = c.mat * Mat4::from_translation(Vec3::new(w[0], w[1], w[2])) * Mat4::from_rotation_y(-steer) * flip * Mat4::from_rotation_z(if w[2] < 0.0 { c.spin } else { -c.spin });
                gl.uniform_matrix_4_f32_slice(lm.as_ref(), false, &m.to_cols_array());
                gl.draw_arrays(glow::TRIANGLES, 0, wheel.count);
            }
        }
        gl.uniform_matrix_4_f32_slice(lm.as_ref(), false, &Mat4::IDENTITY.to_cols_array());
        gl.uniform_3_f32(lp.as_ref(), 1.0, 0.0, 1.0);
    }

    pub fn new(gl: &glow::Context, map: Arc<Map>) -> Result<R3D, String> {
        unsafe {
            let prog = compile(gl, VS, FS)?;
            let shadow_prog = compile(gl, SVS, SFS)?;
            let sky_prog = compile(gl, SKY_VS, SKY_FS)?;
            let (color, normal) = build_textures();
            let tex = array_texture(gl, TEX, TEX, LAYERS, &color, true)?;
            let nrm = array_texture(gl, TEX, TEX, LAYERS, &normal, true)?;
            let mut sky_data = Vec::with_capacity(SKY_W * SKY_H * 4 * 3);
            for f in SKY_FILES {
                let img = image::load_from_memory(f).map(|i| i.to_rgba8()).map_err(|e| e.to_string())?;
                let img = if img.width() as usize != SKY_W || img.height() as usize != SKY_H {
                    image::imageops::resize(&img, SKY_W as u32, SKY_H as u32, image::imageops::FilterType::Triangle)
                } else {
                    img
                };
                sky_data.extend_from_slice(&img.into_raw());
            }
            let sky = array_texture(gl, SKY_W, SKY_H, 3, &sky_data, false)?;

            let shadow_tex = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(shadow_tex));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::DEPTH_COMPONENT24 as i32, SHADOW_SIZE, SHADOW_SIZE, 0, glow::DEPTH_COMPONENT, glow::UNSIGNED_INT, glow::PixelUnpackData::Slice(None));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_COMPARE_MODE, glow::COMPARE_REF_TO_TEXTURE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_COMPARE_FUNC, glow::LEQUAL as i32);
            let shadow_fb = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(shadow_fb));
            gl.framebuffer_texture_2d(glow::FRAMEBUFFER, glow::DEPTH_ATTACHMENT, glow::TEXTURE_2D, Some(shadow_tex), 0);
            gl.draw_buffer(glow::NONE);
            gl.read_buffer(glow::NONE);
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!("framebuffer des ombres incomplet ({status:#x})"));
            }
            let models = Models::new(gl, |vs, fs| compile(gl, vs, fs))?;
            let skinned = Skinned::new(gl, |vs, fs| compile(gl, vs, fs))?;
            let mut vehicles = Vec::new();
            for m in statics::vehicles() {
                let mut gpu = make_gpu(gl);
                upload(gl, &mut gpu, m, glow::STATIC_DRAW);
                vehicles.push(gpu);
            }
            let mut rooms = Vec::new();
            for it in statics::interiors() {
                let mut gpu = make_gpu(gl);
                upload(gl, &mut gpu, &it.mesh, glow::STATIC_DRAW);
                rooms.push(gpu);
            }
            Ok(R3D { prog, shadow_prog, sky_prog, tex, nrm, sky, shadow_tex, shadow_fb, chunks: HashMap::new(), dynamic: make_gpu(gl), trans: make_gpu(gl), map, models, skinned, vehicles, rooms, detail: make_gpu(gl), detail_at: None })
        }
    }

    pub fn paint(&mut self, gl: &glow::Context, info: &egui::PaintCallbackInfo, fd: &FrameData) {
        unsafe {
            let ccx = (fd.cam.x / CHUNK).floor() as i32;
            let ccz = (fd.cam.z / CHUNK).floor() as i32;
            let mut built = 0;
            for ring in 0..=2i32 {
                for dz in -ring..=ring {
                    for dx in -ring..=ring {
                        if dx.abs() != ring && dz.abs() != ring {
                            continue;
                        }
                        let key = (ccx + dx, ccz + dz);
                        if !self.chunks.contains_key(&key) && (ring <= 1 || built < 1) {
                            let mesh = build_chunk(&self.map, key.0, key.1);
                            let mut gpu = make_gpu(gl);
                            upload(gl, &mut gpu, &mesh, glow::STATIC_DRAW);
                            self.chunks.insert(key, gpu);
                            built += 1;
                        }
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
            // Blender kit around the camera (windows, doors, shops, balconies...)
            let here = Pos2::new(fd.cam.x, fd.cam.z);
            if self.detail_at.map_or(true, |p| (p - here).length() > 35.0) {
                let mesh = crate::kit::detail_mesh(&self.map, here, 140.0);
                upload(gl, &mut self.detail, &mesh, glow::STATIC_DRAW);
                self.detail_at = Some(here);
            }
            upload(gl, &mut self.dynamic, &fd.opaque, glow::STREAM_DRAW);
            upload(gl, &mut self.trans, &fd.translucent, glow::STREAM_DRAW);

            // shadow pass
            gl.disable(glow::SCISSOR_TEST);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.shadow_fb));
            gl.viewport(0, 0, SHADOW_SIZE, SHADOW_SIZE);
            gl.depth_mask(true);
            gl.clear_depth_f32(1.0);
            gl.clear(glow::DEPTH_BUFFER_BIT);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::POLYGON_OFFSET_FILL);
            gl.polygon_offset(1.5, 2.0);
            gl.use_program(Some(self.shadow_prog));
            gl.uniform_matrix_4_f32_slice(gl.get_uniform_location(self.shadow_prog, "u_lvp").as_ref(), false, &fd.light_vp.to_cols_array());
            gl.uniform_matrix_4_f32_slice(gl.get_uniform_location(self.shadow_prog, "u_model").as_ref(), false, &Mat4::IDENTITY.to_cols_array());
            let fcx = (fd.focus.x / CHUNK).floor() as i32;
            let fcz = (fd.focus.z / CHUNK).floor() as i32;
            for (k, c) in &self.chunks {
                if (k.0 - fcx).abs() <= 1 && (k.1 - fcz).abs() <= 1 && c.count > 0 {
                    gl.bind_vertex_array(Some(c.vao));
                    gl.draw_arrays(glow::TRIANGLES, 0, c.count);
                }
            }
            if self.dynamic.count > 0 {
                gl.bind_vertex_array(Some(self.dynamic.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, self.dynamic.count);
            }
            if self.detail.count > 0 {
                gl.bind_vertex_array(Some(self.detail.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, self.detail.count);
            }
            let mf = models::Frame {
                vp: &fd.light_vp,
                lvp: &fd.light_vp,
                cam: fd.cam,
                sun: fd.sun,
                sun_col: fd.sun_col,
                ambient: fd.ambient,
                fog_col: fd.sky,
                fog: fd.fog,
                texel: 1.0 / SHADOW_SIZE as f32,
            };
            gl.use_program(Some(self.shadow_prog));
            self.draw_statics(gl, self.shadow_prog, fd);
            self.models.draw(gl, &mf, &fd.insts, true, self.shadow_tex);
            self.skinned.draw(gl, &mf, &fd.chars, true, self.shadow_tex);
            gl.disable(glow::POLYGON_OFFSET_FILL);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            // main pass
            let vp = info.viewport_in_pixels();
            let clip = info.clip_rect_in_pixels();
            gl.viewport(vp.left_px, vp.from_bottom_px, vp.width_px, vp.height_px);
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(clip.left_px, clip.from_bottom_px, clip.width_px, clip.height_px);
            gl.clear_color(fd.sky.x, fd.sky.y, fd.sky.z, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.use_program(Some(self.sky_prog));
            let sl = |n: &str| gl.get_uniform_location(self.sky_prog, n);
            gl.uniform_matrix_4_f32_slice(sl("u_inv").as_ref(), false, &fd.view_proj.inverse().to_cols_array());
            gl.uniform_3_f32(sl("u_cam").as_ref(), fd.cam.x, fd.cam.y, fd.cam.z);
            gl.uniform_3_f32(sl("u_w").as_ref(), fd.sky_w.x, fd.sky_w.y, fd.sky_w.z);
            gl.uniform_3_f32(sl("u_sun").as_ref(), fd.sun.x, fd.sun.y, fd.sun.z);
            gl.uniform_3_f32(sl("u_sun_col").as_ref(), fd.sun_col.x, fd.sun_col.y, fd.sun_col.z);
            gl.uniform_3_f32(sl("u_fogc").as_ref(), fd.sky.x, fd.sky.y, fd.sky.z);
            gl.active_texture(glow::TEXTURE3);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(self.sky));
            gl.uniform_1_i32(sl("u_sky").as_ref(), 3);
            gl.bind_vertex_array(Some(self.dynamic.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.depth_mask(true);
            gl.enable(glow::DEPTH_TEST);

            gl.use_program(Some(self.prog));
            let loc = |n: &str| gl.get_uniform_location(self.prog, n);
            gl.uniform_matrix_4_f32_slice(loc("u_vp").as_ref(), false, &fd.view_proj.to_cols_array());
            gl.uniform_matrix_4_f32_slice(loc("u_lvp").as_ref(), false, &fd.light_vp.to_cols_array());
            gl.uniform_matrix_4_f32_slice(loc("u_model").as_ref(), false, &Mat4::IDENTITY.to_cols_array());
            gl.uniform_3_f32(loc("u_paint").as_ref(), 1.0, 0.0, 1.0);
            gl.uniform_3_f32(loc("u_cam").as_ref(), fd.cam.x, fd.cam.y, fd.cam.z);
            gl.uniform_3_f32(loc("u_sun").as_ref(), fd.sun.x, fd.sun.y, fd.sun.z);
            gl.uniform_3_f32(loc("u_sun_col").as_ref(), fd.sun_col.x, fd.sun_col.y, fd.sun_col.z);
            gl.uniform_3_f32(loc("u_amb").as_ref(), fd.ambient.x, fd.ambient.y, fd.ambient.z);
            gl.uniform_3_f32(loc("u_fogc").as_ref(), fd.sky.x, fd.sky.y, fd.sky.z);
            gl.uniform_1_f32(loc("u_fog").as_ref(), fd.fog);
            gl.uniform_1_f32(loc("u_night").as_ref(), fd.night);
            gl.uniform_1_f32(loc("u_texel").as_ref(), 1.0 / SHADOW_SIZE as f32);
            gl.uniform_3_f32(loc("u_focus").as_ref(), fd.focus.x, fd.focus.y + 0.6, fd.focus.z);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(self.tex));
            gl.uniform_1_i32(loc("u_tex").as_ref(), 0);
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(self.nrm));
            gl.uniform_1_i32(loc("u_nrm").as_ref(), 1);
            gl.active_texture(glow::TEXTURE2);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.shadow_tex));
            gl.uniform_1_i32(loc("u_shadow").as_ref(), 2);

            for (k, c) in &self.chunks {
                if (k.0 - ccx).abs() <= 2 && (k.1 - ccz).abs() <= 2 && c.count > 0 {
                    gl.bind_vertex_array(Some(c.vao));
                    gl.draw_arrays(glow::TRIANGLES, 0, c.count);
                }
            }
            gl.bind_vertex_array(Some(self.dynamic.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, self.dynamic.count);
            if self.detail.count > 0 {
                gl.bind_vertex_array(Some(self.detail.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, self.detail.count);
            }

            let mf = models::Frame {
                vp: &fd.view_proj,
                lvp: &fd.light_vp,
                cam: fd.cam,
                sun: fd.sun,
                sun_col: fd.sun_col,
                ambient: fd.ambient,
                fog_col: fd.sky,
                fog: fd.fog,
                texel: 1.0 / SHADOW_SIZE as f32,
            };
            self.draw_statics(gl, self.prog, fd);
            self.models.draw(gl, &mf, &fd.insts, false, self.shadow_tex);
            self.skinned.draw(gl, &mf, &fd.chars, false, self.shadow_tex);
            // the translucent pass uses the main program again
            gl.use_program(Some(self.prog));

            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.depth_mask(false);
            gl.bind_vertex_array(Some(self.trans.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, self.trans.count);
            gl.depth_mask(true);

            gl.bind_vertex_array(None);
            gl.active_texture(glow::TEXTURE3);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, None);
            gl.active_texture(glow::TEXTURE2);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, None);
            gl.active_texture(glow::TEXTURE0);
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

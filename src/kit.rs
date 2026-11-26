//! Dressing the city's buildings with the Blender kit (blender/build_kit.py): 3D windows over the
//! painted ones, a door on the street side, shops with awnings along the avenues, balconies,
//! air conditioners, water tanks and satellite dishes. Built around the camera, rebuilt as it moves.

use crate::map::{Building, Map, Rng};
use crate::statics::ocs;
use eframe::egui::{self as egui, Pos2};
use glam::Vec3;
use std::collections::HashSet;
use std::sync::OnceLock;

const FENETRE: usize = 0;
const FENETRE_VOLETS: usize = 1;
const PORTE: usize = 2;
const PORTAIL: usize = 3;
const BOUTIQUE: usize = 4;
const BALCON: usize = 5;
const CLIM: usize = 6;
const CITERNE: usize = 7;
const PARABOLE: usize = 8;

fn pieces() -> &'static Vec<Vec<f32>> {
    static D: OnceLock<Vec<Vec<f32>>> = OnceLock::new();
    D.get_or_init(|| {
        vec![
            ocs(include_bytes!("../assets/kit/fenetre.ocs")),
            ocs(include_bytes!("../assets/kit/fenetre_volets.ocs")),
            ocs(include_bytes!("../assets/kit/porte.ocs")),
            ocs(include_bytes!("../assets/kit/portail.ocs")),
            ocs(include_bytes!("../assets/kit/boutique.ocs")),
            ocs(include_bytes!("../assets/kit/balcon.ocs")),
            ocs(include_bytes!("../assets/kit/clim.ocs")),
            ocs(include_bytes!("../assets/kit/citerne.ocs")),
            ocs(include_bytes!("../assets/kit/parabole.ocs")),
        ]
    })
}

const ACCENTS: [[f32; 3]; 8] = [
    [0.16, 0.36, 0.68],
    [0.14, 0.50, 0.30],
    [0.66, 0.16, 0.16],
    [0.86, 0.66, 0.16],
    [0.12, 0.56, 0.60],
    [0.45, 0.28, 0.18],
    [0.30, 0.30, 0.34],
    [0.55, 0.20, 0.50],
];

/// Copy a piece into `out`: x along `xd`, y up, z along `zd`, scaled by (sx, sy).
#[allow(clippy::too_many_arguments)]
fn place(out: &mut Vec<f32>, piece: usize, origin: Vec3, xd: Vec3, zd: Vec3, sx: f32, sy: f32, accent: [f32; 3]) {
    let v = &pieces()[piece];
    out.reserve(v.len());
    for c in v.chunks(13) {
        let p = origin + xd * (c[0] * sx) + Vec3::Y * (c[1] * sy) + zd * c[2];
        let n = (xd * (c[3] / sx) + Vec3::Y * (c[4] / sy) + zd * c[5]).normalize_or_zero();
        let paint = c[6] > 0.99 && c[7] < 0.01 && c[8] > 0.99;
        let col = if paint { [accent[0], accent[1], accent[2]] } else { [c[6], c[7], c[8]] };
        out.extend_from_slice(&[p.x, p.y, p.z, n.x, n.y, n.z, col[0], col[1], col[2], c[9], c[10], c[11], c[12]]);
    }
}

fn dress(map: &Map, b: &Building, out: &mut Vec<f32>) {
    if matches!(b.kind, 'u' | 'r' | 'm' | 'c') {
        return;
    }
    let glass = b.levels >= 5 && matches!(b.kind, 'o' | 'g' | 'h' | 'p' | 'x') && b.seed % 3 != 0;
    let n = b.pts.len();
    let h = b.height();
    let mut rng = Rng(b.seed as u64 * 2654435761 + 1);
    rng.next();
    let accent = ACCENTS[(b.seed / 7) as usize % ACCENTS.len()];
    let shutters = b.seed % 3 == 0;
    let area2: f32 = (0..n).map(|i| {
        let (a, c) = (b.pts[i], b.pts[(i + 1) % n]);
        a.x * c.y - c.x * a.y
    }).sum();
    let sign = if area2 >= 0.0 { 1.0 } else { -1.0 };
    // the wall that faces the street gets the door / the shops
    let mut street_edge = 0;
    let mut best = f32::MAX;
    let mut main_road = false;
    for i in 0..n {
        let (p0, p1) = (b.pts[i], b.pts[(i + 1) % n]);
        if (p1 - p0).length() < 3.0 {
            continue;
        }
        let mid = p0 + (p1 - p0) * 0.5;
        let nrm = egui::vec2((p1 - p0).y, -(p1 - p0).x).normalized() * sign;
        if let Some((ri, _, _, d)) = map.nearest_road(mid + nrm * 4.0, 40.0, 4) {
            if d < best {
                best = d;
                street_edge = i;
                main_road = map.roads[ri].kind <= 2;
            }
        }
    }
    let shops = main_road && best < 25.0 && b.levels <= 4 && b.seed % 5 < 3 && !glass;
    for i in 0..n {
        let (p0, p1) = (b.pts[i], b.pts[(i + 1) % n]);
        let e = p1 - p0;
        let len = e.length();
        if len < 3.0 {
            continue;
        }
        let t2 = e / len;
        let n2 = egui::vec2(e.y, -e.x) / len * sign;
        let xd = Vec3::new(t2.x, 0.0, t2.y);
        let zd = Vec3::new(n2.x, 0.0, n2.y);
        let lu = (len / 4.0).round().max(1.0) as usize;
        let tile = len / lu as f32;
        let street = i == street_edge;
        let door_tile = lu / 2;
        for f in 0..b.levels as usize {
            let fy = f as f32 * 3.2;
            for k in 0..lu {
                let c = p0 + t2 * ((k as f32 + 0.5) * tile);
                let base = Vec3::new(c.x, fy, c.y) + zd * 0.01;
                if f == 0 && street && shops && tile > 3.0 {
                    place(out, BOUTIQUE, base, xd, zd, (tile / 4.0).min(1.1), 1.0, ACCENTS[(b.seed as usize + k) % ACCENTS.len()]);
                    continue;
                }
                if f == 0 && street && k == door_tile {
                    let piece = if b.kind == 'x' && b.levels <= 2 && len > 6.0 && b.seed % 3 == 1 { PORTAIL } else { PORTE };
                    place(out, piece, base, xd, zd, 1.0, 1.0, accent);
                    continue;
                }
                if glass {
                    continue;
                }
                if f >= 1 && street && b.levels >= 2 && (k + f) % 2 == 0 && rng.f() < 0.55 {
                    place(out, BALCON, base + Vec3::Y * 0.02, xd, zd, (tile / 3.6).clamp(0.7, 1.1), 1.0, accent);
                    continue;
                }
                // a 3D window exactly over the painted one (glass u 0.3..0.7, v 0.35..0.8 of the tile)
                let sx = 0.4 * tile / 1.19;
                let sy = 1.44 / 1.29;
                place(out, if shutters { FENETRE_VOLETS } else { FENETRE }, base + Vec3::Y * (1.12 - 0.03 * sy), xd, zd, sx, sy, accent);
                if !street && rng.f() < 0.12 {
                    let side = c + t2 * (tile * 0.5);
                    place(out, CLIM, Vec3::new(side.x, fy + 2.1, side.y) + zd * 0.01, xd, zd, 1.0, 1.0, accent);
                }
            }
        }
    }
    // roofs: water tank and satellite dish on the terraces
    if b.roof == 1 && n >= 4 {
        let c = {
            let mut s = egui::Vec2::ZERO;
            for p in &b.pts {
                s += p.to_vec2();
            }
            (s / n as f32).to_pos2()
        };
        let e = (b.pts[1] - b.pts[0]).normalized();
        let xd = Vec3::new(e.x, 0.0, e.y);
        let zd = Vec3::new(-e.y, 0.0, e.x);
        if b.seed % 5 < 2 {
            place(out, CITERNE, Vec3::new(c.x, h + 0.05, c.y) + xd * 1.2, xd, zd, 1.0, 1.0, accent);
        }
        if b.seed % 7 < 2 {
            place(out, PARABOLE, Vec3::new(c.x, h + 0.05, c.y) - xd * 1.5, xd, zd, 1.0, 1.0, accent);
        }
    }
}

/// Dress every building within `radius` of `center`.
pub fn detail_mesh(map: &Map, center: Pos2, radius: f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let r = egui::vec2(radius, radius);
    map.bld_grid.query(center - r, center + r, |id| {
        if !seen.insert(id) {
            return;
        }
        let b = &map.buildings[id as usize];
        if (b.c - center).length() < radius {
            dress(map, b, &mut out);
        }
    });
    gates_mesh(map, center, radius, &mut out);
    out
}

/// A stretch of courtyard wall along a residential street, with maybe a gate in its middle.
pub struct WallPiece {
    pub a: Pos2,
    pub b: Pos2,
    pub out: egui::Vec2,
    pub gate: bool,
    pub tint: [f32; 3],
}

const WALL_TINTS: [[f32; 3]; 5] = [[0.93, 0.85, 0.72], [0.88, 0.70, 0.52], [0.95, 0.93, 0.88], [0.90, 0.76, 0.70], [0.80, 0.62, 0.46]];

/// Courtyard walls ("murs de clôture") along one residential street segment, both sides.
pub fn walls_for_segment(map: &Map, ri: usize, si: usize) -> Vec<WallPiece> {
    let road = &map.roads[ri];
    let mut out = Vec::new();
    if road.kind != 4 {
        return out;
    }
    let (p0, p1) = (road.pts[si], road.pts[si + 1]);
    let len = (p1 - p0).length();
    if len < 10.0 {
        return out;
    }
    let d = (p1 - p0) / len;
    let nrm = egui::vec2(-d.y, d.x);
    let off = road.half_width() + 1.3;
    let step = 9.0;
    let pieces = ((len - 4.0) / step).floor() as usize;
    for side in [-1.0f32, 1.0] {
        for k in 0..pieces {
            let s0 = 2.0 + k as f32 * step;
            let a = p0 + d * s0 + nrm * side * off;
            let b = p0 + d * (s0 + step) + nrm * side * off;
            let m = a + (b - a) * 0.5;
            let inward = nrm * side;
            let clear = [a, b, m, m + inward * 2.5]
                .iter()
                .all(|q| !map.point_in_building(*q) && map.road_at(*q, 0.3).is_none_or(|(_, r)| r == ri) && !map.in_area(*q, 0) && !map.in_area(*q, 1));
            if !clear {
                continue;
            }
            let h = (ri as u32).wrapping_mul(73856093) ^ (si as u32).wrapping_mul(19349663) ^ (k as u32).wrapping_mul(83492791) ^ if side > 0.0 { 0x5bd1e995 } else { 0 };
            let h = h.wrapping_mul(2654435761);
            out.push(WallPiece { a, b, out: -inward, gate: (h >> 7) % 3 == 0, tint: WALL_TINTS[(h >> 11) as usize % WALL_TINTS.len()] });
        }
    }
    out
}

/// Gates of the courtyard walls around the camera.
pub fn gates_mesh(map: &Map, center: Pos2, radius: f32, out: &mut Vec<f32>) {
    let mut seen = HashSet::new();
    let r = egui::vec2(radius, radius);
    map.road_grid.query(center - r, center + r, |id| {
        if !seen.insert(id) {
            return;
        }
        let (ri, si) = map.segs[id as usize];
        let (ri, si) = (ri as usize, si as usize);
        let road = &map.roads[ri];
        if road.kind != 4 || (road.pts[si] - center).length() > radius + 60.0 {
            return;
        }
        for w in walls_for_segment(map, ri, si) {
            if !w.gate {
                continue;
            }
            let m = w.a + (w.b - w.a) * 0.5;
            let t = (w.b - w.a).normalized();
            let xd = Vec3::new(t.x, 0.0, t.y);
            let zd = Vec3::new(w.out.x, 0.0, w.out.y);
            let accent = ACCENTS[((w.tint[0] * 97.0) as usize + (m.x as i32).unsigned_abs() as usize) % ACCENTS.len()];
            place(out, PORTAIL, Vec3::new(m.x, 0.0, m.y) + zd * 0.12, xd, zd, 1.0, 1.0, accent);
        }
    });
}

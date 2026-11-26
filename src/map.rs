//! Ouagadougou map: real street, lake and park shapes from OpenStreetMap
//! (the same open data Protomaps tiles are built from), converted to metres
//! around the city centre. Buildings and trees are generated along the streets.

use eframe::egui::{self as egui};
use egui::{pos2, vec2, Pos2, Vec2};

pub const MAP_DATA: &str = include_str!("../assets/ouaga.map");

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

pub struct Road {
    pub kind: u8,
    pub name: String,
    pub pts: Vec<Pos2>,
    pub min: Pos2,
    pub max: Pos2,
}
impl Road {
    pub fn half_width(&self) -> f32 {
        half_width(self.kind)
    }
}
pub fn half_width(kind: u8) -> f32 {
    match kind {
        0 => 9.0,
        1 => 7.5,
        2 => 6.5,
        3 => 5.5,
        _ => 4.0,
    }
}

/// kind: 0 water, 1 green, 2 airport
pub struct Area {
    pub kind: u8,
    pub name: String,
    pub pts: Vec<Pos2>,
    pub tris: Vec<[Pos2; 3]>,
    pub min: Pos2,
    pub max: Pos2,
}

pub struct Poi {
    pub kind: char,
    pub name: String,
    pub pos: Pos2,
}
impl Poi {
    pub fn label(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        match self.kind {
            'M' => "Marché".into(),
            'H' => "Centre de santé".into(),
            'F' => "Station-service".into(),
            'P' => "Commissariat".into(),
            'T' => "Gare".into(),
            'S' => "Stade".into(),
            _ => "Quartier".into(),
        }
    }
}

pub struct Building {
    pub pts: Vec<Pos2>,
    pub c: Pos2,
    pub min: Pos2,
    pub max: Pos2,
    pub levels: u8,
    /// m mosque, c church, h hotel, o office/shop, g public, s school, k health, u hut, r shed, x house, p generated
    pub kind: char,
    pub name: String,
    pub roof: u8,
    pub seed: u32,
}
impl Building {
    pub fn height(&self) -> f32 {
        match self.kind {
            'u' => 2.4,
            'r' => 2.8,
            _ => self.levels as f32 * 3.2,
        }
    }
    pub fn real(&self) -> bool {
        self.kind != 'p'
    }
}

pub struct Tree {
    pub p: Pos2,
    /// crown radius; 0 for a pure obstacle (monument base)
    pub r: f32,
    pub trunk: f32,
    pub baobab: bool,
}

pub struct Landmark {
    pub kind: String,
    pub name: String,
    pub pos: Pos2,
}

pub struct Grid {
    pub cell: f32,
    pub min: Pos2,
    pub nx: usize,
    pub ny: usize,
    pub cells: Vec<Vec<u32>>,
}
impl Grid {
    pub fn new(min: Pos2, max: Pos2, cell: f32) -> Self {
        let nx = ((max.x - min.x) / cell).ceil() as usize + 1;
        let ny = ((max.y - min.y) / cell).ceil() as usize + 1;
        Grid { cell, min, nx, ny, cells: vec![Vec::new(); nx * ny] }
    }
    fn cxy(&self, p: Pos2) -> (i64, i64) {
        (((p.x - self.min.x) / self.cell).floor() as i64, ((p.y - self.min.y) / self.cell).floor() as i64)
    }
    pub fn insert_box(&mut self, a: Pos2, b: Pos2, id: u32) {
        let (x0, y0) = self.cxy(a);
        let (x1, y1) = self.cxy(b);
        for y in y0.max(0)..=y1.min(self.ny as i64 - 1) {
            for x in x0.max(0)..=x1.min(self.nx as i64 - 1) {
                self.cells[y as usize * self.nx + x as usize].push(id);
            }
        }
    }
    pub fn insert(&mut self, p: Pos2, id: u32) {
        self.insert_box(p, p, id)
    }
    pub fn query(&self, a: Pos2, b: Pos2, mut f: impl FnMut(u32)) {
        let (x0, y0) = self.cxy(a);
        let (x1, y1) = self.cxy(b);
        for y in y0.max(0)..=y1.min(self.ny as i64 - 1) {
            for x in x0.max(0)..=x1.min(self.nx as i64 - 1) {
                for &id in &self.cells[y as usize * self.nx + x as usize] {
                    f(id)
                }
            }
        }
    }
}

impl Map {
    pub fn poi_named(&self, name: &str) -> Option<Pos2> {
        self.pois.iter().find(|p| p.name.contains(name)).map(|p| p.pos)
    }
    /// Closest point on any road (so objectives are reachable).
    pub fn road_point(&self, p: Pos2) -> Pos2 {
        match self.nearest_road(p, 400.0, 4) {
            Some((r, s, t, _)) => {
                let rd = &self.roads[r];
                rd.pts[s] + (rd.pts[s + 1] - rd.pts[s]) * t
            }
            None => p,
        }
    }
}

pub struct Map {
    pub roads: Vec<Road>,
    pub areas: Vec<Area>,
    pub pois: Vec<Poi>,
    pub landmarks: Vec<Landmark>,
    pub buildings: Vec<Building>,
    pub trees: Vec<Tree>,
    pub segs: Vec<(u32, u32)>,
    pub road_grid: Grid,
    pub bld_grid: Grid,
    pub tree_grid: Grid,
    /// courtyard walls along the residential streets (solid)
    pub walls: Vec<(Pos2, Pos2)>,
    pub wall_grid: Grid,
    pub min: Pos2,
    pub max: Pos2,
}

pub fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> (f32, f32) {
    let ab = b - a;
    let l2 = ab.length_sq();
    let t = if l2 > 0.0 { ((p - a).dot(ab) / l2).clamp(0.0, 1.0) } else { 0.0 };
    ((a + ab * t - p).length(), t)
}

pub fn point_in_poly(p: Pos2, pts: &[Pos2]) -> bool {
    let mut inside = false;
    let n = pts.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (pts[i], pts[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn triangulate(pts: &[Pos2]) -> Vec<[Pos2; 3]> {
    let mut v: Vec<Pos2> = pts.to_vec();
    if v.len() > 1 && v[0] == v[v.len() - 1] {
        v.pop();
    }
    v.dedup();
    let mut out = Vec::new();
    if v.len() < 3 {
        return out;
    }
    let area: f32 = (0..v.len()).map(|i| {
        let (a, b) = (v[i], v[(i + 1) % v.len()]);
        a.x * b.y - b.x * a.y
    }).sum();
    if area < 0.0 {
        v.reverse();
    }
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b - a).x * (c - a).y - (b - a).y * (c - a).x;
    let mut guard = 0;
    while v.len() > 3 && guard < 100_000 {
        guard += 1;
        let n = v.len();
        let mut found = false;
        for i in 0..n {
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            if cross(a, b, c) <= 0.0 {
                continue;
            }
            let mut ear = true;
            for (k, &p) in v.iter().enumerate() {
                if k == i || k == (i + n - 1) % n || k == (i + 1) % n {
                    continue;
                }
                if cross(a, b, p) > 0.0 && cross(b, c, p) > 0.0 && cross(c, a, p) > 0.0 {
                    ear = false;
                    break;
                }
            }
            if ear {
                out.push([a, b, c]);
                v.remove(i);
                found = true;
                break;
            }
        }
        if !found {
            // degenerate: drop a vertex and keep going
            v.remove(0);
        }
    }
    if v.len() == 3 {
        out.push([v[0], v[1], v[2]]);
    }
    out
}

fn bbox(pts: &[Pos2]) -> (Pos2, Pos2) {
    let mut a = pos2(f32::MAX, f32::MAX);
    let mut b = pos2(f32::MIN, f32::MIN);
    for p in pts {
        a = a.min(*p);
        b = b.max(*p);
    }
    (a, b)
}

/// Does segment p0-p1 come within the axis aligned box [-h, h] (Liang-Barsky)?
fn seg_hits_box(p0: Vec2, p1: Vec2, h: Vec2) -> bool {
    let d = p1 - p0;
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;
    for (p, q) in [
        (-d.x, p0.x + h.x),
        (d.x, h.x - p0.x),
        (-d.y, p0.y + h.y),
        (d.y, h.y - p0.y),
    ] {
        if p.abs() < 1e-6 {
            if q < 0.0 {
                return false;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

struct Obb {
    c: Pos2,
    h: Vec2,
    cs: f32,
    sn: f32,
}
impl Obb {
    fn to_local(&self, p: Pos2) -> Vec2 {
        let d = p - self.c;
        vec2(d.x * self.cs + d.y * self.sn, -d.x * self.sn + d.y * self.cs)
    }
    fn corners(&self) -> Vec<Pos2> {
        let ex = vec2(self.cs, self.sn) * self.h.x;
        let ey = vec2(-self.sn, self.cs) * self.h.y;
        vec![self.c - ex - ey, self.c + ex - ey, self.c + ex + ey, self.c - ex + ey]
    }
}

pub const BUILDINGS_DATA: &str = include_str!("../assets/buildings.map");

fn poly_area(pts: &[Pos2]) -> f32 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f32>()
        .abs()
        * 0.5
}

impl Map {
    pub fn load() -> Map {
        let mut roads = Vec::new();
        let mut areas = Vec::new();
        let mut pois = Vec::new();
        let mut raw_buildings: Vec<(String, String, Vec<Pos2>)> = Vec::new();
        let mut landmarks = Vec::new();
        for line in MAP_DATA.lines().chain(BUILDINGS_DATA.lines()) {
            let mut parts = line.splitn(3, '|');
            let (Some(tag), Some(name), Some(coords)) = (parts.next(), parts.next(), parts.next()) else { continue };
            let pts: Vec<Pos2> = coords
                .split(' ')
                .filter_map(|c| {
                    let (x, y) = c.split_once(',')?;
                    Some(pos2(x.parse().ok()?, y.parse().ok()?))
                })
                .collect();
            if pts.is_empty() {
                continue;
            }
            let name = name.to_string();
            if let Some(k) = tag.strip_prefix('R') {
                if pts.len() < 2 {
                    continue;
                }
                let (min, max) = bbox(&pts);
                roads.push(Road { kind: k.parse().unwrap_or(4), name, pts, min, max });
            } else if tag == "W" || tag == "G" || tag == "A" {
                if pts.len() < 3 {
                    continue;
                }
                let kind = match tag {
                    "W" => 0,
                    "G" => 1,
                    _ => 2,
                };
                let (min, max) = bbox(&pts);
                let tris = triangulate(&pts);
                areas.push(Area { kind, name, pts, tris, min, max });
            } else if let Some(b) = tag.strip_prefix('B') {
                if pts.len() >= 3 {
                    raw_buildings.push((b.to_string(), name, pts));
                }
            } else if let Some(k) = tag.strip_prefix('L') {
                landmarks.push(Landmark { kind: k.to_string(), name, pos: pts[0] });
            } else {
                pois.push(Poi { kind: tag.chars().next().unwrap_or('Q'), name, pos: pts[0] });
            }
        }
        // draw order: airport, green, water
        areas.sort_by_key(|a| match a.kind {
            2 => 0,
            1 => 1,
            _ => 2,
        });

        let mut min = pos2(f32::MAX, f32::MAX);
        let mut max = pos2(f32::MIN, f32::MIN);
        for r in &roads {
            min = min.min(r.min);
            max = max.max(r.max);
        }
        for l in &landmarks {
            min = min.min(l.pos - vec2(200.0, 200.0));
            max = max.max(l.pos + vec2(200.0, 200.0));
        }
        min -= vec2(100.0, 100.0);
        max += vec2(100.0, 100.0);

        let mut road_grid = Grid::new(min, max, 80.0);
        let mut segs = Vec::new();
        for (ri, r) in roads.iter().enumerate() {
            let hw = r.half_width();
            for si in 0..r.pts.len() - 1 {
                let (a, b) = (r.pts[si], r.pts[si + 1]);
                let id = segs.len() as u32;
                segs.push((ri as u32, si as u32));
                road_grid.insert_box(a.min(b) - vec2(hw, hw), a.max(b) + vec2(hw, hw), id);
            }
        }

        let mut map = Map {
            roads,
            areas,
            pois,
            landmarks,
            buildings: Vec::new(),
            trees: Vec::new(),
            segs,
            road_grid,
            bld_grid: Grid::new(min, max, 40.0),
            tree_grid: Grid::new(min, max, 50.0),
            walls: Vec::new(),
            wall_grid: Grid::new(min, max, 40.0),
            min,
            max,
        };
        map.add_real_buildings(raw_buildings);
        map.generate_buildings();
        let mut walls = Vec::new();
        for (ri, r) in map.roads.iter().enumerate() {
            if r.kind != 4 {
                continue;
            }
            for si in 0..r.pts.len() - 1 {
                for w in crate::kit::walls_for_segment(&map, ri, si) {
                    walls.push((w.a, w.b));
                }
            }
        }
        for (i, (a, b)) in walls.iter().enumerate() {
            map.wall_grid.insert_box(a.min(*b) - vec2(1.0, 1.0), a.max(*b) + vec2(1.0, 1.0), i as u32);
        }
        map.walls = walls;
        map
    }

    fn push_building(&mut self, b: Building) {
        let id = self.buildings.len() as u32;
        self.bld_grid.insert_box(b.min, b.max, id);
        self.buildings.push(b);
    }

    /// Real OpenStreetMap footprints. Missing heights are estimated from size and place.
    fn add_real_buildings(&mut self, raw: Vec<(String, String, Vec<Pos2>)>) {
        let centre = pos2(-381.0, -364.0);
        for (i, (tag, name, pts)) in raw.into_iter().enumerate() {
            let digits: String = tag.chars().take_while(|c| c.is_ascii_digit()).collect();
            let kind = tag.chars().last().filter(|c| c.is_ascii_alphabetic()).unwrap_or('x');
            let mut levels: u8 = digits.parse().unwrap_or(0);
            let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ (i as u64 + 1).wrapping_mul(0x2545F4914F6CDD1D));
            rng.next();
            let area = poly_area(&pts);
            let (bmin, bmax) = bbox(&pts);
            let c = bmin + (bmax - bmin) * 0.5;
            if levels == 0 {
                let near = (c - centre).length() < 2500.0;
                levels = if area < 60.0 {
                    1
                } else if area < 250.0 {
                    if rng.f() < 0.12 { 2 } else { 1 }
                } else if area < 900.0 {
                    1 + rng.below(if near { 4 } else { 2 }) as u8
                } else {
                    2 + rng.below(if near { 5 } else { 2 }) as u8
                };
                if matches!(kind, 'o' | 'g' | 'h') {
                    levels += 1 + rng.below(2) as u8;
                }
                if matches!(kind, 'm' | 'c') {
                    levels = 2;
                }
            }
            let roof = if kind == 'u' {
                3
            } else {
                let r = rng.f();
                if levels >= 3 || matches!(kind, 'o' | 'g' | 'h' | 'k' | 's') {
                    1
                } else if r < 0.55 {
                    0
                } else if r < 0.9 {
                    1
                } else {
                    2
                }
            };
            self.push_building(Building { pts, c, min: bmin, max: bmax, levels: levels.max(1), kind, name, roof, seed: rng.next() as u32 });
        }
    }

    fn real_buildings_near(&self, p: Pos2, r: f32) -> usize {
        let mut n = 0;
        self.bld_grid.query(p - vec2(r, r), p + vec2(r, r), |_| n += 1);
        n
    }

    /// Height of the building whose walls are within `margin` of a point (for things that stick
    /// out of façades: awnings, signs, balconies).
    pub fn facade_near(&self, p: Pos2, margin: f32) -> Option<f32> {
        let mut h: Option<f32> = None;
        let m = vec2(margin, margin);
        self.bld_grid.query(p - m, p + m, |id| {
            let b = &self.buildings[id as usize];
            if p.x < b.min.x - margin || p.x > b.max.x + margin || p.y < b.min.y - margin || p.y > b.max.y + margin {
                return;
            }
            let n = b.pts.len();
            let close = (0..n).any(|i| seg_dist(p, b.pts[i], b.pts[(i + 1) % n]).0 < margin) || point_in_poly(p, &b.pts);
            if close {
                h = Some(h.unwrap_or(0.0).max(b.height()));
            }
        });
        h
    }

    /// Height of the building standing at a point, if any.
    pub fn building_height_at(&self, p: Pos2) -> Option<f32> {
        let mut h: Option<f32> = None;
        self.bld_grid.query(p, p, |id| {
            let b = &self.buildings[id as usize];
            if p.x >= b.min.x && p.x <= b.max.x && p.y >= b.min.y && p.y <= b.max.y && point_in_poly(p, &b.pts) {
                h = Some(h.unwrap_or(0.0).max(b.height() + 0.8));
            }
        });
        h
    }

    /// Is a 3D point inside a tree (trunk or crown)?
    pub fn tree_blocks(&self, p: Pos2, y: f32) -> bool {
        let mut hit = false;
        let r = vec2(8.0, 8.0);
        self.tree_grid.query(p - r, p + r, |id| {
            let t = &self.trees[id as usize];
            if hit || t.r <= 0.0 {
                return;
            }
            let d = (t.p - p).length();
            let trunk_h = if t.baobab { 5.0 + t.r * 0.4 } else { 1.8 + t.r * 0.3 };
            if d < t.trunk + 0.3 && y < trunk_h + 0.5 {
                hit = true;
            } else if d < t.r * 1.05 + 0.3 && y > trunk_h - 0.6 && y < trunk_h + t.r * 1.5 {
                hit = true;
            }
        });
        hit
    }

    pub fn point_in_building(&self, p: Pos2) -> bool {
        let mut inside = false;
        self.bld_grid.query(p, p, |id| {
            let b = &self.buildings[id as usize];
            if !inside && p.x >= b.min.x && p.x <= b.max.x && p.y >= b.min.y && p.y <= b.max.y && point_in_poly(p, &b.pts) {
                inside = true;
            }
        });
        inside
    }

    /// Trees along every street, plus generated houses where the map has no footprints.
    fn generate_buildings(&mut self) {
        let mut rng = Rng(0x0A6A_D0D0_1234_5678);
        let mut trees: Vec<Tree> = Vec::new();
        let mut gen: Vec<Building> = Vec::new();
        for ri in 0..self.roads.len() {
            let kind = self.roads[ri].kind;
            let hw = self.roads[ri].half_width();
            let n = self.roads[ri].pts.len();
            for si in 0..n - 1 {
                let a = self.roads[ri].pts[si];
                let b = self.roads[ri].pts[si + 1];
                let len = (b - a).length();
                if len < 6.0 {
                    continue;
                }
                let dir = (b - a) / len;
                let nrm = vec2(-dir.y, dir.x);
                for side in [-1.0f32, 1.0] {
                    let mut s = rng.range(0.0, 5.0);
                    while s < len - 4.0 {
                        let big = kind <= 2;
                        let w = if big { rng.range(10.0, 24.0) } else { rng.range(7.0, 14.0) };
                        let d = if big { rng.range(10.0, 20.0) } else { rng.range(7.0, 12.0) };
                        if rng.f() < if kind <= 3 { 0.45 } else { 0.25 } {
                            let tp = a + dir * s + nrm * side * (hw + 1.6);
                            if self.road_clear(tp, 0.8) && !self.in_area(tp, 0) && !self.point_in_building(tp) {
                                let baobab = rng.f() < 0.06;
                                trees.push(Tree {
                                    p: tp,
                                    r: if baobab { rng.range(4.5, 6.5) } else { rng.range(2.5, 5.0) },
                                    trunk: if baobab { 1.1 } else { 0.4 },
                                    baobab,
                                });
                            }
                        }
                        let setback = hw + 2.0 + rng.range(0.0, 3.0);
                        let c = a + dir * (s + w * 0.5) + nrm * side * (setback + d * 0.5);
                        if rng.f() > 0.12 && self.real_buildings_near(c, 45.0) == 0 {
                            let ob = Obb { c, h: vec2(w * 0.5, d * 0.5), cs: dir.x, sn: dir.y };
                            if self.generated_ok(&ob, &gen) {
                                let levels = if big {
                                    if c.to_vec2().length() < 1800.0 { 2 + rng.below(5) as u8 } else { 1 + rng.below(3) as u8 }
                                } else if rng.f() < 0.15 {
                                    2
                                } else {
                                    1
                                };
                                let pts = ob.corners();
                                let (bmin, bmax) = bbox(&pts);
                                gen.push(Building { pts, c, min: bmin, max: bmax, levels, kind: 'p', name: String::new(), roof: rng.below(3) as u8, seed: rng.next() as u32 });
                            }
                        }
                        s += w + rng.range(1.0, 4.0);
                    }
                }
            }
        }
        for b in gen {
            self.push_building(b);
        }
        // monuments block the way like big trunks
        for l in &self.landmarks {
            let r = match l.kind.as_str() {
                "heros" => 14.0,
                "nations" | "cineastes" => 6.0,
                "eau" => 5.0,
                _ => 3.0,
            };
            trees.push(Tree { p: l.pos, r: 0.0, trunk: r, baobab: false });
        }
        for (i, t) in trees.iter().enumerate() {
            self.tree_grid.insert_box(t.p - vec2(t.trunk, t.trunk), t.p + vec2(t.trunk, t.trunk), i as u32);
        }
        self.trees = trees;
    }

    fn generated_ok(&self, bl: &Obb, gen: &[Building]) -> bool {
        if !(self.min.x < bl.c.x && bl.c.x < self.max.x && self.min.y < bl.c.y && bl.c.y < self.max.y) {
            return false;
        }
        for k in 0..3 {
            if self.in_area(bl.c, k) {
                return false;
            }
        }
        let r = bl.h.length();
        let lo = bl.c - vec2(r + 10.0, r + 10.0);
        let hi = bl.c + vec2(r + 10.0, r + 10.0);
        let mut ok = true;
        self.road_grid.query(lo, hi, |id| {
            if !ok {
                return;
            }
            let (ri, si) = self.segs[id as usize];
            let road = &self.roads[ri as usize];
            let m = road.half_width() + 1.0;
            let p0 = bl.to_local(road.pts[si as usize]);
            let p1 = bl.to_local(road.pts[si as usize + 1]);
            if seg_hits_box(p0, p1, bl.h + vec2(m, m)) {
                ok = false;
            }
        });
        if !ok {
            return false;
        }
        let (bmin, bmax) = bbox(&bl.corners());
        for g in gen.iter().rev().take(400) {
            if bmin.x < g.max.x + 1.0 && bmax.x > g.min.x - 1.0 && bmin.y < g.max.y + 1.0 && bmax.y > g.min.y - 1.0 {
                return false;
            }
        }
        true
    }

    pub fn in_area(&self, p: Pos2, kind: u8) -> bool {
        self.areas.iter().any(|a| {
            a.kind == kind && p.x >= a.min.x && p.x <= a.max.x && p.y >= a.min.y && p.y <= a.max.y && point_in_poly(p, &a.pts)
        })
    }

    /// true when no road is closer than its half width + margin
    pub fn road_clear(&self, p: Pos2, margin: f32) -> bool {
        self.road_at(p, margin).is_none()
    }

    /// Road kind under a point (the most important one).
    pub fn road_at(&self, p: Pos2, margin: f32) -> Option<(u8, usize)> {
        let mut best: Option<(u8, usize)> = None;
        self.road_grid.query(p, p, |id| {
            let (ri, si) = self.segs[id as usize];
            let r = &self.roads[ri as usize];
            let (d, _) = seg_dist(p, r.pts[si as usize], r.pts[si as usize + 1]);
            if d < r.half_width() + margin && best.map_or(true, |(k, _)| r.kind < k) {
                best = Some((r.kind, ri as usize));
            }
        });
        best
    }

    /// Nearest road segment within `maxd`: (road, seg, t, dist).
    pub fn nearest_road(&self, p: Pos2, maxd: f32, max_kind: u8) -> Option<(usize, usize, f32, f32)> {
        let mut best: Option<(usize, usize, f32, f32)> = None;
        self.road_grid.query(p - vec2(maxd, maxd), p + vec2(maxd, maxd), |id| {
            let (ri, si) = self.segs[id as usize];
            let r = &self.roads[ri as usize];
            if r.kind > max_kind {
                return;
            }
            let (d, t) = seg_dist(p, r.pts[si as usize], r.pts[si as usize + 1]);
            if d < maxd && best.map_or(true, |b| d < b.3) {
                best = Some((ri as usize, si as usize, t, d));
            }
        });
        best
    }

    /// Push a circle out of buildings, tree trunks and monuments. Returns the push normal.
    pub fn collide(&self, p: &mut Pos2, r: f32) -> Option<Vec2> {
        let mut total = Vec2::ZERO;
        let mut seen: Vec<u32> = Vec::new();
        self.bld_grid.query(*p - vec2(r, r), *p + vec2(r, r), |id| {
            if seen.contains(&id) {
                return;
            }
            seen.push(id);
            let b = &self.buildings[id as usize];
            if p.x < b.min.x - r || p.x > b.max.x + r || p.y < b.min.y - r || p.y > b.max.y + r {
                return;
            }
            let n = b.pts.len();
            let mut best = (f32::MAX, *p);
            for i in 0..n {
                let (a, c) = (b.pts[i], b.pts[(i + 1) % n]);
                let (d, t) = seg_dist(*p, a, c);
                if d < best.0 {
                    best = (d, a + (c - a) * t);
                }
            }
            let (dl, q) = best;
            let w = if point_in_poly(*p, &b.pts) {
                let dir = if dl > 1e-4 { (q - *p) / dl } else { vec2(1.0, 0.0) };
                dir * (dl + r)
            } else if dl < r && dl > 1e-4 {
                (*p - q) / dl * (r - dl)
            } else {
                return;
            };
            *p += w;
            total += w;
        });
        self.wall_grid.query(*p - vec2(r, r), *p + vec2(r, r), |id| {
            let (a, b) = self.walls[id as usize];
            let (d, t) = seg_dist(*p, a, b);
            let rr = r + 0.12;
            if d < rr {
                let q = a + (b - a) * t;
                let n = if d > 1e-4 { (*p - q) / d } else { vec2(-(b - a).y, (b - a).x).normalized() };
                let w = n * (rr - d);
                *p += w;
                total += w;
            }
        });
        self.tree_grid.query(*p - vec2(r, r), *p + vec2(r, r), |id| {
            let t = &self.trees[id as usize];
            let d = *p - t.p;
            let rr = r + t.trunk;
            let dl = d.length();
            if dl < rr && dl > 1e-4 {
                let w = d / dl * (rr - dl);
                *p += w;
                total += w;
            }
        });
        if p.x < self.min.x { total.x += self.min.x - p.x; p.x = self.min.x; }
        if p.y < self.min.y { total.y += self.min.y - p.y; p.y = self.min.y; }
        if p.x > self.max.x { total.x += self.max.x - p.x; p.x = self.max.x; }
        if p.y > self.max.y { total.y += self.max.y - p.y; p.y = self.max.y; }
        if total.length_sq() > 1e-8 { Some(total.normalized()) } else { None }
    }

    pub fn nearest_poi(&self, p: Pos2, kinds: &str) -> Option<&Poi> {
        self.pois
            .iter()
            .filter(|q| kinds.contains(q.kind))
            .min_by(|a, b| (a.pos - p).length_sq().total_cmp(&(b.pos - p).length_sq()))
    }

    pub fn place_name(&self, p: Pos2) -> String {
        let road = self.nearest_road(p, 25.0, 4).and_then(|(r, ..)| {
            let n = &self.roads[r].name;
            if n.is_empty() { None } else { Some(n.clone()) }
        });
        let q = self.nearest_poi(p, "Q").filter(|q| (q.pos - p).length() < 2500.0).map(|q| q.name.clone());
        match (road, q) {
            (Some(r), Some(q)) => format!("{r} · {q}"),
            (Some(r), None) => r,
            (None, Some(q)) => q,
            (None, None) => "Ouagadougou".into(),
        }
    }
}

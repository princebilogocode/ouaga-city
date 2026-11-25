//! Ouagadougou map: real street, lake and park shapes from OpenStreetMap
//! (the same open data Protomaps tiles are built from), converted to metres
//! around the city centre. Buildings and trees are generated along the streets.

use eframe::egui::{self as egui};
use egui::{pos2, vec2, Color32, Pos2, Vec2};

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
    pub c: Pos2,
    pub h: Vec2,
    pub cs: f32,
    pub sn: f32,
    pub color: Color32,
    pub roof: u8,
    pub floors: u8,
}
impl Building {
    pub fn corners(&self) -> [Pos2; 4] {
        let ex = vec2(self.cs, self.sn) * self.h.x;
        let ey = vec2(-self.sn, self.cs) * self.h.y;
        [self.c - ex - ey, self.c + ex - ey, self.c + ex + ey, self.c - ex + ey]
    }
    pub fn to_local(&self, p: Pos2) -> Vec2 {
        let d = p - self.c;
        vec2(d.x * self.cs + d.y * self.sn, -d.x * self.sn + d.y * self.cs)
    }
    pub fn from_local(&self, v: Vec2) -> Vec2 {
        vec2(v.x * self.cs - v.y * self.sn, v.x * self.sn + v.y * self.cs)
    }
}

pub struct Tree {
    pub p: Pos2,
    pub r: f32,
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
    pub buildings: Vec<Building>,
    pub trees: Vec<Tree>,
    pub segs: Vec<(u32, u32)>,
    pub road_grid: Grid,
    pub bld_grid: Grid,
    pub tree_grid: Grid,
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

fn triangulate(pts: &[Pos2]) -> Vec<[Pos2; 3]> {
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

fn obb_overlap(a: &Building, b: &Building, gap: f32) -> bool {
    let axes = [vec2(a.cs, a.sn), vec2(-a.sn, a.cs), vec2(b.cs, b.sn), vec2(-b.sn, b.cs)];
    let d = b.c - a.c;
    for ax in axes {
        let ra = a.h.x * (ax.dot(vec2(a.cs, a.sn))).abs() + a.h.y * (ax.dot(vec2(-a.sn, a.cs))).abs();
        let rb = b.h.x * (ax.dot(vec2(b.cs, b.sn))).abs() + b.h.y * (ax.dot(vec2(-b.sn, b.cs))).abs();
        if d.dot(ax).abs() > ra + rb + gap {
            return false;
        }
    }
    true
}

impl Map {
    pub fn load() -> Map {
        let mut roads = Vec::new();
        let mut areas = Vec::new();
        let mut pois = Vec::new();
        for line in MAP_DATA.lines() {
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
                let kind = match tag { "W" => 0, "G" => 1, _ => 2 };
                let (min, max) = bbox(&pts);
                let tris = triangulate(&pts);
                areas.push(Area { kind, name, pts, tris, min, max });
            } else {
                pois.push(Poi { kind: tag.chars().next().unwrap_or('Q'), name, pos: pts[0] });
            }
        }
        // draw order: airport, green, water
        areas.sort_by_key(|a| match a.kind { 2 => 0, 1 => 1, _ => 2 });

        let mut min = pos2(f32::MAX, f32::MAX);
        let mut max = pos2(f32::MIN, f32::MIN);
        for r in &roads {
            min = min.min(r.min);
            max = max.max(r.max);
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
            buildings: Vec::new(),
            trees: Vec::new(),
            segs,
            road_grid,
            bld_grid: Grid::new(min, max, 50.0),
            tree_grid: Grid::new(min, max, 50.0),
            min,
            max,
        };
        map.generate_buildings();
        map
    }

    fn generate_buildings(&mut self) {
        let mut rng = Rng(0x0A6A_D0D0_1234_5678);
        let wall_colors = [
            Color32::from_rgb(150, 150, 152), // tôle (tin roof)
            Color32::from_rgb(170, 172, 176),
            Color32::from_rgb(128, 132, 138),
            Color32::from_rgb(196, 112, 70), // terracotta
            Color32::from_rgb(226, 214, 190), // concrete terrace
            Color32::from_rgb(214, 186, 140), // banco ochre
            Color32::from_rgb(190, 140, 96),
        ];
        let mut buildings: Vec<Building> = Vec::new();
        let mut trees: Vec<Tree> = Vec::new();
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
                        let w = if big { rng.range(10.0, 26.0) } else { rng.range(7.0, 15.0) };
                        let d = if big { rng.range(10.0, 22.0) } else { rng.range(7.0, 13.0) };
                        if rng.f() < 0.35 {
                            let tp = a + dir * s + nrm * side * (hw + 1.6);
                            if self.road_clear(tp, 0.8) && !self.in_area(tp, 0) {
                                trees.push(Tree { p: tp, r: rng.range(2.5, 5.0) });
                            }
                        }
                        if rng.f() < 0.12 {
                            s += w;
                            continue;
                        }
                        let setback = hw + 2.0 + rng.range(0.0, 3.0);
                        let c = a + dir * (s + w * 0.5) + nrm * side * (setback + d * 0.5);
                        let bl = Building {
                            c,
                            h: vec2(w * 0.5, d * 0.5),
                            cs: dir.x,
                            sn: dir.y,
                            color: wall_colors[rng.below(wall_colors.len())],
                            roof: rng.below(3) as u8,
                            floors: if big {
                                if c.to_vec2().length() < 1800.0 { 2 + rng.below(6) as u8 } else { 1 + rng.below(3) as u8 }
                            } else if rng.f() < 0.15 { 2 } else { 1 },
                        };
                        if self.building_ok(&bl, &buildings) {
                            let id = buildings.len() as u32;
                            self.bld_grid.insert(c, id);
                            buildings.push(bl);
                        }
                        s += w + rng.range(1.0, 4.0);
                    }
                }
            }
        }
        for (i, t) in trees.iter().enumerate() {
            self.tree_grid.insert(t.p, i as u32);
        }
        self.buildings = buildings;
        self.trees = trees;
    }

    fn building_ok(&self, bl: &Building, existing: &[Building]) -> bool {
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
        self.bld_grid.query(bl.c - vec2(r + 26.0, r + 26.0), bl.c + vec2(r + 26.0, r + 26.0), |id| {
            if ok && obb_overlap(bl, &existing[id as usize], 1.0) {
                ok = false;
            }
        });
        ok
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

    /// Push a circle out of buildings and tree trunks. Returns the push normal.
    pub fn collide(&self, p: &mut Pos2, r: f32) -> Option<Vec2> {
        let mut total = Vec2::ZERO;
        let q = 30.0 + r;
        self.bld_grid.query(*p - vec2(q, q), *p + vec2(q, q), |id| {
            let b = &self.buildings[id as usize];
            let l = b.to_local(*p);
            let cl = vec2(l.x.clamp(-b.h.x, b.h.x), l.y.clamp(-b.h.y, b.h.y));
            let d = l - cl;
            let dl = d.length();
            let push_local = if dl > 1e-4 {
                if dl < r { d / dl * (r - dl) } else { return }
            } else {
                // centre inside the box: push out along the shallowest axis
                let px = b.h.x - l.x.abs() + r;
                let py = b.h.y - l.y.abs() + r;
                if px < py { vec2(px * l.x.signum(), 0.0) } else { vec2(0.0, py * l.y.signum()) }
            };
            let w = b.from_local(push_local);
            *p += w;
            total += w;
        });
        self.tree_grid.query(*p - vec2(6.0, 6.0), *p + vec2(6.0, 6.0), |id| {
            let t = &self.trees[id as usize];
            let d = *p - t.p;
            let rr = r + 0.5;
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

//! GPS: shortest route along the real streets of Ouaga (OpenStreetMap road network), for the
//! waypoint the player marks on the map and for mission targets.

use crate::map::{seg_dist, Map};
use eframe::egui::{vec2, Pos2};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::OnceLock;

struct Graph {
    nodes: Vec<Pos2>,
    adj: Vec<Vec<(u32, f32)>>,
    cells: HashMap<(i32, i32), Vec<u32>>,
}

const CELL: f32 = 60.0;

fn key(p: Pos2) -> (i32, i32) {
    ((p.x / CELL).floor() as i32, (p.y / CELL).floor() as i32)
}

fn graph(map: &Map) -> &'static Graph {
    static G: OnceLock<Graph> = OnceLock::new();
    G.get_or_init(|| {
        let mut ids: HashMap<(i32, i32), u32> = HashMap::new();
        let mut nodes: Vec<Pos2> = Vec::new();
        let mut adj: Vec<Vec<(u32, f32)>> = Vec::new();
        let mut node = |p: Pos2, nodes: &mut Vec<Pos2>, adj: &mut Vec<Vec<(u32, f32)>>| -> u32 {
            let k = ((p.x / 2.5).round() as i32, (p.y / 2.5).round() as i32);
            *ids.entry(k).or_insert_with(|| {
                nodes.push(p);
                adj.push(Vec::new());
                (nodes.len() - 1) as u32
            })
        };
        let mut ends: Vec<(u32, usize)> = Vec::new();
        for (ri, r) in map.roads.iter().enumerate() {
            // small streets cost a bit more: the GPS prefers the big avenues, like a local would
            let cost = match r.kind { 0 | 1 => 1.0, 2 | 3 => 1.1, _ => 1.35 };
            let mut prev: Option<u32> = None;
            for (i, &p) in r.pts.iter().enumerate() {
                let n = node(p, &mut nodes, &mut adj);
                if let Some(a) = prev {
                    if a != n {
                        let w = (nodes[a as usize] - nodes[n as usize]).length() * cost;
                        adj[a as usize].push((n, w));
                        adj[n as usize].push((a, w));
                    }
                }
                if i == 0 || i + 1 == r.pts.len() {
                    ends.push((n, ri));
                }
                prev = Some(n);
            }
        }
        // a street that ends on another one in the middle of its segment: join them
        let mut extra = Vec::new();
        for &(n, ri) in &ends {
            if adj[n as usize].len() > 1 {
                continue;
            }
            let p = nodes[n as usize];
            let mut best: Option<(f32, Pos2, Pos2)> = None;
            map.road_grid.query(p - vec2(8.0, 8.0), p + vec2(8.0, 8.0), |id| {
                let (rj, sj) = map.segs[id as usize];
                if rj as usize == ri {
                    return;
                }
                let r = &map.roads[rj as usize];
                let (a, b) = (r.pts[sj as usize], r.pts[sj as usize + 1]);
                let (d, _) = seg_dist(p, a, b);
                if d < 8.0 && best.map_or(true, |x| d < x.0) {
                    best = Some((d, a, b));
                }
            });
            if let Some((_, a, b)) = best {
                extra.push((n, a, b));
            }
        }
        for (n, a, b) in extra {
            for q in [a, b] {
                let m = node(q, &mut nodes, &mut adj);
                if m != n {
                    let w = (nodes[n as usize] - nodes[m as usize]).length() * 1.2;
                    adj[n as usize].push((m, w));
                    adj[m as usize].push((n, w));
                }
            }
        }
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, &p) in nodes.iter().enumerate() {
            cells.entry(key(p)).or_default().push(i as u32);
        }
        Graph { nodes, adj, cells }
    })
}

fn nearest(g: &Graph, p: Pos2) -> Option<u32> {
    let (cx, cy) = key(p);
    for r in 1..12 {
        let mut best: Option<(f32, u32)> = None;
        for x in cx - r..=cx + r {
            for y in cy - r..=cy + r {
                if let Some(v) = g.cells.get(&(x, y)) {
                    for &n in v {
                        let d = (g.nodes[n as usize] - p).length();
                        if !g.adj[n as usize].is_empty() && best.map_or(true, |b| d < b.0) {
                            best = Some((d, n));
                        }
                    }
                }
            }
        }
        if best.is_some() {
            return best.map(|b| b.1);
        }
    }
    None
}

#[derive(PartialEq)]
struct St(f32, u32);
impl Eq for St {}
impl Ord for St {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for St {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// The point of the nearest street under `p`, and the graph nodes at both ends of that stretch
/// of street with the distance to each.
fn on_street(map: &Map, g: &Graph, p: Pos2) -> Option<(Pos2, Vec<(u32, f32)>)> {
    let (ri, si, t, _) = map.nearest_road(p, 250.0, 4)?;
    let r = &map.roads[ri];
    let (a, b) = (r.pts[si], r.pts[si + 1]);
    let proj = a + (b - a) * t;
    let mut ends = Vec::new();
    for q in [a, b] {
        if let Some(n) = nearest(g, q) {
            ends.push((n, (g.nodes[n as usize] - proj).length()));
        }
    }
    if ends.is_empty() {
        None
    } else {
        Some((proj, ends))
    }
}

/// The road route from `from` to `to` (A*), as a polyline starting at `from`, following the streets.
pub fn route(map: &Map, from: Pos2, to: Pos2) -> Vec<Pos2> {
    let g = graph(map);
    let (Some((p0, starts)), Some((p1, goals))) = (on_street(map, g, from), on_street(map, g, to)) else { return vec![from, to] };
    let mut dist: HashMap<u32, f32> = HashMap::new();
    let mut prev: HashMap<u32, u32> = HashMap::new();
    let mut heap = BinaryHeap::new();
    for &(n, c) in &starts {
        if dist.get(&n).map_or(true, |&d| c < d) {
            dist.insert(n, c);
            heap.push(St(c + (g.nodes[n as usize] - p1).length(), n));
        }
    }
    let mut best: Option<(f32, u32)> = None;
    let mut steps = 0;
    while let Some(St(f, u)) = heap.pop() {
        if best.is_some_and(|b| f >= b.0) {
            break;
        }
        steps += 1;
        if steps > 400_000 {
            break;
        }
        let du = dist[&u];
        if let Some(&(_, c)) = goals.iter().find(|(n, _)| *n == u) {
            if best.map_or(true, |b| du + c < b.0) {
                best = Some((du + c, u));
            }
        }
        for &(v, w) in &g.adj[u as usize] {
            let nd = du + w;
            if dist.get(&v).map_or(true, |&d| nd < d) {
                dist.insert(v, nd);
                prev.insert(v, u);
                heap.push(St(nd + (g.nodes[v as usize] - p1).length(), v));
            }
        }
    }
    let Some((_, t)) = best else { return vec![from, p0, p1, to] };
    let mut path = vec![to, p1, g.nodes[t as usize]];
    let mut c = t;
    while let Some(&p) = prev.get(&c) {
        path.push(g.nodes[p as usize]);
        c = p;
    }
    path.push(p0);
    path.push(from);
    path.reverse();
    path.dedup_by(|a, b| (*a - *b).length() < 0.5);
    path
}

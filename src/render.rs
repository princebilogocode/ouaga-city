use crate::game::{Driver, Game, VKind};
use crate::empire::fcfa;
use crate::map::Map;
use crate::net::Remote;
use eframe::egui::{self as egui};
use egui::{pos2, vec2, Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke, Vec2};

pub struct Cam {
    pub center: Pos2,
    pub scale: f32,
    pub screen: Rect,
}
impl Cam {
    pub fn w2s(&self, p: Pos2) -> Pos2 {
        self.screen.center() + (p - self.center) * self.scale
    }
    pub fn view(&self) -> (Pos2, Pos2) {
        let h = self.screen.size() / self.scale * 0.5 + vec2(30.0, 30.0);
        (self.center - h, self.center + h)
    }
}

pub const GROUND: Color32 = Color32::from_rgb(201, 160, 112);
const LATERITE: Color32 = Color32::from_rgb(178, 98, 58);
const ASPHALT: Color32 = Color32::from_rgb(78, 78, 82);
const ASPHALT_EDGE: Color32 = Color32::from_rgb(150, 132, 110);
const WATER: Color32 = Color32::from_rgb(70, 120, 150);
const GREEN: Color32 = Color32::from_rgb(120, 150, 70);
const AIRPORT: Color32 = Color32::from_rgb(196, 186, 150);

fn darken(c: Color32, f: f32) -> Color32 {
    Color32::from_rgb((c.r() as f32 * f) as u8, (c.g() as f32 * f) as u8, (c.b() as f32 * f) as u8)
}

fn area_color(k: u8) -> Color32 {
    match k {
        0 => WATER,
        1 => GREEN,
        _ => AIRPORT,
    }
}

fn draw_areas(p: &Painter, map: &Map, cam: &Cam, lo: Pos2, hi: Pos2) {
    for a in &map.areas {
        if a.max.x < lo.x || a.min.x > hi.x || a.max.y < lo.y || a.min.y > hi.y {
            continue;
        }
        let mut mesh = Mesh::default();
        let c = area_color(a.kind);
        for t in &a.tris {
            let i = mesh.vertices.len() as u32;
            for v in t {
                mesh.colored_vertex(cam.w2s(*v), c);
            }
            mesh.add_triangle(i, i + 1, i + 2);
        }
        p.add(Shape::mesh(mesh));
    }
}

fn draw_roads(p: &Painter, map: &Map, cam: &Cam, lo: Pos2, hi: Pos2, min_px: f32, kinds: std::ops::RangeInclusive<u8>, detail: bool) {
    for pass in 0..2 {
        for k in kinds.clone().rev() {
            for r in map.roads.iter().filter(|r| r.kind == k) {
                if r.max.x < lo.x || r.min.x > hi.x || r.max.y < lo.y || r.min.y > hi.y {
                    continue;
                }
                let w = (r.half_width() * 2.0 * cam.scale).max(min_px);
                let pts: Vec<Pos2> = r.pts.iter().map(|q| cam.w2s(*q)).collect();
                let (color, edge) = if r.kind == 4 { (LATERITE, darken(LATERITE, 0.85)) } else { (ASPHALT, ASPHALT_EDGE) };
                if pass == 0 {
                    if detail {
                        p.add(Shape::line(pts.clone(), Stroke::new(w + 2.0 * cam.scale.min(2.0), edge)));
                        for q in &pts {
                            p.circle_filled(*q, w * 0.5 + cam.scale.min(2.0), edge);
                        }
                    }
                } else {
                    p.add(Shape::line(pts.clone(), Stroke::new(w, color)));
                    if detail {
                        for q in &pts {
                            p.circle_filled(*q, w * 0.5, color);
                        }
                        if r.kind <= 2 && cam.scale > 1.5 {
                            // dashed centre line
                            for s in pts.windows(2) {
                                let d = s[1] - s[0];
                                let len = d.length();
                                let n = (len / (6.0 * cam.scale)) as i32;
                                for i in 0..n {
                                    if i % 2 == 0 {
                                        let a = s[0] + d * (i as f32 / n as f32);
                                        let b = s[0] + d * ((i as f32 + 0.6) / n as f32);
                                        p.line_segment([a, b], Stroke::new(0.25 * cam.scale, Color32::from_rgb(230, 220, 160)));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn rect_shape(cam: &Cam, c: Pos2, ang: f32, hl: f32, hw: f32) -> Vec<Pos2> {
    let f = vec2(ang.cos(), ang.sin());
    let r = vec2(-f.y, f.x);
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .iter()
        .map(|(a, b)| cam.w2s(c + f * hl * *a + r * hw * *b))
        .collect()
}

pub fn draw_vehicle(p: &Painter, cam: &Cam, pos: Pos2, ang: f32, kind: VKind, color: Color32, dead: bool, flash: bool) {
    let (hl, hw, ..) = kind.spec();
    let color = if dead { Color32::from_rgb(40, 36, 34) } else { color };
    let f = vec2(ang.cos(), ang.sin());
    let r = vec2(-f.y, f.x);
    // shadow
    let sh: Vec<Pos2> = rect_shape(cam, pos + vec2(0.5, 0.6), ang, hl, hw);
    p.add(Shape::convex_polygon(sh, Color32::from_black_alpha(60), Stroke::NONE));
    if kind == VKind::Moto {
        p.add(Shape::convex_polygon(rect_shape(cam, pos, ang, hl, hw * 0.6), Color32::from_rgb(30, 30, 30), Stroke::NONE));
        p.add(Shape::convex_polygon(rect_shape(cam, pos + f * 0.1, ang, hl * 0.55, hw), color, Stroke::NONE));
        // rider
        p.circle_filled(cam.w2s(pos - f * 0.25), 0.42 * cam.scale, Color32::from_rgb(60, 40, 30));
        p.circle_filled(cam.w2s(pos - f * 0.2), 0.28 * cam.scale, Color32::from_rgb(20, 20, 25));
        return;
    }
    p.add(Shape::convex_polygon(rect_shape(cam, pos, ang, hl, hw), color, Stroke::new(1.0, darken(color, 0.6))));
    let glass = if dead { Color32::from_rgb(20, 20, 20) } else { Color32::from_rgb(40, 60, 80) };
    if kind == VKind::Minibus {
        p.add(Shape::convex_polygon(rect_shape(cam, pos + f * (hl - 0.6), ang, 0.35, hw * 0.85), glass, Stroke::NONE));
        p.add(Shape::convex_polygon(rect_shape(cam, pos - f * 0.3, ang, hl * 0.7, hw * 0.75), Color32::from_rgb(200, 200, 195), Stroke::NONE));
    } else {
        p.add(Shape::convex_polygon(rect_shape(cam, pos + f * hl * 0.3, ang, hl * 0.18, hw * 0.8), glass, Stroke::NONE));
        p.add(Shape::convex_polygon(rect_shape(cam, pos - f * hl * 0.55, ang, hl * 0.12, hw * 0.75), glass, Stroke::NONE));
    }
    if kind == VKind::Taxi {
        p.add(Shape::convex_polygon(rect_shape(cam, pos - f * 0.2, ang, 0.3, 0.5), Color32::from_rgb(240, 220, 60), Stroke::NONE));
    }
    if kind == VKind::Police {
        let (a, b) = if flash { (Color32::from_rgb(255, 40, 40), Color32::from_rgb(40, 80, 255)) } else { (Color32::from_rgb(40, 80, 255), Color32::from_rgb(255, 40, 40)) };
        p.circle_filled(cam.w2s(pos - f * 0.2 + r * 0.4), 0.3 * cam.scale, a);
        p.circle_filled(cam.w2s(pos - f * 0.2 - r * 0.4), 0.3 * cam.scale, b);
        p.add(Shape::convex_polygon(rect_shape(cam, pos - f * 0.9, ang, 0.5, hw * 0.98), Color32::WHITE, Stroke::NONE));
    }
    if dead {
        p.circle_filled(cam.w2s(pos), 1.3 * cam.scale, Color32::from_rgba_unmultiplied(255, 120, 20, 120));
    }
}

fn draw_ped(p: &Painter, cam: &Cam, pos: Pos2, ang: f32, color: Color32, down: bool) {
    if down {
        p.circle_filled(cam.w2s(pos), 0.5 * cam.scale, Color32::from_rgb(110, 60, 50));
        p.circle_filled(cam.w2s(pos + vec2(0.4, 0.0)), 0.25 * cam.scale, color);
        return;
    }
    let f = vec2(ang.cos(), ang.sin());
    p.circle_filled(cam.w2s(pos + vec2(0.2, 0.25)), 0.45 * cam.scale, Color32::from_black_alpha(50));
    p.circle_filled(cam.w2s(pos), 0.45 * cam.scale, color);
    p.circle_filled(cam.w2s(pos + f * 0.1), 0.25 * cam.scale, Color32::from_rgb(70, 45, 30));
}

pub fn night_factor(h: f32) -> f32 {
    if (6.5..=17.5).contains(&h) {
        0.0
    } else if h > 17.5 && h < 19.5 {
        (h - 17.5) / 2.0
    } else if h > 4.5 && h < 6.5 {
        (6.5 - h) / 2.0
    } else {
        1.0
    }
}

fn star(p: &Painter, c: Pos2, r: f32, fill: Color32) {
    let mut pts = Vec::new();
    for i in 0..10 {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let rr = if i % 2 == 0 { r } else { r * 0.45 };
        pts.push(c + vec2(a.cos(), a.sin()) * rr);
    }
    // non-convex: draw as fan from centre
    for i in 0..10 {
        p.add(Shape::convex_polygon(vec![c, pts[i], pts[(i + 1) % 10]], fill, Stroke::NONE));
    }
}

fn panel(p: &Painter, r: Rect) {
    p.rect_filled(r, 8.0, Color32::from_black_alpha(150));
}

fn outlined(p: &Painter, pos: Pos2, align: Align2, text: &str, size: f32, color: Color32) -> Rect {
    let f = FontId::proportional(size);
    for d in [vec2(-1.5, 0.0), vec2(1.5, 0.0), vec2(0.0, -1.5), vec2(0.0, 1.5), vec2(1.5, 1.5)] {
        p.text(pos + d, align, text, f.clone(), Color32::from_black_alpha(200));
    }
    p.text(pos, align, text, f, color)
}

pub struct Hud<'a> {
    pub map: &'a Map,
    pub g: &'a Game,
    pub screen: Rect,
    pub remotes: &'a [Remote],
    pub net_status: Option<&'a str>,
    pub zone: &'a str,
    pub cam_yaw: f32,
    pub objective: Option<(&'a str, Option<Pos2>)>,
    pub worth: i64,
    pub income: i64,
    pub photo: Option<egui::TextureId>,
    pub prompt: Option<&'a str>,
    pub biz: &'a [(Pos2, bool)],
}

fn bar(p: &Painter, r: Rect, frac: f32, fill: Color32) {
    p.rect_filled(r, 3.0, Color32::from_black_alpha(170));
    let w = (r.width() - 4.0) * frac.clamp(0.0, 1.0);
    p.rect_filled(Rect::from_min_size(r.min + vec2(2.0, 2.0), vec2(w, r.height() - 4.0)), 2.0, fill);
}

pub fn draw_hud(p: &Painter, h: &Hud) {
    let g = h.g;
    let screen = h.screen;
    let gold = Color32::from_rgb(240, 190, 40);

    // ---- top left: Bila's face, health, zone
    let ph = Rect::from_min_size(screen.min + vec2(16.0, 16.0), vec2(76.0, 76.0));
    p.rect_filled(ph.expand(3.0), 8.0, gold);
    match h.photo {
        Some(id) => {
            p.image(id, ph, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => draw_portrait(p, ph.center(), 36.0, Color32::from_rgb(62, 40, 29), Color32::from_rgb(170, 30, 36)),
    }
    outlined(p, ph.right_top() + vec2(12.0, -2.0), Align2::LEFT_TOP, "BILA", 22.0, gold);
    bar(p, Rect::from_min_size(ph.right_top() + vec2(12.0, 28.0), vec2(170.0, 14.0)), g.player.health / 100.0, Color32::from_rgb(70, 200, 90));
    if let Some(vi) = g.player.vehicle {
        let v = &g.vehicles[vi];
        bar(p, Rect::from_min_size(ph.right_top() + vec2(12.0, 48.0), vec2(170.0, 10.0)), v.health / 100.0, Color32::from_rgb(70, 150, 240));
        outlined(p, ph.right_top() + vec2(12.0, 62.0), Align2::LEFT_TOP, v.kind.name(), 14.0, Color32::WHITE);
    } else {
        outlined(p, ph.right_top() + vec2(12.0, 48.0), Align2::LEFT_TOP, "À pied · F pour monter", 14.0, Color32::WHITE);
    }
    outlined(p, ph.left_bottom() + vec2(0.0, 14.0), Align2::LEFT_TOP, h.zone, 20.0, Color32::from_rgb(255, 236, 200));

    // messages
    let mut y = ph.bottom() + 50.0;
    for (m, t) in g.messages.iter() {
        let a = (t.min(1.0) * 255.0) as u8;
        let galley = p.layout(m.clone(), FontId::proportional(15.0), Color32::WHITE, 420.0);
        let r = Rect::from_min_size(pos2(screen.left() + 16.0, y), galley.size() + vec2(16.0, 10.0));
        p.rect_filled(r, 6.0, Color32::from_black_alpha((a as f32 * 0.6) as u8));
        p.galley(r.min + vec2(8.0, 5.0), galley, Color32::from_rgba_unmultiplied(255, 255, 255, a));
        y = r.bottom() + 6.0;
    }

    // ---- top right: clock, money, fortune, wanted
    let hh = g.hours as i32;
    let mm = ((g.hours - hh as f32) * 60.0) as i32;
    let tr = screen.right_top() + vec2(-20.0, 14.0);
    outlined(p, tr, Align2::RIGHT_TOP, &format!("{hh:02}:{mm:02}"), 20.0, Color32::WHITE);
    outlined(p, tr + vec2(0.0, 26.0), Align2::RIGHT_TOP, &fcfa(g.player.money), 30.0, Color32::from_rgb(110, 225, 110));
    outlined(p, tr + vec2(0.0, 64.0), Align2::RIGHT_TOP, &format!("Fortune {}", fcfa(h.worth)), 15.0, Color32::from_rgb(255, 230, 160));
    if h.income > 0 {
        outlined(p, tr + vec2(0.0, 84.0), Align2::RIGHT_TOP, &format!("+{} / heure", fcfa(h.income)), 14.0, Color32::from_rgb(160, 240, 160));
    }
    for i in 0..5 {
        let c = tr + vec2(-12.0 - i as f32 * 30.0, 120.0);
        let on = (5 - i) as u8 <= g.player.wanted;
        star(p, c, 13.0, if on { Color32::from_rgb(255, 210, 40) } else { Color32::from_black_alpha(120) });
    }
    if let Some(s) = h.net_status {
        outlined(p, tr + vec2(0.0, 142.0), Align2::RIGHT_TOP, s, 13.0, Color32::from_rgb(180, 230, 255));
    }
    if g.player.arrest_timer > 0.0 {
        outlined(p, screen.center_top() + vec2(0.0, 120.0), Align2::CENTER_TOP, "LA POLICE T'ARRÊTE… fuis !", 24.0, Color32::from_rgb(255, 80, 80));
    }

    // ---- objective
    let obj: Option<(String, Option<Pos2>)> = match h.objective {
        Some((t, p)) => Some((t.to_string(), p)),
        None => g.mission.as_ref().map(|m| {
            let d = (m.to - g.player_pos()).length();
            (format!("Livrer à {} · {:.0} m · {:.0} s · {}", m.to_name, d, m.time_left.max(0.0), fcfa(m.reward)), Some(m.to))
        }),
    };
    if let Some((text, target)) = obj {
        if !text.is_empty() {
            let galley = p.layout_no_wrap(text.clone(), FontId::proportional(17.0), Color32::from_rgb(120, 220, 255));
            let mr = Rect::from_center_size(screen.center_top() + vec2(0.0, 34.0), vec2(galley.size().x + 44.0, 42.0));
            panel(p, mr);
            p.rect_stroke(mr, 8.0, Stroke::new(1.5, Color32::from_rgb(60, 200, 255)), egui::StrokeKind::Inside);
            p.galley(mr.center() - galley.size() * 0.5, galley, Color32::WHITE);
        }
        if let Some(t) = target {
            let w = t - g.player_pos();
            let rel = w.y.atan2(w.x) - h.cam_yaw;
            let dir = vec2(rel.sin(), -rel.cos());
            let c = screen.center_top() + vec2(0.0, 96.0);
            let rgt = vec2(-dir.y, dir.x);
            p.add(Shape::convex_polygon(vec![c + dir * 18.0, c - dir * 10.0 + rgt * 12.0, c - dir * 10.0 - rgt * 12.0], Color32::from_rgb(60, 200, 255), Stroke::new(2.0, Color32::BLACK)));
            outlined(p, c + vec2(0.0, 22.0), Align2::CENTER_TOP, &format!("{:.0} m", w.length()), 13.0, Color32::WHITE);
        }
    }

    // ---- prompt (buy a business...)
    if let Some(t) = h.prompt {
        let galley = p.layout_no_wrap(t.to_string(), FontId::proportional(18.0), Color32::WHITE);
        let r = Rect::from_center_size(screen.center_bottom() + vec2(0.0, -120.0), galley.size() + vec2(40.0, 20.0));
        p.rect_filled(r, 8.0, Color32::from_black_alpha(190));
        p.rect_stroke(r, 8.0, Stroke::new(1.5, Color32::from_rgb(90, 230, 120)), egui::StrokeKind::Inside);
        p.galley(r.center() - galley.size() * 0.5, galley, Color32::WHITE);
    }

    // ---- bottom right: speedometer
    if let Some(vi) = g.player.vehicle {
        let v = &g.vehicles[vi];
        let kmh = v.speed().abs() * 3.6;
        let c = screen.right_bottom() + vec2(-110.0, -100.0);
        p.circle_filled(c, 82.0, Color32::from_black_alpha(140));
        let max = v.kind.spec().2 * 3.6;
        let a0 = std::f32::consts::PI * 0.75;
        let sweep = std::f32::consts::PI * 1.5;
        let arc = |frac: f32| -> Vec<Pos2> {
            (0..=40).map(|i| {
                let a = a0 + sweep * frac * i as f32 / 40.0;
                c + vec2(a.cos(), a.sin()) * 68.0
            }).collect()
        };
        p.add(Shape::line(arc(1.0), Stroke::new(8.0, Color32::from_white_alpha(40))));
        let frac = (kmh / max).clamp(0.0, 1.0);
        let col = if frac > 0.85 { Color32::from_rgb(255, 90, 60) } else { Color32::from_rgb(90, 220, 255) };
        p.add(Shape::line(arc(frac.max(0.001)), Stroke::new(8.0, col)));
        outlined(p, c + vec2(0.0, -6.0), Align2::CENTER_CENTER, &format!("{:.0}", kmh), 38.0, Color32::WHITE);
        outlined(p, c + vec2(0.0, 26.0), Align2::CENTER_CENTER, "km/h", 14.0, Color32::from_white_alpha(200));
    }

    draw_minimap(p, h);
    p.text(screen.right_bottom() + vec2(-8.0, -4.0), Align2::RIGHT_BOTTOM, "Carte © OpenStreetMap · textures ambientCG · ciel Poly Haven", FontId::proportional(10.0), Color32::from_white_alpha(140));
}

fn draw_minimap(p: &Painter, h: &Hud) {
    let g = h.g;
    let size = 250.0;
    let r = Rect::from_min_size(h.screen.left_bottom() + vec2(18.0, -size - 22.0), vec2(size, size));
    p.rect_filled(r.expand(4.0), 14.0, Color32::from_black_alpha(170));
    let pp = g.player_pos();
    let cam = Cam { center: pp, scale: size / 1400.0, screen: r };
    let clip = p.with_clip_rect(r);
    clip.rect_filled(r, 12.0, darken(GROUND, 0.75));
    let (lo, hi) = cam.view();
    draw_areas(&clip, h.map, &cam, lo, hi);
    draw_roads(&clip, h.map, &cam, lo, hi, 0.6, 4..=4, false);
    draw_roads(&clip, h.map, &cam, lo, hi, 1.6, 0..=3, false);
    for (bp, owned) in h.biz {
        let s = cam.w2s(*bp);
        if !r.shrink(6.0).contains(s) {
            continue;
        }
        clip.circle_filled(s, 7.0, if *owned { Color32::from_rgb(40, 150, 60) } else { Color32::from_rgb(90, 230, 120) });
        clip.text(s, Align2::CENTER_CENTER, "$", FontId::proportional(11.0), Color32::BLACK);
    }
    for v in &g.vehicles {
        if v.driver == Driver::Police {
            clip.circle_filled(cam.w2s(v.pos), 3.5, if (g.time * 6.0) as i32 % 2 == 0 { Color32::RED } else { Color32::BLUE });
        }
    }
    if g.mission.is_none() && !g.story {
        for s in &g.spots {
            clip.circle_filled(cam.w2s(*s), 3.5, Color32::from_rgb(255, 210, 0));
        }
    }
    for rm in h.remotes {
        clip.circle_filled(cam.w2s(pos2(rm.x, rm.y)), 4.5, Color32::from_rgb(rm.color[0], rm.color[1], rm.color[2]));
    }
    let target = h.objective.and_then(|o| o.1).or(g.mission.as_ref().map(|m| m.to));
    if let Some(t) = target {
        let s = cam.w2s(t).clamp(r.min + vec2(7.0, 7.0), r.max - vec2(7.0, 7.0));
        clip.circle_filled(s, 6.0, Color32::from_rgb(60, 200, 255));
        clip.circle_stroke(s, 6.0, Stroke::new(1.5, Color32::WHITE));
    }
    let f = vec2(g.player.angle.cos(), g.player.angle.sin());
    let c = r.center();
    let rg = vec2(-f.y, f.x);
    clip.add(Shape::convex_polygon(vec![c + f * 9.0, c - f * 6.0 + rg * 6.0, c - f * 6.0 - rg * 6.0], Color32::WHITE, Stroke::new(1.5, Color32::BLACK)));
    p.rect_stroke(r, 12.0, Stroke::new(2.0, Color32::from_rgb(240, 190, 40)), egui::StrokeKind::Outside);
    outlined(p, r.center_top() + vec2(0.0, 6.0), Align2::CENTER_TOP, "N", 13.0, Color32::WHITE);
}

pub fn draw_full_map(p: &Painter, map: &Map, g: &Game, screen: Rect, remotes: &[Remote]) {
    p.rect_filled(screen, 0.0, Color32::from_rgb(30, 28, 24));
    let ext = map.max - map.min;
    let scale = ((screen.width() - 40.0) / ext.x).min((screen.height() - 60.0) / ext.y);
    let cam = Cam { center: map.min + ext * 0.5, scale, screen };
    let r = Rect::from_center_size(screen.center(), ext * scale);
    p.rect_filled(r, 4.0, darken(GROUND, 0.85));
    let (lo, hi) = (map.min, map.max);
    draw_areas(p, map, &cam, lo, hi);
    draw_roads(p, map, &cam, lo, hi, 0.5, 4..=4, false);
    draw_roads(p, map, &cam, lo, hi, 1.5, 0..=3, false);
    for a in &map.areas {
        if !a.name.is_empty() {
            let c = a.min + (a.max - a.min) * 0.5;
            p.text(cam.w2s(c), Align2::CENTER_CENTER, &a.name, FontId::proportional(12.0), Color32::from_rgb(200, 230, 255));
        }
    }
    for q in &map.pois {
        let s = cam.w2s(q.pos);
        match q.kind {
            'Q' => {
                p.text(s, Align2::CENTER_CENTER, &q.name, FontId::proportional(14.0), Color32::from_rgb(255, 240, 200));
            }
            'P' => {
                p.circle_filled(s, 3.0, Color32::from_rgb(80, 120, 255));
            }
            'H' => {
                p.circle_filled(s, 2.5, Color32::from_rgb(255, 90, 90));
            }
            'M' => {
                p.circle_filled(s, 3.5, Color32::from_rgb(255, 210, 0));
            }
            _ => {}
        }
    }
    if let Some(m) = &g.mission {
        p.circle_filled(cam.w2s(m.to), 6.0, Color32::from_rgb(60, 200, 255));
    }
    for rm in remotes {
        let s = cam.w2s(pos2(rm.x, rm.y));
        p.circle_filled(s, 5.0, Color32::from_rgb(rm.color[0], rm.color[1], rm.color[2]));
        p.text(s + vec2(0.0, -8.0), Align2::CENTER_BOTTOM, &rm.name, FontId::proportional(12.0), Color32::WHITE);
    }
    let s = cam.w2s(g.player_pos());
    p.circle_filled(s, 6.0, Color32::WHITE);
    p.circle_stroke(s, 9.0, Stroke::new(2.0, Color32::from_rgb(255, 60, 60)));
    p.text(screen.center_top() + vec2(0.0, 10.0), Align2::CENTER_TOP, "Ouagadougou — M pour fermer · jaune : marchés (courses) · bleu : police · rouge : santé", FontId::proportional(15.0), Color32::WHITE);
    p.text(screen.right_bottom() + vec2(-10.0, -6.0), Align2::RIGHT_BOTTOM, "Données cartographiques © contributeurs OpenStreetMap (ODbL)", FontId::proportional(11.0), Color32::from_white_alpha(170));
    let _ = Vec2::ZERO;
}

pub fn draw_portrait(p: &Painter, c: Pos2, r: f32, skin: Color32, clothes: Color32) {
    p.circle_filled(c, r + 3.0, Color32::from_rgb(240, 190, 40));
    p.circle_filled(c, r, Color32::from_rgb(60, 80, 110));
    let clip = p.with_clip_rect(Rect::from_center_size(c, vec2(r * 2.0, r * 2.0)));
    clip.circle_filled(c + vec2(0.0, r * 1.05), r * 0.8, clothes);
    clip.rect_filled(Rect::from_center_size(c + vec2(0.0, r * 0.35), vec2(r * 0.3, r * 0.4)), 2.0, skin);
    p.circle_filled(c + vec2(0.0, -r * 0.1), r * 0.45, skin);
    p.circle_filled(c + vec2(0.0, -r * 0.45), r * 0.35, Color32::from_rgb(25, 20, 18));
    p.circle_filled(c + vec2(0.0, -r * 0.1), r * 0.42, skin);
    for s in [-1.0, 1.0] {
        p.circle_filled(c + vec2(s * r * 0.15, -r * 0.15), r * 0.06, Color32::WHITE);
        p.circle_filled(c + vec2(s * r * 0.15, -r * 0.15), r * 0.03, Color32::BLACK);
    }
    p.line_segment([c + vec2(-r * 0.12, r * 0.12), c + vec2(r * 0.12, r * 0.12)], Stroke::new(2.0, Color32::from_rgb(60, 30, 25)));
}

#[allow(clippy::too_many_arguments)]
pub fn draw_dialog(p: &Painter, screen: Rect, speaker: &str, text: &str, skin: Color32, clothes: Color32, photo: Option<egui::TextureId>) {
    let r = Rect::from_min_size(pos2(screen.left() + 290.0, screen.bottom() - 190.0), vec2(screen.width() - 600.0, 150.0));
    p.rect_filled(r, 12.0, Color32::from_black_alpha(210));
    p.rect_stroke(r, 12.0, Stroke::new(2.0, Color32::from_rgb(240, 190, 40)), egui::StrokeKind::Inside);
    let pc = r.left_center() + vec2(72.0, 0.0);
    match photo {
        Some(id) => {
            let pr = Rect::from_center_size(pc, vec2(110.0, 110.0));
            p.rect_filled(pr.expand(3.0), 8.0, Color32::from_rgb(240, 190, 40));
            p.image(id, pr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => draw_portrait(p, pc, 52.0, skin, clothes),
    }
    p.text(r.min + vec2(146.0, 16.0), Align2::LEFT_TOP, speaker, FontId::proportional(21.0), Color32::from_rgb(240, 190, 40));
    let galley = p.layout(text.to_string(), FontId::proportional(18.0), Color32::WHITE, r.width() - 170.0);
    p.galley(r.min + vec2(146.0, 50.0), galley, Color32::WHITE);
    p.text(r.right_bottom() + vec2(-14.0, -10.0), Align2::RIGHT_BOTTOM, "Entrée / Espace / clic : continuer", FontId::proportional(13.0), Color32::from_white_alpha(170));
}

pub fn draw_banner(p: &Painter, screen: Rect, text: &str, t: f32) {
    let a = (t.min(1.0) * 255.0) as u8;
    let c = screen.center() - vec2(0.0, screen.height() * 0.22);
    let galley = p.layout_no_wrap(text.to_string(), FontId::proportional(34.0), Color32::WHITE);
    let r = Rect::from_center_size(c, galley.size() + vec2(50.0, 24.0));
    p.rect_filled(r, 8.0, Color32::from_black_alpha((a as f32 * 0.7) as u8));
    p.text(c, Align2::CENTER_CENTER, text, FontId::proportional(34.0), Color32::from_rgba_unmultiplied(255, 215, 90, a));
}

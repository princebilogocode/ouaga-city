use crate::game::{Driver, Game, VKind};
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

#[allow(clippy::too_many_arguments)]
pub fn draw_world(p: &Painter, map: &Map, g: &Game, cam: &Cam, remotes: &[Remote]) {
    p.rect_filled(cam.screen, 0.0, GROUND);
    let (lo, hi) = cam.view();
    draw_areas(p, map, cam, lo, hi);
    draw_roads(p, map, cam, lo, hi, 1.0, 0..=4, true);

    // mission spots and destination
    let pulse = (g.time * 4.0).sin() * 0.5 + 0.5;
    if g.mission.is_none() {
        for s in &g.spots {
            if s.x > lo.x && s.x < hi.x && s.y > lo.y && s.y < hi.y {
                p.circle_filled(cam.w2s(*s), (8.0 + pulse * 2.0) * cam.scale, Color32::from_rgba_unmultiplied(255, 210, 0, 70));
                p.circle_stroke(cam.w2s(*s), 8.0 * cam.scale, Stroke::new(2.0, Color32::from_rgb(255, 210, 0)));
                p.text(cam.w2s(*s), Align2::CENTER_CENTER, "COURSE", FontId::proportional(12.0), Color32::BLACK);
            }
        }
    }
    if let Some(m) = &g.mission {
        p.circle_filled(cam.w2s(m.to), (12.0 + pulse * 3.0) * cam.scale, Color32::from_rgba_unmultiplied(60, 200, 255, 80));
        p.circle_stroke(cam.w2s(m.to), 12.0 * cam.scale, Stroke::new(3.0, Color32::from_rgb(60, 200, 255)));
    }

    for pd in &g.peds {
        draw_ped(p, cam, pd.pos, pd.angle, pd.color, pd.down > 0.0);
    }
    let flash = (g.time * 6.0) as i32 % 2 == 0;
    for v in &g.vehicles {
        if v.pos.x > lo.x && v.pos.x < hi.x && v.pos.y > lo.y && v.pos.y < hi.y {
            draw_vehicle(p, cam, v.pos, v.angle, v.kind, v.color, v.dead, flash);
        }
    }
    for r in remotes {
        let pos = pos2(r.x, r.y);
        let c = Color32::from_rgb(r.color[0], r.color[1], r.color[2]);
        if r.kind == 0 {
            draw_ped(p, cam, pos, r.angle, c, false);
        } else {
            draw_vehicle(p, cam, pos, r.angle, VKind::from_net(r.kind), c, false, flash);
        }
        let s = cam.w2s(pos) - vec2(0.0, 3.0 * cam.scale + 8.0);
        p.text(s, Align2::CENTER_BOTTOM, &r.name, FontId::proportional(13.0), Color32::WHITE);
    }
    if let Some(vi) = g.player.vehicle {
        let v = &g.vehicles[vi];
        p.circle_stroke(cam.w2s(v.pos), (v.kind.spec().0 + 1.2) * cam.scale, Stroke::new(2.0, Color32::from_white_alpha(170)));
    }
    if g.player.vehicle.is_none() {
        draw_ped(p, cam, g.player.pos, g.player.angle, g.player.color, false);
        p.circle_stroke(cam.w2s(g.player.pos), 0.9 * cam.scale, Stroke::new(1.5, Color32::WHITE));
    }

    // buildings and trees above street level
    let mut blds = Vec::new();
    map.bld_grid.query(lo - vec2(30.0, 30.0), hi + vec2(30.0, 30.0), |id| blds.push(id));
    for &id in &blds {
        let b = &map.buildings[id as usize];
        let cs: Vec<Pos2> = b.corners().iter().map(|c| cam.w2s(*c + vec2(2.0, 2.5))).collect();
        p.add(Shape::convex_polygon(cs, Color32::from_black_alpha(55), Stroke::NONE));
    }
    for &id in &blds {
        let b = &map.buildings[id as usize];
        let cs = b.corners();
        let sc: Vec<Pos2> = cs.iter().map(|c| cam.w2s(*c)).collect();
        p.add(Shape::convex_polygon(sc.clone(), b.color, Stroke::new(1.0, darken(b.color, 0.7))));
        if cam.scale > 1.2 {
            match b.roof {
                0 => {
                    // ridge of a tin roof
                    let m1 = sc[0] + (sc[3] - sc[0]) * 0.5;
                    let m2 = sc[1] + (sc[2] - sc[1]) * 0.5;
                    p.line_segment([m1, m2], Stroke::new(1.5, darken(b.color, 0.75)));
                }
                1 => {
                    let inner: Vec<Pos2> = cs.iter().map(|c| cam.w2s(b.c + (*c - b.c) * 0.7)).collect();
                    p.add(Shape::convex_polygon(inner, darken(b.color, 0.92), Stroke::NONE));
                }
                _ => {}
            }
        }
    }
    let mut trees = Vec::new();
    map.tree_grid.query(lo, hi, |id| trees.push(id));
    for id in trees {
        let t = &map.trees[id as usize];
        p.circle_filled(cam.w2s(t.p + vec2(1.0, 1.2)), t.r * cam.scale, Color32::from_black_alpha(45));
        p.circle_filled(cam.w2s(t.p), t.r * cam.scale, Color32::from_rgb(70, 110, 45));
        p.circle_filled(cam.w2s(t.p - vec2(t.r * 0.25, t.r * 0.25)), t.r * 0.55 * cam.scale, Color32::from_rgb(95, 140, 60));
    }

    // night
    let night = night_factor(g.hours);
    if night > 0.0 {
        p.rect_filled(cam.screen, 0.0, Color32::from_rgba_unmultiplied(10, 15, 45, (night * 150.0) as u8));
        for v in &g.vehicles {
            if v.dead || v.driver == Driver::None || v.pos.x < lo.x || v.pos.x > hi.x || v.pos.y < lo.y || v.pos.y > hi.y {
                continue;
            }
            let f = v.fwd();
            let r = vec2(-f.y, f.x);
            let hl = v.kind.spec().0;
            let a = cam.w2s(v.pos + f * hl);
            let b = cam.w2s(v.pos + f * (hl + 18.0) + r * 5.0);
            let c = cam.w2s(v.pos + f * (hl + 18.0) - r * 5.0);
            p.add(Shape::convex_polygon(vec![a, b, c], Color32::from_rgba_unmultiplied(255, 240, 170, (night * 45.0) as u8), Stroke::NONE));
        }
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
    p.rect_filled(r, 6.0, Color32::from_black_alpha(150));
}

#[allow(clippy::too_many_arguments)]
pub fn draw_hud(p: &Painter, map: &Map, g: &Game, screen: Rect, remotes: &[Remote], net_status: Option<&str>, zone: &str, cam_yaw: f32, objective: Option<(&str, Option<Pos2>)>) {
    let white = Color32::WHITE;
    // top-left: money, health, time
    let r = Rect::from_min_size(screen.min + vec2(12.0, 12.0), vec2(260.0, 96.0));
    panel(p, r);
    p.text(r.min + vec2(160.0, 12.0), Align2::LEFT_TOP, "BILA", FontId::proportional(14.0), Color32::from_rgb(240, 190, 40));
    p.text(r.min + vec2(12.0, 8.0), Align2::LEFT_TOP, format!("{} FCFA", g.player.money), FontId::proportional(22.0), Color32::from_rgb(120, 230, 120));
    let hb = Rect::from_min_size(r.min + vec2(12.0, 40.0), vec2(150.0, 10.0));
    p.rect_filled(hb, 3.0, Color32::from_rgb(60, 20, 20));
    p.rect_filled(Rect::from_min_size(hb.min, vec2(150.0 * (g.player.health / 100.0).clamp(0.0, 1.0), 10.0)), 3.0, Color32::from_rgb(220, 50, 50));
    let hh = g.hours as i32;
    let mm = ((g.hours - hh as f32) * 60.0) as i32;
    p.text(r.min + vec2(175.0, 36.0), Align2::LEFT_TOP, format!("{hh:02}:{mm:02}"), FontId::monospace(16.0), white);
    if let Some(vi) = g.player.vehicle {
        let v = &g.vehicles[vi];
        p.text(
            r.min + vec2(12.0, 58.0),
            Align2::LEFT_TOP,
            format!("{} · {:.0} km/h · état {:.0}%", v.kind.name(), v.speed().abs() * 3.6, v.health.max(0.0)),
            FontId::proportional(14.0),
            white,
        );
    } else {
        p.text(r.min + vec2(12.0, 58.0), Align2::LEFT_TOP, "À pied (F : monter)", FontId::proportional(14.0), white);
    }
    p.text(r.min + vec2(12.0, 76.0), Align2::LEFT_TOP, zone, FontId::proportional(12.0), Color32::from_rgb(230, 210, 160));

    // wanted stars (top right)
    for i in 0..5 {
        let c = screen.right_top() + vec2(-30.0 - i as f32 * 30.0, 28.0);
        let on = (5 - i) as u8 <= g.player.wanted;
        star(p, c, 12.0, if on { Color32::from_rgb(255, 210, 40) } else { Color32::from_black_alpha(110) });
    }
    if g.player.arrest_timer > 0.0 {
        p.text(screen.center_top() + vec2(0.0, 60.0), Align2::CENTER_TOP, "LA POLICE T'ARRÊTE… fuis !", FontId::proportional(22.0), Color32::from_rgb(255, 80, 80));
    }

    // objective (story) or delivery (free mode)
    let obj: Option<(String, Option<Pos2>)> = match objective {
        Some((t, p)) => Some((t.to_string(), p)),
        None => g.mission.as_ref().map(|m| {
            let d = (m.to - g.player_pos()).length();
            (format!("Livrer à {} · {:.0} m · {:.0} s · {} FCFA", m.to_name, d, m.time_left.max(0.0), m.reward), Some(m.to))
        }),
    };
    if let Some((text, target)) = obj {
        let galley = p.layout_no_wrap(text.clone(), FontId::proportional(16.0), Color32::from_rgb(120, 220, 255));
        let mr = Rect::from_center_size(screen.center_top() + vec2(0.0, 32.0), vec2(galley.size().x + 40.0, 40.0));
        panel(p, mr);
        p.text(mr.center(), Align2::CENTER_CENTER, &text, FontId::proportional(16.0), Color32::from_rgb(120, 220, 255));
        if let Some(t) = target {
            let w = t - g.player_pos();
            let rel = w.y.atan2(w.x) - cam_yaw;
            let dir = vec2(rel.sin(), -rel.cos());
            let c = screen.center_top() + vec2(0.0, 92.0);
            let rgt = vec2(-dir.y, dir.x);
            p.add(Shape::convex_polygon(vec![c + dir * 18.0, c - dir * 10.0 + rgt * 12.0, c - dir * 10.0 - rgt * 12.0], Color32::from_rgb(60, 200, 255), Stroke::new(2.0, Color32::BLACK)));
        }
    }

    // messages (bottom left)
    let mut y = screen.bottom() - 20.0;
    for (m, t) in g.messages.iter().rev() {
        let a = (t.min(1.0) * 255.0) as u8;
        let galley_pos = pos2(screen.left() + 16.0, y);
        let rr = p.text(galley_pos, Align2::LEFT_BOTTOM, m, FontId::proportional(15.0), Color32::from_rgba_unmultiplied(255, 255, 255, a));
        p.rect_filled(rr.expand(4.0), 4.0, Color32::from_black_alpha(a / 2));
        p.text(galley_pos, Align2::LEFT_BOTTOM, m, FontId::proportional(15.0), Color32::from_rgba_unmultiplied(255, 255, 255, a));
        y -= 26.0;
    }

    if let Some(s) = net_status {
        p.text(screen.right_top() + vec2(-14.0, 50.0), Align2::RIGHT_TOP, s, FontId::proportional(13.0), Color32::from_rgb(180, 230, 255));
    }

    draw_minimap(p, map, g, screen, remotes);
    p.text(
        screen.right_bottom() + vec2(-14.0, -4.0),
        Align2::RIGHT_BOTTOM,
        "Carte © contributeurs OpenStreetMap",
        FontId::proportional(10.0),
        Color32::from_white_alpha(160),
    );
}

fn draw_minimap(p: &Painter, map: &Map, g: &Game, screen: Rect, remotes: &[Remote]) {
    let size = 210.0;
    let r = Rect::from_min_size(screen.right_bottom() - vec2(size + 14.0, size + 20.0), vec2(size, size));
    p.rect_filled(r, 8.0, Color32::from_rgb(40, 36, 30));
    let pp = g.player_pos();
    let cam = Cam { center: pp, scale: size / 1800.0, screen: r };
    let clip = p.with_clip_rect(r.shrink(2.0));
    clip.rect_filled(r, 8.0, darken(GROUND, 0.8));
    let (lo, hi) = cam.view();
    draw_areas(&clip, map, &cam, lo, hi);
    draw_roads(&clip, map, &cam, lo, hi, 1.0, 0..=3, false);
    for v in &g.vehicles {
        if v.driver == Driver::Police {
            clip.circle_filled(cam.w2s(v.pos), 3.0, if (g.time * 6.0) as i32 % 2 == 0 { Color32::RED } else { Color32::BLUE });
        }
    }
    if g.mission.is_none() {
        for s in &g.spots {
            clip.circle_filled(cam.w2s(*s), 3.0, Color32::from_rgb(255, 210, 0));
        }
    }
    for rm in remotes {
        clip.circle_filled(cam.w2s(pos2(rm.x, rm.y)), 4.0, Color32::from_rgb(rm.color[0], rm.color[1], rm.color[2]));
    }
    if let Some(m) = &g.mission {
        let mut s = cam.w2s(m.to);
        s = s.clamp(r.min + vec2(6.0, 6.0), r.max - vec2(6.0, 6.0));
        clip.circle_filled(s, 5.0, Color32::from_rgb(60, 200, 255));
    }
    let f = vec2(g.player.angle.cos(), g.player.angle.sin());
    let c = r.center();
    let rg = vec2(-f.y, f.x);
    clip.add(Shape::convex_polygon(vec![c + f * 8.0, c - f * 5.0 + rg * 5.0, c - f * 5.0 - rg * 5.0], Color32::WHITE, Stroke::new(1.0, Color32::BLACK)));
    p.rect_stroke(r, 8.0, Stroke::new(2.0, Color32::from_rgb(230, 210, 160)), egui::StrokeKind::Inside);
    p.text(r.center_top() + vec2(0.0, 4.0), Align2::CENTER_TOP, "N", FontId::proportional(12.0), Color32::WHITE);
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

pub fn draw_dialog(p: &Painter, screen: Rect, speaker: &str, text: &str, skin: Color32, clothes: Color32) {
    let r = Rect::from_min_size(pos2(screen.left() + 40.0, screen.bottom() - 170.0), vec2(screen.width() - 300.0, 140.0));
    p.rect_filled(r, 10.0, Color32::from_black_alpha(200));
    p.rect_stroke(r, 10.0, Stroke::new(2.0, Color32::from_rgb(240, 190, 40)), egui::StrokeKind::Inside);
    draw_portrait(p, r.left_center() + vec2(70.0, 0.0), 50.0, skin, clothes);
    p.text(r.min + vec2(140.0, 16.0), Align2::LEFT_TOP, speaker, FontId::proportional(20.0), Color32::from_rgb(240, 190, 40));
    let galley = p.layout(text.to_string(), FontId::proportional(18.0), Color32::WHITE, r.width() - 180.0);
    p.galley(r.min + vec2(140.0, 48.0), galley, Color32::WHITE);
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

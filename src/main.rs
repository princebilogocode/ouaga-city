#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod empire;
mod game;
mod kit;
mod life;
mod skinned;
mod statics;
mod map;
mod models;
mod people;
mod net;
mod r3d;
mod render;
mod story;

use eframe::egui;
use egui::{vec2, Align2, Color32, FontId, Key, Pos2, Rect};
use empire::{fcfa, Empire};
use game::{Game, Input};
use glam::{Mat4, Vec3};
use map::Map;
use net::{Net, Remote};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use story::{Obj, Story};

#[derive(PartialEq)]
enum Screen {
    Menu,
    Playing,
}

struct App {
    map: Arc<Map>,
    game: Option<Game>,
    story: Option<Story>,
    empire: Option<Empire>,
    screen: Screen,
    name: String,
    join_ip: String,
    net: Option<Net>,
    error: String,
    show_map: bool,
    show_empire: bool,
    paused: bool,
    shot: Option<PathBuf>,
    frame: u32,
    shot_sent: bool,
    color: Color32,
    zone: String,
    zone_timer: f32,
    r3d: r3d::Shared,
    cam_yaw: f32,
    orbit: f32,
    dist_mul: f32,
    high_cam: bool,
    cam_pos: Option<Vec3>,
    aim_yaw: f32,
    was_aiming: bool,
    fov: f32,
    steer: f32,
    pitch: f32,
    bila_face: Option<egui::TextureHandle>,
    bila_full: Option<egui::TextureHandle>,
    menu_sky: Option<egui::TextureHandle>,
    faces: Vec<(&'static str, egui::TextureHandle)>,
}

const PLAYER_COLORS: [Color32; 6] = [
    Color32::from_rgb(230, 60, 60),
    Color32::from_rgb(60, 140, 230),
    Color32::from_rgb(60, 200, 90),
    Color32::from_rgb(240, 180, 30),
    Color32::from_rgb(200, 80, 220),
    Color32::from_rgb(250, 130, 40),
];

fn rand_seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
}

fn load_tex(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(name, ci, egui::TextureOptions::LINEAR))
}

impl App {
    fn new(shot: Option<PathBuf>) -> App {
        let map = Arc::new(Map::load());
        let seed = rand_seed();
        let mut app = App {
            map,
            game: None,
            story: None,
            empire: None,
            screen: Screen::Menu,
            name: format!("Bila{}", seed % 1000),
            join_ip: String::new(),
            net: None,
            error: String::new(),
            show_map: false,
            show_empire: false,
            paused: false,
            shot,
            frame: 0,
            shot_sent: false,
            color: PLAYER_COLORS[(seed % 6) as usize],
            zone: String::new(),
            zone_timer: 0.0,
            r3d: Arc::new(Mutex::new(None)),
            cam_yaw: 0.0,
            orbit: 0.0,
            dist_mul: 1.0,
            high_cam: false,
            cam_pos: None,
            aim_yaw: 0.0,
            was_aiming: false,
            fov: 62.0,
            steer: 0.0,
            pitch: 0.12,
            bila_face: None,
            bila_full: None,
            menu_sky: None,
            faces: Vec::new(),
        };
        if app.shot.is_some() {
            if std::env::args().any(|a| a == "--story") {
                app.start_story(0);
            } else if !std::env::args().any(|a| a == "--menu") {
                app.start_free(42);
                if let (Ok(p), Some(g)) = (std::env::var("OC_POS"), app.game.as_mut()) {
                    let v: Vec<f32> = p.split(',').filter_map(|x| x.parse().ok()).collect();
                    if v.len() >= 2 {
                        let at = app.map.road_point(Pos2::new(v[0], v[1]));
                        let ang = v.get(2).copied().unwrap_or(0.0);
                        g.place_player(&app.map, at, ang);
                        app.cam_yaw = ang;
                        if std::env::var("OC_CAR").is_ok() {
                            let ahead = at + egui::vec2(ang.cos(), ang.sin()) * 7.0;
                            g.spawn_vehicle(&app.map, ahead, game::VKind::Taxi, game::Driver::Traffic, Color32::from_rgb(30, 140, 60), 9);
                            if let Some(v) = g.vehicles.last_mut() {
                                v.route = None;
                                v.pos = ahead + egui::vec2(-ang.sin(), ang.cos()) * 3.0;
                                v.angle = ang + 0.5;
                            }
                        }
                    }
                }
                // test hook: OC_INSIDE=<room> [OC_ACT=<spot name>]
                if let (Ok(r), Some(g)) = (std::env::var("OC_INSIDE"), app.game.as_mut()) {
                    let i: usize = r.parse().unwrap_or(0);
                    g.player.vehicle = None;
                    g.vehicles.clear();
                    g.outside = (g.player.pos, 0.0);
                    g.inside = Some(i);
                    let it = &statics::interiors()[i];
                    let a = statics::anchor(i);
                    g.player.pos = Pos2::new(a.x - it.size[0] / 2.0 + 1.6, a.y + 0.4);
                    g.player.angle = 0.0;
                    if let Ok(spot) = std::env::var("OC_ACT") {
                        if let Some(sp) = it.spots.iter().find(|s| s.name == spot) {
                            g.player.pos = Pos2::new(a.x + sp.x, a.y + sp.z);
                            g.player.angle = sp.yaw;
                            g.player.money += 5000;
                            g.interact(&app.map);
                        }
                    }
                }
            }
        }
        app
    }

    fn start_free(&mut self, seed: u64) {
        let g = Game::new(&self.map, seed, self.color);
        self.cam_yaw = g.player.angle;
        self.game = Some(g);
        self.story = None;
        self.empire = Some(Empire::new(&self.map));
        self.cam_pos = None;
        self.screen = Screen::Playing;
    }

    fn start_story(&mut self, from: usize) {
        self.net = None;
        let mut g = Game::new(&self.map, rand_seed(), self.color);
        g.messages.clear();
        let mut emp = Empire::new(&self.map);
        emp.persist = true;
        if from == 0 {
            Empire::reset_save();
        } else if let Some(m) = emp.load() {
            g.player.money = m.max(g.player.money);
        }
        let st = Story::new(&self.map, from, &mut g);
        self.cam_yaw = g.player.angle;
        self.game = Some(g);
        self.story = Some(st);
        self.empire = Some(emp);
        self.cam_pos = None;
        self.screen = Screen::Playing;
    }

    fn ensure_textures(&mut self, ctx: &egui::Context) {
        if self.bila_face.is_none() {
            self.bila_face = load_tex(ctx, "bila_face", include_bytes!("../assets/bila_portrait.jpg"));
            self.bila_full = load_tex(ctx, "bila_full", include_bytes!("../assets/bila_full.jpg"));
            self.menu_sky = load_tex(ctx, "menu_sky", include_bytes!("../assets/portraits/affiche.png"));
            for (who, bytes) in [
                ("Issouf", &include_bytes!("../assets/portraits/issouf.png")[..]),
                ("Mme Kaboré", &include_bytes!("../assets/portraits/kabore.png")[..]),
                ("Le Commandant", &include_bytes!("../assets/portraits/commandant.png")[..]),
            ] {
                if let Some(t) = load_tex(ctx, who, bytes) {
                    self.faces.push((who, t));
                }
            }
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        let screen = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(screen, 0.0, Color32::from_rgb(40, 28, 20));
        if let Some(t) = &self.menu_sky {
            // the sunset band of the panorama, stretched over the screen
            let img_aspect = 1376.0 / 768.0;
            let scr_aspect = screen.width() / screen.height();
            let uv = if scr_aspect < img_aspect {
                let w = scr_aspect / img_aspect;
                Rect::from_min_max(Pos2::new(0.5 - w * 0.5, 0.0), Pos2::new(0.5 + w * 0.5, 1.0))
            } else {
                let h = img_aspect / scr_aspect;
                Rect::from_min_max(Pos2::new(0.0, 0.5 - h * 0.5), Pos2::new(1.0, 0.5 + h * 0.5))
            };
            p.image(t.id(), screen, uv, Color32::WHITE);
        }
        // the real street map of Ouaga, faint, over the sky
        let ext = self.map.max - self.map.min;
        let scale = (screen.width() / ext.x).max(screen.height() / ext.y);
        let cam = render::Cam { center: self.map.min + ext * 0.5, scale, screen };
        for r in self.map.roads.iter().filter(|r| r.kind <= 2) {
            let pts: Vec<_> = r.pts.iter().map(|q| cam.w2s(*q)).collect();
            p.add(egui::Shape::line(pts, egui::Stroke::new(1.2, Color32::from_rgba_unmultiplied(255, 230, 190, 50))));
        }
        p.rect_filled(screen, 0.0, Color32::from_black_alpha(90));
        // Bila
        let left = Rect::from_center_size(screen.left_center() + vec2(screen.width() * 0.2, -20.0), vec2(360.0, 360.0));
        if let Some(t) = &self.bila_full {
            p.rect_filled(left.expand(5.0), 14.0, Color32::from_rgb(240, 190, 40));
            p.image(t.id(), left, Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        p.text(left.center_bottom() + vec2(0.0, 16.0), Align2::CENTER_TOP, "BILA", FontId::proportional(40.0), Color32::from_rgb(240, 190, 40));
        p.text(left.center_bottom() + vec2(0.0, 62.0), Align2::CENTER_TOP, "du taxi-moto à l'homme le plus riche d'Afrique", FontId::proportional(17.0), Color32::WHITE);

        let saved = story::saved_step();
        egui::Window::new("Ouaga City")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::RIGHT_CENTER, vec2(-screen.width() * 0.1, 0.0))
            .fixed_size(vec2(420.0, 0.0))
            .frame(egui::Frame::window(&ui.ctx().global_style()).fill(Color32::from_rgba_unmultiplied(20, 16, 12, 225)).corner_radius(14.0).inner_margin(20.0))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("OUAGA CITY").size(48.0).strong().color(Color32::from_rgb(240, 180, 40)));
                    ui.label(egui::RichText::new("Le rêve ouagalais").size(22.0).italics().color(Color32::from_rgb(255, 230, 190)));
                    ui.label(egui::RichText::new("Monde ouvert 3D dans les vraies rues de Ouagadougou").color(Color32::from_gray(210)));
                });
                ui.add_space(14.0);
                let big = |t: &str| egui::Button::new(egui::RichText::new(t).size(19.0).strong()).min_size(vec2(380.0, 42.0)).corner_radius(8.0);
                ui.label(egui::RichText::new("MODE HISTOIRE").strong().color(Color32::from_rgb(240, 190, 40)));
                if saved > 0 && ui.add(big("▶  Continuer l'histoire")).clicked() {
                    self.start_story(saved);
                }
                if ui.add(big("Nouvelle histoire")).clicked() {
                    self.start_story(0);
                }
                ui.add_space(10.0);
                ui.label(egui::RichText::new("MODE LIBRE").strong().color(Color32::from_rgb(240, 190, 40)));
                if ui.add(big("Mode libre (solo)")).clicked() {
                    self.net = None;
                    self.start_free(rand_seed());
                }
                ui.horizontal(|ui| {
                    ui.label("Pseudo en ligne :");
                    ui.text_edit_singleline(&mut self.name);
                });
                if ui.add(big("En ligne : héberger une partie")).clicked() {
                    match Net::host() {
                        Ok(n) => {
                            self.net = Some(n);
                            self.start_free(rand_seed());
                        }
                        Err(e) => self.error = e,
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("IP de l'hôte :");
                    ui.add(egui::TextEdit::singleline(&mut self.join_ip).hint_text("ex. 192.168.1.20").desired_width(150.0));
                    if ui.button("Rejoindre").clicked() {
                        match Net::join(self.join_ip.trim()) {
                            Ok(n) => {
                                self.net = Some(n);
                                self.start_free(rand_seed());
                            }
                            Err(e) => self.error = e,
                        }
                    }
                });
                ui.label(egui::RichText::new(format!("Ton adresse : {}  (port {})", net::local_ip(), net::PORT)).small());
                if !self.error.is_empty() {
                    ui.colored_label(Color32::from_rgb(255, 100, 100), &self.error);
                }
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "ZQSD / WASD / flèches · F monter/descendre · E acheter · B empire · Espace frein · Maj courir · \
                         H klaxon · souris : caméra · molette : zoom · C caméra haute · M carte · Échap pause",
                    )
                    .small(),
                );
                ui.label(egui::RichText::new("Carte © contributeurs OpenStreetMap (ODbL) · textures ambientCG (CC0) · ciels Poly Haven (CC0)").small().weak());
            });
    }

    fn read_input(&self, ctx: &egui::Context) -> (Input, [bool; 7], f32, (f32, f32), bool, f32) {
        ctx.input(|i| {
            let k = |a: Key, b: Key, c: Key| i.key_down(a) || i.key_down(b) || i.key_down(c);
            let inp = Input {
                up: k(Key::W, Key::Z, Key::ArrowUp),
                down: k(Key::S, Key::S, Key::ArrowDown),
                left: k(Key::A, Key::Q, Key::ArrowLeft),
                right: k(Key::D, Key::D, Key::ArrowRight),
                brake: i.key_down(Key::Space),
                sprint: i.modifiers.shift,
                action: i.key_pressed(Key::F),
                horn: i.key_down(Key::H),
                aim: i.pointer.secondary_down() || i.key_down(Key::V),
                fire: (i.pointer.secondary_down() && i.pointer.primary_pressed()) || i.key_pressed(Key::X) || (!i.pointer.secondary_down() && i.pointer.primary_clicked()),
                fire_held: (i.pointer.secondary_down() && i.pointer.primary_down()) || i.key_down(Key::X),
                aim_yaw: 0.0,
                switch: if i.key_pressed(Key::Num1) {
                    Some(0)
                } else if i.key_pressed(Key::Num2) {
                    Some(1)
                } else if i.key_pressed(Key::Num3) {
                    Some(2)
                } else {
                    None
                },
            };
            let aiming = i.pointer.secondary_down() || i.key_down(Key::V);
            let drag = if i.pointer.primary_down() && !aiming { i.pointer.delta().x } else { 0.0 };
            let drag_y = if i.pointer.primary_down() && !aiming { i.pointer.delta().y } else { 0.0 };
            let aim_dx = if aiming { i.pointer.delta().x } else { 0.0 };
            let advance = i.key_pressed(Key::Enter) || i.key_pressed(Key::Space) || i.pointer.primary_clicked();
            let keys = [i.key_pressed(Key::M), i.key_pressed(Key::Escape), i.key_pressed(Key::C), i.key_pressed(Key::E), i.key_pressed(Key::B), i.key_pressed(Key::Tab), i.key_pressed(Key::T)];
            (inp, keys, i.smooth_scroll_delta.y, (drag, drag_y), advance, aim_dx)
        })
    }

    fn play(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let screen = ui.max_rect();
        let dt = if self.shot.is_some() { 1.0 / 60.0 } else { ctx.input(|i| i.stable_dt).min(0.05) };
        let (mut inp, [map_key, esc, cam_key, buy_key, emp_key, tab_key, talk_key], scroll, (drag, drag_y), advance, aim_dx) = self.read_input(&ctx);
        if self.shot.is_some() {
            if std::env::args().any(|a| a == "--aim") {
                inp = Input { action: self.frame == 3, aim: self.frame > 20 && std::env::var("OC_NOAIM").is_err(), fire: std::env::var("OC_FIRE").is_ok() && self.frame > 60 && self.frame % 12 == 0, switch: if self.frame == 5 && std::env::var("OC_W0").is_err() { Some(1) } else { None }, ..Default::default() };
            } else {
                inp = Input { up: self.frame > 10 && self.story.is_none() && std::env::var("OC_STILL").is_err(), action: std::env::var("OC_WALK").is_ok() && self.frame == 3, ..Default::default() };
                if std::env::var("OC_WALK").is_ok() {
                    inp.up = self.frame > 20;
                    inp.sprint = std::env::var("OC_RUN").is_ok();
                }
            }
        }
        if map_key {
            self.show_map = !self.show_map;
        }
        if cam_key {
            self.high_cam = !self.high_cam;
        }
        if emp_key {
            self.show_empire = !self.show_empire;
        }
        if esc {
            self.paused = !self.paused;
        }
        self.dist_mul = (self.dist_mul * (1.0 - scroll * 0.002)).clamp(0.5, 3.0);
        let map = self.map.clone();
        let game = self.game.as_mut().unwrap();
        let empire = self.empire.as_mut().unwrap();
        if tab_key {
            let mut w = game.weapon;
            for _ in 0..3 {
                w = (w + 1) % 3;
                if game.owned[w] {
                    break;
                }
            }
            inp.switch = Some(w);
        }
        if inp.aim && game.player.vehicle.is_none() {
            if !self.was_aiming {
                self.aim_yaw = self.cam_yaw + self.orbit;
                self.orbit = 0.0;
            }
            self.aim_yaw += aim_dx * 0.006 + (inp.right as i32 - inp.left as i32) as f32 * 1.8 * dt;
            inp.left = false;
            inp.right = false;
        }
        self.was_aiming = inp.aim && game.player.vehicle.is_none();
        inp.aim_yaw = self.aim_yaw;

        let in_dialog = self.story.as_ref().is_some_and(|s| s.in_dialog);
        if in_dialog {
            if advance {
                self.story.as_mut().unwrap().next_line();
            }
            inp = Input::default();
        }
        if !self.paused && !in_dialog {
            game.update(&map, inp, dt);
            empire.update(game, dt);
        }
        if let Some(st) = &mut self.story {
            if !self.paused {
                st.update(&map, game, empire, dt);
            }
            if st.finished && st.banner.is_none() {
                self.story = None;
            }
        }

        // businesses: buy with E
        let pp = game.player_pos();
        let mut prompt: Option<String> = None;
        if let Some(i) = empire.nearby(pp) {
            let b = &empire.list[i];
            if b.owned {
                prompt = Some(format!("{} — à toi · +{} / heure", b.name, fcfa(b.income)));
            } else {
                prompt = Some(format!("E : acheter {} ({}) — {} · +{} / heure", b.name, b.what, fcfa(b.price), fcfa(b.income)));
                if buy_key {
                    match empire.buy(i, game) {
                        Ok(m) | Err(m) => game.msg(m),
                    }
                }
            }
        }
        if prompt.is_none() {
            if let Some(_) = game.shops.iter().find(|s| (**s - pp).length() < 9.0) {
                prompt = Some("E : armurerie — pistolet 15 000 · munitions 2 000 · fusil d'assaut 120 000 FCFA".into());
                if buy_key {
                    let m = game.buy_at_shop();
                    game.msg(m);
                }
            }
        }
        // doors, interiors, activities, conversations
        let mut used = prompt.is_some() && buy_key;
        if prompt.is_none() || game.inside.is_some() {
            if let Some(t) = game.interact_prompt() {
                prompt = Some(t);
            }
            if buy_key && !used {
                used = game.interact(&map);
            }
        }
        let _ = used;
        if talk_key {
            game.start_talk();
        }
        if game.ev_saved {
            game.ev_saved = false;
            empire.save(game.player.money);
        }
        let pp = game.player_pos();
        let worth = empire.net_worth(game.player.money);
        let income = empire.income();
        let biz: Vec<(Pos2, bool)> = empire.list.iter().map(|b| (b.place, b.owned)).collect();

        // third-person camera
        let (speed, heading) = match game.player.vehicle {
            Some(v) => (game.vehicles[v].speed(), game.vehicles[v].angle),
            None => (0.0, game.player.angle),
        };
        self.orbit += drag * 0.008;
        self.pitch = (self.pitch + drag_y * 0.004).clamp(-0.35, 0.9);
        // the camera only swings back behind Bila once he moves
        let moving = game.player.moving > 0.1 || game.player.vehicle.is_some_and(|v| game.vehicles[v].speed().abs() > 1.5);
        if drag == 0.0 && moving {
            self.orbit *= (-dt * 0.8).exp();
            self.pitch *= (-dt * 0.6).exp();
        }
        let mut d = heading - self.cam_yaw;
        while d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        while d < -std::f32::consts::PI {
            d += std::f32::consts::TAU;
        }
        let in_vehicle = game.player.vehicle.is_some();
        self.cam_yaw += d * (dt * if in_vehicle { 3.0 } else { 4.0 }).min(1.0);
        if game.aiming {
            self.cam_yaw = self.aim_yaw;
            self.orbit = 0.0;
        }
        let yaw = self.cam_yaw + self.orbit;
        let fwd = Vec3::new(yaw.cos(), 0.0, yaw.sin());
        let right = Vec3::new(-yaw.sin(), 0.0, yaw.cos());
        let inside = game.inside;
        let (dist, height) = if inside.is_some() {
            (3.2, 1.1)
        } else if self.high_cam {
            (24.0, 22.0)
        } else if in_vehicle {
            (7.5 + speed.abs() * 0.1, 2.8 + speed.abs() * 0.03)
        } else {
            (5.2, 1.6)
        };
        let height = height + self.pitch * dist;
        let focus = Vec3::new(pp.x, if in_vehicle { 1.3 } else { 1.15 }, pp.y);
        let mut eye = if game.aiming {
            focus - fwd * 2.6 + right * 0.6 + Vec3::Y * 0.65
        } else {
            focus - fwd * dist * self.dist_mul + Vec3::Y * height * self.dist_mul.sqrt()
        };
        eye.y = eye.y.max(0.6);
        // keep the camera out of the walls
        let mut reach = 1.0;
        for k in 1..=24 {
            let s = k as f32 / 24.0;
            let q = focus + (eye - focus) * s;
            let q2 = Pos2::new(q.x, q.z);
            let protrusion = q.y > 2.2 && q.y < 4.6 && k > 2 && map.facade_near(q2, 2.4).is_some_and(|h| h > 2.8);
            if map.building_height_at(q2).is_some_and(|h| q.y < h) || protrusion {
                reach = ((k - 1) as f32 / 24.0 - 0.03).max(0.3);
                break;
            }
        }
        let mut eye_target = focus + (eye - focus) * reach;
        // keep a margin from the walls too (shoulder camera)
        let mut e2 = Pos2::new(eye_target.x, eye_target.z);
        if map.collide(&mut e2, 0.8).is_some() {
            eye_target.x = e2.x;
            eye_target.z = e2.y;
        }
        // and out of the vehicles around (a parked minibus behind Bila...)
        for (vi, v) in game.vehicles.iter().enumerate() {
            if Some(vi) == game.player.vehicle {
                continue;
            }
            let r = v.kind.spec().0 + 0.6;
            let d = Pos2::new(eye_target.x, eye_target.z) - v.pos;
            if d.length() < r && eye_target.y < 2.6 {
                let n = if d.length() > 1e-3 { d.normalized() } else { egui::vec2(1.0, 0.0) };
                eye_target.x = v.pos.x + n.x * r;
                eye_target.z = v.pos.y + n.y * r;
            }
        }
        // inside a room: stay between the walls and under the ceiling
        if let Some(ii) = inside {
            let it = &statics::interiors()[ii];
            let a = statics::anchor(ii);
            eye_target = focus - fwd * 3.2 * self.dist_mul.min(1.4) + Vec3::Y * 0.9;
            eye_target.x = eye_target.x.clamp(a.x - it.size[0] / 2.0 + 0.3, a.x + it.size[0] / 2.0 - 0.3);
            eye_target.z = eye_target.z.clamp(a.y - it.size[1] / 2.0 + 0.3, a.y + it.size[1] / 2.0 - 0.3);
            eye_target.y = eye_target.y.min(it.size[2] - 0.35);
            reach = 0.5;
        }
        let blocked = reach < 1.0;
        let eye = eye_target;
        let eye = match self.cam_pos {
            Some(prev) if prev.distance(eye) < 40.0 && !blocked => prev.lerp(eye, (dt * if game.aiming { 18.0 } else { 8.0 }).min(1.0)),
            _ => eye,
        };
        self.cam_pos = Some(eye);
        let look = if game.aiming { focus + Vec3::Y * 0.4 + fwd * 25.0 + right * 0.6 } else if in_vehicle { focus + fwd * 3.0 } else { focus + Vec3::Y * 0.2 + fwd * 1.5 };
        let aspect = screen.width() / screen.height().max(1.0);
        // a wider field of view at speed, as in the big open-world games
        let target_fov = if in_vehicle { 60.0 + (speed.abs() * 0.45).min(16.0) } else if game.aiming { 52.0 } else { 62.0 };
        self.fov += (target_fov - self.fov) * (dt * 3.0).min(1.0);
        let proj = Mat4::perspective_rh_gl(self.fov.to_radians(), aspect, 0.15, 1600.0);
        let view = Mat4::look_at_rh(eye, look, Vec3::Y);

        let mut remotes: Vec<Remote> = Vec::new();
        let mut status = None;
        if let Some(n) = &mut self.net {
            let (kind, color) = match game.player.vehicle {
                Some(v) => (game.vehicles[v].kind.net_id(), game.vehicles[v].color),
                None => (0, game.player.color),
            };
            n.update(&self.name, pp.x, pp.y, game.player.angle, kind, [color.r(), color.g(), color.b()], game.player.wanted);
            remotes = n.remotes.values().cloned().collect();
            status = Some(n.status.clone());
        }

        self.zone_timer -= dt;
        if self.zone_timer <= 0.0 {
            self.zone = match game.inside {
                Some(i) => statics::interiors()[i].title.to_string(),
                None => map.place_name(pp),
            };
            self.zone_timer = 0.5;
        }

        let story_target = self.story.as_ref().and_then(|s| s.target(game));
        let chase_tag = self.story.as_ref().and_then(|s| s.current()).and_then(|st| match st.obj {
            Obj::Chase { tag, .. } => Some(tag),
            _ => None,
        });
        let marker = if chase_tag.is_some() { None } else { story_target };
        let night = render::night_factor(game.hours);
        let mut light = r3d::lighting(game.hours);
        let night = if inside.is_some() {
            // indoor light: lamps and daylight through the windows
            light.sun_col *= 0.12;
            light.ambient = Vec3::new(0.86, 0.8, 0.72);
            light.fog = 2000.0;
            0.0
        } else {
            night
        };
        let steer_in = (inp.right as i32 - inp.left as i32) as f32 * 0.45;
        self.steer += (steer_in - self.steer) * (dt * 8.0).min(1.0);
        let scene = r3d::build_scene(&r3d::SceneInput {
            game,
            remotes: &remotes,
            cam: Pos2::new(eye.x, eye.z),
            marker,
            chase_tag,
            bila_suit: worth >= 1_000_000,
            biz_markers: &biz,
            steer: self.steer,
        });
        let fd = r3d::FrameData {
            view_proj: proj * view,
            light_vp: r3d::light_matrix(focus, light.sun),
            cam: eye,
            sun: light.sun,
            sun_col: light.sun_col,
            ambient: light.ambient,
            sky: light.fog_col,
            sky_w: light.sky_w,
            fog: light.fog,
            night,
            focus,
            opaque: scene.opaque,
            translucent: scene.translucent,
            insts: scene.insts,
            chars: scene.chars,
            cars: scene.cars,
            interior: inside,
        };

        let p = ui.painter().clone();
        let face = self.bila_face.as_ref().map(|t| t.id());
        if self.show_map {
            render::draw_full_map(&p, &map, game, screen, &remotes);
        } else {
            p.add(r3d::callback(screen, self.r3d.clone(), map.clone(), fd));
            if let Some(Err(e)) = self.r3d.lock().unwrap().as_ref() {
                p.text(screen.center(), Align2::CENTER_CENTER, format!("3D indisponible : {e}"), FontId::proportional(16.0), Color32::RED);
            }
            let obj_text = self.story.as_ref().map(|s| s.objective_text(game));
            let objective = obj_text.as_deref().map(|t| (t, story_target));
            render::draw_hud(
                &p,
                &render::Hud {
                    map: &map,
                    g: game,
                    screen,
                    remotes: &remotes,
                    net_status: status.as_deref(),
                    zone: &self.zone,
                    cam_yaw: yaw,
                    objective,
                    worth,
                    income,
                    photo: face,
                    prompt: prompt.as_deref(),
                    biz: &biz,
                },
            );
        }
        if let Some(st) = &self.story {
            if let Some((who, text)) = st.dialog_line() {
                let (skin, clothes) = story::speaker_colors(who);
                let photo = if who == "Bila" { face } else { self.faces.iter().find(|(n, _)| *n == who).map(|(_, t)| t.id()) };
                render::draw_dialog(&p, screen, who, text, skin, clothes, photo);
            }
            if let Some((b, t)) = &st.banner {
                render::draw_banner(&p, screen, b, *t);
            }
        }
        if self.show_empire {
            let empire = self.empire.as_ref().unwrap();
            egui::Window::new("L'empire de Bila")
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .resizable(false)
                .collapsible(false)
                .show(&ctx, |ui| {
                    ui.label(egui::RichText::new(format!("Fortune : {}   ·   Revenus : +{} / heure", fcfa(worth), fcfa(income))).strong());
                    ui.label(format!("Objectif : {} (homme le plus riche d'Afrique)", fcfa(empire::RICHEST)));
                    ui.add(egui::ProgressBar::new((worth as f64 / empire::RICHEST as f64).clamp(0.0, 1.0) as f32).show_percentage());
                    ui.separator();
                    egui::Grid::new("biz").striped(true).spacing(vec2(16.0, 6.0)).show(ui, |ui| {
                        ui.strong("Entreprise");
                        ui.strong("Prix");
                        ui.strong("Revenu / h");
                        ui.strong("État");
                        ui.end_row();
                        for b in &empire.list {
                            ui.label(format!("{}\n{}", b.name, b.what));
                            ui.label(fcfa(b.price));
                            ui.label(fcfa(b.income));
                            if b.owned {
                                ui.colored_label(Color32::from_rgb(110, 220, 110), "À toi");
                            } else {
                                ui.label("À vendre · $ vert sur la carte");
                            }
                            ui.end_row();
                        }
                    });
                    ui.label(egui::RichText::new("Va devant une entreprise (marqueur vert) et appuie sur E pour l'acheter. B pour fermer.").small());
                });
        }
        if self.paused {
            p.rect_filled(screen, 0.0, Color32::from_black_alpha(170));
            let lines = [
                "PAUSE (Échap pour reprendre)",
                "",
                "ZQSD / WASD / flèches : marcher (tourner + avancer) ou conduire",
                "F : monter / descendre d'un véhicule (voler = recherché !)",
                "E : entrer (portes violettes : maquis, appartement, salle de sport, garage), utiliser, acheter · B : empire",
                "T : parler aux passants · au maquis : manger · à l'appartement : dormir (sauvegarde) · salle : s'entraîner",
                "Clic droit (ou V) : viser · clic gauche (ou X) : tirer / frapper · 1/2/3 ou Tab : changer d'arme",
                "Espace : frein à main · Maj : courir · H : klaxon",
                "Souris (glisser) : caméra · molette : distance · C : caméra haute · M : carte",
                "Cercles jaunes (marchés) : courses de taxi-moto payées en FCFA",
                "Stations-service : réparation à l'arrêt (500 FCFA)",
            ];
            for (i, l) in lines.iter().enumerate() {
                p.text(screen.center() + vec2(0.0, -175.0 + i as f32 * 26.0), Align2::CENTER_CENTER, l, FontId::proportional(if i == 0 { 26.0 } else { 17.0 }), Color32::WHITE);
            }
            let r = Rect::from_center_size(screen.center() + vec2(0.0, 150.0), vec2(220.0, 36.0));
            if ui.put(r, egui::Button::new("Retour au menu")).clicked() {
                if let (Some(e), Some(g)) = (&self.empire, &self.game) {
                    e.save(g.player.money);
                }
                self.screen = Screen::Menu;
                self.game = None;
                self.story = None;
                self.empire = None;
                self.net = None;
                self.paused = false;
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.ensure_textures(&ctx);
        match self.screen {
            Screen::Menu => self.menu(ui),
            Screen::Playing => self.play(ui),
        }
        ctx.request_repaint();
        self.frame += 1;
        if let Some(path) = self.shot.clone() {
            if self.frame >= 120 && !self.shot_sent {
                self.shot_sent = true;
                ctx.request_screenshot(move |img: Arc<egui::ColorImage>| {
                    let [w, h] = img.size;
                    let mut raw = Vec::with_capacity(w * h * 4);
                    for c in &img.pixels {
                        raw.extend_from_slice(&[c.r(), c.g(), c.b(), 255]);
                    }
                    if let Some(dir) = path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    if let Err(e) = image::save_buffer(&path, &raw, w as u32, h as u32, image::ColorType::Rgba8) {
                        eprintln!("screenshot failed: {e}");
                        std::process::exit(1);
                    }
                    std::process::exit(0);
                });
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--export-textures") {
        let dir = args.get(i + 1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("textures"));
        r3d::export_textures(&dir);
        return Ok(());
    }
    let shot = args.iter().position(|a| a == "--screenshot").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Ouaga City — Le rêve ouagalais")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([960.0, 600.0]);
    if shot.is_some() {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        depth_buffer: 24,
        multisampling: 4,
        ..Default::default()
    };
    #[cfg(target_os = "macos")]
    if shot.is_some() {
        options.event_loop_builder = Some(Box::new(|b| {
            use winit::platform::macos::EventLoopBuilderExtMacOS;
            b.with_activate_ignoring_other_apps(false);
        }));
    }
    eframe::run_native("Ouaga City", options, Box::new(move |_cc| Ok(Box::new(App::new(shot)))))
}

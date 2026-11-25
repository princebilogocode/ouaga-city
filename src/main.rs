#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod game;
mod map;
mod net;
mod r3d;
mod render;
mod story;

use eframe::egui;
use egui::{vec2, Align2, Color32, FontId, Key, Pos2, Rect};
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
    screen: Screen,
    name: String,
    join_ip: String,
    net: Option<Net>,
    error: String,
    show_map: bool,
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

impl App {
    fn new(shot: Option<PathBuf>) -> App {
        let map = Arc::new(Map::load());
        let seed = rand_seed();
        let mut app = App {
            map,
            game: None,
            story: None,
            screen: Screen::Menu,
            name: format!("Bila{}", seed % 1000),
            join_ip: String::new(),
            net: None,
            error: String::new(),
            show_map: false,
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
        };
        if app.shot.is_some() {
            if std::env::args().any(|a| a == "--story") {
                app.start_story(0);
            } else {
                app.start_free(42);
            }
        }
        app
    }

    fn start_free(&mut self, seed: u64) {
        let g = Game::new(&self.map, seed, self.color);
        self.cam_yaw = g.player.angle;
        self.game = Some(g);
        self.story = None;
        self.cam_pos = None;
        self.screen = Screen::Playing;
    }

    fn start_story(&mut self, from: usize) {
        self.net = None;
        let mut g = Game::new(&self.map, rand_seed(), self.color);
        g.messages.clear();
        let st = Story::new(&self.map, from, &mut g);
        self.cam_yaw = g.player.angle;
        self.game = Some(g);
        self.story = Some(st);
        self.cam_pos = None;
        self.screen = Screen::Playing;
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        let screen = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(screen, 0.0, Color32::from_rgb(40, 28, 20));
        let ext = self.map.max - self.map.min;
        let scale = (screen.width() / ext.x).max(screen.height() / ext.y);
        let cam = render::Cam { center: self.map.min + ext * 0.5, scale, screen };
        for r in self.map.roads.iter().filter(|r| r.kind <= 3) {
            let pts: Vec<_> = r.pts.iter().map(|q| cam.w2s(*q)).collect();
            p.add(egui::Shape::line(pts, egui::Stroke::new(1.5, Color32::from_rgb(120, 80, 50))));
        }
        p.rect_filled(screen, 0.0, Color32::from_black_alpha(120));
        let (skin, clothes) = story::speaker_colors("Bila");
        render::draw_portrait(&p, screen.left_center() + vec2(screen.width() * 0.17, 0.0), 110.0, skin, clothes);
        p.text(screen.left_center() + vec2(screen.width() * 0.17, 140.0), Align2::CENTER_TOP, "Bila", FontId::proportional(30.0), Color32::from_rgb(240, 190, 40));

        let saved = story::saved_step();
        egui::Window::new("Ouaga City")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .fixed_size(vec2(400.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("OUAGA CITY").size(42.0).strong().color(Color32::from_rgb(240, 180, 40)));
                    ui.label(egui::RichText::new("L'histoire de Bila — monde ouvert 3D dans les vraies rues de Ouagadougou").italics());
                });
                ui.add_space(12.0);
                let big = |t: &str| egui::Button::new(egui::RichText::new(t).size(18.0)).min_size(vec2(380.0, 38.0));
                ui.label(egui::RichText::new("MODE HISTOIRE").strong());
                if saved > 0 && ui.add(big("Continuer l'histoire")).clicked() {
                    self.start_story(saved);
                }
                if ui.add(big("Nouvelle histoire")).clicked() {
                    self.start_story(0);
                }
                ui.add_space(8.0);
                ui.label(egui::RichText::new("MODE LIBRE").strong());
                if ui.add(big("Mode libre (solo)")).clicked() {
                    self.net = None;
                    self.start_free(rand_seed());
                }
                ui.horizontal(|ui| {
                    ui.label("Pseudo en ligne :");
                    ui.text_edit_singleline(&mut self.name);
                });
                if ui.add(big("Mode libre en ligne : héberger")).clicked() {
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
                ui.label(egui::RichText::new(format!("Ton adresse sur le réseau : {}  (port {})", net::local_ip(), net::PORT)).small());
                if !self.error.is_empty() {
                    ui.colored_label(Color32::from_rgb(255, 100, 100), &self.error);
                }
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "Commandes : ZQSD / WASD / flèches · F monter/descendre · Espace frein à main · Maj courir · \
                         H klaxon · souris (glisser) tourner la caméra · molette zoom · C caméra haute · M carte · Échap pause",
                    )
                    .small(),
                );
                ui.label(egui::RichText::new("Carte : données © contributeurs OpenStreetMap (ODbL), la base de Protomaps.").small().weak());
            });
    }

    fn read_input(&self, ctx: &egui::Context) -> (Input, bool, bool, f32, f32, bool, bool) {
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
            };
            let drag = if i.pointer.primary_down() || i.pointer.secondary_down() { i.pointer.delta().x } else { 0.0 };
            let advance = i.key_pressed(Key::Enter) || i.key_pressed(Key::Space) || i.pointer.primary_clicked();
            (inp, i.key_pressed(Key::M), i.key_pressed(Key::Escape), i.smooth_scroll_delta.y, drag, i.key_pressed(Key::C), advance)
        })
    }

    fn play(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let screen = ui.max_rect();
        let dt = if self.shot.is_some() { 1.0 / 60.0 } else { ctx.input(|i| i.stable_dt).min(0.05) };
        let (mut inp, map_key, esc, scroll, drag, cam_key, advance) = self.read_input(&ctx);
        if self.shot.is_some() {
            inp = Input { up: self.frame > 10 && self.story.is_none(), ..Default::default() };
        }
        if map_key {
            self.show_map = !self.show_map;
        }
        if cam_key {
            self.high_cam = !self.high_cam;
        }
        if esc {
            self.paused = !self.paused;
        }
        self.dist_mul = (self.dist_mul * (1.0 - scroll * 0.002)).clamp(0.5, 3.0);
        let map = self.map.clone();
        let game = self.game.as_mut().unwrap();

        // dialogue pauses the game
        let in_dialog = self.story.as_ref().is_some_and(|s| s.in_dialog);
        if in_dialog {
            if advance {
                self.story.as_mut().unwrap().next_line();
            }
            inp = Input::default();
        }
        if !self.paused && !in_dialog {
            game.update(&map, inp, dt);
        }
        if let Some(st) = &mut self.story {
            if !self.paused {
                st.update(&map, game, dt);
            }
            if st.finished && st.banner.is_none() {
                self.story = None;
            }
        }

        // third-person camera
        let pp = game.player_pos();
        let (speed, heading) = match game.player.vehicle {
            Some(v) => (game.vehicles[v].speed(), game.vehicles[v].angle),
            None => (0.0, game.player.angle),
        };
        self.orbit += drag * 0.008;
        if drag == 0.0 {
            self.orbit *= (-dt * 0.8).exp();
        }
        let mut d = heading - self.cam_yaw;
        while d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        while d < -std::f32::consts::PI {
            d += std::f32::consts::TAU;
        }
        let follow = if game.player.vehicle.is_some() { 3.0 } else { 4.0 };
        self.cam_yaw += d * (dt * follow).min(1.0);
        let yaw = self.cam_yaw + self.orbit;
        let fwd = Vec3::new(yaw.cos(), 0.0, yaw.sin());
        let in_vehicle = game.player.vehicle.is_some();
        let (dist, height) = if self.high_cam {
            (22.0, 20.0)
        } else if in_vehicle {
            (7.5 + speed.abs() * 0.1, 3.0 + speed.abs() * 0.03)
        } else {
            (4.6, 2.0)
        };
        let focus = Vec3::new(pp.x, if in_vehicle { 1.3 } else { 1.6 }, pp.y);
        let mut eye = focus - fwd * dist * self.dist_mul + Vec3::Y * height * self.dist_mul.sqrt();
        eye.y = eye.y.max(0.6);
        let eye = match self.cam_pos {
            Some(prev) if prev.distance(eye) < 40.0 => prev.lerp(eye, (dt * 8.0).min(1.0)),
            _ => eye,
        };
        self.cam_pos = Some(eye);
        let look = focus + fwd * 3.0;
        let aspect = screen.width() / screen.height().max(1.0);
        let proj = Mat4::perspective_rh_gl(62f32.to_radians(), aspect, 0.25, 1400.0);
        let view = Mat4::look_at_rh(eye, look, Vec3::Y);

        // network
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
            self.zone = map.place_name(pp);
            self.zone_timer = 0.5;
        }

        let story_target = self.story.as_ref().and_then(|s| s.target(game));
        let chase_tag = self.story.as_ref().and_then(|s| s.current()).and_then(|st| match st.obj {
            Obj::Chase { tag, .. } => Some(tag),
            _ => None,
        });
        let marker = match chase_tag {
            Some(_) => None,
            None => story_target,
        };
        let night = render::night_factor(game.hours);
        let (sun, sun_col, ambient, sky, fog) = r3d::lighting(game.hours, night);
        let (opaque, translucent) = r3d::build_scene(game, &remotes, Pos2::new(eye.x, eye.z), marker, chase_tag);
        let fd = r3d::FrameData { view_proj: proj * view, cam: eye, sun, sun_col, ambient, sky, fog, night, opaque, translucent };

        let p = ui.painter().clone();
        if self.show_map {
            render::draw_full_map(&p, &map, game, screen, &remotes);
        } else {
            p.add(r3d::callback(screen, self.r3d.clone(), map.clone(), fd));
            if let Some(Err(e)) = self.r3d.lock().unwrap().as_ref() {
                p.text(screen.center(), Align2::CENTER_CENTER, format!("3D indisponible : {e}"), FontId::proportional(16.0), Color32::RED);
            }
            let obj_text = self.story.as_ref().map(|s| s.objective_text(game));
            let objective = obj_text.as_deref().map(|t| (t, story_target));
            render::draw_hud(&p, &map, game, screen, &remotes, status.as_deref(), &self.zone, yaw, objective);
        }
        if let Some(st) = &self.story {
            if let Some((who, text)) = st.dialog_line() {
                let (skin, clothes) = story::speaker_colors(who);
                render::draw_dialog(&p, screen, who, text, skin, clothes);
            }
            if let Some((b, t)) = &st.banner {
                render::draw_banner(&p, screen, b, *t);
            }
        }
        if self.paused {
            p.rect_filled(screen, 0.0, Color32::from_black_alpha(170));
            let lines = [
                "PAUSE (Échap pour reprendre)",
                "",
                "ZQSD / WASD / flèches : marcher (tourner + avancer) ou conduire",
                "F : monter / descendre d'un véhicule (voler = recherché !)",
                "Espace : frein à main · Maj : courir · H : klaxon",
                "Souris (glisser) : tourner la caméra · molette : distance · C : caméra haute",
                "M : carte de Ouagadougou",
                "Mode libre : cercles jaunes (marchés) = courses payées en FCFA",
                "Stations-service : réparation à l'arrêt (500 FCFA)",
            ];
            for (i, l) in lines.iter().enumerate() {
                p.text(screen.center() + vec2(0.0, -120.0 + i as f32 * 26.0), Align2::CENTER_CENTER, l, FontId::proportional(if i == 0 { 26.0 } else { 17.0 }), Color32::WHITE);
            }
            let r = Rect::from_center_size(screen.center() + vec2(0.0, 150.0), vec2(220.0, 36.0));
            if ui.put(r, egui::Button::new("Retour au menu")).clicked() {
                self.screen = Screen::Menu;
                self.game = None;
                self.story = None;
                self.net = None;
                self.paused = false;
            }
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        match self.screen {
            Screen::Menu => self.menu(ui),
            Screen::Playing => self.play(ui),
        }
        let ctx = ui.ctx().clone();
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
        .with_title("Ouaga City — L'histoire de Bila")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([800.0, 560.0]);
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

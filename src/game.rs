use crate::map::{Map, Rng};
use eframe::egui::{self as egui};
use egui::{vec2, Color32, Pos2, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VKind {
    Car,
    Moto,
    Taxi,
    Police,
    Minibus,
}
impl VKind {
    /// half length, half width, max speed, acceleration, steering rate
    pub fn spec(self) -> (f32, f32, f32, f32, f32) {
        match self {
            VKind::Car => (2.2, 0.95, 38.0, 9.0, 2.3),
            VKind::Taxi => (2.2, 0.95, 36.0, 8.5, 2.3),
            VKind::Moto => (1.0, 0.4, 30.0, 11.0, 3.2),
            VKind::Police => (2.35, 1.0, 42.0, 11.0, 2.5),
            VKind::Minibus => (3.0, 1.15, 28.0, 6.0, 1.8),
        }
    }
    pub fn net_id(self) -> u8 {
        match self {
            VKind::Car => 1,
            VKind::Moto => 2,
            VKind::Taxi => 3,
            VKind::Police => 4,
            VKind::Minibus => 5,
        }
    }
    pub fn from_net(k: u8) -> VKind {
        match k {
            2 => VKind::Moto,
            3 => VKind::Taxi,
            4 => VKind::Police,
            5 => VKind::Minibus,
            _ => VKind::Car,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            VKind::Car => "Voiture",
            VKind::Taxi => "Taxi vert",
            VKind::Moto => "Moto",
            VKind::Police => "Police",
            VKind::Minibus => "Minibus",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Driver {
    None,
    Player,
    Traffic,
    Police,
    Gang,
}

#[derive(Clone, Copy)]
pub struct Route {
    pub road: usize,
    pub i: usize,
    pub dir: i32,
}

pub struct Vehicle {
    pub pos: Pos2,
    pub vel: Vec2,
    pub angle: f32,
    pub kind: VKind,
    pub color: Color32,
    pub driver: Driver,
    pub health: f32,
    pub route: Option<Route>,
    pub stuck: f32,
    pub reverse: f32,
    pub tag: u8,
    pub dead: bool,
    pub remove: bool,
}
impl Vehicle {
    pub fn fwd(&self) -> Vec2 {
        vec2(self.angle.cos(), self.angle.sin())
    }
    pub fn speed(&self) -> f32 {
        self.vel.dot(self.fwd())
    }
}

pub struct Ped {
    pub pos: Pos2,
    pub angle: f32,
    pub road: usize,
    pub i: usize,
    pub dir: i32,
    pub side: f32,
    pub flee: f32,
    pub down: f32,
    pub color: Color32,
    pub remove: bool,
}

pub struct Mission {
    pub to: Pos2,
    pub to_name: String,
    pub time_left: f32,
    pub reward: i64,
}

pub struct Player {
    pub pos: Pos2,
    pub angle: f32,
    pub vehicle: Option<usize>,
    pub health: f32,
    pub money: i64,
    pub wanted: u8,
    pub wanted_timer: f32,
    pub arrest_timer: f32,
    pub color: Color32,
    pub walk: f32,
    pub moving: f32,
}

#[derive(Default, Clone, Copy)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub brake: bool,
    pub sprint: bool,
    pub action: bool,
    pub horn: bool,
    pub aim: bool,
    pub fire: bool,
    pub fire_held: bool,
    pub aim_yaw: f32,
    pub switch: Option<usize>,
}

/// Fists, pistol, assault rifle: (name, cooldown s, damage, range m, spread rad, automatic)
pub const WEAPONS: [(&str, f32, f32, f32, f32, bool); 3] = [
    ("Poings", 0.45, 60.0, 1.8, 0.0, false),
    ("Pistolet", 0.32, 40.0, 70.0, 0.02, false),
    ("Fusil d'assaut", 0.1, 30.0, 110.0, 0.035, true),
];

#[derive(Clone, Copy)]
pub struct Fx {
    /// 0 tracer, 1 muzzle flash, 2 impact, 3 blood
    pub kind: u8,
    pub a: Pos2,
    pub b: Pos2,
    pub t: f32,
}

pub struct Game {
    pub rng: Rng,
    pub vehicles: Vec<Vehicle>,
    pub peds: Vec<Ped>,
    pub player: Player,
    pub hours: f32,
    pub messages: Vec<(String, f32)>,
    pub mission: Option<Mission>,
    pub spots: Vec<Pos2>,
    pub repair_cd: f32,
    pub time: f32,
    pub honk: f32,
    pub story: bool,
    pub ev_arrested: bool,
    pub ev_died: bool,
    pub owned: [bool; 3],
    pub ammo: [i32; 3],
    pub weapon: usize,
    pub fire_cd: f32,
    pub aiming: bool,
    pub aim_yaw: f32,
    pub fx: Vec<Fx>,
    pub enemy_cd: f32,
    pub shops: Vec<Pos2>,
}

const CAR_COLORS: [Color32; 8] = [
    Color32::from_rgb(230, 230, 228),
    Color32::from_rgb(40, 40, 45),
    Color32::from_rgb(180, 30, 35),
    Color32::from_rgb(40, 80, 170),
    Color32::from_rgb(160, 160, 165),
    Color32::from_rgb(200, 170, 60),
    Color32::from_rgb(90, 50, 30),
    Color32::from_rgb(30, 110, 70),
];
const CLOTHES: [Color32; 8] = [
    Color32::from_rgb(220, 160, 30),
    Color32::from_rgb(200, 50, 60),
    Color32::from_rgb(40, 120, 180),
    Color32::from_rgb(30, 150, 90),
    Color32::from_rgb(240, 240, 230),
    Color32::from_rgb(150, 60, 160),
    Color32::from_rgb(230, 110, 40),
    Color32::from_rgb(60, 60, 70),
];

fn angle_diff(a: f32, b: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

fn right_of(d: Vec2) -> Vec2 {
    vec2(-d.y, d.x)
}

impl Game {
    pub fn new(map: &Map, seed: u64, color: Color32) -> Game {
        let mut rng = Rng(seed | 1);
        // start near the city centre on a main road
        let centre = Pos2::new(-300.0, -150.0);
        let (ri, si, t, _) = map.nearest_road(centre, 3000.0, 1).unwrap_or((0, 0, 0.0, 0.0));
        let r = &map.roads[ri];
        let a = r.pts[si];
        let b = r.pts[si + 1];
        let d = (b - a).normalized();
        let start = a + (b - a) * t + right_of(d) * r.half_width() * 0.45;
        let spots = map.pois.iter().filter(|p| p.kind == 'M').map(|p| p.pos).collect();
        let mut g = Game {
            rng: Rng(rng.next()),
            vehicles: Vec::new(),
            peds: Vec::new(),
            player: Player {
                pos: start,
                angle: d.y.atan2(d.x),
                vehicle: None,
                health: 100.0,
                money: 2500,
                wanted: 0,
                wanted_timer: 0.0,
                arrest_timer: 0.0,
                color,
                walk: 0.0,
                moving: 0.0,
            },
            hours: 10.0,
            messages: Vec::new(),
            mission: None,
            spots,
            repair_cd: 0.0,
            time: 0.0,
            honk: 0.0,
            story: false,
            ev_arrested: false,
            ev_died: false,
            owned: [true, true, false],
            ammo: [0, 48, 0],
            weapon: 0,
            fire_cd: 0.0,
            aiming: false,
            aim_yaw: 0.0,
            fx: Vec::new(),
            enemy_cd: 1.0,
            shops: ["Marché de Larlé", "Marché Katr yaar", "Kalgodin Yare"]
                .iter()
                .filter_map(|n| map.poi_named(n))
                .map(|p| map.road_point(p + vec2(25.0, 25.0)))
                .collect(),
        };
        // the player starts on a moto, the city's favourite ride
        g.vehicles.push(Vehicle {
            pos: start,
            vel: Vec2::ZERO,
            angle: d.y.atan2(d.x),
            kind: VKind::Moto,
            color: Color32::from_rgb(200, 30, 40),
            driver: Driver::Player,
            health: 100.0,
            route: None,
            stuck: 0.0,
            reverse: 0.0,
            tag: 0,
            dead: false,
            remove: false,
        });
        g.player.vehicle = Some(0);
        g.msg("Bienvenue à Ouaga ! Va vers un marché (cercle jaune) pour une course.");
        g.msg("F : monter/descendre · ZQSD/WASD/flèches : conduire · M : carte");
        for _ in 0..80 {
            g.populate(map, true);
        }
        g
    }

    pub fn msg(&mut self, s: impl Into<String>) {
        self.messages.push((s.into(), 6.0));
        if self.messages.len() > 5 {
            self.messages.remove(0);
        }
    }

    pub fn player_pos(&self) -> Pos2 {
        match self.player.vehicle {
            Some(v) => self.vehicles[v].pos,
            None => self.player.pos,
        }
    }

    pub fn set_wanted(&mut self, n: u8) {
        self.player.wanted = n;
        self.player.wanted_timer = 0.0;
    }

    /// Put the player on foot at a road point.
    pub fn place_player(&mut self, map: &Map, at: Pos2, angle: f32) {
        self.player.vehicle = None;
        self.player.pos = at;
        self.player.angle = angle;
        map.collide(&mut self.player.pos, 0.4);
        self.vehicles.retain(|v| v.driver != Driver::Player);
        self.vehicles.clear();
        self.peds.clear();
        for _ in 0..60 {
            self.populate(map, true);
        }
    }

    /// Spawn a vehicle on the lane closest to `at`.
    pub fn spawn_vehicle(&mut self, map: &Map, at: Pos2, kind: VKind, driver: Driver, color: Color32, tag: u8) {
        let (pos, angle, route) = match map.nearest_road(at, 400.0, if driver == Driver::None { 4 } else { 3 }) {
            Some((ri, si, t, _)) => {
                let r = &map.roads[ri];
                let (pa, pb) = (r.pts[si], r.pts[si + 1]);
                let d = (pb - pa).normalized();
                let side = if driver == Driver::None { r.half_width() * 0.5 + 0.6 } else { r.half_width() * 0.45 };
                (pa + (pb - pa) * t + right_of(d) * side, d.y.atan2(d.x), Some(Route { road: ri, i: si + 1, dir: 1 }))
            }
            None => (at, 0.0, None),
        };
        if tag != 0 {
            let pv = self.player.vehicle;
            for (i, v) in self.vehicles.iter_mut().enumerate() {
                if v.tag == tag && Some(i) != pv {
                    v.remove = true;
                }
                if v.tag == tag {
                    v.tag = 0;
                }
            }
        }
        self.vehicles.push(Vehicle {
            pos,
            vel: Vec2::ZERO,
            angle,
            kind,
            color,
            driver,
            health: 100.0,
            route: if driver == Driver::None { None } else { route },
            stuck: 0.0,
            reverse: 0.0,
            tag,
            dead: false,
            remove: false,
        });
    }

    pub fn tagged(&self, tag: u8) -> Option<usize> {
        self.vehicles.iter().position(|v| v.tag == tag && !v.remove)
    }

    /// Fire the current weapon along the aim (or facing) direction.
    pub fn shoot(&mut self, map: &Map) {
        if self.fire_cd > 0.0 {
            return;
        }
        let (_, cd, dmg, range, spread, _) = WEAPONS[self.weapon];
        if self.weapon > 0 && self.ammo[self.weapon] <= 0 {
            self.msg("Plus de munitions ! (armurerie : marqueur rouge)");
            self.fire_cd = 0.6;
            return;
        }
        self.fire_cd = cd;
        let yaw = if self.aiming { self.aim_yaw } else { self.player.angle } + self.rng.range(-spread, spread);
        let dir = vec2(yaw.cos(), yaw.sin());
        let origin = self.player.pos + dir * 0.4;
        if self.weapon == 0 {
            // a punch: the closest person in front
            let hit = self.peds.iter().position(|p| p.down <= 0.0 && (p.pos - origin).length() < range && (p.pos - origin).normalized().dot(dir) > 0.5);
            if let Some(i) = hit {
                self.peds[i].down = 12.0;
                self.fx.push(Fx { kind: 2, a: self.peds[i].pos, b: self.peds[i].pos, t: 0.2 });
                self.msg("Coup de poing !");
                if self.police_near(60.0) {
                    self.add_wanted(1);
                }
            }
            return;
        }
        self.ammo[self.weapon] -= 1;
        let mut end = origin + dir * range;
        let mut hit_ped: Option<usize> = None;
        let mut hit_car: Option<usize> = None;
        let mut t = 0.6;
        'march: while t < range {
            let p = origin + dir * t;
            if map.point_in_building(p) {
                end = p;
                break;
            }
            for (i, pd) in self.peds.iter().enumerate() {
                if pd.down <= 0.0 && (pd.pos - p).length() < 0.55 {
                    hit_ped = Some(i);
                    end = p;
                    break 'march;
                }
            }
            for (i, v) in self.vehicles.iter().enumerate() {
                if Some(i) != self.player.vehicle && !v.remove && (v.pos - p).length() < v.kind.spec().1 + 0.7 {
                    hit_car = Some(i);
                    end = p;
                    break 'march;
                }
            }
            t += 0.7;
        }
        self.fx.push(Fx { kind: 0, a: origin, b: end, t: 0.07 });
        self.fx.push(Fx { kind: 1, a: origin, b: origin + dir, t: 0.05 });
        self.fx.push(Fx { kind: 2, a: end, b: end, t: 0.25 });
        // gunfire scares the street
        for p in &mut self.peds {
            if p.down <= 0.0 && (p.pos - origin).length() < 40.0 {
                p.flee = 4.0;
            }
        }
        if let Some(i) = hit_ped {
            self.peds[i].down = 30.0;
            self.fx.push(Fx { kind: 3, a: self.peds[i].pos, b: self.peds[i].pos, t: 30.0 });
            self.add_wanted(if self.police_near(80.0) { 2 } else { 1 });
        } else if let Some(i) = hit_car {
            let v = &mut self.vehicles[i];
            v.health -= dmg * 0.5;
            if v.driver == Driver::Police {
                self.add_wanted(1);
            } else if v.driver == Driver::Traffic {
                // the driver jumps out and runs
                v.driver = Driver::None;
                v.vel = Vec2::ZERO;
                let pos = v.pos + vec2(-v.angle.sin(), v.angle.cos()) * 2.0;
                let road = v.route.map(|r| r.road).unwrap_or(0);
                self.peds.push(Ped { pos, angle: 0.0, road, i: 0, dir: 1, side: 1.0, flee: 6.0, down: 0.0, color: CLOTHES[self.rng.below(8)], remove: false });
            }
            if self.police_near(80.0) && self.player.wanted == 0 {
                self.add_wanted(1);
            }
        } else if self.police_near(50.0) && self.player.wanted == 0 {
            self.add_wanted(1);
        }
    }

    fn police_near(&self, r: f32) -> bool {
        let pp = self.player_pos();
        self.vehicles.iter().any(|v| v.driver == Driver::Police && !v.dead && (v.pos - pp).length() < r)
    }

    /// Police (from 2 stars) and gangsters shoot back.
    fn enemy_fire(&mut self) {
        if self.enemy_cd > 0.0 {
            return;
        }
        self.enemy_cd = 0.8;
        let pp = self.player_pos();
        let shooters: Vec<Pos2> = self
            .vehicles
            .iter()
            .filter(|v| !v.dead && ((v.driver == Driver::Police && self.player.wanted >= 2) || v.driver == Driver::Gang))
            .filter(|v| (v.pos - pp).length() < 40.0)
            .map(|v| v.pos)
            .collect();
        if shooters.is_empty() {
            return;
        }
        let from = shooters[self.rng.below(shooters.len())];
        let moving = self.player.vehicle.map_or(self.player.moving, |v| self.vehicles[v].speed().abs() / 10.0);
        let chance = (0.45 - moving.min(1.5) * 0.18).max(0.12);
        let hit = self.rng.f() < chance;
        let miss = if hit { Vec2::ZERO } else { vec2(self.rng.range(-2.0, 2.0), self.rng.range(-2.0, 2.0)) };
        self.fx.push(Fx { kind: 0, a: from, b: pp + miss, t: 0.07 });
        self.fx.push(Fx { kind: 1, a: from, b: pp, t: 0.05 });
        if hit {
            self.player.health -= 6.0 + self.player.wanted as f32;
            self.fx.push(Fx { kind: 2, a: pp, b: pp, t: 0.2 });
        }
    }

    /// Buy at a gun shop: the pistol, the rifle or ammunition.
    pub fn buy_at_shop(&mut self) -> String {
        if !self.owned[1] {
            if self.player.money < 15_000 {
                return "Pistolet : 15 000 FCFA".into();
            }
            self.player.money -= 15_000;
            self.owned[1] = true;
            self.ammo[1] += 36;
            return "Pistolet acheté (+36 balles)".into();
        }
        if !self.owned[2] {
            if self.player.money < 120_000 {
                if self.player.money >= 2_000 {
                    self.player.money -= 2_000;
                    self.ammo[1] += 36;
                    return "+36 balles de pistolet (2 000 FCFA). Fusil d'assaut : 120 000 FCFA".into();
                }
                return "Il te faut 2 000 FCFA pour des munitions".into();
            }
            self.player.money -= 120_000;
            self.owned[2] = true;
            self.ammo[2] += 120;
            self.weapon = 2;
            return "Fusil d'assaut acheté (+120 balles)".into();
        }
        if self.player.money < 5_000 {
            return "Munitions : 5 000 FCFA".into();
        }
        self.player.money -= 5_000;
        self.ammo[1] += 36;
        self.ammo[2] += 90;
        "Munitions rechargées (5 000 FCFA)".into()
    }

    fn add_wanted(&mut self, n: u8) {
        let before = self.player.wanted;
        self.player.wanted = (self.player.wanted + n).min(5);
        self.player.wanted_timer = 0.0;
        if self.player.wanted > before {
            self.msg(format!("Recherché par la police : {} étoile(s)", self.player.wanted));
        }
    }

    /// Pick a random lane position on a road near `around`.
    fn random_lane(&mut self, map: &Map, around: Pos2, rmin: f32, rmax: f32, max_kind: u8) -> Option<(Pos2, f32, Route)> {
        for _ in 0..6 {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let d = self.rng.range(rmin, rmax);
            let p = around + vec2(a.cos(), a.sin()) * d;
            if let Some((ri, si, t, _)) = map.nearest_road(p, 120.0, max_kind) {
                let r = &map.roads[ri];
                let (pa, pb) = (r.pts[si], r.pts[si + 1]);
                let dir: i32 = if self.rng.f() < 0.5 { 1 } else { -1 };
                let dvec = if dir == 1 { (pb - pa).normalized() } else { (pa - pb).normalized() };
                let pos = pa + (pb - pa) * t + right_of(dvec) * r.half_width() * 0.45;
                let target = if dir == 1 { si + 1 } else { si };
                return Some((pos, dvec.y.atan2(dvec.x), Route { road: ri, i: target, dir }));
            }
        }
        None
    }

    fn populate(&mut self, map: &Map, initial: bool) {
        let pp = self.player_pos();
        let rmin = if initial { 30.0 } else { 220.0 };
        // traffic
        let traffic = self.vehicles.iter().filter(|v| v.driver == Driver::Traffic).count();
        if traffic < 45 {
            let r = self.rng.f();
            let kind = if r < 0.45 { VKind::Moto } else if r < 0.75 { VKind::Car } else if r < 0.9 { VKind::Taxi } else { VKind::Minibus };
            let max_kind = if kind == VKind::Moto { 4 } else { 3 };
            if let Some((pos, angle, route)) = self.random_lane(map, pp, rmin, 450.0, max_kind) {
                let color = match kind {
                    VKind::Taxi => Color32::from_rgb(30, 140, 60),
                    VKind::Minibus => Color32::from_rgb(230, 230, 220),
                    _ => CAR_COLORS[self.rng.below(CAR_COLORS.len())],
                };
                self.vehicles.push(Vehicle {
                    pos,
                    vel: vec2(angle.cos(), angle.sin()) * 8.0,
                    angle,
                    kind,
                    color,
                    driver: Driver::Traffic,
                    health: 100.0,
                    route: Some(route),
                    stuck: 0.0,
                    reverse: 0.0,
            tag: 0,
                    dead: false,
                    remove: false,
                });
            }
        }
        // parked vehicles on the roadside
        let parked = self.vehicles.iter().filter(|v| v.driver == Driver::None && !v.dead).count();
        if parked < 14 {
            if let Some((pos, angle, route)) = self.random_lane(map, pp, rmin.min(60.0), 400.0, 4) {
                let r = &map.roads[route.road];
                let side = right_of(vec2(angle.cos(), angle.sin())) * (r.half_width() * 0.5 + 0.6);
                let kind = if self.rng.f() < 0.5 { VKind::Moto } else { VKind::Car };
                self.vehicles.push(Vehicle {
                    pos: pos + side,
                    vel: Vec2::ZERO,
                    angle,
                    kind,
                    color: CAR_COLORS[self.rng.below(CAR_COLORS.len())],
                    driver: Driver::None,
                    health: 100.0,
                    route: None,
                    stuck: 0.0,
                    reverse: 0.0,
            tag: 0,
                    dead: false,
                    remove: false,
                });
            }
        }
        // pedestrians
        if self.peds.len() < 55 {
            if let Some((pos, angle, route)) = self.random_lane(map, pp, rmin.min(40.0), 300.0, 4) {
                let r = &map.roads[route.road];
                let side = if self.rng.f() < 0.5 { 1.0 } else { -1.0 };
                let off = right_of(vec2(angle.cos(), angle.sin())) * (r.half_width() * (0.55 * side) + 1.4 * side);
                let color = CLOTHES[self.rng.below(CLOTHES.len())];
                self.peds.push(Ped {
                    pos: pos + off,
                    angle,
                    road: route.road,
                    i: route.i,
                    dir: route.dir,
                    side,
                    flee: 0.0,
                    down: 0.0,
                    color,
                    remove: false,
                });
            }
        }
        // police chasing a wanted player
        let police = self.vehicles.iter().filter(|v| v.driver == Driver::Police).count();
        let want = (self.player.wanted as usize * 2).min(8);
        if police < want {
            if let Some((pos, angle, _)) = self.random_lane(map, pp, 180.0, 300.0, 4) {
                self.vehicles.push(Vehicle {
                    pos,
                    vel: Vec2::ZERO,
                    angle,
                    kind: VKind::Police,
                    color: Color32::from_rgb(25, 50, 120),
                    driver: Driver::Police,
                    health: 100.0,
                    route: None,
                    stuck: 0.0,
                    reverse: 0.0,
            tag: 0,
                    dead: false,
                    remove: false,
                });
            }
        }
    }

    pub fn update(&mut self, map: &Map, inp: Input, dt: f32) {
        self.time += dt;
        self.hours = (self.hours + dt * 24.0 / 480.0) % 24.0;
        for m in &mut self.messages {
            m.1 -= dt;
        }
        self.messages.retain(|m| m.1 > 0.0);
        self.repair_cd -= dt;
        self.honk = (self.honk - dt).max(0.0);
        if inp.horn {
            self.honk = 0.4;
        }

        self.populate(map, false);
        if let Some(w) = inp.switch {
            if self.owned[w] {
                self.weapon = w;
            }
        }
        self.fire_cd -= dt;
        self.enemy_cd -= dt;
        for f in &mut self.fx {
            f.t -= dt;
        }
        self.fx.retain(|f| f.t > 0.0);
        self.aiming = inp.aim && self.player.vehicle.is_none();
        self.aim_yaw = inp.aim_yaw;
        if self.aiming {
            self.player.angle = inp.aim_yaw;
        }
        self.update_player(map, inp, dt);
        let (_, cd, _, _, _, auto) = WEAPONS[self.weapon];
        let _ = cd;
        if self.player.vehicle.is_none() && (inp.fire || (auto && inp.fire_held && self.aiming)) {
            self.shoot(map);
        }
        self.enemy_fire();
        self.update_traffic(map, dt);
        self.update_police(map, dt);
        self.update_peds(map, dt);
        self.vehicle_collisions(dt);
        self.update_mission(map, dt);
        self.update_wanted(map, dt);
        self.cleanup();
    }

    fn update_player(&mut self, map: &Map, inp: Input, dt: f32) {
        if inp.action {
            self.toggle_vehicle(map);
        }
        match self.player.vehicle {
            Some(vi) => {
                let throttle = (inp.up as i32 - inp.down as i32) as f32;
                let steer = (inp.right as i32 - inp.left as i32) as f32;
                let on_road = map.road_at(self.vehicles[vi].pos, 0.5).is_some();
                drive(&mut self.vehicles[vi], map, throttle, steer, inp.brake, on_road, dt);
                let v = &self.vehicles[vi];
                self.player.pos = v.pos;
                self.player.angle = v.angle;
                if v.dead {
                    self.eject(map);
                    self.player.health -= 30.0;
                    self.msg("Ton véhicule est détruit !");
                }
                // repair at a fuel station
                if let Some(vi) = self.player.vehicle {
                    let v = &self.vehicles[vi];
                    if v.health < 99.0 && v.speed().abs() < 3.0 && self.repair_cd <= 0.0 {
                        if let Some(f) = map.nearest_poi(v.pos, "F") {
                            if (f.pos - v.pos).length() < 18.0 && self.player.money >= 500 {
                                self.vehicles[vi].health = 100.0;
                                self.player.money -= 500;
                                self.repair_cd = 5.0;
                                self.msg("Véhicule réparé à la station (500 FCFA)");
                            }
                        }
                    }
                }
            }
            None => {
                self.player.angle += (inp.right as i32 - inp.left as i32) as f32 * 2.6 * dt;
                let mv = (inp.up as i32 - inp.down as i32) as f32;
                let sp = if mv < 0.0 { 2.0 } else if inp.sprint { 7.0 } else { 3.2 };
                let d = vec2(self.player.angle.cos(), self.player.angle.sin()) * mv;
                self.player.pos += d * sp * dt;
                self.player.walk += mv.abs() * sp * dt;
                let target = if mv != 0.0 { (sp / 4.0).min(1.3) } else { 0.0 };
                self.player.moving += (target - self.player.moving) * (dt * 8.0).min(1.0);
                map.collide(&mut self.player.pos, 0.4);
            }
        }
        if self.player.health <= 0.0 {
            let pos = self.player_pos();
            let h = map.nearest_poi(pos, "H").map(|h| h.pos).unwrap_or(pos);
            self.eject(map);
            self.player.pos = h + vec2(0.0, 12.0);
            map.collide(&mut self.player.pos, 0.4);
            self.player.health = 100.0;
            self.player.money = (self.player.money - 1000).max(0);
            self.player.wanted = 0;
            self.mission = None;
            self.ev_died = true;
            self.msg("Hospitalisé ! Les soins coûtent 1000 FCFA.");
        }
    }

    fn eject(&mut self, map: &Map) {
        if let Some(vi) = self.player.vehicle.take() {
            let v = &mut self.vehicles[vi];
            if v.driver == Driver::Player {
                v.driver = Driver::None;
            }
            let side = right_of(v.fwd()) * -(v.kind.spec().1 + 1.0);
            self.player.pos = v.pos + side;
            map.collide(&mut self.player.pos, 0.4);
        }
    }

    fn toggle_vehicle(&mut self, map: &Map) {
        if let Some(vi) = self.player.vehicle {
            if self.vehicles[vi].speed().abs() < 4.0 {
                self.eject(map);
            } else {
                self.msg("Ralentis pour descendre !");
            }
            return;
        }
        let p = self.player.pos;
        let best = self
            .vehicles
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.dead && !v.remove)
            .map(|(i, v)| (i, (v.pos - p).length() - v.kind.spec().0))
            .filter(|(_, d)| *d < 2.5)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((vi, _)) = best {
            let (drv, kind) = (self.vehicles[vi].driver, self.vehicles[vi].kind);
            match drv {
                Driver::Traffic => {
                    let pos = self.vehicles[vi].pos + right_of(self.vehicles[vi].fwd()) * -2.5;
                    let road = self.vehicles[vi].route.map(|r| r.road).unwrap_or(0);
                    self.peds.push(Ped {
                        pos,
                        angle: 0.0,
                        road,
                        i: 0,
                        dir: 1,
                        side: 1.0,
                        flee: 6.0,
                        down: 0.0,
                        color: CLOTHES[self.rng.below(8)],
                        remove: false,
                    });
                    self.msg(format!("Tu as volé un(e) {} !", kind.name().to_lowercase()));
                    self.add_wanted(1);
                }
                Driver::Police => {
                    self.msg("Tu as volé une voiture de police !");
                    self.add_wanted(2);
                }
                _ => {}
            }
            let v = &mut self.vehicles[vi];
            v.driver = Driver::Player;
            v.route = None;
            self.player.vehicle = Some(vi);
        }
    }

    fn update_traffic(&mut self, map: &Map, dt: f32) {
        let pp = self.player_pos();
        // obstacles: positions of everything (for braking)
        let obstacles: Vec<Pos2> = self
            .vehicles
            .iter()
            .filter(|v| !v.remove)
            .map(|v| v.pos)
            .chain(self.peds.iter().filter(|p| p.down <= 0.0).map(|p| p.pos))
            .chain(std::iter::once(pp))
            .collect();
        for vi in 0..self.vehicles.len() {
            let gang = self.vehicles[vi].driver == Driver::Gang;
            if !(self.vehicles[vi].driver == Driver::Traffic || gang) || self.vehicles[vi].dead {
                continue;
            }
            let Some(mut route) = self.vehicles[vi].route else { continue };
            let road = &map.roads[route.road];
            let target = road.pts[route.i];
            let prev_i = (route.i as i32 - route.dir).clamp(0, road.pts.len() as i32 - 1) as usize;
            let segd = (target - road.pts[prev_i]).normalized();
            let aim = target + right_of(segd) * road.half_width() * 0.45;
            let v = &self.vehicles[vi];
            if (aim - v.pos).length() < 5.0 || (target - v.pos).length() < 4.0 {
                let ni = route.i as i32 + route.dir;
                if ni < 0 || ni >= road.pts.len() as i32 {
                    route = self.next_road(map, route);
                } else {
                    route.i = ni as usize;
                }
                self.vehicles[vi].route = Some(route);
                continue;
            }
            let to = aim - v.pos;
            let want_angle = to.y.atan2(to.x);
            let turn = angle_diff(v.angle, want_angle);
            let base = match road.kind { 0 => 20.0, 1 => 16.0, 2 => 14.0, 3 => 12.0, _ => 8.0 };
            let mut desired = base * (1.0 - turn.abs().min(1.2) / 1.6);
            if v.kind == VKind::Moto {
                desired *= 1.1;
            }
            if gang {
                desired = (base * 1.6).max(14.0) * (1.0 - turn.abs().min(1.2) / 2.0);
            }
            let fwd = v.fwd();
            let me = v.pos;
            let hl = v.kind.spec().0;
            for (k, &o) in obstacles.iter().enumerate() {
                if k == vi {
                    continue;
                }
                let d = o - me;
                let ahead = d.dot(fwd);
                if !gang && ahead > hl && ahead < hl + 9.0 && (d - fwd * ahead).length() < 2.2 {
                    desired = desired.min(((ahead - hl - 3.0) * 1.5).max(0.0));
                }
            }
            let v = &mut self.vehicles[vi];
            let sp = v.speed();
            let nsp = if desired > sp { (sp + 4.0 * dt).min(desired) } else { (sp - 9.0 * dt).max(desired) };
            v.angle += turn.clamp(-2.0 * dt, 2.0 * dt) * (nsp / 5.0).clamp(0.3, 1.0) * 1.6;
            v.vel = v.fwd() * nsp.max(0.0);
            v.pos += v.vel * dt;
            if map.collide(&mut v.pos, v.kind.spec().1).is_some() {
                v.stuck += dt;
            } else {
                v.stuck = (v.stuck - dt).max(0.0);
            }
            if v.stuck > 4.0 && v.tag == 0 {
                v.remove = true; // give up and vanish (rare)
            }
            if v.stuck > 2.0 && v.tag != 0 {
                v.angle += 1.5; // gang driver: wriggle out
                v.stuck = 0.0;
            }
            if (v.pos - pp).length() > 700.0 && v.tag == 0 {
                v.remove = true;
            }
        }
    }

    fn next_road(&mut self, map: &Map, route: Route) -> Route {
        let road = &map.roads[route.road];
        let end = road.pts[route.i];
        let main = road.kind <= 3;
        let mut cands: Vec<(usize, usize)> = Vec::new();
        map.road_grid.query(end, end, |id| {
            let (ri, si) = map.segs[id as usize];
            let (ri, si) = (ri as usize, si as usize);
            if ri == route.road {
                return;
            }
            let r = &map.roads[ri];
            for k in [si, si + 1] {
                if (r.pts[k] - end).length() < 4.0 && !cands.contains(&(ri, k)) {
                    cands.push((ri, k));
                }
            }
        });
        let preferred: Vec<(usize, usize)> =
            cands.iter().copied().filter(|(r, _)| !main || map.roads[*r].kind <= 3).collect();
        let pool = if preferred.is_empty() { &cands } else { &preferred };
        if pool.is_empty() {
            // dead end: U-turn
            let dir = -route.dir;
            let i = (route.i as i32 + dir).clamp(0, road.pts.len() as i32 - 1) as usize;
            return Route { road: route.road, i, dir };
        }
        let (ri, k) = pool[self.rng.below(pool.len())];
        let n = map.roads[ri].pts.len();
        let dir = if k == 0 { 1 } else if k == n - 1 { -1 } else if self.rng.f() < 0.5 { 1 } else { -1 };
        Route { road: ri, i: (k as i32 + dir) as usize, dir }
    }

    fn update_police(&mut self, map: &Map, dt: f32) {
        let pp = self.player_pos();
        let pvel = self.player.vehicle.map(|v| self.vehicles[v].vel).unwrap_or(Vec2::ZERO);
        for vi in 0..self.vehicles.len() {
            if self.vehicles[vi].driver != Driver::Police || self.vehicles[vi].dead {
                continue;
            }
            let v = &mut self.vehicles[vi];
            if self.player.wanted == 0 {
                // drive away and disappear
                let on_road = map.road_at(v.pos, 0.5).is_some();
                drive(v, map, 0.6, 0.0, false, on_road, dt);
                if (v.pos - pp).length() > 250.0 {
                    v.remove = true;
                }
                continue;
            }
            let dist = (pp - v.pos).length();
            let target = pp + pvel * (dist / 40.0).min(1.5);
            let to = target - v.pos;
            let want = to.y.atan2(to.x);
            let mut turn = angle_diff(v.angle, want);
            let mut throttle = if dist < 12.0 { (v.speed() > 3.0) as i32 as f32 * -1.0 } else { 1.0 };
            if v.reverse > 0.0 {
                v.reverse -= dt;
                throttle = -1.0;
                turn = -turn;
            } else if v.speed().abs() < 1.0 && dist > 14.0 {
                v.stuck += dt;
                if v.stuck > 1.5 {
                    v.reverse = 1.2;
                    v.stuck = 0.0;
                }
            } else {
                v.stuck = 0.0;
            }
            let steer = (turn * 2.0).clamp(-1.0, 1.0);
            let on_road = map.road_at(v.pos, 0.5).is_some();
            drive(v, map, throttle, steer, false, on_road, dt);
            if dist > 650.0 {
                v.remove = true;
            }
        }
    }

    fn update_peds(&mut self, map: &Map, dt: f32) {
        let pp = self.player_pos();
        let pv = self.player.vehicle.map(|v| (self.vehicles[v].vel, self.vehicles[v].kind.spec().0));
        let mut hits = 0;
        for i in 0..self.peds.len() {
            let p = &mut self.peds[i];
            if p.down > 0.0 {
                p.down -= dt;
                if p.down <= 0.0 {
                    p.remove = true;
                }
                continue;
            }
            // hit by the player's vehicle
            if let Some((vel, hl)) = pv {
                let d = (p.pos - pp).length();
                if d < hl * 0.8 + 0.6 && vel.length() > 4.0 {
                    p.down = 25.0;
                    hits += 1;
                    continue;
                }
                if d < 14.0 && vel.length() > 7.0 && p.flee <= 0.0 {
                    p.flee = 2.0;
                }
            }
            if p.flee > 0.0 {
                p.flee -= dt;
                let away = (p.pos - pp).normalized();
                p.angle = away.y.atan2(away.x);
                p.pos += away * 6.0 * dt;
                map.collide(&mut p.pos, 0.4);
                continue;
            }
            let road = &map.roads[p.road];
            if p.i >= road.pts.len() {
                p.remove = true;
                continue;
            }
            let target = road.pts[p.i];
            let prev = (p.i as i32 - p.dir).clamp(0, road.pts.len() as i32 - 1) as usize;
            let segd = (target - road.pts[prev]).normalized();
            let aim = target + right_of(segd) * (road.half_width() + 1.4) * p.side;
            if (aim - p.pos).length() < 2.0 {
                let ni = p.i as i32 + p.dir;
                if ni < 0 || ni >= road.pts.len() as i32 {
                    p.dir = -p.dir;
                    p.side = -p.side;
                } else {
                    p.i = ni as usize;
                }
                continue;
            }
            let to = (aim - p.pos).normalized();
            p.angle = to.y.atan2(to.x);
            p.pos += to * 1.4 * dt;
            map.collide(&mut p.pos, 0.35);
            if (p.pos - pp).length() > 400.0 {
                p.remove = true;
            }
        }
        if hits > 0 {
            self.msg("Un piéton a été renversé !");
            self.add_wanted(1);
        }
    }

    fn vehicle_collisions(&mut self, _dt: f32) {
        let n = self.vehicles.len();
        let mut police_hit = false;
        let mut player_damage = 0.0;
        for a in 0..n {
            for b in a + 1..n {
                let (va, vb) = (&self.vehicles[a], &self.vehicles[b]);
                if va.remove || vb.remove {
                    continue;
                }
                let ra = va.kind.spec().0 * 0.75 + 0.3;
                let rb = vb.kind.spec().0 * 0.75 + 0.3;
                let d = vb.pos - va.pos;
                let dl = d.length();
                if dl >= ra + rb || dl < 1e-4 {
                    continue;
                }
                let nrm = d / dl;
                let overlap = ra + rb - dl;
                let rel = (va.vel - vb.vel).dot(nrm);
                let (ma, mb) = (mass(va.kind), mass(vb.kind));
                let player_involved = va.driver == Driver::Player || vb.driver == Driver::Player;
                let police_involved = va.driver == Driver::Police || vb.driver == Driver::Police;
                let (pa, pb) = self.vehicles.split_at_mut(b);
                let va = &mut pa[a];
                let vb = &mut pb[0];
                va.pos -= nrm * overlap * mb / (ma + mb);
                vb.pos += nrm * overlap * ma / (ma + mb);
                if rel > 0.0 {
                    let j = rel * 1.3 / (1.0 / ma + 1.0 / mb);
                    va.vel -= nrm * j / ma;
                    vb.vel += nrm * j / mb;
                    if rel > 6.0 {
                        let dmg = (rel - 6.0) * 3.0;
                        va.health -= dmg * mb / (ma + mb) * 2.0;
                        vb.health -= dmg * ma / (ma + mb) * 2.0;
                        if player_involved {
                            player_damage += dmg * 0.3;
                            if police_involved {
                                police_hit = true;
                            }
                        }
                    }
                }
            }
        }
        for v in &mut self.vehicles {
            if v.health <= 0.0 && !v.dead {
                v.dead = true;
                v.vel = Vec2::ZERO;
                if v.driver != Driver::Player {
                    v.driver = Driver::None;
                }
            }
        }
        if player_damage > 0.0 && self.player.vehicle.map_or(false, |v| self.vehicles[v].kind == VKind::Moto) {
            self.player.health -= player_damage;
        }
        if police_hit && self.player.wanted < 2 {
            self.msg("Tu as percuté la police !");
            self.add_wanted(1);
        }
    }

    fn update_mission(&mut self, map: &Map, dt: f32) {
        let pp = self.player_pos();
        let in_vehicle = self.player.vehicle.is_some();
        if let Some(m) = &mut self.mission {
            m.time_left -= dt;
            if in_vehicle && (m.to - pp).length() < 14.0 {
                let bonus = (m.time_left.max(0.0) * 20.0) as i64 / 50 * 50;
                let total = m.reward + bonus;
                self.player.money += total;
                let name = m.to_name.clone();
                self.mission = None;
                self.msg(format!("Livré à {name} ! +{total} FCFA"));
            } else if m.time_left <= 0.0 {
                self.mission = None;
                self.msg("Trop tard, le client a annulé la course.");
            }
            return;
        }
        if !in_vehicle || self.story {
            return;
        }
        let Some(&spot) = self.spots.iter().find(|s| (**s - pp).length() < 12.0) else { return };
        // choose a destination 700-3500 m away
        let cands: Vec<usize> = map
            .pois
            .iter()
            .enumerate()
            .filter(|(_, p)| "MHFTS".contains(p.kind))
            .filter(|(_, p)| {
                let d = (p.pos - spot).length();
                (700.0..3500.0).contains(&d)
            })
            .map(|(i, _)| i)
            .collect();
        if cands.is_empty() {
            return;
        }
        let poi = &map.pois[cands[self.rng.below(cands.len())]];
        let dist = (poi.pos - spot).length();
        let to = map
            .nearest_road(poi.pos, 200.0, 4)
            .map(|(r, s, t, _)| {
                let rd = &map.roads[r];
                rd.pts[s] + (rd.pts[s + 1] - rd.pts[s]) * t
            })
            .unwrap_or(poi.pos);
        let reward = ((dist * 2.5) as i64 / 50) * 50 + 500;
        let label = poi.label();
        let zone = map.place_name(poi.pos);
        self.mission = Some(Mission { to, to_name: label.clone(), time_left: dist / 10.0 + 30.0, reward });
        self.msg(format!("Course : livre ce colis à {label} ({zone}) — {reward} FCFA"));
        // move the start spot slightly so it does not retrigger instantly
        self.spots.retain(|s| *s != spot);
        self.spots.push(spot);
    }

    fn update_wanted(&mut self, map: &Map, dt: f32) {
        if self.player.wanted == 0 {
            self.player.arrest_timer = 0.0;
            return;
        }
        let pp = self.player_pos();
        let slow = self.player.vehicle.map_or(true, |v| self.vehicles[v].speed().abs() < 3.0);
        let mut seen = false;
        let mut close = false;
        for v in &self.vehicles {
            if v.driver == Driver::Police && !v.dead {
                let d = (v.pos - pp).length();
                if d < 90.0 {
                    seen = true;
                }
                if d < 8.0 {
                    close = true;
                }
            }
        }
        if seen {
            self.player.wanted_timer = 0.0;
        } else {
            self.player.wanted_timer += dt;
            if self.player.wanted_timer > 18.0 {
                self.player.wanted -= 1;
                self.player.wanted_timer = 0.0;
                if self.player.wanted == 0 {
                    self.msg("Tu as semé la police.");
                } else {
                    self.msg(format!("La police perd ta trace… {} étoile(s)", self.player.wanted));
                }
            }
        }
        if close && slow {
            self.player.arrest_timer += dt;
            if self.player.arrest_timer > 2.5 {
                let fine = (self.player.wanted as i64 * 2000).min(self.player.money);
                self.player.money -= fine;
                self.player.wanted = 0;
                self.player.arrest_timer = 0.0;
                self.mission = None;
                let station = map.nearest_poi(pp, "P").map(|p| p.pos).unwrap_or(pp);
                self.eject(map);
                self.player.pos = station + vec2(0.0, 10.0);
                map.collide(&mut self.player.pos, 0.4);
                self.ev_arrested = true;
                self.msg(format!("Arrêté ! Amende de {fine} FCFA."));
            }
        } else {
            self.player.arrest_timer = (self.player.arrest_timer - dt).max(0.0);
        }
    }

    fn cleanup(&mut self) {
        let pp = self.player_pos();
        for v in &mut self.vehicles {
            if v.tag == 0 && v.driver == Driver::None && (v.pos - pp).length() > 600.0 {
                v.remove = true;
            }
        }
        if let Some(pv) = self.player.vehicle {
            self.vehicles[pv].remove = false;
        }
        // remove while keeping the player's vehicle index valid
        let mut new_index = None;
        let mut k = 0;
        let mut out = Vec::with_capacity(self.vehicles.len());
        for (i, v) in self.vehicles.drain(..).enumerate() {
            if !v.remove {
                if Some(i) == self.player.vehicle {
                    new_index = Some(k);
                }
                out.push(v);
                k += 1;
            }
        }
        self.vehicles = out;
        if self.player.vehicle.is_some() {
            self.player.vehicle = new_index;
        }
        self.peds.retain(|p| !p.remove);
    }
}

fn mass(k: VKind) -> f32 {
    match k {
        VKind::Moto => 0.3,
        VKind::Minibus => 2.5,
        _ => 1.3,
    }
}

/// Arcade driving physics shared by the player and the police.
pub fn drive(v: &mut Vehicle, map: &Map, throttle: f32, steer: f32, handbrake: bool, on_road: bool, dt: f32) {
    let (hl, hw, max, accel, steer_rate) = v.kind.spec();
    let healthf = if v.health < 30.0 { 0.6 } else { 1.0 };
    let fwd = v.fwd();
    let right = right_of(fwd);
    let mut fs = v.vel.dot(fwd);
    let mut ls = v.vel.dot(right);
    if throttle > 0.0 {
        fs += if fs < 0.0 { accel * 2.0 } else { accel * healthf } * throttle * dt;
    } else if throttle < 0.0 {
        fs += if fs > 0.5 { -accel * 2.2 } else { -accel * 0.6 } * -throttle * dt;
    }
    let maxs = if on_road { max * healthf } else { max * 0.5 };
    fs = fs.clamp(-max * 0.3, maxs.max(fs - 12.0 * dt));
    let drag = if on_road { 0.25 } else { 0.9 };
    fs -= fs * drag * dt;
    if throttle == 0.0 {
        fs -= fs.signum() * (2.0 * dt).min(fs.abs());
    }
    if handbrake {
        fs -= fs.signum() * (10.0 * dt).min(fs.abs());
    }
    let grip: f32 = if handbrake { 1.5 } else { 9.0 };
    ls *= (-grip * dt).exp();
    let turn_amount = (fs.abs() / 6.0).min(1.0) * fs.signum();
    let speed_damp = 1.0 / (1.0 + fs.abs() / 35.0);
    v.angle += steer * steer_rate * turn_amount * speed_damp * dt;
    let fwd = v.fwd();
    let right = right_of(fwd);
    v.vel = fwd * fs + right * ls;
    v.pos += v.vel * dt;
    // collide with buildings using a front and a back circle
    let mut hit = Vec2::ZERO;
    for s in [hl - hw, -(hl - hw)] {
        let mut c = v.pos + fwd * s;
        if let Some(n) = map.collide(&mut c, hw) {
            let moved = c - (v.pos + fwd * s);
            v.pos += moved;
            hit += n;
        }
    }
    if hit.length_sq() > 0.0 {
        let n = hit.normalized();
        let into = v.vel.dot(n);
        if into < 0.0 {
            if -into > 7.0 {
                v.health -= (-into - 7.0) * 3.0;
            }
            v.vel -= n * into * 1.3;
        }
    }
    if map.in_area(v.pos, 0) {
        v.vel *= 0.9;
        v.health -= 25.0 * dt;
    }
}

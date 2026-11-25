//! Everyday life: doors and interiors (maquis, apartment, gym, garage), eating, sleeping,
//! working out, talking to people, collisions of Bila with vehicles and passers-by.

use crate::game::{ActKind, Activity, Door, Driver, Game, Talk, VKind};
use crate::map::Map;
use crate::statics::{self, I_APPART, I_GARAGE, I_GYM, I_MAQUIS};
use eframe::egui::{vec2, Color32, Pos2, Vec2};

const GREETINGS: [&str; 10] = [
    "Ne y windiga ! (Bonjour !)",
    "Laafi bala ? (Ça va ?)",
    "Wẽnd na kõ-d laafi ! (Que Dieu te donne la santé !)",
    "Ah Bila ! On dit quoi ?",
    "Ouaga, c'est le pays des hommes intègres, mon frère.",
    "Le dolo du maquis d'à côté est très bon.",
    "Attention aux Vautours dans le quartier…",
    "Il fait chaud hein ! Le harmattan arrive bientôt.",
    "Tu as vu le match des Étalons hier ?",
    "Si tu cherches du travail, va voir au Grand Marché.",
];
const REPLIES: [&str; 6] = ["Laafi bala ! Ça va bien.", "Merci, à plus tard !", "Bon courage, Bila !", "Wẽnd na kõ-d laafi.", "On est ensemble !", "Bonne journée, mon frère."];

pub fn make_doors(map: &Map) -> Vec<Door> {
    let mut doors = Vec::new();
    let mut add = |name: &str, fallback: Pos2, off: Vec2, interior: usize, label: &str| {
        let p = map.poi_named(name).unwrap_or(fallback);
        doors.push(Door { interior, pos: map.road_point(p + off), label: label.to_string() });
    };
    add("Gare de Ouagadougou", Pos2::new(-714.0, -952.0), vec2(-40.0, 25.0), I_APPART, "Appartement de Bila");
    add("Gare de Ouagadougou", Pos2::new(-714.0, -952.0), vec2(-25.0, 45.0), I_GARAGE, "Garage de Bila");
    add("Grand Marché", Pos2::new(-381.0, -364.0), vec2(-45.0, 30.0), I_MAQUIS, "Maquis-café « Le Faso »");
    add("Marché de Zogona", Pos2::new(2799.0, -1629.0), vec2(30.0, -20.0), I_MAQUIS, "Maquis de Zogona");
    add("Marché de Gounghin", Pos2::new(-2830.0, 940.0), vec2(-25.0, 20.0), I_MAQUIS, "Maquis de Gounghin");
    add("Patte d'Oie", Pos2::new(509.0, 4140.0), vec2(35.0, 10.0), I_MAQUIS, "Maquis de Patte d'Oie");
    add("Stade du 4-Août", Pos2::new(-3670.0, -408.0), vec2(60.0, 40.0), I_GYM, "Salle de sport du 4-Août");
    add("Marché de Dassaqgho", Pos2::new(4560.0, -2710.0), vec2(-30.0, 25.0), I_GYM, "Salle de sport de Dassasgho");
    doors
}

impl Game {
    fn local(&self) -> Option<(usize, Pos2)> {
        let i = self.inside?;
        let a = statics::anchor(i);
        Some((i, Pos2::new(self.player.pos.x - a.x, self.player.pos.y - a.y)))
    }

    /// What E does here, for the on-screen hint.
    pub fn interact_prompt(&self) -> Option<String> {
        if let Some(a) = &self.activity {
            return Some(format!("{} · F : arrêter", activity_label(a.kind)));
        }
        if let Some((i, l)) = self.local() {
            let it = &statics::interiors()[i];
            if let Some(s) = it.spot_near(l, 1.3) {
                if s.name == "vehicule" && self.garage_car.is_none() {
                    return Some("Ton garage est vide : gare un véhicule devant la porte (E)".into());
                }
                return Some(s.label.clone());
            }
            return Some(format!("{} · les cercles dorés sont interactifs · sortie : la porte", it.title));
        }
        if let Some(d) = self.doors.iter().find(|d| (d.pos - self.player_pos()).length() < 6.0) {
            if d.interior == I_GARAGE {
                return Some(match (self.player.vehicle, self.garage_car) {
                    (Some(_), None) => "E : garer ce véhicule dans ton garage".into(),
                    (Some(_), Some(_)) => "Ton garage est déjà occupé".into(),
                    (None, _) => format!("E : entrer dans {}", d.label),
                });
            }
            if self.player.vehicle.is_some() {
                return Some(format!("{} — descends du véhicule (F) pour entrer", d.label));
            }
            return Some(format!("E : entrer · {}", d.label));
        }
        if self.player.vehicle.is_none() {
            if let Some(i) = self.ped_in_front(2.6) {
                let _ = i;
                return Some("T : parler à cette personne".into());
            }
        }
        None
    }

    /// E pressed: enter a door, use a spot, leave.
    pub fn interact(&mut self, map: &Map) -> bool {
        if self.activity.is_some() {
            return false;
        }
        if let Some((i, l)) = self.local() {
            let it = &statics::interiors()[i];
            let Some(s) = it.spot_near(l, 1.3).cloned() else { return false };
            let a = statics::anchor(i);
            let spot = Pos2::new(a.x + s.x, a.y + s.z);
            match s.name.as_str() {
                "sortie" => self.leave_interior(map),
                "comptoir" => {
                    if self.pay(1500) {
                        self.hunger = (self.hunger + 50.0).min(100.0);
                        self.player.health = (self.player.health + 15.0).min(100.0);
                        self.msg("Riz gras, poulet bicyclette et bissap : +santé, faim calmée (1 500 FCFA)");
                    }
                }
                n if n.starts_with("table") => {
                    if self.pay(1500) {
                        self.start_activity(ActKind::Eat, 7.0, spot, s.yaw);
                        self.msg("Tô sauce gombo et poulet braisé… (1 500 FCFA)");
                    }
                }
                "lit" => self.start_activity(ActKind::Sleep, 4.0, spot, s.yaw),
                "canape" => self.start_activity(ActKind::Tv, 6.0, spot, s.yaw),
                "frigo" => {
                    self.hunger = (self.hunger + 30.0).min(100.0);
                    self.msg("Un morceau de pain et une banane : faim calmée.");
                }
                n if n.starts_with("tapis") => self.start_activity(ActKind::Run, 8.0, spot, s.yaw),
                "sac" => self.start_activity(ActKind::Punch, 6.0, spot, s.yaw),
                n if n.starts_with("squat") => self.start_activity(ActKind::Squat, 7.0, spot, s.yaw),
                "vehicule" => {
                    if let Some((kind, color)) = self.garage_car.take() {
                        self.leave_interior(map);
                        let door = self.doors.iter().find(|d| d.interior == I_GARAGE).map(|d| d.pos).unwrap_or(self.player.pos);
                        self.spawn_vehicle(map, door, kind, Driver::None, color, 0);
                        if let Some(vi) = self.vehicles.len().checked_sub(1) {
                            self.vehicles[vi].driver = Driver::Player;
                            self.player.vehicle = Some(vi);
                        }
                        self.msg("Tu sors ton véhicule du garage.");
                    }
                }
                _ => {}
            }
            return true;
        }
        let pp = self.player_pos();
        let Some(d) = self.doors.iter().find(|d| (d.pos - pp).length() < 6.0).cloned() else { return false };
        if d.interior == I_GARAGE {
            if let Some(vi) = self.player.vehicle {
                if self.garage_car.is_none() {
                    let v = &mut self.vehicles[vi];
                    self.garage_car = Some((v.kind, v.color));
                    v.remove = true;
                    self.player.vehicle = None;
                    self.player.pos = d.pos;
                    self.msg("Véhicule garé à l'abri. Reviens le chercher quand tu veux.");
                }
                return true;
            }
        }
        if self.player.vehicle.is_some() {
            return true;
        }
        self.outside = (self.player.pos, self.player.angle);
        self.inside = Some(d.interior);
        let it = &statics::interiors()[d.interior];
        let a = statics::anchor(d.interior);
        self.player.pos = Pos2::new(a.x - it.size[0] / 2.0 + 1.4, a.y);
        self.player.angle = 0.0;
        self.aiming = false;
        self.msg(format!("Tu entres : {}", d.label));
        true
    }

    fn leave_interior(&mut self, map: &Map) {
        if self.inside.take().is_some() {
            let (p, a) = self.outside;
            self.player.pos = p;
            self.player.angle = a + std::f32::consts::PI;
            map.collide(&mut self.player.pos, 0.4);
        }
    }

    fn pay(&mut self, n: i64) -> bool {
        if self.player.money < n {
            self.msg(format!("Il te faut {n} FCFA."));
            return false;
        }
        self.player.money -= n;
        true
    }

    fn start_activity(&mut self, kind: ActKind, dur: f32, spot: Pos2, yaw: f32) {
        self.player.pos = spot;
        self.player.angle = yaw;
        self.activity = Some(Activity { kind, t: 0.0, dur });
    }

    pub fn ped_in_front(&self, r: f32) -> Option<usize> {
        let p = self.player.pos;
        let f = vec2(self.player.angle.cos(), self.player.angle.sin());
        self.peds
            .iter()
            .enumerate()
            .filter(|(_, q)| q.down <= 0.0 && q.flee <= 0.0)
            .filter(|(_, q)| (q.pos - p).length() < r && (q.pos - p).normalized().dot(f) > 0.2)
            .min_by(|a, b| (a.1.pos - p).length().total_cmp(&(b.1.pos - p).length()))
            .map(|(i, _)| i)
    }

    pub fn start_talk(&mut self) {
        if self.player.vehicle.is_some() || self.activity.is_some() {
            return;
        }
        if let Some(i) = self.ped_in_front(2.6) {
            let line = GREETINGS[self.rng.below(GREETINGS.len())].to_string();
            let reply = REPLIES[self.rng.below(REPLIES.len())].to_string();
            let d = self.player.pos - self.peds[i].pos;
            self.peds[i].angle = d.y.atan2(d.x);
            self.talk = Some(Talk { ped: i, t: 0.0, line, reply });
        }
    }

    /// Hunger, activities, conversations.
    pub fn update_life(&mut self, dt: f32) {
        self.punch_t = (self.punch_t - dt).max(0.0);
        // one game hour lasts 20 s
        self.hunger = (self.hunger - dt * 4.0 / 20.0).max(0.0);
        if self.hunger <= 0.0 {
            self.player.health -= dt * 0.15;
        }
        if let Some(t) = &mut self.talk {
            t.t += dt;
            if t.t > 6.0 || t.ped >= self.peds.len() {
                self.talk = None;
            }
        }
        let Some(mut a) = self.activity.take() else { return };
        a.t += dt;
        if a.t < a.dur {
            self.activity = Some(a);
            return;
        }
        match a.kind {
            ActKind::Eat => {
                self.hunger = 100.0;
                self.player.health = (self.player.health + 30.0).min(100.0);
                self.msg("Repas terminé : santé +30, faim calmée.");
            }
            ActKind::Sleep => {
                let skip = (24.0 - self.hours + 7.0) % 24.0;
                self.hours = 7.0;
                self.player.health = 100.0;
                self.hunger = (self.hunger - skip * 2.0).max(20.0);
                self.ev_saved = true;
                self.msg("Bonne nuit… Il est 7 h. Santé au maximum, partie sauvegardée.");
            }
            ActKind::Tv | ActKind::Sit => {
                self.player.health = (self.player.health + 8.0).min(100.0);
                self.msg("Un moment de détente devant le match.");
            }
            ActKind::Run => {
                self.forme = (self.forme + 6.0).min(100.0);
                self.hunger = (self.hunger - 6.0).max(0.0);
                self.msg(format!("Endurance +6 (forme {:.0}/100) : tu cours plus vite.", self.forme));
            }
            ActKind::Punch => {
                self.force = (self.force + 6.0).min(100.0);
                self.hunger = (self.hunger - 5.0).max(0.0);
                self.msg(format!("Force +6 ({:.0}/100) : tes coups de poing font plus mal.", self.force));
            }
            ActKind::Squat => {
                self.forme = (self.forme + 3.0).min(100.0);
                self.force = (self.force + 3.0).min(100.0);
                self.hunger = (self.hunger - 4.0).max(0.0);
                self.msg("Squats terminés : forme +3, force +3.");
            }
        }
    }

    /// Bila's body against the world: walls, furniture, vehicles and people.
    pub fn collide_player(&mut self, map: &Map) {
        if let Some((i, mut l)) = self.local() {
            statics::interiors()[i].collide(&mut l, 0.32);
            let a = statics::anchor(i);
            self.player.pos = Pos2::new(a.x + l.x, a.y + l.y);
            return;
        }
        map.collide(&mut self.player.pos, 0.35);
        let p = &mut self.player.pos;
        let mut hit = 0.0f32;
        for v in &self.vehicles {
            if v.remove {
                continue;
            }
            let (hl, hw, ..) = v.kind.spec();
            let f = v.fwd();
            let n = if v.kind == VKind::Moto { 1 } else { 3 };
            for k in 0..n {
                let off = if n == 1 { 0.0 } else { (k as f32 - 1.0) * (hl - hw) };
                let c = v.pos + f * off;
                let r = hw + 0.3;
                let d = *p - c;
                let dl = d.length();
                if dl < r && dl > 1e-4 {
                    *p += d / dl * (r - dl);
                    hit = hit.max(v.vel.length());
                }
            }
        }
        for q in &self.peds {
            if q.down > 0.0 {
                continue;
            }
            let d = *p - q.pos;
            let dl = d.length();
            if dl < 0.6 && dl > 1e-4 {
                *p += d / dl * (0.6 - dl) * 0.5;
            }
        }
        map.collide(&mut self.player.pos, 0.35);
        if hit > 6.0 {
            self.player.health -= hit * 1.6;
            self.msg("Bila a été percuté par un véhicule !");
        }
    }

    pub fn garage_paint(&self) -> Option<(VKind, Color32)> {
        self.garage_car
    }
}

pub fn activity_label(k: ActKind) -> &'static str {
    match k {
        ActKind::Eat => "Bila mange",
        ActKind::Sleep => "Bila dort",
        ActKind::Tv | ActKind::Sit => "Bila se repose",
        ActKind::Run => "Bila court sur le tapis",
        ActKind::Punch => "Bila boxe le sac",
        ActKind::Squat => "Bila fait des squats",
    }
}

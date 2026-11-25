//! Story mode: "L'histoire de Bila", six chapters in the real streets of Ouagadougou.

use crate::game::{Driver, Game, Ped, VKind};
use crate::map::Map;
use eframe::egui::{self as egui};
use egui::{pos2, Color32, Pos2};

#[derive(Clone)]
pub enum Obj {
    Enter { tag: u8, label: String },
    GoTo { to: Pos2, label: String },
    Timed { to: Pos2, label: String, time: f32 },
    Chase { tag: u8, label: String },
    Escape,
}

#[derive(Clone)]
pub enum Setup {
    None,
    StartAt(Pos2),
    ParkMoto(Pos2),
    Gang(Color32),
    Wanted(u8),
}

#[derive(Clone)]
pub struct Step {
    pub chapter: usize,
    pub title: &'static str,
    pub dialog: Vec<(&'static str, String)>,
    pub setup: Setup,
    pub obj: Obj,
}

pub struct Story {
    pub steps: Vec<Step>,
    pub step: usize,
    pub dialog_i: usize,
    pub in_dialog: bool,
    pub timer: f32,
    pub catch: f32,
    pub banner: Option<(String, f32)>,
    pub finished: bool,
    pub retrying: bool,
}

pub fn speaker_colors(name: &str) -> (Color32, Color32) {
    // (skin, clothes)
    match name {
        "Bila" => (Color32::from_rgb(92, 58, 38), Color32::from_rgb(180, 40, 40)),
        "Issouf" => (Color32::from_rgb(80, 50, 32), Color32::from_rgb(40, 120, 70)),
        "Mme Kaboré" => (Color32::from_rgb(98, 62, 42), Color32::from_rgb(220, 150, 30)),
        "Policier" => (Color32::from_rgb(70, 45, 30), Color32::from_rgb(40, 60, 110)),
        "Le Commandant" => (Color32::from_rgb(60, 38, 26), Color32::from_rgb(30, 30, 30)),
        _ => (Color32::from_rgb(85, 55, 36), Color32::from_rgb(130, 80, 160)),
    }
}

fn place(map: &Map, name: &str, fallback: Pos2) -> Pos2 {
    map.road_point(map.poi_named(name).unwrap_or(fallback))
}

pub fn build_steps(map: &Map) -> Vec<Step> {
    let gare = place(map, "Gare de Ouagadougou", pos2(-714.0, -952.0));
    let marche = place(map, "Grand Marché", pos2(-381.0, -364.0));
    let chu = place(map, "CHU Yalgado", pos2(1454.0, -2052.0));
    let barrage = map.road_point(pos2(-4700.0, -1350.0));
    let patte = map.road_point(pos2(509.0, 4140.0));
    let commissariat = place(map, "Commissariat de Police Central", pos2(36.0, -380.0));
    let d = |a: Pos2, b: Pos2| (a - b).length();
    let l = |s: &'static str, t: &str| (s, t.to_string());
    vec![
        Step {
            chapter: 1,
            title: "Chapitre 1 — Le retour de Bila",
            dialog: vec![
                l("Bila", "Ouaga… Trois ans que je n'ai pas vu la capitale. Ça a bien changé."),
                l("Issouf", "(au téléphone) Bila ! Bienvenue mon frère ! Ma moto rouge est garée juste devant la gare. Prends-la."),
                l("Bila", "Merci Issouf. J'arrive."),
            ],
            setup: Setup::StartAt(gare),
            obj: Obj::Enter { tag: 1, label: "Monte sur la moto rouge d'Issouf (touche F)".into() },
        },
        Step {
            chapter: 1,
            title: "Chapitre 1 — Le retour de Bila",
            dialog: vec![l("Issouf", "Rejoins-moi au Grand Marché. Fais attention, ça roule fort à Ouaga !")],
            setup: Setup::None,
            obj: Obj::GoTo { to: marche, label: "Va au Grand Marché".into() },
        },
        Step {
            chapter: 2,
            title: "Chapitre 2 — Taxi-moto",
            dialog: vec![
                l("Issouf", "Te voilà ! Pour gagner ta vie ici, tu vas faire taxi-moto. Première cliente : elle doit aller au CHU Yalgado."),
                l("Cliente", "Vite mon frère, mon rendez-vous à l'hôpital commence bientôt !"),
            ],
            setup: Setup::None,
            obj: Obj::Timed { to: chu, label: "Dépose la cliente au CHU Yalgado Ouédraogo".into(), time: d(marche, chu) / 9.0 + 50.0 },
        },
        Step {
            chapter: 3,
            title: "Chapitre 3 — Les Vautours",
            dialog: vec![
                l("Issouf", "Bila !! Les Vautours ont volé la recette de la semaine ! Un 4x4 noir… il passe tout près de toi !"),
                l("Bila", "Je m'en occupe. Ils ne vont pas loin."),
            ],
            setup: Setup::Gang(Color32::from_rgb(20, 20, 22)),
            obj: Obj::Chase { tag: 2, label: "Rattrape le 4x4 noir des Vautours et colle-le".into() },
        },
        Step {
            chapter: 4,
            title: "Chapitre 4 — Sirènes",
            dialog: vec![
                l("Policier", "(radio) Moto rouge impliquée dans une course-poursuite. À toutes les unités !"),
                l("Bila", "Ils croient que c'est moi le voleur… Il faut que je les sème."),
            ],
            setup: Setup::Wanted(2),
            obj: Obj::Escape,
        },
        Step {
            chapter: 5,
            title: "Chapitre 5 — Le rendez-vous du barrage",
            dialog: vec![
                l("Mme Kaboré", "(au téléphone) On m'a parlé de toi, Bila. Un garçon qui roule bien et qui ne parle pas trop."),
                l("Mme Kaboré", "J'ai un colis important. Viens au bord du Barrage n°2."),
            ],
            setup: Setup::None,
            obj: Obj::GoTo { to: barrage, label: "Va au bord du Barrage n°2".into() },
        },
        Step {
            chapter: 5,
            title: "Chapitre 5 — Le rendez-vous du barrage",
            dialog: vec![
                l("Mme Kaboré", "Ce colis doit arriver à Patte d'Oie avant que les Vautours ne l'apprennent. La police surveille : ne traîne pas."),
            ],
            setup: Setup::Wanted(1),
            obj: Obj::Timed { to: patte, label: "Livre le colis à Patte d'Oie".into(), time: d(barrage, patte) / 10.0 + 60.0 },
        },
        Step {
            chapter: 6,
            title: "Chapitre 6 — Le Commandant",
            dialog: vec![
                l("Issouf", "Bila ! Le chef des Vautours, le Commandant, est dans le coin. C'est lui qui a volé notre recette !"),
                l("Le Commandant", "Tu crois pouvoir m'attraper, petit taxi-moto ?"),
            ],
            setup: Setup::Gang(Color32::from_rgb(110, 20, 25)),
            obj: Obj::Chase { tag: 2, label: "Arrête la voiture rouge du Commandant".into() },
        },
        Step {
            chapter: 6,
            title: "Chapitre 6 — Le Commandant",
            dialog: vec![l("Bila", "C'est fini, Commandant. On va au Commissariat central.")],
            setup: Setup::None,
            obj: Obj::GoTo { to: commissariat, label: "Va au Commissariat de Police Central".into() },
        },
    ]
}

fn save_path() -> std::path::PathBuf {
    let base = std::env::var("APPDATA").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".into());
    std::path::Path::new(&base).join("OuagaCity").join("histoire.txt")
}

pub fn saved_step() -> usize {
    std::fs::read_to_string(save_path()).ok().and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

fn save_step(i: usize) {
    let p = save_path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(p, i.to_string());
}

impl Story {
    pub fn new(map: &Map, start: usize, g: &mut Game) -> Story {
        let steps = build_steps(map);
        let start = start.min(steps.len() - 1);
        // resume at the first step of the saved chapter
        let ch = steps[start].chapter;
        let start = steps.iter().position(|s| s.chapter == ch).unwrap_or(0);
        let mut s = Story {
            steps,
            step: start,
            dialog_i: 0,
            in_dialog: true,
            timer: 0.0,
            catch: 0.0,
            banner: None,
            finished: false,
            retrying: false,
        };
        g.story = true;
        if start > 0 {
            // resuming: Bila starts on Issouf's moto near the last place
            let at = match s.steps[start - 1].obj {
                Obj::GoTo { to, .. } | Obj::Timed { to, .. } => to,
                _ => g.player_pos(),
            };
            g.place_player(map, at, 0.0);
            g.spawn_vehicle(map, at, VKind::Moto, Driver::None, Color32::from_rgb(200, 30, 40), 1);
            if let Some(m) = g.tagged(1) {
                g.vehicles[m].driver = Driver::Player;
                g.player.vehicle = Some(m);
            }
        }
        s.begin(map, g);
        s
    }

    pub fn current(&self) -> Option<&Step> {
        if self.finished { None } else { self.steps.get(self.step) }
    }

    fn begin(&mut self, map: &Map, g: &mut Game) {
        let Some(st) = self.steps.get(self.step).cloned() else { return };
        if st.chapter != self.steps.get(self.step.wrapping_sub(1)).map_or(0, |p| p.chapter) || self.step == 0 {
            save_step(self.step);
            if !self.retrying {
                self.banner = Some((st.title.to_string(), 5.0));
            }
        }
        self.catch = 0.0;
        g.ev_arrested = false;
        g.ev_died = false;
        match st.setup {
            Setup::None => {}
            Setup::StartAt(p) => {
                g.place_player(map, p + egui::vec2(0.0, 0.0), 0.0);
                let moto_at = p + egui::vec2(25.0, 10.0);
                g.spawn_vehicle(map, moto_at, VKind::Moto, Driver::None, Color32::from_rgb(200, 30, 40), 1);
            }
            Setup::ParkMoto(p) => g.spawn_vehicle(map, p, VKind::Moto, Driver::None, Color32::from_rgb(200, 30, 40), 1),
            Setup::Gang(c) => {
                let pp = g.player_pos();
                let f = egui::vec2(g.player.angle.cos(), g.player.angle.sin());
                g.spawn_vehicle(map, pp + f * 70.0, VKind::Car, Driver::Gang, c, 2);
            }
            Setup::Wanted(n) => g.set_wanted(n),
        }
        if let Obj::Timed { time, .. } = st.obj {
            self.timer = time;
        }
        self.dialog_i = 0;
        self.in_dialog = !self.retrying && !st.dialog.is_empty();
        self.retrying = false;
    }

    /// Advance the dialogue; returns true when a click/key was consumed.
    pub fn next_line(&mut self) {
        if !self.in_dialog {
            return;
        }
        self.dialog_i += 1;
        if self.dialog_i >= self.steps[self.step].dialog.len() {
            self.in_dialog = false;
        }
    }

    pub fn dialog_line(&self) -> Option<(&'static str, &str)> {
        if !self.in_dialog {
            return None;
        }
        let st = self.steps.get(self.step)?;
        st.dialog.get(self.dialog_i).map(|(a, b)| (*a, b.as_str()))
    }

    /// Where the objective is, for the markers and the arrow.
    pub fn target(&self, g: &Game) -> Option<Pos2> {
        match &self.current()?.obj {
            Obj::Enter { tag, .. } => g.tagged(*tag).map(|i| g.vehicles[i].pos),
            Obj::GoTo { to, .. } | Obj::Timed { to, .. } => Some(*to),
            Obj::Chase { tag, .. } => g.tagged(*tag).map(|i| g.vehicles[i].pos),
            Obj::Escape => None,
        }
    }

    pub fn objective_text(&self, g: &Game) -> String {
        let Some(st) = self.current() else { return "Histoire terminée — la ville est à toi !".into() };
        let pp = g.player_pos();
        match &st.obj {
            Obj::Enter { label, .. } => label.clone(),
            Obj::GoTo { to, label } => format!("{label} · {:.0} m", (*to - pp).length()),
            Obj::Timed { to, label, .. } => format!("{label} · {:.0} m · {:.0} s", (*to - pp).length(), self.timer.max(0.0)),
            Obj::Chase { tag, label } => {
                let d = g.tagged(*tag).map(|i| (g.vehicles[i].pos - pp).length()).unwrap_or(0.0);
                format!("{label} · {:.0} m · prise {:.0}%", d, (self.catch / 3.0 * 100.0).min(100.0))
            }
            Obj::Escape => format!("Sème la police ({} étoile(s))", g.player.wanted),
        }
    }

    pub fn update(&mut self, map: &Map, g: &mut Game, dt: f32) {
        if let Some(b) = &mut self.banner {
            b.1 -= dt;
            if b.1 <= 0.0 {
                self.banner = None;
            }
        }
        if self.finished || self.in_dialog {
            return;
        }
        let Some(st) = self.steps.get(self.step).cloned() else { return };
        let pp = g.player_pos();
        let in_vehicle = g.player.vehicle.is_some();
        let mut fail: Option<&str> = None;
        let mut ok = false;
        if g.ev_died {
            fail = Some("Bila a été blessé.");
        } else if g.ev_arrested && !matches!(st.obj, Obj::Escape) {
            fail = Some("Bila a été arrêté.");
        }
        match &st.obj {
            Obj::Enter { tag, .. } => {
                if g.tagged(*tag).is_none() {
                    g.spawn_vehicle(map, pp + egui::vec2(15.0, 0.0), VKind::Moto, Driver::None, Color32::from_rgb(200, 30, 40), *tag);
                }
                ok = g.player.vehicle.is_some_and(|v| g.vehicles[v].tag == *tag);
            }
            Obj::GoTo { to, .. } => ok = in_vehicle && (*to - pp).length() < 15.0,
            Obj::Timed { to, .. } => {
                self.timer -= dt;
                ok = in_vehicle && (*to - pp).length() < 15.0;
                if !ok && self.timer <= 0.0 {
                    fail = Some("Trop tard !");
                }
            }
            Obj::Chase { tag, .. } => match g.tagged(*tag) {
                None => fail = Some("Ils se sont échappés."),
                Some(i) => {
                    let d = (g.vehicles[i].pos - pp).length();
                    if d < 9.0 {
                        self.catch += dt;
                    } else {
                        self.catch = (self.catch - dt * 0.3).max(0.0);
                    }
                    if d > 450.0 {
                        fail = Some("Ils se sont échappés.");
                    }
                    if self.catch >= 3.0 || g.vehicles[i].dead {
                        ok = true;
                        let v = &mut g.vehicles[i];
                        v.driver = Driver::None;
                        v.vel = egui::Vec2::ZERO;
                        v.tag = 0;
                        let pos = v.pos + egui::vec2(3.0, 0.0);
                        let road = v.route.map(|r| r.road).unwrap_or(0);
                        g.peds.push(Ped {
                            pos,
                            angle: 0.0,
                            road,
                            i: 0,
                            dir: 1,
                            side: 1.0,
                            flee: 0.0,
                            down: 20.0,
                            color: Color32::from_rgb(30, 30, 30),
                            remove: false,
                        });
                        g.msg("Tu les as eus ! La recette est récupérée.");
                    }
                }
            },
            Obj::Escape => {
                if g.player.wanted == 0 {
                    ok = !g.ev_arrested;
                    if g.ev_arrested {
                        fail = Some("Bila a été arrêté.");
                    }
                }
            }
        }
        if let Some(reason) = fail {
            self.banner = Some((format!("Mission échouée — {reason} On recommence."), 4.0));
            g.set_wanted(0);
            self.retrying = true;
            self.begin(map, g);
            return;
        }
        if ok {
            g.player.money += 1500 + 500 * st.chapter as i64;
            self.step += 1;
            if self.step >= self.steps.len() {
                self.finished = true;
                g.story = false;
                save_step(0);
                self.banner = Some(("FIN — Bila est devenu une légende de Ouaga. Mode libre débloqué !".into(), 8.0));
                g.msg("Les marchés proposent maintenant des courses (cercles jaunes).");
            } else {
                self.banner = Some(("Mission réussie !".into(), 2.5));
                self.begin(map, g);
            }
        }
    }
}

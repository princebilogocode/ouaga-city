//! Le rêve ouagalais: Bila buys businesses at real places of Ouagadougou, each paying an income
//! every game hour, until he becomes the richest man in Africa.

use crate::game::Game;
use crate::map::Map;
use eframe::egui::{pos2, vec2, Pos2};

pub struct Business {
    pub name: &'static str,
    pub what: &'static str,
    pub place: Pos2,
    pub price: i64,
    /// FCFA per game hour
    pub income: i64,
    pub owned: bool,
}

/// About 20 billion dollars: the fortune of the richest person in Africa.
pub const RICHEST: i64 = 12_000_000_000_000;

pub struct Empire {
    pub list: Vec<Business>,
    carry: f64,
    /// only the story mode keeps its empire between sessions
    pub persist: bool,
}

pub fn fcfa(n: i64) -> String {
    let a = n.abs() as f64;
    let s = if a >= 1e12 {
        format!("{:.1} mille milliards", a / 1e12)
    } else if a >= 1e9 {
        format!("{:.1} milliards", a / 1e9)
    } else if a >= 1e6 {
        format!("{:.1} millions", a / 1e6)
    } else {
        let digits = (a as i64).to_string();
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push('\u{202F}');
            }
            out.push(c);
        }
        out
    };
    format!("{}{} FCFA", if n < 0 { "-" } else { "" }, s.replace(".0 ", " ").replace('.', ","))
}

fn near_road(map: &Map, name: &str, fallback: Pos2, off: f32) -> Pos2 {
    let p = map.poi_named(name).or_else(|| map.landmarks.iter().find(|l| l.name.contains(name)).map(|l| l.pos)).unwrap_or(fallback);
    map.road_point(p + vec2(off, off * 0.5))
}

impl Empire {
    pub fn new(map: &Map) -> Empire {
        let b = |name, what, place, price, income| Business { name, what, place, price, income, owned: false };
        let list = vec![
            b("Maquis « Le Faso »", "grillades, dolo et musique live", near_road(map, "Grand Marché", pos2(-381.0, -364.0), 40.0), 50_000, 6_000),
            b("Flotte de taxis-motos Bila", "30 motos et leurs pilotes", near_road(map, "Gare de Ouagadougou", pos2(-714.0, -952.0), 30.0), 300_000, 40_000),
            b("Transports Bila Express", "cars entre Ouaga, Bobo et Accra", near_road(map, "Marché de Zogona", pos2(2799.0, -1629.0), 30.0), 3_000_000, 380_000),
            b("Hôtel Bila Ouaga 2000", "5 étoiles face au monument", near_road(map, "Héros nationaux", pos2(1680.0, 5830.0), 90.0), 30_000_000, 3_800_000),
            b("Usine Faso Dan Fani", "le pagne tissé du Burkina pour le monde", near_road(map, "Marché de Gounghin", pos2(-2830.0, 940.0), 30.0), 250_000_000, 32_000_000),
            b("Agro-industrie Karité & Coton", "beurre de karité et coton bio", near_road(map, "Marché de Cissin", pos2(-3810.0, 3141.0), 30.0), 2_500_000_000, 330_000_000),
            b("Faso Connect", "le réseau mobile et la fibre du Sahel", near_road(map, "Place des Nations Unies", pos2(36.0, -707.0), 40.0), 30_000_000_000, 3_800_000_000),
            b("Banque Panafricaine du Sahel", "crédit pour les entrepreneurs africains", near_road(map, "Rond-point de la Femme", pos2(1450.0, -300.0), 30.0), 400_000_000_000, 48_000_000_000),
            b("Groupe Sahel Énergie", "centrales solaires et mines responsables", near_road(map, "Patte d'Oie", pos2(509.0, 4140.0), 40.0), 4_000_000_000_000, 520_000_000_000),
        ];
        Empire { list, carry: 0.0, persist: false }
    }

    pub fn income(&self) -> i64 {
        self.list.iter().filter(|b| b.owned).map(|b| b.income).sum()
    }

    pub fn net_worth(&self, cash: i64) -> i64 {
        cash + self.list.iter().filter(|b| b.owned).map(|b| b.price).sum::<i64>()
    }

    pub fn owned_count(&self) -> usize {
        self.list.iter().filter(|b| b.owned).count()
    }

    /// Pay the income for `dt` real seconds (one game hour lasts 20 s).
    pub fn update(&mut self, g: &mut Game, dt: f32) {
        self.carry += self.income() as f64 * dt as f64 / 20.0;
        if self.carry >= 1.0 {
            let whole = self.carry.floor();
            g.player.money += whole as i64;
            self.carry -= whole;
        }
    }

    /// The business within reach of the player, if any.
    pub fn nearby(&self, p: Pos2) -> Option<usize> {
        self.list.iter().position(|b| (b.place - p).length() < 9.0)
    }

    pub fn buy(&mut self, i: usize, g: &mut Game) -> Result<String, String> {
        let b = &mut self.list[i];
        if b.owned {
            return Err(format!("{} t'appartient déjà.", b.name));
        }
        if g.player.money < b.price {
            return Err(format!("Il te manque {} pour acheter {}.", fcfa(b.price - g.player.money), b.name));
        }
        g.player.money -= b.price;
        b.owned = true;
        let msg = format!("Tu as acheté {} ! +{} par heure", b.name, fcfa(b.income));
        self.save(g.player.money);
        Ok(msg)
    }

    fn path() -> std::path::PathBuf {
        let base = std::env::var("APPDATA").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".into());
        std::path::Path::new(&base).join("OuagaCity").join("empire.txt")
    }

    pub fn save(&self, money: i64) {
        if !self.persist {
            return;
        }
        let owned: String = self.list.iter().map(|b| if b.owned { '1' } else { '0' }).collect();
        let p = Self::path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(p, format!("{money};{owned}"));
    }

    /// Restore the saved empire; returns the saved cash.
    pub fn load(&mut self) -> Option<i64> {
        let s = std::fs::read_to_string(Self::path()).ok()?;
        let (m, owned) = s.trim().split_once(';')?;
        for (b, c) in self.list.iter_mut().zip(owned.chars()) {
            b.owned = c == '1';
        }
        m.parse().ok()
    }

    pub fn reset_save() {
        let _ = std::fs::remove_file(Self::path());
    }
}

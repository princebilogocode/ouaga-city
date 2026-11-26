//! Static meshes modelled in Blender (blender/build_vehicles.py, blender/build_interiors.py).
use egui::Pos2;
use eframe::egui;
use std::sync::OnceLock;

pub fn ocs(bytes: &[u8]) -> Vec<f32> {
    if bytes.len() < 8 || &bytes[0..4] != b"OCS1" {
        return Vec::new();
    }
    let n = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let mut v = Vec::with_capacity(n * 13);
    for k in 0..n * 13 {
        let o = 8 + k * 4;
        v.push(f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap()));
    }
    v
}

pub const V_SEDAN: usize = 0;
pub const V_TAXI: usize = 1;
pub const V_POLICE: usize = 2;
pub const V_MINIBUS: usize = 3;
pub const V_WHEEL: usize = 4;
pub const V_PICKUP: usize = 5;
pub const V_SUV: usize = 6;
pub const V_LUXE: usize = 7;
pub const V_ANCIENNE: usize = 8;
pub const V_AMBULANCE: usize = 9;
pub const V_AUTOECOLE: usize = 10;

pub fn vehicles() -> &'static Vec<Vec<f32>> {
    static D: OnceLock<Vec<Vec<f32>>> = OnceLock::new();
    D.get_or_init(|| {
        vec![
            ocs(include_bytes!("../assets/vehicles/sedan.ocs")),
            ocs(include_bytes!("../assets/vehicles/taxi.ocs")),
            ocs(include_bytes!("../assets/vehicles/police.ocs")),
            ocs(include_bytes!("../assets/vehicles/minibus.ocs")),
            ocs(include_bytes!("../assets/vehicles/wheel.ocs")),
            ocs(include_bytes!("../assets/vehicles/pickup.ocs")),
            ocs(include_bytes!("../assets/vehicles/suv.ocs")),
            ocs(include_bytes!("../assets/vehicles/luxe.ocs")),
            ocs(include_bytes!("../assets/vehicles/ancienne.ocs")),
            ocs(include_bytes!("../assets/vehicles/ambulance.ocs")),
            ocs(include_bytes!("../assets/vehicles/autoecole.ocs")),
        ]
    })
}

pub const T_NEEM: usize = 0;
pub const T_MANGUIER: usize = 1;
pub const T_FLAMBOYANT: usize = 2;
pub const T_BAOBAB: usize = 3;
pub const T_PALMIER: usize = 4;

/// Trees modelled in Blender (blender/build_trees.py): for each species the detailed mesh, then the far one.
pub fn trees() -> &'static Vec<Vec<f32>> {
    static D: OnceLock<Vec<Vec<f32>>> = OnceLock::new();
    D.get_or_init(|| {
        vec![
            ocs(include_bytes!("../assets/trees/neem.ocs")),
            ocs(include_bytes!("../assets/trees/neem_lod.ocs")),
            ocs(include_bytes!("../assets/trees/manguier.ocs")),
            ocs(include_bytes!("../assets/trees/manguier_lod.ocs")),
            ocs(include_bytes!("../assets/trees/flamboyant.ocs")),
            ocs(include_bytes!("../assets/trees/flamboyant_lod.ocs")),
            ocs(include_bytes!("../assets/trees/baobab.ocs")),
            ocs(include_bytes!("../assets/trees/baobab_lod.ocs")),
            ocs(include_bytes!("../assets/trees/palmier.ocs")),
            ocs(include_bytes!("../assets/trees/palmier_lod.ocs")),
        ]
    })
}

/// Wheel centres (x forward, y up, z right) for a vehicle mesh.
pub fn wheels(kind: usize) -> [[f32; 3]; 4] {
    match kind {
        V_MINIBUS => [[2.0, 0.42, 0.88], [2.0, 0.42, -0.88], [-1.95, 0.42, 0.88], [-1.95, 0.42, -0.88]],
        V_PICKUP => [[1.6, 0.38, 0.84], [1.6, 0.38, -0.84], [-1.7, 0.38, 0.84], [-1.7, 0.38, -0.84]],
        V_SUV => [[1.45, 0.38, 0.87], [1.45, 0.38, -0.87], [-1.45, 0.38, 0.87], [-1.45, 0.38, -0.87]],
        V_LUXE => [[1.45, 0.36, 0.84], [1.45, 0.36, -0.84], [-1.5, 0.36, 0.84], [-1.5, 0.36, -0.84]],
        V_ANCIENNE => [[1.3, 0.36, 0.74], [1.3, 0.36, -0.74], [-1.3, 0.36, 0.74], [-1.3, 0.36, -0.74]],
        V_AMBULANCE => [[1.75, 0.4, 0.9], [1.75, 0.4, -0.9], [-1.8, 0.4, 0.9], [-1.8, 0.4, -0.9]],
        V_AUTOECOLE => [[1.25, 0.36, 0.75], [1.25, 0.36, -0.75], [-1.25, 0.36, 0.75], [-1.25, 0.36, -0.75]],
        _ => [[1.35, 0.36, 0.8], [1.35, 0.36, -0.8], [-1.35, 0.36, 0.8], [-1.35, 0.36, -0.8]],
    }
}

#[derive(Clone, Debug)]
pub struct Spot {
    pub name: String,
    pub label: String,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
}

#[derive(Clone, Debug)]
pub struct Interior {
    pub name: &'static str,
    pub title: &'static str,
    pub mesh: Vec<f32>,
    pub size: [f32; 3],
    /// xmin, zmin, xmax, zmax in room space
    pub colliders: Vec<[f32; 4]>,
    pub spots: Vec<Spot>,
}

impl Interior {
    pub fn spot_near(&self, local: Pos2, r: f32) -> Option<&Spot> {
        self.spots
            .iter()
            .filter(|s| (Pos2::new(s.x, s.z) - local).length() < r)
            .min_by(|a, b| (Pos2::new(a.x, a.z) - local).length().total_cmp(&(Pos2::new(b.x, b.z) - local).length()))
    }

    /// Keep a circle inside the room and out of the furniture.
    pub fn collide(&self, p: &mut Pos2, r: f32) {
        let (hx, hz) = (self.size[0] / 2.0 - r, self.size[1] / 2.0 - r);
        p.x = p.x.clamp(-hx, hx);
        p.y = p.y.clamp(-hz, hz);
        for c in &self.colliders {
            let (x0, z0, x1, z1) = (c[0] - r, c[1].min(c[3]) - r, c[2] + r, c[1].max(c[3]) + r);
            if p.x > x0 && p.x < x1 && p.y > z0 && p.y < z1 {
                let d = [p.x - x0, x1 - p.x, p.y - z0, z1 - p.y];
                let m = d.iter().cloned().fold(f32::MAX, f32::min);
                if m == d[0] {
                    p.x = x0
                } else if m == d[1] {
                    p.x = x1
                } else if m == d[2] {
                    p.y = z0
                } else {
                    p.y = z1
                }
            }
        }
    }
}

fn parse(name: &'static str, title: &'static str, mesh: &[u8], json: &str) -> Interior {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or(serde_json::Value::Null);
    let f = |x: &serde_json::Value| x.as_f64().unwrap_or(0.0) as f32;
    let size = v["size"].as_array().map(|a| [f(&a[0]), f(&a[1]), f(&a[2])]).unwrap_or([10.0, 10.0, 3.0]);
    let colliders = v["colliders"].as_array().map(|a| a.iter().map(|c| [f(&c[0]), f(&c[1]), f(&c[2]), f(&c[3])]).collect()).unwrap_or_default();
    let spots = v["spots"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|s| Spot {
                    name: s["name"].as_str().unwrap_or("").to_string(),
                    label: s["label"].as_str().unwrap_or("").to_string(),
                    x: f(&s["x"]),
                    z: f(&s["z"]),
                    yaw: f(&s["yaw"]),
                })
                .collect()
        })
        .unwrap_or_default();
    Interior { name, title, mesh: ocs(mesh), size, colliders, spots }
}

pub const I_MAQUIS: usize = 0;
pub const I_APPART: usize = 1;
pub const I_GYM: usize = 2;
pub const I_GARAGE: usize = 3;
pub const I_HOPITAL: usize = 4;
pub const I_BOUTIQUE: usize = 5;
pub const I_PHARMACIE: usize = 6;
pub const I_BANQUE: usize = 7;

pub fn interiors() -> &'static Vec<Interior> {
    static D: OnceLock<Vec<Interior>> = OnceLock::new();
    D.get_or_init(|| {
        vec![
            parse("maquis", "Maquis-café", include_bytes!("../assets/interiors/maquis.ocs"), include_str!("../assets/interiors/maquis.json")),
            parse("appartement", "Appartement de Bila", include_bytes!("../assets/interiors/appartement.ocs"), include_str!("../assets/interiors/appartement.json")),
            parse("salle_sport", "Salle de sport", include_bytes!("../assets/interiors/salle_sport.ocs"), include_str!("../assets/interiors/salle_sport.json")),
            parse("garage", "Garage de Bila", include_bytes!("../assets/interiors/garage.ocs"), include_str!("../assets/interiors/garage.json")),
            parse("hopital", "Centre de santé", include_bytes!("../assets/interiors/hopital.ocs"), include_str!("../assets/interiors/hopital.json")),
            parse("boutique", "Boutique", include_bytes!("../assets/interiors/boutique.ocs"), include_str!("../assets/interiors/boutique.json")),
            parse("pharmacie", "Pharmacie", include_bytes!("../assets/interiors/pharmacie.ocs"), include_str!("../assets/interiors/pharmacie.json")),
            parse("banque", "Banque", include_bytes!("../assets/interiors/banque.ocs"), include_str!("../assets/interiors/banque.json")),
        ]
    })
}

/// Where each interior sits in the world (far outside the city, out of sight).
pub fn anchor(i: usize) -> Pos2 {
    Pos2::new(40_000.0 + i as f32 * 200.0, 40_000.0)
}

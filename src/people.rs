//! Every passer-by and driver is a real person of Ouaga: a stable identity (name, age, job,
//! neighbourhood, a bit of life story and what they say) drawn from their identity number.

use crate::skinned::{C_FEMME_VILLE, C_FEMME_WAX, C_HOMME_BOUBOU, C_JEUNE, C_MPFB0, MPFB};

const NOMS: [&str; 24] = [
    "Ouédraogo", "Sawadogo", "Kaboré", "Compaoré", "Traoré", "Zongo", "Ouattara", "Kouanda", "Sanou", "Ilboudo", "Kiemtoré", "Nikiéma",
    "Zoungrana", "Tapsoba", "Yaméogo", "Somé", "Dabiré", "Bationo", "Kinda", "Belem", "Konaté", "Diallo", "Barry", "Sankara",
];
const PRENOMS_F: [&str; 16] = ["Aminata", "Mariam", "Awa", "Fatimata", "Salimata", "Rasmata", "Adjaratou", "Pauline", "Odile", "Bintou", "Alizèta", "Habibou", "Clarisse", "Nafissatou", "Sali", "Ramata"];
const PRENOMS_H: [&str; 16] = ["Salif", "Rasmané", "Issa", "Boukary", "Adama", "Inoussa", "Moussa", "Ousmane", "Hamidou", "Seydou", "Arouna", "Lassané", "Paul", "Thierry", "Abdoul", "Karim"];
const QUARTIERS: [&str; 14] = ["Gounghin", "Dapoya", "Tanghin", "Zogona", "Pissy", "Patte d'Oie", "Dassasgho", "Wemtenga", "Kalgodin", "Cissin", "Paspanga", "Koulouba", "Larlé", "Ouaga 2000"];

struct Job {
    title: &'static str,
    female: bool,
    male: bool,
    story: &'static str,
    line: &'static str,
}

const JOBS: [Job; 18] = [
    Job { title: "vendeuse de beignets", female: true, male: false, story: "se lève à 4 h pour préparer ses beignets de mil", line: "Mes beignets sont chauds, tu en veux deux ? Cent francs seulement." },
    Job { title: "couturière", female: true, male: true, story: "coud des tenues en Faso Dan Fani pour les mariages", line: "Le Faso Dan Fani, c'est notre fierté. Passe à l'atelier, je te fais une belle tenue." },
    Job { title: "étudiant à l'université Joseph Ki-Zerbo", female: true, male: true, story: "prépare son master d'informatique en révisant sous les lampadaires", line: "Les examens arrivent… mais un jour je créerai ma propre entreprise." },
    Job { title: "mécanicien moto", female: false, male: true, story: "répare les motos de tout le quartier dans un petit garage sous un neem", line: "Ta moto fait un bruit ? Amène-la, je règle ça en dix minutes." },
    Job { title: "commerçante au Grand Marché", female: true, male: false, story: "vend des pagnes wax au Grand Marché depuis vingt ans", line: "Le marché Rood Woko, c'est le cœur de Ouaga. Viens voir mes pagnes !" },
    Job { title: "chauffeur de taxi", female: false, male: true, story: "conduit un taxi vert depuis quinze ans et connaît chaque rue", line: "Ouaga, je la connais par cœur. Toi aussi tu fais le taxi-moto ?" },
    Job { title: "infirmière au CHU Yalgado", female: true, male: true, story: "enchaîne les gardes de nuit au CHU Yalgado Ouédraogo", line: "Je sors d'une garde de nuit… Prends soin de toi sur la route." },
    Job { title: "enseignant", female: true, male: true, story: "enseigne les mathématiques à soixante élèves au lycée", line: "L'éducation, c'est l'avenir du Faso. Mes élèves seront ingénieurs." },
    Job { title: "orpailleur revenu de Bobo", female: false, male: true, story: "a quitté les mines d'or pour ouvrir une boutique", line: "J'ai travaillé à la mine. Maintenant je veux une affaire honnête en ville." },
    Job { title: "coiffeuse", female: true, male: false, story: "tient un salon de tresses à côté du marché", line: "Des tresses, des nattes, tout ce que tu veux ! Viens au salon." },
    Job { title: "griot et musicien", female: false, male: true, story: "chante les louanges des familles aux baptêmes et mariages", line: "Je connais l'histoire de ta famille, Bila… les Mossi sont un grand peuple." },
    Job { title: "agricultrice de Kaya en visite", female: true, male: true, story: "est venue vendre son sésame et ses arachides en ville", line: "Cette année, la pluie a été bonne. Le sésame se vend bien." },
    Job { title: "développeuse web", female: true, male: true, story: "code des applications de paiement mobile pour les commerçants", line: "Avec le paiement mobile, même la vendeuse de beignets est connectée !" },
    Job { title: "policier de la circulation", female: true, male: true, story: "règle le trafic au rond-point des Nations Unies aux heures de pointe", line: "Casque, permis, et pas d'excès de vitesse sur Kwamé N'Krumah !" },
    Job { title: "artisan bronzier", female: false, male: true, story: "fond des statuettes en bronze à la cire perdue, un savoir-faire ancestral", line: "Nos bronzes partent jusqu'au SIAO et dans le monde entier." },
    Job { title: "gérante de maquis", female: true, male: false, story: "fait le meilleur poulet bicyclette du secteur", line: "Ce soir il y a de la musique au maquis, viens danser !" },
    Job { title: "conducteur de minibus", female: false, male: true, story: "relie Ouaga à Koudougou trois fois par jour", line: "Koudougou, Bobo, Kaya… mon minibus va partout !" },
    Job { title: "retraité de la fonction publique", female: true, male: true, story: "raconte aux jeunes la Révolution de 1983 sous l'arbre à palabres", line: "Du temps de Sankara, on plantait des arbres et on construisait ensemble…" },
];

pub struct Person {
    pub name: String,
    pub age: u32,
    pub job: &'static str,
    pub quartier: &'static str,
    pub story: String,
    pub line: &'static str,
    pub female: bool,
    pub model: usize,
}

fn h(id: u32, k: u32) -> usize {
    let mut x = id ^ k.wrapping_mul(0x9E37_79B9);
    x = (x ^ (x >> 16)).wrapping_mul(0x85eb_ca6b);
    x = (x ^ (x >> 13)).wrapping_mul(0xc2b2_ae35);
    (x ^ (x >> 16)) as usize
}

/// The 3D model of a person: two of the four rigged citizens are women.
pub fn model_of(id: u32) -> usize {
    // 4 people generated from pictures + 10 built in Blender with MPFB
    let all = 4 + MPFB.len();
    let k = h(id, 1) % all;
    if k < 4 { [C_FEMME_WAX, C_HOMME_BOUBOU, C_JEUNE, C_FEMME_VILLE][k] } else { C_MPFB0 + (k - 4) }
}

fn traits(model: usize) -> (bool, u8) {
    match model {
        m if m == C_FEMME_WAX || m == C_FEMME_VILLE => (true, 1),
        m if m == C_HOMME_BOUBOU => (false, 2),
        m if m >= C_MPFB0 => {
            let (_, f, a) = MPFB[(m - C_MPFB0).min(MPFB.len() - 1)];
            (f, a)
        }
        _ => (false, 0),
    }
}

pub fn person(id: u32) -> Person {
    let model = model_of(id);
    let (female, age_group) = traits(model);
    let candidates: Vec<&Job> = JOBS.iter().filter(|j| if female { j.female } else { j.male }).collect();
    let job = candidates[h(id, 2) % candidates.len()];
    let first = if female { PRENOMS_F[h(id, 3) % PRENOMS_F.len()] } else { PRENOMS_H[h(id, 3) % PRENOMS_H.len()] };
    let last = NOMS[h(id, 4) % NOMS.len()];
    let age = match age_group {
        0 => 17 + (h(id, 5) % 14) as u32,
        1 => 30 + (h(id, 5) % 22) as u32,
        _ => 62 + (h(id, 5) % 20) as u32,
    };
    let quartier = QUARTIERS[h(id, 6) % QUARTIERS.len()];
    Person {
        name: format!("{first} {last}"),
        age,
        job: job.title,
        quartier,
        story: format!("{first}, {age} ans, de {quartier}, {}", job.story),
        line: job.line,
        female,
        model,
    }
}

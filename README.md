# Ouaga City — L'histoire de Bila

Le rêve ouagalais : jeu en monde ouvert 3D dans les vraies rues et les vrais bâtiments de Ouagadougou (Burkina Faso), écrit en Rust (eframe/egui + OpenGL).

- **Mode histoire** : 12 chapitres — Bila passe du taxi-moto à l'homme le plus riche d'Afrique (maquis, flotte de motos, cars, hôtel à Ouaga 2000, Faso Dan Fani, karité, télécom, banque, énergie solaire).
- 108 000 bâtiments réels (OpenStreetMap), monuments (Héros nationaux, Place des Cinéastes, Nations Unies, Flambeau de la Révolution…), mosquées, églises, baobabs.
- Textures photo et reliefs (ambientCG, CC0), ciels réels (Poly Haven, CC0), ombres du soleil, nuit avec fenêtres éclairées.
- **Mode libre** : solo ou en ligne (un joueur héberge, les autres rejoignent par IP, port UDP 47777).
- Circulation, piétons, police et niveau de recherche, courses de livraison payées en FCFA, cycle jour/nuit.

## Lancer

```
cargo run --release
```

Commandes : ZQSD/WASD/flèches · F monter/descendre · Espace frein à main · Maj courir · H klaxon ·
E acheter · B empire · souris (glisser) caméra · molette zoom · C caméra haute · M carte · Échap pause.

Option `--screenshot fichier.png` : dessine ~120 images, enregistre une capture et quitte.

## Données de carte

Rues, lacs, parcs et lieux : © contributeurs OpenStreetMap, sous licence ODbL
(https://www.openstreetmap.org/copyright). Fichier converti : `assets/ouaga.map` (script `assets/conv.py`).

Voir aussi `assets/CREDITS.md`.

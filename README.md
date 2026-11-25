# Ouaga City — L'histoire de Bila

Jeu en monde ouvert 3D dans les vraies rues de Ouagadougou (Burkina Faso), écrit en Rust (eframe/egui + OpenGL).

- **Mode histoire** : 6 chapitres avec Bila (taxi-moto, les Vautours, la police, le Barrage n°2, le Commandant).
- **Mode libre** : solo ou en ligne (un joueur héberge, les autres rejoignent par IP, port UDP 47777).
- Circulation, piétons, police et niveau de recherche, courses de livraison payées en FCFA, cycle jour/nuit.

## Lancer

```
cargo run --release
```

Commandes : ZQSD/WASD/flèches · F monter/descendre · Espace frein à main · Maj courir · H klaxon ·
souris (glisser) caméra · molette zoom · C caméra haute · M carte · Échap pause.

Option `--screenshot fichier.png` : dessine ~120 images, enregistre une capture et quitte.

## Données de carte

Rues, lacs, parcs et lieux : © contributeurs OpenStreetMap, sous licence ODbL
(https://www.openstreetmap.org/copyright). Fichier converti : `assets/ouaga.map` (script `assets/conv.py`).

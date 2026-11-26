# Ouaga City — L'histoire de Bila

Le rêve ouagalais : jeu en monde ouvert 3D dans les vraies rues et les vrais bâtiments de Ouagadougou (Burkina Faso), écrit en Rust (eframe/egui + OpenGL).

- **Mode histoire** : 12 chapitres — Bila passe du taxi-moto à l'homme le plus riche d'Afrique (maquis, flotte de motos, cars, hôtel à Ouaga 2000, Faso Dan Fani, karité, télécom, banque, énergie solaire).
- 108 000 bâtiments réels (OpenStreetMap), monuments (Héros nationaux, Place des Cinéastes, Nations Unies, Flambeau de la Révolution…), mosquées, églises, baobabs.
- Personnages et moto en vraie 3D générés avec fal.ai (Bila, passants burkinabè), armes (poings, pistolet, fusil d'assaut), visée à l'épaule, police et gangs qui ripostent, armureries.
- Modélisé avec Blender (scripts dans `blender/`, à relancer avec `blender -b -P <script> -- <dossier>`) :
  squelettes + animations des personnages (marche, course, visée, moto, conduite, assis, manger, squats, boxe, parler),
  voitures, taxis verts, police et minibus (carrosserie arrondie, vitres, passages de roues), intérieurs
  (maquis-café, appartement de Bila, salle de sport, garage) avec meubles, collisions et points d'interaction.
- Vie quotidienne : entrer dans les bâtiments (portes violettes), manger, dormir (sauvegarde), s'entraîner (forme, force),
  faim, parler aux passants (T), garer et ressortir son véhicule, monter/descendre des véhicules en marchant jusqu'à la portière.
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

## Image → 3D sur cet ordinateur (gratuit)

`python tools/image_vers_3d.py <image.png> <nom> [personnage|objet]` : TripoSR (open source, licence MIT)
reconstruit le modèle sur le processeur (la Quadro M1000M n'a que 2 Go de mémoire vidéo), puis Blender le
nettoie, le redresse, l'allège et, pour un personnage, ajoute squelette, doigts et animations.
Installation : `tools/venv` (PyTorch CPU), `tools/TripoSR` (marching cubes remplacé par PyMCubes).
Environ 2-3 min par objet, 4-5 min par personnage ; qualité inférieure au service en ligne (visages flous).

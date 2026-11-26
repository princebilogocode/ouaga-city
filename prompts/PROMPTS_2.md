# Images à générer — série 2 (personnages, motos, voitures, bâtiments, garde-robe)

Merci pour la série 1 : façades, enseignes, latérite, tôle, trottoir, wax et murs de clôture
sont dans le jeu. Déposez ces nouvelles images dans **app/assets/user/** avec exactement les noms
indiqués (PNG).

## Comment ces images deviennent de la 3D

- **Personnages, motos, voitures** : chaque image (un seul sujet, entier, centré, fond gris clair
  uni) est transformée en modèle 3D texturé (Tripo via fal.ai, environ 0,50 $ par modèle), puis
  **je le rigge et l'anime dans Blender** (squelette, doigts, marche, conduite…) comme Bila.
  Il reste 0,50 $ de budget : augmentez le budget « assets » du projet (par ex. 30 $ pour
  ~55 modèles) pour que je convertisse toute la série.
- **Bâtiments** : les images servent de références et de textures ; je modélise les bâtiments
  dans Blender (tours de Ouaga 2000, banques, hôtels…) et j'applique vos images en façades.
- **Garde-robe** : la méthode fiable est « une tenue = un modèle complet du même personnage »
  (même visage, même corps, même pose). Le corps en sous-vêtements sert de base et de référence.
  Habiller automatiquement un corps nu avec des vêtements séparés demanderait une simulation de
  tissu que je ne peux pas garantir proprement ; le changement de tenue se fera dans l'armoire de
  l'appartement.

**Règles communes aux personnages** : format carré 1024×1024, personnage ENTIER de la tête aux
pieds, centré, vue de face, **pose en A** (bras écartés à 30° du corps, paumes vers les cuisses,
jambes légèrement écartées), **mains bien dessinées avec cinq doigts séparés**, fond gris clair
uni, lumière douce sans ombre, style photo réaliste, aucun logo ni texte.

**Règles communes aux véhicules** : carré 1024×1024, véhicule entier en vue de trois-quarts avant,
fond gris clair uni, **aucune marque, aucun logo, aucune plaque lisible**.

---

## A. Bila — garde-robe (même visage que la photo, même corps, pose en A)

| Fichier | Prompt |
|---|---|
| `bila_base.png` | Full-body A-pose photo of Bila, a slim young Black man from Burkina Faso, 25, very dark ebony skin, short black hair, wearing only plain dark grey boxer shorts, arms 30 degrees away from the body, legs slightly apart, five separated fingers on each hand, plain light grey background, realistic |
| `bila_fasodanfani.png` | Same man, same A-pose, wearing a tailored long-sleeved shirt of Faso Dan Fani handwoven cloth (indigo, white and red stripes), beige trousers, leather sandals |
| `bila_mecano.png` | Same man, same A-pose, wearing dark blue mechanic overalls with grease stains, work boots, a rag in the back pocket |
| `bila_sport.png` | Same man, same A-pose, wearing a green and yellow football jersey, black sports shorts, running shoes |
| `bila_decontracte.png` | Same man, same A-pose, wearing a white T-shirt, blue jeans and white sneakers |
| `bila_boubou.png` | Same man, same A-pose, wearing an elegant light blue embroidered grand boubou with matching trousers and a kufi cap |
| `bila_soiree.png` | Same man, same A-pose, wearing a black tuxedo with a bow tie for a gala night |
| `bila_taxi_moto.png` | Same man, same A-pose, wearing a yellow high-visibility vest over a polo shirt, cargo trousers, an open-face motorcycle helmet held under one arm |

## B. Habitants de Ouaga (20 personnes, pose en A)

| Fichier | Prompt (ajouter les règles communes) |
|---|---|
| `pnj_issouf.png` | Issouf, 28, cheerful Burkinabe man, small moustache, green and white Faso Dan Fani shirt, jeans, sandals |
| `pnj_kabore.png` | Madame Kaboré, 52, elegant powerful businesswoman, golden and indigo wax dress with a tall matching headwrap, gold jewellery |
| `pnj_awa.png` | Awa, 30, logistics engineer, braided hair tied back, royal blue blouse, black trousers, flat shoes, a tablet in her hand |
| `pnj_commandant.png` | The Commandant, 45, gang boss, shaved head, black leather jacket, dark shirt, gold chain, cargo trousers, boots |
| `pnj_vautour_1.png` | Young gang member of the Vautours, 22, red bandana, sleeveless black top, ripped jeans, sneakers |
| `pnj_vautour_2.png` | Gang member, 26, sunglasses on the head, camouflage jacket, black trousers, boots |
| `pnj_policier.png` | Burkinabe traffic police officer, 35, khaki uniform, white belt and white gloves, beret (no real insignia) |
| `pnj_vendeuse_beignets.png` | Street food seller, 40, colourful wax pagne and headwrap, apron, carrying a large aluminium basin |
| `pnj_mecanicien.png` | Motorbike mechanic, 30, oil-stained grey T-shirt and trousers, spanner in hand |
| `pnj_etudiante.png` | University student, 21, braids, purple T-shirt, jeans, backpack |
| `pnj_imam.png` | Elderly imam, 65, white grand boubou, white kufi, prayer beads, grey beard |
| `pnj_pretre.png` | Catholic priest, 50, white cassock with a green stole |
| `pnj_infirmiere.png` | Nurse of the CHU Yalgado, 33, white uniform, stethoscope, short hair |
| `pnj_ecolier.png` | Schoolboy, 10, khaki school uniform (shirt and shorts), small backpack, sandals |
| `pnj_grand_mere.png` | Grandmother, 75, faded wax pagne, headscarf, walking stick |
| `pnj_vigile.png` | Security guard, 40, dark blue uniform, cap, torch on the belt |
| `pnj_macon.png` | Construction worker, 28, dusty blue overalls, yellow hard hat, gloves |
| `pnj_coiffeuse.png` | Hairdresser, 27, long braids, pink blouse with an apron, comb in hand |
| `pnj_griot.png` | Griot musician, 55, colourful embroidered boubou, carrying a kora |
| `pnj_homme_affaires.png` | Businessman of Ouaga 2000, 45, navy suit, light blue shirt, leather briefcase |

## C. Motos et deux-roues de Ouaga (vue 3/4 avant, sans marque)

| Fichier | Prompt |
|---|---|
| `moto_125_rouge.png` | Realistic 125cc commuter motorcycle, red tank, chrome exhaust, rear luggage rack, no brand |
| `moto_125_noire.png` | Realistic 110cc underbone moped-style motorcycle, black and silver plastic body, no brand |
| `moto_scooter_blanc.png` | Realistic white step-through scooter with a front basket, no brand |
| `moto_scooter_rose.png` | Realistic pink and cream ladies' scooter with chrome mirrors, no brand |
| `moto_mobylette.png` | Vintage 1970s moped (mobylette) with pedals, faded blue paint, rusty spokes, no brand |
| `moto_tricycle_cargo.png` | Motor tricycle cargo with an open rear metal box full of sacks, as used in Ouagadougou markets, no brand |
| `moto_tricycle_taxi.png` | Covered motor tricycle passenger taxi with a canvas roof, no brand |
| `moto_trail.png` | Dusty 200cc trail / dual-sport motorcycle with knobby tyres, no brand |
| `moto_sport.png` | Modern sport motorcycle with full fairing, yellow and black, no brand |
| `moto_police.png` | Police motorcycle, white with blue stripes, siren light, no real insignia |
| `moto_livraison.png` | 125cc motorcycle with a large red delivery box on the back, no brand |
| `moto_custom.png` | Customised commuter motorcycle with colourful seat cover, extra mirrors and stickers (no brands), as loved in Ouaga |
| `velo.png` | Old black steel bicycle with a rear rack, as used by street vendors |

## D. Voitures, taxis, bus, camions (vue 3/4 avant, sans marque)

| Fichier | Prompt |
|---|---|
| `voiture_taxi_vert.png` | Green city taxi sedan of Ouagadougou, older 4-door sedan, roof taxi sign, no brand, no readable plate |
| `voiture_berline_ancienne.png` | Old 1980s-style 4-door sedan, beige paint with some rust, no brand |
| `voiture_berline_moderne.png` | Modern silver compact 4-door sedan, no brand |
| `voiture_suv.png` | Large white off-road 4x4 SUV with roof rack, dusty, no brand |
| `voiture_pickup.png` | Double-cab pickup truck, dark grey, loaded with bags in the back, no brand |
| `voiture_luxe.png` | Black luxury executive sedan as seen in Ouaga 2000, tinted windows, no brand |
| `voiture_police.png` | Police pickup, white and blue, light bar on the roof, no real insignia |
| `voiture_ambulance.png` | White ambulance van with red stripes and light bar, no real logos |
| `minibus_brousse.png` | Old white minibus (bush taxi) with green stripe and a roof rack piled with luggage and sacks, no brand |
| `bus_urbain.png` | Modern green and yellow city bus, no brand, no real logos |
| `camion_benne.png` | Old orange dump truck covered with red dust, no brand |
| `camion_citerne.png` | Water tanker truck, white tank, no brand |
| `voiture_auto_ecole.png` | Small red hatchback with a learner-driver roof sign, no brand |

## E. Bâtiments et quartiers (1536×1024, références pour Blender + textures)

| Fichier | Prompt |
|---|---|
| `bat_tour_ouaga2000.png` | Modern 20-storey glass and white concrete office tower in Ouaga 2000, Burkina Faso, blue reflective glass, palm trees, front three-quarter view, clear sky, realistic |
| `bat_banque.png` | Headquarters of a large West African bank (fictional), 15 storeys, beige stone and bronze glass, grand entrance with columns, no logos, realistic |
| `bat_ministere.png` | Government ministry building in Ouagadougou, 6 storeys, sand-coloured concrete with sun-shading fins, flags on poles (plain colours), realistic |
| `bat_hotel_luxe.png` | Five-star hotel in Ouaga 2000, curved white facade with balconies, fountain and palm-lined driveway, no logos |
| `bat_villa_ouaga2000.png` | Luxury two-storey villa in Ouaga 2000 behind a high cream wall with an ornate gate, terraces and bougainvillea |
| `bat_immeuble_r4.png` | Ordinary 4-storey apartment building in central Ouagadougou, ochre walls, balconies with laundry, air conditioners, shops below |
| `bat_station_service.png` | Petrol station in Ouagadougou with canopy and pumps, fictional brand colours green and yellow, no real logos |
| `bat_pharmacie.png` | Neighbourhood pharmacy in Ouagadougou with green cross sign, glass door, steps and ramp |
| `bat_ecole.png` | Primary school in Burkina Faso, long single-storey classrooms with ochre walls and blue shutters, schoolyard with a flagpole |
| `bat_mosquee.png` | Neighbourhood mosque in Ouagadougou, white and green, one minaret, small dome |
| `bat_eglise.png` | Neighbourhood church in Ouagadougou, red brick, bell tower with a cross |
| `bat_marche_couvert.png` | Large modern covered market hall with rows of stalls, inspired by the central market of Ouagadougou, red and ochre arches |
| `bat_maquis.png` | Popular maquis bar-restaurant: low ochre building, straw paillotes, plastic chairs, string lights, hand-painted sign |
| `bat_garage_moto.png` | Street motorbike repair shop under a tin roof, motorbikes waiting, tools and tyres everywhere |
| `facade_verre_bleu.png` | Seamless tileable texture of a blue reflective glass curtain wall with thin aluminium mullions, straight-on, 1024×1024 |
| `facade_beton_moderne.png` | Seamless tileable texture of modern beige concrete cladding panels with fine joints, straight-on, 1024×1024 |
| `facade_pierre.png` | Seamless tileable texture of light sandstone facade blocks, straight-on, 1024×1024 |

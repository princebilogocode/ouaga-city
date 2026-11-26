# Script généré par mam3d : importe les modèles dans Blender (4.x et plus).
# Utilisation : onglet Scripting > Ouvrir ce fichier > Exécuter le script.
import bpy

FICHIERS = [
  "C:\\Users\\Prince\\Documents\\projets\\recr-er-un-jeu-comme-gta\\app\\assets\\user\\mam3d_sortie\\pnj_homme_affaires.glb"
]
FORMAT = "glb"
ECART = 0.5  # espace entre deux modèles, en mètres

x = 0.0
for chemin in FICHIERS:
    avant = set(bpy.context.scene.objects)
    if FORMAT == "glb":
        bpy.ops.import_scene.gltf(filepath=chemin)
    elif FORMAT == "obj":
        bpy.ops.wm.obj_import(filepath=chemin)
    else:
        bpy.ops.wm.stl_import(filepath=chemin)
    nouveaux = [o for o in bpy.context.scene.objects if o not in avant and o.parent is None]
    largeur = max((o.dimensions.x for o in nouveaux), default=1.0)
    for o in nouveaux:
        o.location.x = x + largeur / 2
    x += largeur + ECART
print("mam3d :", len(FICHIERS), "modèle(s) importé(s)")

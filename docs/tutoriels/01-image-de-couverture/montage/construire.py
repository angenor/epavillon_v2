"""Engendre index.html et compositions/*.html du tutoriel « image de couverture ».

Les positions viennent de reperes.json (relevées par Playwright, en pixels CSS
d'une fenêtre 1440 × 810) ; les temps, de la voix (pauses mesurées).
Usage : python3 construire.py
"""
import json
from pathlib import Path

ICI = Path(__file__).parent
REPERES = json.loads((ICI / "reperes.json").read_text())

W, H = 1920, 1080
FX, FY, FW, FH = 160, 36, 1600, 900          # l'écran dans le cadre
K = FW / 1440                                  # pixels CSS -> pixels de l'écran

NUIT = "#1d1a5b"
CYAN = "#00a1e4"
CYAN_CLAIR = "#71c6fa"

# Début de chaque voix dans le film (s). 0,5 s entre deux scènes.
VOIX = {"s1": 7.76, "s2": 10.0, "s3": 9.12, "s4": 24.64, "s5": 12.08, "s6": 23.44, "s7": 8.32}
DEBUT = {}
t = 2.4
for s, d in VOIX.items():
    DEBUT[s] = round(t, 2)
    t += d + 0.5
FIN = round(DEBUT["s7"] + VOIX["s7"] + 3.2, 2)


def boite(plan, cle):
    return REPERES[plan][cle]


def centre(b):
    return (b["x"] + b["w"] / 2, b["y"] + b["h"] / 2)


def union(*bs):
    x0 = min(b["x"] for b in bs); y0 = min(b["y"] for b in bs)
    x1 = max(b["x"] + b["w"] for b in bs); y1 = max(b["y"] + b["h"] for b in bs)
    return {"x": x0, "y": y0, "w": x1 - x0, "h": y1 - y0}


def cadrage(z, cible):
    """Translation de la caméra pour centrer `cible` au zoom z, bornée à l'écran."""
    if cible is None or z == 1:
        return 0.0, 0.0
    cx, cy = centre(cible)
    tx = FW / 2 - K * z * cx
    ty = FH / 2 - K * z * cy
    tx = min(0, max(FW - FW * z, tx))
    ty = min(0, max(FH - FH * z, ty))
    return round(tx, 1), round(ty, 1)


def vers_toile(p, etat):
    """Point CSS d'une capture -> point du canvas 1920 × 1080, selon le zoom."""
    z, tx, ty = etat
    return FX + K * z * p[0] + tx, FY + K * z * p[1] + ty


def boite_toile(b, etat, marge=6):
    z, tx, ty = etat
    x, y = vers_toile((b["x"], b["y"]), etat)
    return {"x": round(x - marge), "y": round(y - marge),
            "w": round(K * z * b["w"] + 2 * marge), "h": round(K * z * b["h"] + 2 * marge)}


class Scene:
    def __init__(self, nom, duree):
        self.nom, self.duree = nom, round(duree, 2)
        self.html, self.js = [], []
        self.plans = []
        self.etat = {}          # plan -> (z, tx, ty) courant, pour placer curseur et cadres
        self.n = 0
        self.curseur_vu = False
        self.curseur_visible = False
        self.clics = []         # temps des clics, locaux à la scène

    def id(self, base):
        self.n += 1
        return f"{self.nom}-{base}{self.n}"

    # --- plans -------------------------------------------------------------
    def plan(self, image, debut, fin, z=1.0, cible=None, fondu=0.35):
        pid = self.id("plan")
        tx, ty = cadrage(z, cible)
        self.etat[image] = (z, tx, ty)
        self.plans.append(pid)
        self.html.append(
            f'<div id="{pid}" class="clip plan" data-start="{debut:.2f}" data-duration="{fin - debut:.2f}">'
            f'<div class="cam" id="{pid}-cam"><img src="assets/plans/{image}.jpg" alt=""></div></div>')
        self.js.append(f'tl.set("#{pid}-cam", {{x:{tx}, y:{ty}, scale:{z}}}, {debut:.2f});')
        if fondu:
            self.js.append(f'tl.fromTo("#{pid}", {{opacity:0}}, {{opacity:1, duration:{fondu}, ease:"power1.inOut"}}, {debut:.2f});')
        return pid

    def zoom(self, pid, image, t, z, cible, duree=1.1):
        tx, ty = cadrage(z, cible)
        self.etat[image] = (z, tx, ty)
        self.js.append(f'tl.to("#{pid}-cam", {{x:{tx}, y:{ty}, scale:{z}, duration:{duree}, ease:"power2.inOut"}}, {t:.2f});')

    # --- curseur -----------------------------------------------------------
    def curseur(self, image, cible, t, duree=0.8, depart=None):
        x, y = vers_toile(centre(cible) if isinstance(cible, dict) else cible, self.etat[image])
        if not self.curseur_visible:
            if depart or not self.curseur_vu:
                sx, sy = depart or (W / 2 + 120, H / 2 + 160)
                self.js.append(f'tl.set("#{self.nom}-curseur", {{x:{sx:.0f}, y:{sy:.0f}}}, {max(0, t - 0.3):.2f});')
            self.js.append(f'tl.to("#{self.nom}-curseur", {{opacity:1, duration:0.25}}, {max(0, t - 0.25):.2f});')
            self.curseur_vu = self.curseur_visible = True
        self.js.append(f'tl.to("#{self.nom}-curseur", {{x:{x:.0f}, y:{y:.0f}, duration:{duree}, ease:"power2.inOut"}}, {t:.2f});')
        return x, y

    def clic(self, image, cible, t):
        x, y = vers_toile(centre(cible) if isinstance(cible, dict) else cible, self.etat[image])
        rid = self.id("onde")
        self.html.append(f'<div id="{rid}" class="onde" style="left:{x - 30:.0f}px; top:{y - 30:.0f}px"></div>')
        self.js.append(f'tl.fromTo("#{rid}", {{opacity:0.9, scale:0.35}}, {{opacity:0, scale:1.5, duration:0.5, ease:"power2.out", immediateRender:false}}, {t:.2f});')
        self.js.append(f'tl.to("#{self.nom}-curseur-pointe", {{scale:0.86, duration:0.08, yoyo:true, repeat:1, transformOrigin:"0 0"}}, {t:.2f});')
        self.clics.append(t)

    def cacher_curseur(self, t):
        self.curseur_visible = False
        self.js.append(f'tl.to("#{self.nom}-curseur", {{opacity:0, duration:0.25}}, {t:.2f});')

    def glisser(self, image, de, a, t, duree):
        x0, y0 = vers_toile(de, self.etat[image]); x1, y1 = vers_toile(a, self.etat[image])
        self.js.append(f'tl.to("#{self.nom}-curseur", {{x:{x1:.0f}, y:{y1:.0f}, duration:{duree}, ease:"power1.inOut"}}, {t:.2f});')

    # --- repères visuels -----------------------------------------------------
    def cerne(self, image, cible, debut, fin, marge=6):
        b = boite_toile(cible, self.etat[image], marge)
        rid = self.id("cerne")
        self.html.append(f'<div id="{rid}" class="cerne" style="left:{b["x"]}px; top:{b["y"]}px; width:{b["w"]}px; height:{b["h"]}px"></div>')
        self.js.append(f'tl.fromTo("#{rid}", {{opacity:0, scale:1.04}}, {{opacity:1, scale:1, duration:0.3, ease:"power2.out"}}, {debut:.2f});')
        self.js.append(f'tl.to("#{rid}", {{opacity:0, duration:0.25}}, {fin - 0.25:.2f});')
        return b

    def etiquette(self, texte, x, y, debut, fin, cote="droite"):
        rid = self.id("etiquette")
        style = f"left:{x:.0f}px; top:{y:.0f}px" if cote == "droite" else f"right:{W - x:.0f}px; top:{y:.0f}px"
        self.html.append(f'<div id="{rid}" class="etiquette" style="{style}">{texte}</div>')
        self.js.append(f'tl.fromTo("#{rid}", {{opacity:0, x:-12}}, {{opacity:1, x:0, duration:0.35, ease:"power3.out"}}, {debut:.2f});')
        self.js.append(f'tl.to("#{rid}", {{opacity:0, duration:0.25}}, {fin - 0.25:.2f});')

    def chapitre(self, numero, texte, debut, fin):
        rid = self.id("chapitre")
        self.html.append(f'<div id="{rid}" class="chapitre"><b>Façon {numero}</b><span>{texte}</span></div>')
        self.js.append(f'tl.fromTo("#{rid}", {{opacity:0, y:16}}, {{opacity:1, y:0, duration:0.45, ease:"power3.out"}}, {debut:.2f});')
        self.js.append(f'tl.to("#{rid}", {{opacity:0, y:8, duration:0.3}}, {fin - 0.3:.2f});')

    def brut(self, html, js=()):
        self.html.append(html); self.js.extend(js)

    # --- sortie ----------------------------------------------------------------
    def ecrire(self):
        n = self.nom
        script = "\n    ".join(self.js)
        # Plans dans l'écran, repères au-dessus.
        plans = [h for h in self.html if 'class="clip plan"' in h]
        autres = [h for h in self.html if 'class="clip plan"' not in h]
        page = f"""<template>
  <style>
    #root {{ position:absolute; inset:0; }}
    {POLICES}
  </style>
  <div id="root" data-composition-id="{n}" data-width="{W}" data-height="{H}">
    <div class="ecran" id="{n}-ecran" data-layout-allow-overflow>
      {chr(10).join('      ' + p for p in plans).strip()}
    </div>
    {chr(10).join('    ' + a for a in autres).strip()}
    <div class="curseur" id="{n}-curseur" style="opacity:0" data-layout-ignore><svg id="{n}-curseur-pointe" viewBox="0 0 28 34" width="34" height="41"><path d="M2 2 L2 27 L8.5 21 L13 31.5 L17.5 29.5 L13 19.5 L22 19.5 Z" fill="#ffffff" stroke="{NUIT}" stroke-width="2.2" stroke-linejoin="round"/></svg></div>
  </div>
  <script>
    const tl = gsap.timeline({{ paused: true }});
    {script}
    tl.set({{}}, {{}}, {self.duree - 0.01:.2f});
    window.__timelines["{n}"] = tl;
  </script>
</template>
"""
        (ICI / "compositions" / f"{n}.html").write_text(page)


def scene_s1():
    s = Scene("s1", DEBUT["s2"] - DEBUT["s1"])
    haut = s.plan("modele-public", 0, 2.2, fondu=0)
    s.zoom(haut, "modele-public", 0.1, 1.06, boite("modele-public", "titre"), duree=2.0)
    s.plan("modele-public-bas", 1.8, s.duree, fondu=0.45)
    couv = {"x": 105, "y": 135, "w": 810, "h": 457}
    s.cerne("modele-public-bas", couv, 2.0, 3.7, marge=8)
    # « C'est aussi elle qui s'affiche quand quelqu'un partage le lien » : 3,94 s.
    s.brut(
        f'<div id="s1-voile" class="voile"></div>'
        f'<div id="s1-partage" class="partage"><img src="assets/couverture-modele.jpg" alt="">'
        f'<div class="partage-texte"><span class="partage-site">epavillonclimatique.francophonie.org</span>'
        f'<strong>Financer l\'adaptation côtière en Afrique de l\'Ouest</strong>'
        f'<span class="partage-lieu">Pavillon de la Francophonie, CdP31, Antalya</span></div></div>',
        ['tl.fromTo("#s1-voile", {opacity:0}, {opacity:1, duration:0.5}, 3.8);',
         'tl.fromTo("#s1-partage", {opacity:0, y:40}, {opacity:1, y:0, duration:0.6, ease:"power3.out"}, 3.95);',
         'tl.fromTo("#s1-partage img", {scale:1}, {scale:1.06, duration:3.9, ease:"none"}, 4.4);'])
    return s


def scene_s2():
    s = Scene("s2", DEBUT["s3"] - DEBUT["s2"])
    p = s.plan("a1-espace", 0, s.duree, fondu=0.4)
    entete = {"x": 205, "y": 142, "w": 470, "h": 128}
    s.cerne("a1-espace", entete, 1.1, 2.9)
    action, titre = boite("a1-espace", "action"), boite("a1-espace", "afaire")
    s.zoom(p, "a1-espace", 3.05, 1.35, union(action, titre, {"x": 104, "y": 416, "w": 832, "h": 300}), duree=1.0)
    s.cerne("a1-espace", action, 4.55, 9.9)
    s.curseur("a1-espace", (action["x"] + action["w"] - 60, action["y"] + 50), 7.1, duree=0.9)
    s.clic("a1-espace", (action["x"] + action["w"] - 60, action["y"] + 50), 8.2)
    return s


def scene_s3():
    s = Scene("s3", DEBUT["s4"] - DEBUT["s3"])
    p = s.plan("a2-dossier", 0, s.duree, fondu=0.4)
    sect = boite("a2-dossier", "section")
    s.zoom(p, "a2-dossier", 0.4, 1.45, sect, duree=1.2)
    modele, deposer = boite("a2-dossier", "modele"), boite("a2-dossier", "deposer")
    b = s.cerne("a2-dossier", modele, 5.6, s.duree)
    s.etiquette("Façon 1 : un modèle", b["x"] + b["w"] + 18, b["y"] + b["h"] / 2 - 26, 5.7, s.duree)
    b = s.cerne("a2-dossier", deposer, 6.9, s.duree)
    s.etiquette("Façon 2 : votre image", b["x"] + b["w"] + 18, b["y"] + b["h"] / 2 - 26, 7.0, s.duree)
    return s


def scene_s4():
    s = Scene("s4", DEBUT["s5"] - DEBUT["s4"])
    s.chapitre(1, "Partir d'un modèle", 0.2, 4.2)
    sect = boite("a2-dossier", "section")
    s.plan("a2-dossier", 0, 4.75, z=1.45, cible=sect, fondu=0)
    modele = boite("a2-dossier", "modele")
    s.curseur("a2-dossier", modele, 2.1, duree=0.9)
    s.clic("a2-dossier", modele, 3.3)
    atelier = s.plan("a3-atelier", 4.4, 15.5, fondu=0.35)
    s.cacher_curseur(4.4)
    colonne = {"x": 72, "y": 180, "w": 300, "h": 600}
    s.zoom(atelier, "a3-atelier", 4.9, 1.55, colonne, duree=1.1)
    apercu = {"x": 556, "y": 318, "w": 416, "h": 236}
    s.zoom(atelier, "a3-atelier", 8.9, 1.75, apercu, duree=1.2)
    s.zoom(atelier, "a3-atelier", 10.15, 1.86, apercu, duree=3.6)
    s.zoom(atelier, "a3-atelier", 13.95, 1.0, None, duree=0.7)
    # Trois modèles cliqués au rythme de « Cliquez sur un modèle : l'image change aussitôt ».
    modeles = [("b0-modele", 15.1), ("b1-modele", 16.0), ("b2-modele", 16.9)]
    for i, (image, tc) in enumerate(modeles):
        fin = modeles[i + 1][1] + 0.3 if i + 1 < len(modeles) else 18.6
        s.plan(image, tc + 0.05, fin, fondu=0.25)
        bouton = boite(image, "bouton")
        if i == 0:
            s.curseur("a3-atelier", bouton, 14.65, duree=0.45, depart=(W / 2, H / 2))
        else:
            s.curseur(image, bouton, tc - 0.5, duree=0.45)
        s.clic(image, bouton, tc)
    couleur = boite("b3-couleur", "couleur")
    s.plan("b3-couleur", 18.35, 22.1, fondu=0.25)
    s.curseur("b2-modele", couleur, 17.75, duree=0.55)
    s.clic("b3-couleur", couleur, 18.3)
    panneau = {"x": 1160, "y": 186, "w": 270, "h": 404}
    s.cerne("b3-couleur", panneau, 19.2, s.duree, marge=4)
    option = boite("b4-sans-intervenants", "option")
    s.plan("b4-sans-intervenants", 22.05, 23.0, fondu=0.2)
    s.curseur("b3-couleur", option, 21.2, duree=0.7)
    s.clic("b4-sans-intervenants", option, 22.0)
    s.plan("b5-avec-intervenants", 22.95, s.duree, fondu=0.2)
    s.clic("b5-avec-intervenants", option, 22.9)
    return s


def scene_s5():
    s = Scene("s5", DEBUT["s6"] - DEBUT["s5"])
    s.plan("b5-avec-intervenants", 0, 2.9, fondu=0)
    onglet = boite("b6-textes", "onglet")
    option = centre(boite("b5-avec-intervenants", "option"))
    s.curseur("b5-avec-intervenants", onglet, 1.9, duree=0.7, depart=vers_toile(option, s.etat["b5-avec-intervenants"]))
    s.clic("b5-avec-intervenants", onglet, 2.6)
    s.plan("b6-textes", 2.65, 6.8, fondu=0.25)
    s.cerne("b6-textes", boite("b6-textes", "titre"), 3.0, 4.6)
    enreg = boite("b7-avant-enregistrer", "enregistrer")
    s.curseur("b6-textes", enreg, 5.5, duree=0.9)
    s.cerne("b6-textes", enreg, 5.7, 6.9, marge=5)
    s.clic("b6-textes", enreg, 6.55)
    s.plan("b8-enregistre", 6.6, 8.9, fondu=0.2)
    s.cacher_curseur(8.3)
    sect = boite("modele-dossier", "section")
    s.plan("modele-dossier", 8.4, s.duree, z=1.45, cible=sect, fondu=0.45)
    s.cerne("modele-dossier", boite("modele-dossier", "image"), 9.2, s.duree)
    return s


def scene_s6():
    s = Scene("s6", DEBUT["s7"] - DEBUT["s6"])
    s.chapitre(2, "Déposer votre image", 0.2, 4.0)
    sect = boite("c0-dossier", "section")
    s.plan("c0-dossier", 0, 6.3, z=1.45, cible=sect, fondu=0.4)
    deposer = boite("c0-dossier", "deposer")
    s.curseur("c0-dossier", deposer, 3.4, duree=0.9)
    s.clic("c0-dossier", deposer, 4.5)
    # Le sélecteur de fichiers du système n'est pas filmé : le fichier choisi apparaît.
    x, y = vers_toile(centre(deposer), s.etat["c0-dossier"])
    s.brut(f'<div id="s6-fichier" class="fichier" style="left:{x + 40:.0f}px; top:{y + 40:.0f}px">'
           f'<img src="assets/port-littoral.jpg" alt=""><span>port-littoral.jpg</span></div>',
           ['tl.fromTo("#s6-fichier", {opacity:0, y:14}, {opacity:1, y:0, duration:0.35, ease:"power3.out"}, 5.0);',
            'tl.to("#s6-fichier", {opacity:0, duration:0.25}, 6.15);'])
    s.cacher_curseur(6.0)
    dialogue = boite("c1-recadrage", "dialog")
    s.plan("c1-recadrage", 6.2, 12.4, z=1.15, cible=dialogue, fondu=0.35)
    forme = {"x": 855, "y": 183, "w": 135, "h": 24}
    cadre1 = {"x": 385, "y": 270, "w": 362, "h": 204}
    s.cerne("c1-recadrage", forme, 7.5, 10.6, marge=6)
    s.cerne("c1-recadrage", cadre1, 7.9, 10.6, marge=2)
    depart = (566, 372)
    s.curseur("c1-recadrage", depart, 10.9, duree=0.5)
    s.plan("c2-cadre", 11.55, 15.0, z=1.15, cible=dialogue, fondu=0.75)
    s.glisser("c1-recadrage", depart, (475, 372), 11.55, 0.8)
    desc = boite("c1-recadrage", "desc")
    s.curseur("c2-cadre", desc, 13.7, duree=0.7)
    s.clic("c2-cadre", desc, 14.45)
    s.plan("c3-description", 14.7, 19.4, z=1.15, cible=dialogue, fondu=0.6)
    s.cerne("c3-description", boite("c3-description", "desc"), 14.6, 17.3)
    tele = boite("c3-description", "televerser")
    s.curseur("c3-description", tele, 17.8, duree=0.7)
    s.clic("c3-description", tele, 18.6)
    s.cacher_curseur(19.2)
    sect = boite("photo-dossier", "section")
    s.plan("photo-dossier", 19.2, s.duree, z=1.45, cible=sect, fondu=0.45)
    s.cerne("photo-dossier", boite("photo-dossier", "image"), 20.4, s.duree)
    return s


def scene_s7():
    s = Scene("s7", FIN - DEBUT["s7"])
    p = s.plan("photo-public-bas", 0, s.duree, fondu=0.45)
    s.zoom(p, "photo-public-bas", 0.2, 1.08, {"x": 105, "y": 135, "w": 810, "h": 457}, duree=2.4)
    etapes = [("Mon organisation", 3.1), ("Votre dossier", 5.0), ("Image de couverture", 6.6)]
    morceaux, js = [], []
    for i, (texte, t0) in enumerate(etapes):
        eid = f"s7-etape{i}"
        fleche = '<svg viewBox="0 0 12 20" width="14" height="22"><path d="M2 2 L10 10 L2 18" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/></svg>' if i else ""
        morceaux.append(f'<span id="{eid}" class="etape">{fleche}<b>{texte}</b></span>')
        js.append(f'tl.fromTo("#{eid}", {{opacity:0, x:-10}}, {{opacity:1, x:0, duration:0.35, ease:"power3.out"}}, {t0});')
    s.brut(f'<div id="s7-voile" class="voile"></div><div id="s7-chemin" class="chemin">{"".join(morceaux)}</div>',
           ['tl.fromTo("#s7-voile", {opacity:0}, {opacity:1, duration:0.5}, 2.9);'] + js
           + [f'tl.fromTo("#s7-noir", {{opacity:0}}, {{opacity:1, duration:0.9}}, {s.duree - 1.0:.2f});'])
    s.brut(f'<div id="s7-noir" class="noir"><img src="assets/oif-nom-ifdd-epavillon-inverse.svg" alt=""></div>')
    return s


def ecrire_intro():
    page = f"""<template>
  <style>
    #root {{ position:absolute; inset:0; }}
    {POLICES}
  </style>
  <div id="root" data-composition-id="intro" data-width="{W}" data-height="{H}">
    <div id="intro-fond" class="intro-fond">
      <img id="intro-logo" class="intro-logo" src="assets/oif-nom-ifdd-epavillon-inverse.svg" alt="">
      <div class="intro-bloc">
        <p id="intro-sur" class="intro-sur">Espace organisation de l'ePavillon</p>
        <h1 id="intro-titre" class="intro-titre">Ajouter l'image de couverture de votre activité</h1>
      </div>
      <div class="liseré"><i></i><i></i><i></i><i></i><i></i></div>
    </div>
  </div>
  <script>
    const tl = gsap.timeline({{ paused: true }});
    tl.fromTo("#intro-titre", {{y:10}}, {{y:0, duration:2.4, ease:"power1.out"}}, 0);
    tl.to("#intro-fond", {{opacity:0, duration:0.5, ease:"power1.in"}}, 2.45);
    tl.set({{}}, {{}}, 2.94);
    window.__timelines["intro"] = tl;
  </script>
</template>
"""
    (ICI / "compositions" / "intro.html").write_text(page)


# Sous-titres : (scène, début, fin, texte), temps locaux à la voix.
SOUS_TITRES = [
    ("s1", 0.1, 3.49, "L'image de couverture illustre votre activité sur la plateforme."),
    ("s1", 3.94, 7.76, "C'est aussi elle qui s'affiche quand quelqu'un partage le lien de votre activité."),
    ("s2", 0.1, 2.70, "Pour l'ajouter, ouvrez « Mon organisation »."),
    ("s2", 3.20, 6.53, "Tant que l'image manque, la carte « À faire » vous le rappelle."),
    ("s2", 7.05, 10.0, "Cliquez sur « Image de couverture à ajouter »."),
    ("s3", 0.1, 3.74, "Vous arrivez dans votre dossier, à la rubrique « Image de couverture »."),
    ("s3", 4.26, 8.89, "Deux façons de faire : partir d'un modèle, ou déposer votre propre image."),
    ("s4", 0.1, 4.33, "Commençons par les modèles. Cliquez sur « Créer ou choisir un modèle »."),
    ("s4", 4.85, 8.58, "Les modèles sont préparés par l'IFDD, aux couleurs de la Francophonie."),
    ("s4", 9.14, 13.93, "Le titre, la date, le lieu et les intervenants de votre activité sont déjà en place."),
    ("s4", 14.54, 17.31, "Cliquez sur un modèle : l'image change aussitôt."),
    ("s4", 17.85, 20.72, "Choisissez une couleur, puis ce qui apparaît sur l'image :"),
    ("s4", 21.13, 24.64, "la date, les intervenants, le logo de votre organisation."),
    ("s5", 0.1, 3.95, "Le titre est trop long sur l'image ? Raccourcissez-le dans l'onglet « Textes »."),
    ("s5", 4.47, 7.92, "Quand l'image vous plaît, cliquez sur « Enregistrer la couverture »."),
    ("s5", 8.44, 12.08, "Elle s'affiche dans votre dossier, et vous pourrez la modifier quand vous voudrez."),
    ("s6", 0.1, 1.79, "Vous avez déjà votre propre visuel ?"),
    ("s6", 2.32, 6.81, "Dans le dossier, cliquez sur « Déposer une image » et choisissez votre fichier."),
    ("s6", 7.38, 13.17, "L'image doit être au format paysage, 16 sur 9 : déplacez le cadre pour garder l'essentiel."),
    ("s6", 13.69, 17.26, "Vous pouvez décrire l'image en une phrase, pour les personnes qui ne la voient pas."),
    ("s6", 17.83, 19.37, "Cliquez sur « Téléverser »."),
    ("s6", 19.86, 23.44, "Après quelques secondes de préparation, l'image s'affiche partout."),
    ("s7", 0.1, 2.64, "Et voilà : votre activité a son image."),
    ("s7", 3.06, 8.10, "Pour la changer, il suffit de revenir dans votre dossier, rubrique « Image de couverture »."),
]


def ecrire_sous_titres():
    groupes, js = [], []
    for i, (s, a, b, texte) in enumerate(SOUS_TITRES):
        a, b = DEBUT[s] + a, DEBUT[s] + b + 0.15
        gid = f"st{i}"
        groupes.append(f'<p id="{gid}" class="st">{texte.replace(" :", "&nbsp;:").replace(" ?", "&nbsp;?").replace("« ", "«&nbsp;").replace(" »", "&nbsp;»")}</p>')
        js.append(f'tl.fromTo("#{gid}", {{opacity:0}}, {{opacity:1, duration:0.15}}, {a:.2f});')
        js.append(f'tl.to("#{gid}", {{opacity:0, duration:0.12, ease:"power2.in"}}, {b - 0.12:.2f});')
        js.append(f'tl.set("#{gid}", {{visibility:"hidden"}}, {b:.2f});')
    page = f"""<template>
  <style>
    #root {{ position:absolute; inset:0; }}
    {POLICES}
  </style>
  <div id="root" data-composition-id="soustitres" data-width="{W}" data-height="{H}">
    <div class="st-bande" data-layout-allow-caption-zone>
      {chr(10).join('      ' + g for g in groupes).strip()}
    </div>
  </div>
  <script>
    const tl = gsap.timeline({{ paused: true }});
    {chr(10).join('    ' + j for j in js).strip()}
    tl.set({{}}, {{}}, {FIN - 0.01:.2f});
    window.__timelines["soustitres"] = tl;
  </script>
</template>
"""
    (ICI / "compositions" / "soustitres.html").write_text(page)


POLICES = """
  @font-face { font-family:"Barlow Condensed"; font-weight:600; src:url("assets/fonts/BarlowCondensed-600-Latin.woff2") format("woff2"); }
  @font-face { font-family:"Barlow Condensed"; font-weight:700; src:url("assets/fonts/BarlowCondensed-700-Latin.woff2") format("woff2"); }
  @font-face { font-family:"Inter Site"; font-weight:400 700; src:url("assets/fonts/Inter-400-700-Latin.woff2") format("woff2"); }
"""

STYLE = f"""
  @font-face {{ font-family:"Barlow Condensed"; font-weight:600; src:url("assets/fonts/BarlowCondensed-600-Latin.woff2") format("woff2"); }}
  @font-face {{ font-family:"Barlow Condensed"; font-weight:700; src:url("assets/fonts/BarlowCondensed-700-Latin.woff2") format("woff2"); }}
  @font-face {{ font-family:"Inter Site"; font-weight:400 700; src:url("assets/fonts/Inter-400-700-Latin.woff2") format("woff2"); }}
  * {{ margin:0; padding:0; box-sizing:border-box; }}
  html, body {{ width:{W}px; height:{H}px; overflow:hidden; background:{NUIT}; }}
  #fond {{ position:absolute; inset:0; background:{NUIT}; }}
  .ecran {{ position:absolute; left:{FX}px; top:{FY}px; width:{FW}px; height:{FH}px; overflow:hidden;
           border-radius:12px; background:#efeeec; box-shadow:0 0 0 1px rgba(255,255,255,.14); }}
  .plan {{ position:absolute; inset:0; }}
  .cam {{ position:absolute; left:0; top:0; width:{FW}px; height:{FH}px; transform-origin:0 0; }}
  .cam img {{ width:{FW}px; height:{FH}px; display:block; }}
  .curseur {{ position:absolute; left:0; top:0; width:34px; height:41px; z-index:20; }}
  .curseur svg {{ display:block; filter:drop-shadow(0 2px 2px rgba(0,0,0,.25)); }}
  .onde {{ position:absolute; width:60px; height:60px; border-radius:50%; border:3px solid {CYAN}; z-index:19; opacity:0; }}
  .cerne {{ position:absolute; border:4px solid {CYAN}; border-radius:10px; z-index:10; opacity:0; }}
  .etiquette {{ position:absolute; z-index:11; opacity:0; background:{NUIT}; color:#fff; padding:10px 18px 11px;
               border-radius:8px; font:600 30px/1 "Barlow Condensed"; white-space:nowrap; }}
  .chapitre {{ position:absolute; left:{FX + 32}px; bottom:{H - FY - FH + 32}px; z-index:12; opacity:0;
              display:flex; flex-direction:column; gap:4px; background:#fff; color:{NUIT};
              border-left:8px solid {CYAN}; border-radius:8px; padding:16px 26px 18px 22px;
              box-shadow:0 6px 18px rgba(29,26,91,.18); }}
  .chapitre b {{ font:700 30px/1 "Barlow Condensed"; color:#0c6792; }}
  .chapitre span {{ font:700 44px/1 "Barlow Condensed"; }}
  .voile {{ position:absolute; left:{FX}px; top:{FY}px; width:{FW}px; height:{FH}px; border-radius:12px;
           background:rgba(29,26,91,.62); z-index:8; opacity:0; }}
  .partage {{ position:absolute; left:{FX + FW / 2 - 380}px; top:{FY + 120}px; width:760px; z-index:9; opacity:0;
             background:#fff; border-radius:14px; overflow:hidden; box-shadow:0 18px 40px rgba(0,0,0,.28); }}
  .partage img {{ width:760px; height:428px; display:block; transform-origin:50% 60%; }}
  .partage-texte {{ display:flex; flex-direction:column; gap:8px; padding:20px 26px 24px; font-family:"Inter Site"; }}
  .partage-site {{ font-size:28px; color:#5b5a6e; }}
  .partage-texte strong {{ font:700 34px/1.1 "Barlow Condensed"; color:{NUIT}; }}
  .partage-lieu {{ font-size:28px; color:#5b5a6e; }}
  .fichier {{ position:absolute; z-index:15; opacity:0; display:flex; align-items:center; gap:14px; background:#fff;
             border-radius:10px; padding:10px 18px 10px 10px; box-shadow:0 10px 26px rgba(29,26,91,.25);
             font:600 28px "Inter Site"; color:{NUIT}; }}
  .fichier img {{ width:96px; height:54px; object-fit:cover; border-radius:6px; }}
  .chemin {{ position:absolute; left:{FX}px; width:{FW}px; top:{FY + FH / 2 - 60}px; z-index:9; display:flex;
            justify-content:center; align-items:center; gap:22px; }}
  .etape {{ display:flex; align-items:center; gap:22px; color:#fff; opacity:0; }}
  .etape b {{ font:700 64px/1 "Barlow Condensed"; background:#fff; color:{NUIT}; padding:16px 26px 18px; border-radius:10px; }}
  .etape svg {{ color:{CYAN_CLAIR}; }}
  .noir {{ position:absolute; inset:0; background:{NUIT}; z-index:30; opacity:0; display:grid; place-items:center; }}
  .noir img {{ height:96px; }}
  .intro-fond {{ position:absolute; inset:0; background:{NUIT}; }}
  .intro-logo {{ position:absolute; left:{FX}px; top:84px; height:64px; }}
  .intro-bloc {{ position:absolute; left:{FX}px; right:{FX}px; top:380px; }}
  .intro-sur {{ font:400 40px/1.2 "Inter Site"; color:{CYAN_CLAIR}; margin-bottom:22px; }}
  .intro-titre {{ font:700 132px/0.98 "Barlow Condensed"; color:#fff; max-width:1400px; }}
  .liseré {{ position:absolute; left:0; right:0; bottom:0; height:10px; display:flex; }}
  .liseré i {{ flex:1; }}
  .liseré i:nth-child(1) {{ background:#00a1e4; }} .liseré i:nth-child(2) {{ background:#732f85; }}
  .liseré i:nth-child(3) {{ background:#ffd500; }} .liseré i:nth-child(4) {{ background:#8fbf2f; }}
  .liseré i:nth-child(5) {{ background:#e63132; }}
  .st-bande {{ position:absolute; left:0; right:0; top:{FY + FH + 8}px; height:{H - FY - FH - 16}px; }}
  .st {{ position:absolute; left:0; right:0; top:50%; transform:translateY(-50%); text-align:center; color:#fff;
        font:400 34px/1.28 "Inter Site"; padding:0 220px; opacity:0; }}
"""


def ecrire_index(scenes):
    hotes = [f'<div id="intro" data-composition-id="intro" data-composition-src="compositions/intro.html" data-start="0" data-duration="2.95" data-track-index="1" data-track-kind="graphics" data-width="{W}" data-height="{H}" style="z-index:5"></div>']
    audios = []
    for s in scenes:
        fin = FIN if s.nom == "s7" else DEBUT[f"s{int(s.nom[1]) + 1}"]
        hotes.append(f'<div id="{s.nom}" data-composition-id="{s.nom}" data-composition-src="compositions/{s.nom}.html" data-start="{DEBUT[s.nom]:.2f}" data-duration="{fin - DEBUT[s.nom]:.2f}" data-track-index="1" data-track-kind="graphics" data-width="{W}" data-height="{H}" style="z-index:2"></div>')
        audios.append(f'<audio id="voix-{s.nom}" class="clip" src="assets/voix/{s.nom}.mp3" data-start="{DEBUT[s.nom]:.2f}" data-duration="{VOIX[s.nom]:.2f}" data-track-index="3" data-volume="1"></audio>')
        for j, tc in enumerate(s.clics):
            audios.append(f'<audio id="clic-{s.nom}-{j}" class="clip" src="assets/sons/clic.wav" data-start="{DEBUT[s.nom] + tc:.2f}" data-duration="0.05" data-track-index="4" data-volume="0.35"></audio>')
    hotes.append(f'<div id="soustitres" data-composition-id="soustitres" data-composition-src="compositions/soustitres.html" data-start="0" data-duration="{FIN:.2f}" data-track-index="2" data-track-kind="captions" data-width="{W}" data-height="{H}" style="z-index:6"></div>')
    page = f"""<!doctype html>
<html lang="fr">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width={W}, height={H}" />
  <script src="https://cdn.jsdelivr.net/npm/gsap@3.15.0/dist/gsap.min.js"></script>
  <style>{STYLE}</style>
</head>
<body>
  <div id="root" data-composition-id="main" data-start="0" data-duration="{FIN:.2f}" data-width="{W}" data-height="{H}">
    <div id="fond"></div>
    {chr(10).join('    ' + h for h in hotes).strip()}
    {chr(10).join('    ' + a for a in audios).strip()}
  </div>
  <script>
    window.__timelines = window.__timelines || {{}};
    const tl = gsap.timeline({{ paused: true }});
    tl.set({{}}, {{}}, {FIN - 0.01:.2f});
    window.__timelines["main"] = tl;
  </script>
</body>
</html>
"""
    (ICI / "index.html").write_text(page)


if __name__ == "__main__":
    (ICI / "compositions").mkdir(exist_ok=True)
    ecrire_intro()
    scenes = [f() for f in (scene_s1, scene_s2, scene_s3, scene_s4, scene_s5, scene_s6, scene_s7)]
    for s in scenes:
        s.ecrire()
    ecrire_sous_titres()
    ecrire_index(scenes)
    print("durée", FIN, {k: v for k, v in DEBUT.items()})

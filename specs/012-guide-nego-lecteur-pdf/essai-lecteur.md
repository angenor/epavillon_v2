# Essai — le vrai guide lu par pdf.js (phase 1)

**À remplir en phase 1**, avant tout code du cycle. Le prototype est jetable : une page construite qui lit le vrai guide de la CdP30 (`.essais/guide-cdp30.pdf`, hors de Git) par `PDFViewer`, build `legacy`, depuis une copie en Cache Storage puis par plages. Voir [research.md](research.md), R2.

## Conditions

| | |
|---|---|
| Version de `pdfjs-dist` | … |
| Poste, navigateur, profil | … (Chrome, émulation mobile 360 × 780, CPU ×4) |
| WebKit | … (Safari du Mac, simulateur iOS …) |
| Appareils réels | … (Android : modèle, version ; iPhone : modèle, iOS) — par le commanditaire |

## Le fichier

| Question | Outil | Résultat |
|---|---|---|
| Polices non incorporées | `pdffonts` | … |
| Polices CID (cMaps nécessaires ?) | `pdffonts` | … |
| Images JPEG 2000, JBIG2 | `pdfimages -list` | … |
| Linéarisé | `qpdf --check` | … |

## La grille

| # | Critère | Seuil | Mesure | Verdict |
|---|---|---|---|---|
| 1 | Défilement des 90 pages | aucune tâche longue > 100 ms pendant le geste ; aucune page blanche > 1 s après l'arrêt | … | … |
| 2 | Mémoire, trois allers-retours | stable d'un tour à l'autre | … | … |
| 3 | Grossissement ×4, tableau des sigles | net < 1 s après le geste, aucune erreur de canevas | … | … |
| 4 | Première page en ligne, « 3G lente » | < 8 s ; requêtes `206` de 256 Ko | … | … |
| 5 | Dix expressions repérées sur leur page (R7) | 10 sur 10, ou « ne peut pas marquer », jamais ailleurs | … | … |
| 6 | Paragraphe à deux colonnes, page 59, copié | ordre de lecture gardé | … | … |
| 7 | Hors connexion : travailleur, `wasm`, polices servis par la coquille | Chrome : … · WebKit : … | … | … |
| 8 | Taille de la coquille ajoutée | ≈ 1,1 Mo compressé | … | … |

## Issue

- **A** — tout passe : ADR-022, puis la phase 2.
- **B** — WebKit hors connexion sans `wasm` : `wasmUrl` à `null` hors connexion ; dire ce que le guide y perd.
- **C** — mémoire ou fluidité insuffisantes : réglages baissés et remesurés, ou retour au commanditaire.

**Issue retenue** : …

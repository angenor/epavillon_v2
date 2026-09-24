# Essai — le vrai guide lu par pdf.js (phase 1)

**À remplir en phase 1**, avant tout code du cycle. Le prototype est jetable : une page construite qui lit le vrai guide de la CdP30 (`.essais/guide-cdp30.pdf`, hors de Git) par `PDFViewer`, build `legacy`, depuis une copie en Cache Storage puis par plages. Voir [research.md](research.md), R2.

## Conditions

| | |
|---|---|
| Version de `pdfjs-dist` | … |
| Poste, navigateur, profil | … (Chrome, émulation mobile 360 × 780, CPU ×4) |
| WebKit | … (Safari du Mac ; simulateurs iOS 16, 17, 18 par Xcode). **Au 24/09, Xcode n'est pas installé sur le poste** — seuls les outils en ligne de commande : l'installer avec les environnements iOS 16, 17 et 18 avant l'essai, en surveillant la place sur le disque |
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
| 4 | Première page en ligne | < 3 s sur un réseau ordinaire, < 8 s en « 3G lente » ; requêtes `206` de 256 Ko | ordinaire : … · 3G lente : … | … |
| 5a | Passages de recherche retrouvés sur leur page (R7) — cinquante expressions, dont césures, ligatures, insécables, apostrophes, deux colonnes | taux mesuré ; jamais marqué ailleurs | … / 50 | … |
| 5b | Passages de note retrouvés — dix passages cités | taux mesuré ; un introuvable va en tête de page sans rien perdre | … / 10 | … |
| 6 | Paragraphe à deux colonnes, page 59, copié | ordre de lecture gardé | … | … |
| 7 | Hors connexion : travailleur, `wasm`, polices servis par la coquille | Chrome : … · WebKit : … | … | … |
| 8 | Taille de la coquille ajoutée | ≈ 1,1 Mo compressé | … | … |
| 9 | Version d'iOS réellement exigée par le legacy — **si aucun iPhone ancien ni aucun simulateur n'est disponible, l'écrire ici et renvoyer à T084** : rien n'est bloqué, la seconde sécurité tient sans mesure | simulateur iOS 16 (Xcode), puis 17 et 18 : pages affichées ou non, et **quelles fonctions manquent** | iOS 16 : … · 17 : … · 18 : … | … |
| 10 | Fonctions à détecter | la liste de 9, sans polyfill possible, reprise dans `charger.ts` et son test | … | … |

## Issue

- **A** — tout passe : ADR-022, puis la phase 2.
- **B** — WebKit hors connexion sans `wasm` : `wasmUrl` à `null` hors connexion ; dire ce que le guide y perd.
- **C** — mémoire ou fluidité insuffisantes : réglages baissés et remesurés, ou retour au commanditaire.

**Issue retenue** : …

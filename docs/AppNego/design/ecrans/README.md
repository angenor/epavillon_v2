# Guide Négo — maquette COP31 (export HTML statique)

Maquette de l'application mobile Guide Négo (IFDD), destinée à être reprise en code.
Chaque page est un fichier HTML autonome : styles en ligne, pictogrammes en SVG inline,
aucune dépendance JavaScript. Seule ressource externe : la police Atkinson Hyperlegible Next
(Google Fonts) — à embarquer dans l'application (OFL 1.1, voir `../passation/police/`).

État du 20 septembre 2026 : dix-huit pages (00 à 17), le système, le journal et les fichiers
de passation. La maquette est terminée ; les lignes « En attente » du journal restent à trancher.

## Lire dans cet ordre

1. `01-systeme.html` — jetons (couleurs, typographie, mesures), pictogrammes, composants,
   états, mouvement. Les sections « 4 bis » à « 4 septdecies » sont les composants ajoutés
   page par page, avant usage.
2. `../passation/` — les mêmes jetons et composants en fichiers pour le code (voir plus bas).
3. `journal-de-maquette.html` — une ligne par décision, avec l'écran où la voir.
   Les lignes « En attente » sont des choix de maquette à confirmer (données inventées,
   à remplacer par les vraies).
4. Les pages 00 à 17, dans l'ordre. Chaque écran est un cadre de 360 × 800 px
   (Android milieu de gamme), identifié par un badge (1a, 1b…) et un attribut
   `data-screen-label`.

## Dossier `../passation/`

Extrait de « 01 — Système », sans rien inventer : chaque valeur existe dans la maquette.

- `tokens.json` — deux niveaux : la charte IFDD et les nuances du sombre avec leurs valeurs,
  puis les rôles (fond, texte, texte secondaire, titre, accent, filets, attention, succès, danger,
  information, réseau, neutre, et chaque état) qui les référencent, en clair et en sombre ;
  typographie (famille, échelle, graisses, interlignes) ; mesures ; durées et courbes.
- `theme.css` — les mêmes jetons en variables CSS. Charte sous `:root` ; rôles bornés à
  `[data-app="guide-nego"]` ; sombre sous `[data-app="guide-nego"][data-theme="sombre"]`.
  Un rôle référence toujours une couleur de charte par `var()`.
- `mesures.css` — grille, espacements, cibles, filets, rayons, zones sûres haute et basse,
  dimensions des composants.
- `pictogrammes.svg` — la famille entière en symboles, un `id` par pictogramme (`gn-check`…),
  grille 24, trait 2 px, bouts ronds, `currentColor`. `pictogrammes-planche.html` les affiche.
- `composants.md` — chaque composant : rôle, variantes, états, mesures, règles d'usage, écran
  où le voir (`page · data-screen-label`).
- `mouvement.md` — durées, courbes, ce qui s'anime, « réduire les animations ».
- `police/LISEZMOI.md` — fichiers à embarquer, graisses, `@font-face`, licence. Les fichiers
  de police eux-mêmes ne sont pas dans l'export.
- `ecarts.md` — les écarts entre « 01 — Système » et les pages, listés au lieu d'être tranchés.

## Repères pour le code

- Un seul instant partout : jeudi 12 novembre 2026, 11:40, heure d'Antalya.
- Grille de 4 px ; marge d'écran 16 px ; cibles 48 px ; rayons 0 / 4 / 24 px.
- Couleurs de charte : #233400 · #3C5404 · #557607 · #8FBF2F · #D3B011 · #816B06 · #FFD500 ·
  #231F20 · #565554 · #D9D8D6 · #ECFFD3 · #FFF7DA · #FFFFFF ; états : #C7101D · #732F85 · #0C6792.
  Thème sombre : #101704 fond, #182208 bloc, #B5D66A vert, #6E9A2A filets, #D9C878 jaune,
  #EEF2E4 / #B9C0AA textes, #63B9E0 · #F08A90 · #CB9AD8 · #A7A6A3 états.
- Un état = un pictogramme + un mot + une couleur, jamais la couleur seule.
- Le jaune (#D3B011) signale ce qui vous concerne à l'instant — session en cours,
  non-lu, mention — et rien d'autre. La marque de rôle est une étiquette bordée, plus jamais sur jaune.
- Trois agendas jamais confondus : Sessions de négociation · Réunions de la Francophonie ·
  Pavillon de la Francophonie. Le mot « Programme » seul n'apparaît nulle part.
- Barre à cinq onglets — Accueil · Négociations · Francophonie · Échanges · Ressources —,
  libellés 13 px jamais tronqués ; onglets à largeur de libellé (mesure sur `12-echanges.html`, 1a).
- Les Échanges : « Échanges hébergés et modérés par l'IFDD », factuels, jamais la stratégie d'un groupe.

## Dossier `docs/`

Brief, intentions de design et lexique de l'interface, tels que fournis à la maquette.

## Ce qui manque

- Icône de l'application et écran d'installation (le monogramme « GN » tient la place).
- Les fichiers de police (à télécharger, voir `../passation/police/`).
- Les données réelles : programme des 16 modules, textes des messages, questions de FAQ
  et de quiz, lexique complet, annuaire (voir les lignes « En attente » du journal).
- Les décisions listées dans `../passation/ecarts.md`.

# Police — Atkinson Hyperlegible Next

Extrait de « 01 — Système », section 2 et point F.

## Famille

**Atkinson Hyperlegible Next** — une seule famille dans toute l'application. Repli : `system-ui, sans-serif`.

## Graisses à embarquer

| Fichier attendu (WOFF2) | Graisse | Style | Emploi |
|---|---|---|---|
| `AtkinsonHyperlegibleNext-Regular.woff2` | 400 | droit | Corps 17, secondaire 15, sous-parties de sommaire |
| `AtkinsonHyperlegibleNext-SemiBold.woff2` | 600 | droit | Titres de ligne, libellés de réglage, onglets au repos, « EN » |
| `AtkinsonHyperlegibleNext-Bold.woff2` | 700 | droit | Titres, heures, salles, marques d'état, boutons, onglet actif |
| `AtkinsonHyperlegibleNext-Italic.woff2` | 400 | italique | Terme anglais dans un texte français |
| `AtkinsonHyperlegibleNext-SemiBoldItalic.woff2` | 600 | italique | Terme anglais en tête de ligne de lexique, correction proposée |

Cinq fichiers, aucun autre. Les noms de fichiers ci-dessus sont ceux de la distribution officielle ; à vérifier au téléchargement.

## Réglages

- `font-variant-numeric: tabular-nums` partout — les heures s'alignent dans la colonne de 62 px. À vérifier à l'export que la fonctionnalité `tnum` est bien active dans les fichiers embarqués.
- Rien sous 15 px, hors libellés d'onglets (13 px), marque de rôle (13 px) et compteur (13 px).
- Échelle et interlignes : `tokens.json › typographie.echelle`.
- Tailles de lecture réglables : 17 · 20 · 24, interligne 1,5 ; les titres ne bougent pas.

## Embarquement

L'application embarque les WOFF2 dans son paquet : **aucune requête réseau en salle**. La maquette HTML, elle, charge la police depuis Google Fonts (`css2?family=Atkinson+Hyperlegible+Next:ital,wght@0,400;0,600;0,700;1,400;1,600`) — ce n'est pas la solution à livrer.

```css
@font-face { font-family: 'Atkinson Hyperlegible Next'; font-weight: 400; font-style: normal; src: url('AtkinsonHyperlegibleNext-Regular.woff2') format('woff2'); font-display: swap; }
@font-face { font-family: 'Atkinson Hyperlegible Next'; font-weight: 600; font-style: normal; src: url('AtkinsonHyperlegibleNext-SemiBold.woff2') format('woff2'); font-display: swap; }
@font-face { font-family: 'Atkinson Hyperlegible Next'; font-weight: 700; font-style: normal; src: url('AtkinsonHyperlegibleNext-Bold.woff2') format('woff2'); font-display: swap; }
@font-face { font-family: 'Atkinson Hyperlegible Next'; font-weight: 400; font-style: italic; src: url('AtkinsonHyperlegibleNext-Italic.woff2') format('woff2'); font-display: swap; }
@font-face { font-family: 'Atkinson Hyperlegible Next'; font-weight: 600; font-style: italic; src: url('AtkinsonHyperlegibleNext-SemiBoldItalic.woff2') format('woff2'); font-display: swap; }
```

## Licence

**SIL Open Font License 1.1** (Braille Institute of America). Elle autorise l'embarquement dans l'application et la redistribution ; le fichier `OFL.txt` doit accompagner les polices dans le dépôt. La police ne peut pas être vendue seule.

## Ce que ce dossier ne contient pas

Les fichiers de police eux-mêmes ne sont pas dans cet export : ils se téléchargent depuis la distribution officielle de la Braille Institute (ou Google Fonts) et se placent ici avec `OFL.txt`.

## Fichiers présents — ajout du 20/09/2026

Les cinq graisses sont dans ce dossier, avec `OFL.txt`, prises du paquet `@fontsource/atkinson-hyperlegible-next` (mêmes sources que Google Fonts). Chaque graisse tient en **deux fichiers** : le sous-ensemble *latin*, sous le nom attendu, et le sous-ensemble *latin étendu*, suffixé `-LatinExt`.

**Les deux sont nécessaires au français** : « œ » et « Œ » (U+0152–0153) vivent dans le latin étendu. Chaque `@font-face` ci-dessus se double donc d'une seconde déclaration, même famille, même graisse, avec sa plage :

```css
/* latin */      unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
/* latin étendu */ unicode-range: U+0100-02BA, U+02BD-02C5, U+02C7-02CC, U+02CE-02D7, U+02DD-02FF, U+0304, U+0308, U+0329, U+1D00-1DBF, U+1E00-1E9F, U+1EF2-1EFF, U+2020, U+20A0-20AB, U+20AD-20C0, U+2113, U+2C60-2C7F, U+A720-A7FF;
```

À vérifier à l'étape 0a, sur l'écran : « cœur », « Œuvre », « É À Ç « » », et les chiffres tabulaires.

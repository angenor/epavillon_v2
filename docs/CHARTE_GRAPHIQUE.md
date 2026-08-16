# Charte graphique IFDD — Couleurs

> Contenu extrait du fichier `Charte_graphique_Ifdd_couleurs.ai`
> (Adobe Illustrator 27.6 / Windows — bibliothèque de nuances, 1er septembre 2023)

**Remarque importante :** ce fichier est une **bibliothèque de nuanciers** (`illustrator:Type = Swatches`). Il ne contient aucun texte rédactionnel ni logo dessiné — uniquement la définition des couleurs de la charte. Tout son contenu utile est donc la palette ci-dessous.

---

## 1. Couleurs principales IFDD (tons directs / *spot colors*)

Ces six couleurs sont définies en **CMJN comme tons directs**. Les valeurs CMJN sont celles du fichier ; les équivalents RVB/HEX sont des conversions approximatives (profil Coated FOGRA39 → sRGB) à utiliser pour l'écran.

| Nom de la nuance | C | M | J | N | HEX (approx.) | RVB (approx.) |
|---|---|---|---|---|---|---|
| cyan IFDD | 100 | 0 | 0 | 0 | `#00A1E4` | 0, 161, 228 |
| rouge IFDD | 0 | 94 | 87 | 0 | `#E63132` | 230, 49, 50 |
| jaune IFDD | 0 | 15 | 100 | 0 | `#FFD500` | 255, 213, 0 |
| vert IFDD | 53 | 0 | 100 | 0 | `#8FBF2F` | 143, 191, 47 |
| violet IFDD | 70 | 100 | 0 | 0 | `#732F85` | 115, 47, 133 |
| gris IFDD | 10 | 10 | 10 | 80 | `#565554` | 86, 85, 84 |

## 2. Couleurs complémentaires (quadrichromie)

| Nom de la nuance | HEX | RVB | C | M | J | N |
|---|---|---|---|---|---|---|
| bleu riche IFDD | `#1D1A5B` | 29, 26, 91 | 100 | 100 | 32,3 | 27,1 |
| gris pale IFDD | `#D9D8D6` | 217, 216, 214 | 13,7 | 10,7 | 12,3 | 0 |

## 3. Couleurs d'accent (quadrichromie)

| Nom de la nuance | HEX | RVB | C | M | J | N |
|---|---|---|---|---|---|---|
| Cyan accent | `#8CD5F2` | 140, 213, 242 | 41,2 | 0,7 | 1,9 | 0 |
| rouge accent | `#F28385` | 242, 131, 133 | 0,7 | 61,0 | 36,3 | 0 |
| jaune accent | `#FFF08C` | 255, 240, 140 | 1,5 | 1,3 | 56,0 | 0 |
| Vert accent | `#ADD476` | 173, 212, 118 | 35,6 | 0 | 70,5 | 0 |
| violet accent | `#906CAD` | 144, 108, 173 | 47,7 | 64,9 | 1,2 | 0 |

## 4. Neutres de base

| Nom de la nuance | HEX | RVB | C | M | J | N |
|---|---|---|---|---|---|---|
| White | `#FFFFFF` | 255, 255, 255 | 0 | 0 | 0 | 0 |
| Black | `#231F20` | 35, 31, 32 | 69,8 | 67,4 | 63,9 | 73,9 |
| [Registration] | — | 0, 0, 0 | 74,97 | 67,92 | 67,05 | 90,15 |

---

## 5. Polices d'écriture

| Police |
|---|
| Helvetica |
| NeueMaverick |

```css
:root {
  --ifdd-police-1: "Helvetica", Arial, sans-serif;
  --ifdd-police-2: "NeueMaverick", "Helvetica", sans-serif;
}
```

---

## 6. Palette complète en un coup d'œil

```
Principales   cyan #00A1E4 · rouge #E63132 · jaune #FFD500 · vert #8FBF2F · violet #732F85 · gris #565554
Complément.   bleu riche #1D1A5B · gris pâle #D9D8D6
Accents       cyan #8CD5F2 · rouge #F28385 · jaune #FFF08C · vert #ADD476 · violet #906CAD
Neutres       blanc #FFFFFF · noir #231F20
```

### Variables CSS prêtes à l'emploi

```css
:root {
  /* Couleurs principales */
  --ifdd-cyan:   #00A1E4;
  --ifdd-rouge:  #E63132;
  --ifdd-jaune:  #FFD500;
  --ifdd-vert:   #8FBF2F;
  --ifdd-violet: #732F85;
  --ifdd-gris:   #565554;

  /* Complémentaires */
  --ifdd-bleu-riche: #1D1A5B;
  --ifdd-gris-pale:  #D9D8D6;

  /* Accents */
  --ifdd-cyan-accent:   #8CD5F2;
  --ifdd-rouge-accent:  #F28385;
  --ifdd-jaune-accent:  #FFF08C;
  --ifdd-vert-accent:   #ADD476;
  --ifdd-violet-accent: #906CAD;

  /* Neutres */
  --ifdd-blanc: #FFFFFF;
  --ifdd-noir:  #231F20;
}
```

---

## 7. Informations techniques du fichier

| Champ | Valeur |
|---|---|
| Titre | Charte_graphique_Ifdd_couleurs |
| Fichier source d'origine | Logos_OIF-IFDD_variantes.ai |
| Auteur | Vanessa Cardoso |
| Logiciel | Adobe Illustrator 27.6 (Windows) — format AI 14.0 |
| Type de document | Bibliothèque de nuances (Swatches) |
| Profil de départ | Print (impression) |
| Mode colorimétrique | CMJN |
| Date de création / modification | 1er septembre 2023, 11 h 32 (UTC−4) |
| Format de page | 612 × 588 pt |
| Police intégrée | Myriad Pro Regular (uniquement pour le message d'avertissement d'Illustrator) |
| Transparence / surimpression visibles | Non / Non |

> Le fichier a été enregistré **sans contenu compatible PDF** : ouvert dans une application autre qu'Illustrator, il n'affiche qu'un message d'avertissement d'Adobe et non les nuances. Pour le rendre lisible ailleurs, il faut le ré-enregistrer depuis Illustrator avec l'option « Créer un fichier compatible PDF » activée.

### Annexe — plans de travail hérités du fichier source

Le document dont provient ce nuancier (`Logos_OIF-IFDD_variantes.ai`) comportait les plans de travail suivants ; leurs noms subsistent dans les métadonnées, mais les tracés correspondants ne sont pas présents dans ce fichier :

1. Logos_OIF-IFDD-Superposes
2. Logo_IFDD-LongHorizontal
3. Logo_IFDD-Court
4. Logo_IFDD_avec_Slogan
5. Logos_OIF-IFDD-Juxtaposes-Horizontaux

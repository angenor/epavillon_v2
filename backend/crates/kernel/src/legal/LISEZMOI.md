# Les textes qui engagent

Quatre fichiers : `privacy` (politique de confidentialité) et `terms` (conditions
d'utilisation), en `fr` et en `en`. **Ils sont écrits par l'IFDD, jamais par un outil** :
chaque inscription sur le site enregistre un accord à la version servie ici, et rien de
produit par une IA ne se publie sans validation humaine.

## L'en-tête

```
---
etat: en_attente | publie
version: …
en_vigueur: AAAA-MM-JJ
---
```

- **`en_attente`** : aucun corps. La route le dit, l'écran affiche « Texte en préparation
  par l'IFDD ». La version reste `2026-01`, celle que l'inscription enregistrait jusqu'ici.
- **`publie`** : le corps suit l'en-tête. **La date d'entrée en vigueur devient la version**
  (`version: 2026-11-01`, `en_vigueur: 2026-11-01`), et les deux langues d'un texte déclarent
  la même.

## Ce que les tests refusent

- Un texte modifié **sans nouvelle version** : `tests/legal_empreinte.rs` garde l'empreinte
  attendue de chaque fichier pour sa version ; la lever oblige à y inscrire la nouvelle.
- Une construction que l'application ne sait pas rendre : `tests/legal_grammaire.rs`. Sont
  admis les titres `##` et `###`, les paragraphes, les listes (`- ` ou `1. `), les liens
  `[texte](adresse)` et l'emphase `*…*`, `**…**`. Tableaux, notes de bas de page, images,
  blocs de code, citations, titres `#` et HTML échouent, avec la ligne en cause.

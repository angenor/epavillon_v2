# Direction « Nuit » : la maquette qui fait foi

Arbitrée le 05/10/2026 ([ADR-023](../../adr/023-direction-nuit.md)). Elle remplace la direction « Typographique » ([ADR-018](../../adr/018-direction-typographique-quatre-onglets.md)) et les écrans de [`../ecrans/`](../ecrans/), qui restent pour l'historique.

**Le commanditaire veut ces écrans au pixel près** : mêmes couleurs, mêmes polices, mêmes tailles, mêmes rayons, mêmes espacements. Seule la disposition s'ajuste à ce que l'application porte réellement (un bloc que la maquette ne montre pas prend la forme du bloc le plus proche qu'elle montre).

| Fichier | Écran | Moment |
|---|---|---|
| [01-ma-journee.html](01-ma-journee.html) | Accueil, compte connecté | Pendant la COP (mar. 10/11, 10:48) |
| [02-sessions.html](02-sessions.html) | Sessions de négociation | Pendant la COP |
| [03-lexique.html](03-lexique.html) | Lexique, un terme trouvé | |
| [04-premiere-venue.html](04-premiere-venue.html) | Accueil, sans compte | Pendant la COP |
| [05-avant-la-cop.html](05-avant-la-cop.html) | Accueil, compte connecté | Avant l'ouverture (lun. 5/10) |

Cadre : 390 × 844. Chaque fichier s'ouvre seul dans un navigateur, sans réseau sauf les polices Google (l'application, elle, les embarque).

**Thème unique.** L'application est sombre, toujours ; le choix Clair / Sombre / Système disparaît des réglages.

## Couleurs

| Rôle | Valeur | Usage dans la maquette |
|---|---|---|
| Fond | `#0B1018` | fond d'écran ; texte posé sur l'accent |
| Bloc | `#161D29` | carte, bouton rond, champ, ligne de lexique |
| Bloc relevé | `#1F2836` | piste du fil du jour, avatar, marque « EN », pastille d'icône, filet entre lignes |
| Barre d'onglets | `#11161F` | fond de la barre ; filet haut `#1F2836` |
| Filet | `#2A3445` | bord de champ, de pilule inactive, de bouton secondaire, axe de la frise |
| Neutre marqué | `#4A5870` | créneau d'un agenda dans le fil, nœud d'une session prévue |
| Texte | `#EEF2F7` | texte, titres, pictogrammes, repère « maintenant » du fil |
| Texte secondaire | `#9AA5B5` | méta, libellés d'onglets inactifs, graduations |
| Texte de lecture | `#C9D1DC` | définition, paragraphe |
| Accent | `#C8F169` | compte à rebours, onglet actif, jour choisi, bouton principal, « MAINTENANT », session de mon agenda |
| Attention | `#FFC857` | en cours, déplacée, salle changée |
| Alerte | `#FF8E72` | annulée, pastille de notification |

États absents de la maquette, dérivés à la même clarté (le cyan et le violet sont ceux des options d'accent) : information `#7FD8F5`, réseau / non annoncée `#C9A7FF`, terminée `#9AA5B5`, prévue `#EEF2F7`, succès = accent.

Un aplat d'accent, d'attention ou d'alerte porte toujours du texte `#0B1018`, en graisse 800.

## Polices

Deux familles, embarquées (aucune police ne se télécharge en salle) : **Sora** pour les titres et les chiffres qui portent le sens, **Manrope** pour tout le reste. Chiffres tabulaires.

| Usage | Famille | Taille / graisse | Interligne · approche |
|---|---|---|---|
| Compte à rebours (« 42 min », « 35 jours ») | Sora | 76 / 800 | 0,95 · −0,04em |
| Terme du lexique en français | Sora | 44 / 800 | 1,02 · −0,03em |
| Titre d'écran | Sora | 28 / 700 | 1,1 · −0,02em |
| Titre d'écran secondaire (avec retour) | Sora | 22 / 700 | |
| Titre de la session vedette | Sora | 20 / 600 | 1,25 |
| Titre de section | Sora | 18 / 700 | |
| Titre de bloc (« Le fil du jour ») | Sora | 16 / 700 | |
| Heure de la frise | Sora | 15 / 700 | |
| Initiales de l'avatar | Sora | 15 / 700 | |
| Texte de lecture | Manrope | 17 / 400 | 1,6 |
| Titre de ligne | Manrope | 16 / 700 | 1,3 |
| Texte courant, bouton | Manrope | 15 / 600–800 | 1,5 |
| Secondaire | Manrope | 14 / 400–600 | |
| Méta, étiquette | Manrope | 13 / 400–800 | |
| Libellé d'onglet | Manrope | 12 / 600, actif 800 | |
| « MAINTENANT · 10:48 » | Manrope | 12 / 800 | capitales · +0,04em |
| Marque « EN », graduation | Manrope | 11 / 800, graduation 400 | capitales · +0,08em |

## Mesures

- Marge d'écran **20 px** ; entre blocs **22 px** (accueil), 18 (sessions), 20 (lexique). Le haut de l'écran garde la zone sûre réelle, la maquette la figure par 60 px.
- Rayons : 24 bloc · 22 bouton rond de 44 · 20 carte de la frise · 18 champ · 16 bouton de 50–52, ligne de lexique · 14 pastille d'icône de 44 · 12 pastille de 36 · 6 marque « EN », piste et créneau du fil · pilule pleine (999) pour les jours et étiquettes.
- Cibles : bouton rond **44**, pilule de jour 44, champ 56, bouton principal 50–52, ligne 56 à 68, onglet 80 × 56.
- Barre d'onglets : **88 px** (zone de geste comprise), fond `#11161F`, filet haut 1 px `#1F2836`, pictogramme 24, libellé 12 ; l'onglet actif passe à l'accent, sans fond.
- Pictogrammes : trait de 2 px, bouts et angles arrondis, sur une grille de 24. Les dessins de la maquette font foi ; ceux qu'elle ne montre pas se redessinent dans le même trait.

## Composition

- **Accueil pendant la COP** : en-tête (avatar ou nom de l'application, date, jour de la COP, boutons ronds) · compte à rebours de la prochaine session · « Le fil du jour » : une piste par agenda (Négociation, Francophonie, Pavillon) sur 08 h – 20 h, ma session à l'accent, repère vertical « maintenant » · « Changements du jour » en lignes à pastille de couleur.
- **Sans compte** : le compte à rebours vise la prochaine session de toute la COP ; une carte bordée invite à créer un compte sans rien bloquer.
- **Avant la COP** : le compte à rebours compte les jours ; « Jusqu'à la COP » montre la piste jusqu'au dernier jour et les jalons ; « Se préparer » liste le parcours, les thématiques, les documents à emporter.
- **Sessions** : jours en pilules, ligne de contexte, frise verticale (heure, axe à nœuds colorés, carte), barre « MAINTENANT » à l'accent.
- **Lexique** : champ bordé, terme en anglais sous une marque « EN », traduction en grand à l'accent, définition, bouton favori plein et bouton secondaire, source ; derniers consultés en lignes-cartes.

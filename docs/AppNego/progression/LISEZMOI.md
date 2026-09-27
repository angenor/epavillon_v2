# progression/ — le suivi de Guide Négo en détail

Le point d'entrée reste [`../progress.md`](../progress.md), **le point central** : l'état de chaque étape en une phrase, les préalables, les points ouverts en une ligne, les dernières nouvelles. Il se lit en entier à chaque session. Ce dossier porte le détail — **on n'en ouvre que le fichier utile à la tâche du jour.**

Même esprit que le suivi de l'ePavillon ([`docs/PROGRESSION.md`](../../PROGRESSION.md) et `docs/progression/`), dont il reste séparé : une session Guide Négo n'écrit pas là-bas — [ADR-017](../adr/017-le-suivi-vit-dans-progress-md.md). Seule exception : une modification de `docs/database/` se consigne dans [`docs/progression/modele.md`](../../progression/modele.md), puisque le modèle est commun.

| Fichier | Ce qu'il porte | Quand l'ouvrir |
|---|---|---|
| [`etapes/<étape>.md`](etapes/) | Un fichier par étape de la [feuille de route](../04-roadmap.md) : l'état détaillé, les liens vers `specs/`, les décisions tranchées (avec la date et qui a tranché), les écarts, l'historique des phases, ce qui reste sur appareil réel | Avant de reprendre ou de corriger une étape |
| [`points-ouverts.md`](points-ouverts.md) | Le détail de chaque point ouvert, et les points levés, rayés | Avant de trancher ou de lever un point |
| [`journal/AAAA-MM-JJ.md`](journal/) | Un fichier par jour : ce qui a été fait, ce qui vient | Pour savoir ce qu'a fait la session précédente |

## En partant — quoi écrire où

1. La ligne de l'étape dans **État**, au point central : une phrase, et le lien vers le fichier de l'étape. Une étape nouvelle ouvre son fichier dans `etapes/`.
2. **Le fichier de l'étape** : l'état détaillé ; une ligne à l'**Historique** par phase livrée ; les décisions tranchées, datées, avec leur auteur ; les écarts ; ce qui reste sur appareil réel.
3. Une ligne au **journal** du jour, `journal/AAAA-MM-JJ.md` : ce qui a été fait, ce qui vient (« **À suivre** »). **Une ligne par phase, trois phrases au plus** ; le détail va dans le fichier de l'étape. Le fichier du jour se crée s'il n'existe pas.
4. Un **ADR** de plus si quelque chose a été tranché ; un point ouvert de moins s'il est résolu — rayé dans `points-ouverts.md`, sa ligne retirée du point central. Un point nouveau : son détail dans `points-ouverts.md`, une ligne et son lien au point central.
5. **Dernières nouvelles**, au point central : cinq faits d'une ligne, les plus récents ; le plus ancien sort quand un nouveau entre — il est déjà au journal.

## Les plafonds

- **Le point central : cent lignes et 15 Ko au plus.** Il ne garde que le chapeau, « État », « Préalables », « Points ouverts » et « Dernières nouvelles ». Rien d'autre.
- « État » : une phrase par étape. « Points ouverts » : une ligne par point. « Dernières nouvelles » : cinq faits d'une ligne.
- Le journal : une ligne par phase, trois phrases au plus.

## Avant le 27/09

Jusqu'au 27/09, `progress.md` portait tout : 180 lignes, 200 Ko. Son contenu a été **déplacé tel quel**, sans réécriture — les cellules « État » et « Note » de chaque étape dans `etapes/`, les points ouverts dans `points-ouverts.md`, le tableau du journal dans `journal/`, entrées dans l'ordre de l'ancien tableau (la plus récente en tête). Ces entrées dépassent souvent trois phrases : elles restent telles quelles. Une phrase datée de la ligne d'état d'une étape est aussi recopiée au journal de sa date, sous « Repris de la ligne d'état des étapes ». L'ancien fichier entier : `git show 6d16b49:docs/AppNego/progress.md`.

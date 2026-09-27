# ADR-017 — Le suivi vit dans `progress.md`

**Statut** : accepté — 18/09/2026

## Contexte

L'ePavillon tient sa mémoire dans `docs/PROGRESSION.md` et `docs/progression/` : journal du jour, décisions, fichier par écran. Guide Négo partage son dépôt, mais son suivi doit rester distinct.

## Décision

Toute la documentation de Guide Négo vit dans `docs/AppNego/`. Son avancement se tient dans **[progress.md](../progress.md)** ; ses décisions, dans `adr/`. Une session qui travaille sur Guide Négo **n'écrit ni dans `docs/PROGRESSION.md`, ni dans `docs/progression/`**.

## Conséquences

- La progression de l'ePavillon ne grossit pas d'un second projet.
- L'exception : une modification de `docs/database/` reste consignée dans `docs/progression/modele.md`, puisque le modèle est commun.
- Une session lit `progress.md` en arrivant et le met à jour en partant — son mode d'emploi est en bas du fichier.

## Complément — 26/09/2026

Le suivi est désormais **un point central et un dossier**, à la demande du commanditaire : `progress.md` pesait 200 Ko pour 180 lignes, et tout agent qui l'ouvrait payait ce poids. **Le nom du fichier ne change pas** : [progress.md](../progress.md) reste le point d'entrée, lu en entier à chaque session — cent lignes et 15 Ko au plus. Le détail vit dans [progression/](../progression/LISEZMOI.md) : un fichier par étape, les points ouverts, un journal par jour. Le mode d'emploi, qui était en bas de `progress.md`, est dans `progression/LISEZMOI.md`. La règle de séparation d'avec l'ePavillon ne change pas.

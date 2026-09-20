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

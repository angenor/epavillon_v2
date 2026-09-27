# Étape 4 — Réunions de la Francophonie

> Le détail de l'étape, déplacé tel quel de l'ancien suivi le 27/09. Son état en une phrase est au [point central](../../progress.md) ; ce qui s'est fait jour par jour, au [journal](../journal/).

**État** : 🟡 Close côté code, 26/09/2026 — phases 1 à 9 livrées, recette sur la version construite, `make check-safe` complet au vert (305 programmes de test) ; `main` (étape 2) fusionnée le 27/09, groupe « Réunions de la Francophonie » dans la recherche globale ; reste l'appareil réel, au § 15 de [DEPLOIEMENT.md](../../../DEPLOIEMENT.md)

## Références

[spec](../../../../specs/016-guide-nego-reunions/spec.md) · [plan](../../../../specs/016-guide-nego-reunions/plan.md) · [recherche](../../../../specs/016-guide-nego-reunions/research.md) · [modèle](../../../../specs/016-guide-nego-reunions/data-model.md) · [deux contrats](../../../../specs/016-guide-nego-reunions/contracts/) · [recette](../../../../specs/016-guide-nego-reunions/quickstart.md) · [tâches](../../../../specs/016-guide-nego-reunions/tasks.md)

## Cadre et décisions

- quatre questions tranchées le 26/09 (accès limité informatif ; lien visio aux seules inscrites ; la liste d'attente monte d'elle-même ; « Ma journée » : lignes Sessions et Réunions, ligne Pavillon vide pour l'étape 5).
- **Rien de doublé** : `meetings` et `meeting_registrations` complétées ; l'inscription se valide en base, sous verrou (capacité jamais dépassée, testé en concurrence) ; le lien au Pavillon n'est qu'une clé `xmod_fk_` ; garde du back-office globale (décision du 21/09).
- L'onglet est partagé : `GnSectionPavillon.vue` et `GnJourneeLignePavillon.vue` vides pour l'étape 5.
- Écarts 65 à 73

## Historique

- `main` (étape 2) fusionnée le 27/09 : groupe « Réunions de la Francophonie » dans la recherche globale, fil de la fiche en surtitre (écart 65 levé).

## Au journal

[26/09](../journal/2026-09-26.md) · [27/09](../journal/2026-09-27.md)

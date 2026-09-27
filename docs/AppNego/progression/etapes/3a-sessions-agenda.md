# Étape 3a — Sessions : l'agenda et son import

> Le détail de l'étape, déplacé tel quel de l'ancien suivi le 27/09. Son état en une phrase est au [point central](../../progress.md) ; ce qui s'est fait jour par jour, au [journal](../journal/).

**État** : 🟡 Close côté code, 25/09/2026 — phases 1 à 10 livrées, recette sur la version construite, `make check-safe` au vert ; reste l'appareil réel, au § 15 de [DEPLOIEMENT.md](../../../DEPLOIEMENT.md)

## Références

[spec](../../../../specs/014-guide-nego-sessions-agenda/spec.md) · [plan](../../../../specs/014-guide-nego-sessions-agenda/plan.md) · [recherche](../../../../specs/014-guide-nego-sessions-agenda/research.md) · [modèle](../../../../specs/014-guide-nego-sessions-agenda/data-model.md) · [trois contrats](../../../../specs/014-guide-nego-sessions-agenda/contracts/) · [recette](../../../../specs/014-guide-nego-sessions-agenda/quickstart.md) · [tâches](../../../../specs/014-guide-nego-sessions-agenda/tasks.md)

## Cadre et décisions

- cinq récits, quatre questions tranchées le 25/09 (groupes cochés sur « Mes thématiques », thématique par point de l'ordre du jour au back-office, pas de documents de Guide Négo, rappel dans l'application ouverte seulement).
- **La source réelle est lue** : le calendrier JSON de unfccc.int, sans état ni document, point de l'ordre du jour dans le titre, heures décalées d'une heure, derrière un anti-robot — **le lecteur réel attend l'accès ouvert par l'accord** ; tout s'éprouve sur 47 réunions réelles de la COP30.
- **L'import coupe l'affichage** au seuil ou quand la dernière réussite vieillit (worker arrêté) ; **les titres sont traduits par OpenRouter sans relecture** (écart 45, exception d'ADR-004).
- Plénières, consultations de la présidence et coordinations, sans point d'ordre du jour, passent toujours « Mes thématiques » : l'état vide n'apparaît presque jamais.

## Ce qui reste

- À la fusion de l'étape 2 : la feuille du lexique depuis la fiche, le fil « Sessions · type » au-dessus du titre

## Au journal

[25/09](../journal/2026-09-25.md)

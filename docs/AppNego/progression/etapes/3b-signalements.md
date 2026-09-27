# Étape 3b — Sessions : signalements et notifications

> Le détail de l'étape, déplacé tel quel de l'ancien suivi le 27/09. Son état en une phrase est au [point central](../../progress.md) ; ce qui s'est fait jour par jour, au [journal](../journal/).

**État** : 🟡 Close côté code, 26/09/2026 — phases 1 à 9 livrées, recette sur la version construite, contrôles : base, site, fmt et clippy au vert, tests de negotiation, engagement, kernel, api et worker au vert — **la suite Rust complète de `make check-safe` reste à passer avant la fusion** (arrêtée à la demande du commanditaire après 1 h 52, machine partagée) ; reste l'appareil réel, au § 15 de [DEPLOIEMENT.md](../../../DEPLOIEMENT.md)

## Références

[spec](../../../../specs/015-guide-nego-signalements/spec.md) · [plan](../../../../specs/015-guide-nego-signalements/plan.md) · [recherche](../../../../specs/015-guide-nego-signalements/research.md) · [modèle](../../../../specs/015-guide-nego-signalements/data-model.md) · [trois contrats](../../../../specs/015-guide-nego-signalements/contracts/) · [recette](../../../../specs/015-guide-nego-signalements/quickstart.md) · [tâches](../../../../specs/015-guide-nego-signalements/tasks.md)

## Cadre et décisions

- cinq récits ; trois questions tranchées le 25/09 (l'interrupteur « Notifications » d'« À propos » gouverne le courriel, allumé par défaut — l'écart 40 se referme pour lui seul ; une thématique élargit les notifications, éteinte par défaut ; une réunion non annoncée se retire d'un geste ou à la fin du jour).
- **Un signalement ne touche jamais la donnée officielle** : deux tables à part, un encart sans nom par-dessus, retiré quand l'import rattrape.
- **Valider puis annuler n'envoie jamais rien** : la publication, 30 s après, verrouille la ligne et ne publie qu'une fois (testé en concurrence).
- `negotiation` calcule les destinataires et compose l'avis, `engagement` l'écrit par une branche générique ; la cloche filtre par origine ; un courriel par personne et par tranche de dix minutes.
- Écarts 50 à 60

## Au journal

[25/09](../journal/2026-09-25.md) · [26/09](../journal/2026-09-26.md)

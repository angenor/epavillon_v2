# Étape 0b — Compte et admission

> Le détail de l'étape, déplacé tel quel de l'ancien suivi le 27/09. Son état en une phrase est au [point central](../../progress.md) ; ce qui s'est fait jour par jour, au [journal](../journal/).

**État** : 🟡 Recette faite sauf l'appareil réel, 22/09/2026

## Références

[spec](../../../../specs/009-guide-nego-compte-admission/spec.md) · [plan](../../../../specs/009-guide-nego-compte-admission/plan.md) · [recherche](../../../../specs/009-guide-nego-compte-admission/research.md) · [modèle](../../../../specs/009-guide-nego-compte-admission/data-model.md) · [trois contrats](../../../../specs/009-guide-nego-compte-admission/contracts/)

## Cadre et décisions

- cinq récits, 52 exigences, quatorze critères ; **le crate `negotiation` naît à cette étape**
- deux questions tranchées par le commanditaire : la portée d'un code se choisit à sa création (une COP, ou tout Guide Négo), et la réponse à une demande part par courriel.
- Rien n'existe en base pour les codes, les usages, les demandes, le réseau ni le type d'appareil ; la permission, le rôle et la portée existent déjà.

## Historique

- **Phases 1 à 7 livrées les 21 et 22/09** — 112 tâches sur 124. La base locale a été migrée **sans être détruite**, `negotiation` est monté dans l'API et le worker, et **les cinq récits sont entiers** : la session dit d'où elle vient et dure 90 jours glissants, le code d'invitation ouvre l'accès (neuf issues en 200, quota tenu par la base, essais comptés par personne), **l'IFDD tient ses codes** (créer, lister, révoquer, voir les usages, retirer un accès), **durcit l'admission sans redéployer** (trois modes, file des demandes, deux décisions, deux courriels), et **le verrou dit ce qu'il ferme** — `GnVerrou`, « Mon accès » dans ses cinq états.
- **Phase 8 déroulée le 22/09** — 122 tâches sur 124 : migration répétée sur une copie de la base et schémas recomparés, parcours mené au navigateur sur la **version construite**, 84 mesures d'écran, non-régression du site, `make check-safe` au vert API arrêtée.

## Ce qui reste

- **Reste T112**, qui ne se fait que sur un Android et un iPhone réels

## Au journal

[21/09](../journal/2026-09-21.md) · [22/09](../journal/2026-09-22.md)

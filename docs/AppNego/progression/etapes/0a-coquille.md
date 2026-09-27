# Étape 0a — Coquille et système de design

> Le détail de l'étape, déplacé tel quel de l'ancien suivi le 27/09. Son état en une phrase est au [point central](../../progress.md) ; ce qui s'est fait jour par jour, au [journal](../journal/).

**État** : 🟡 Recette faite sauf l'appareil réel, 21/09/2026

## Références

[spec](../../../../specs/008-guide-nego-coquille/spec.md) · [plan](../../../../specs/008-guide-nego-coquille/plan.md) · [tâches](../../../../specs/008-guide-nego-coquille/tasks.md) — 73 tâches, branche `008-guide-nego-coquille`

## Historique

- **72 livrées** : drapeau, fondations du design, garde des lectures, coquille installable, hors-connexion, thème, page des composants, recette.
- **T071 déroulée au navigateur le 21/09** — § 1, § 2 (hors installations) et § 3 vérifiés sur la version construite, **un défaut corrigé** (« Prête hors connexion » ne paraissait qu'à la deuxième ouverture).
- **Second défaut, trouvé et corrigé le 24/09 pendant l'étape 1b** : le manifeste de construction de Nuxt (`_nuxt/builds/meta/<id>.json`) n'était pas gardé — hors connexion, chaque navigation écrivait `NUXT_E5002` ; il est gardé, et `latest.json` se lit réseau d'abord, copie de la version ensuite.

## Ce qui reste

- **Reste sur appareil réel** : l'installation Android puis iPhone, et le débit bridé. La procédure d'ouverture est au § 14 de [DEPLOIEMENT.md](../../../DEPLOIEMENT.md)

## Au journal

[21/09](../journal/2026-09-21.md) · [24/09](../journal/2026-09-24.md)

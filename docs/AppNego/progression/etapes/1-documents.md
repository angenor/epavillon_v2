# Étape 1 — Documents

> Le détail de l'étape, déplacé tel quel de l'ancien suivi le 27/09. Son état en une phrase est au [point central](../../progress.md) ; ce qui s'est fait jour par jour, au [journal](../journal/).

**État** : 🟡 Close côté code, 24/09/2026 — phases 1 à 13 livrées ; restent la recette des réservés (T102) et l'appareil réel (T116), au § 15 de DEPLOIEMENT.md — **critère de sortie atteint sur poste** — l'essai rend l'issue A, les fichiers privés sont privés, le modèle est en base, le worker extrait, l'API sert les documents, le back-office publie le guide, le téléphone garde et efface les copies, la bibliothèque et la fiche se lisent sans compte et sans réseau, le guide se lit en salle sans réseau, avec sommaire, recherche et réglages, « Mes documents » rend la place d'un geste, et un expert corrige un passage sans republier

## Références

[tâches](../../../../specs/011-guide-nego-documents/tasks.md) · [spec](../../../../specs/011-guide-nego-documents/spec.md) · [plan](../../../../specs/011-guide-nego-documents/plan.md) · [recherche](../../../../specs/011-guide-nego-documents/research.md) · [essai](../../../../specs/011-guide-nego-documents/essai-extraction.md) · [modèle](../../../../specs/011-guide-nego-documents/data-model.md) · [quatre contrats](../../../../specs/011-guide-nego-documents/contracts/) · [recette](../../../../specs/011-guide-nego-documents/quickstart.md) · [ADR-021](../../adr/021-pdfium-dans-le-worker.md).

## Cadre et décisions

- **Relue par le commanditaire le 22/09** : téléchargement sans compte (écart 41), « Nouveau » = moins de sept jours et jamais ouvert, aperçu de chaque page et « ouvrir tel quel », triangle rouge (écart 20 élargi), « Marquer » non livré (écart 42), thématiques par `entity_terms`, rôle `expert`.

## Historique

- **Phase 1, l'essai, le 23/09** sur la branche `011-guide-nego-documents` : **PDFium tient les huit critères** sur le vrai guide, extraction complète en 1,8 s — pas de service Python. **Le téléphone garde tout**, tranché par le commanditaire : texte (74 Ko) et images des 19 pages à tableau ou figure (4,1 Mo, plus que le PDF). R7 ajusté, `origin` porte le texte de sa zone.
- **Phase 2 le 23/09** : bucket `epavillon-prive` fermé au web, la garde des documents ouverte, `MediaFileField`.
- **Phase 3 le 23/09** : le modèle migré sans destruction.
- **Phase 4 le 23/09** : le worker extrait, le vrai guide en 18,5 s.
- **Phase 5 le 23/09** : l'API, 25 routes.
- **Phase 6 le 23/09** : le back-office des documents, recette § 1 sur le vrai guide.
- **Phase 7 le 23/09** : la plomberie hors connexion, prouvée sans navigateur.
- **Phase 8 le 23/09** : la bibliothèque et la fiche.
- **Phase 9 le 23/09** : le lecteur, recette § 3 sur la version construite.
- **Phase 10 le 23/09** : lire confortablement.
- **Phase 11 le 23/09** : « Mes documents », la place et ce qui s'efface — sauf la recette des réservés, faute d'un compte avec l'accès.
- **Suite : phase 12**, la note de correction

## Au journal

[22/09](../journal/2026-09-22.md) · [23/09](../journal/2026-09-23.md) · [24/09](../journal/2026-09-24.md)

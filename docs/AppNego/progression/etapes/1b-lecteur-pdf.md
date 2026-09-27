# Étape 1b — Le lecteur montre le PDF d'origine

> Le détail de l'étape, déplacé tel quel de l'ancien suivi le 27/09. Son état en une phrase est au [point central](../../progress.md) ; ce qui s'est fait jour par jour, au [journal](../journal/).

**État** : 🟡 Close côté code, 25/09/2026 — phases 1 à 11 livrées, recette déroulée sur la version construite, `make check-safe` au vert ; reste T084 sur téléphones réels, liste à cocher au § 15.4 de [DEPLOIEMENT.md](../../../DEPLOIEMENT.md) ; **la branche n'est pas fusionnée dans `main`** — le commanditaire la fusionnera après les téléphones : **le guide se lit sur ses pages, en ligne et en mode avion ; la recherche marque le passage à sa place** (6 174 sur 6 174) ; **« Texte agrandi » se choisit dans la barre et se garde** ; **les notes de correction se signalent en marge de la page, à la hauteur de leur passage** ; FR-029 et le document sans texte vérifiés au navigateur ; [ADR-022](../../adr/022-pdfjs-dans-le-client.md), [grille](../../../../specs/012-guide-nego-lecteur-pdf/essai-lecteur.md). Reste T084 sur appareil réel

## Références

[tâches](../../../../specs/012-guide-nego-lecteur-pdf/tasks.md) · [spec](../../../../specs/012-guide-nego-lecteur-pdf/spec.md) · [plan](../../../../specs/012-guide-nego-lecteur-pdf/plan.md) · [recherche](../../../../specs/012-guide-nego-lecteur-pdf/research.md) · [essai](../../../../specs/012-guide-nego-lecteur-pdf/essai-lecteur.md) · [modèle](../../../../specs/012-guide-nego-lecteur-pdf/data-model.md) · [trois contrats](../../../../specs/012-guide-nego-lecteur-pdf/contracts/) · [recette](../../../../specs/012-guide-nego-lecteur-pdf/quickstart.md), branche `012-guide-nego-lecteur-pdf` partie de `011-guide-nego-documents`

## Cadre et décisions

- six récits, douze critères
- **trois hypothèses tranchées par le commanditaire le 24/09** — « Texte agrandi » remplace « ouvrir tel quel » et suit par défaut le verdict de l'extraction, le choix « Pages · Texte » prend la place de « Marquer » dans la barre dépliée (écarts 43 et 44), la copie gardée porte un numéro de format.
- **L'étape 1 ne se fusionne pas dans `main` avant ce cycle**

## Historique

- `main` fusionnée dans la branche, qui reçoit la règle des clés d'API.
- Décidé le 24/09 ; prompt dans [04-roadmap.md](../../04-roadmap.md).

## Au journal

[24/09](../journal/2026-09-24.md) · [25/09](../journal/2026-09-25.md)

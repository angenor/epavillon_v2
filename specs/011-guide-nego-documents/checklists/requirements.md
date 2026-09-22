# Specification Quality Checklist: Guide Négo — la bibliothèque de documents et le lecteur (étape 1)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-22
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- **Le marqueur de FR-037 est levé** : le texte du lecteur est tiré du PDF publié, et de lui seul — arbitré par le commanditaire le 22/09.
- **Noms de tables dans « Ce que le modèle ne porte pas encore »** : convention du dépôt, reprise des spécifications 008 à 010. La règle d'or (SQL d'abord) exige de nommer ce qui manque au modèle. Les récits, les exigences et les critères de succès, eux, n'en contiennent aucun.
- **Les notes de correction ne se posent que depuis le back-office**, comme le prompt le dit. La maquette 11-validation.html dessine aussi une pose côté application ; elle viendra avec l'étape qui construit cet écran.
- **Relecture du commanditaire, 22/09** : téléchargement sans compte (FR-025, écart 41, feuille de route corrigée) ; « Nouveau » = moins de sept jours et jamais ouvert sur ce téléphone (FR-018) ; aperçu de chaque page et repli « ouvrir tel quel » (FR-005, FR-005 bis) ; triangle rouge de la note (écart 20 élargi) ; thématiques par `entity_terms` et rôle `expert` tranchés. Tous les points repassent.

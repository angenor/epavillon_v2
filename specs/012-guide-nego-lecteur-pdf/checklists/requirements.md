# Specification Quality Checklist: Guide Négo — le lecteur montre le PDF d'origine (étape 1b)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-24
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

- Comme la spec 011, celle-ci nomme les fichiers SQL touchés et « l'API » qui tient un refus : c'est la convention du dépôt (constitution, principe I), pas un choix de technique. Le moyen de dessiner les pages — bibliothèque, rendu, positions du texte — est laissé au plan.
- Un choix que le prompt ne tranchait pas est posé en *Assumptions* et mérite l'œil du commanditaire en `/speckit-clarify` : l'administratrice peut retirer « Texte agrandi » d'un document (FR-022, FR-037), successeur plus étroit d'« ouvrir tel quel ».
- Le plus fort grossissement (« au moins quatre fois ») et les seuils de SC-002 et SC-003 sont des défauts raisonnables, à mesurer au plan sur le vrai guide.

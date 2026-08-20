# Specification Quality Checklist: Organisations (B2)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-20
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

**Sur le premier point de la première section.** Les 84 exigences ne nomment **aucune** table, colonne
ni contrainte : elles énoncent des règles (« la direction de l'attente », « l'unicité du nom sur les
fiches vivantes »). Les noms du modèle vivent dans la section des entités et dans les vérifications, où
ils ont leur place — c'est la règle de `CLAUDE.md` sur les questions posées en mots simples, appliquée
au corps normatif de la spécification. Vérifié mécaniquement sur la section « Requirements ».

**Deux réserves assumées, qui ne bloquent pas la planification.**

- **SC-002 dépend d'un jeu de 5 000 organisations qui n'existe pas encore.** Le semis appartient au
  plan, pas à la spécification ; sans lui, le critère n'est pas vérifiable.
- **FR-041 (le départ du dernier référent) tient une option par défaut, pas un arbitrage.** La question
  est posée au commanditaire et inscrite dans `docs/progression/points-bloques.md`. L'option retenue est
  celle qui ne perd rien ; la trancher autrement ne coûterait qu'une exigence.

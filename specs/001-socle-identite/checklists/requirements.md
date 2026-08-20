# Specification Quality Checklist: Socle technique et Identité (B1)

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

- **Une seule question a été posée au commanditaire**, et elle est tranchée : le second facteur reste un
  emplacement réservé (FR-026, et la première ligne des hypothèses). Aucun marqueur
  `[NEEDS CLARIFICATION]` ne subsiste.

- **Deux écarts au critère « pas de détail d'implémentation », assumés et bornés.**
  1. Les **noms de tables et de colonnes** apparaissent dans « Key Entities » et dans quelques
     exigences. Ce ne sont pas des choix d'implémentation : `docs/database/*.sql` est la source de
     vérité du projet (règle d'or de `CLAUDE.md`, principe I de la constitution), et une exigence qui
     paraphraserait un nom de colonne au lieu de le citer rouvrirait précisément la porte que cette
     règle ferme.
  2. **Argon2id** est nommé en FR-022. C'est une exigence du commanditaire, reprise du prompt B1, et
     `COMMENT ON COLUMN accounts.password_hash` la porte déjà en base.

  Les **chemins d'API** et les **noms de champs de réponse**, eux, ne sont volontairement **pas**
  énumérés ici : ils vivent dans `frontend/app/composables/useApi.ts` et `frontend/app/types/`, qui en
  sont la seule source. FR-062 les y renvoie plutôt que d'en tenir une copie qui divergerait.

- **La pile technique n'apparaît nulle part dans les exigences.** Elle est imposée par la constitution
  (§ Contraintes techniques) et n'a pas à être rediscutée au niveau de la spécification. Le contexte en
  tête de document la mentionne une fois, pour situer le lecteur.

- Prêt pour `/speckit-plan`. `/speckit-clarify` n'est pas nécessaire — la seule ambiguïté de périmètre
  a été levée en séance.

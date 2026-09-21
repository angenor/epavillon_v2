# Specification Quality Checklist: Guide Négo — compte et admission (étape 0b)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-21
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

## Relecture du commanditaire — 21/09/2026

Quatre corrections appliquées après lecture du plan. Elles ont **augmenté** le périmètre ; la
spécification les porte :

- [x] **La base ne se détruit pas** — `migration.sql` livré, rejouable ; `down -v`, `make check` et
      `make check-db` proscrits ; contrôle par comparaison de schémas (§ 13 de `DEPLOIEMENT.md`)
- [x] **Durée de session de l'application** — 90 jours glissants, d'office (FR-006 bis, ter ; SC-012)
- [x] **Les liens des courriels ramènent dans l'application** — trois écrans de plus, écart 34 inscrit
      à `05-design.md` (FR-004 bis, ter ; SC-013)
- [x] **Deux verrous en base** — essais comptés **par personne** (FR-017, SC-014) et quota tenu par
      `ck_invitation_codes_quota` (FR-038 bis)

## Notes

- **Les deux questions posées au commanditaire ont été tranchées le 21/09/2026**, et la spécification les porte :
  1. *La portée d'un accès* — l'administrateur choisit **à la création de chaque code** s'il ouvre une COP précise ou Guide Négo en entier (FR-012, FR-012 bis, FR-036 ; défaut proposé : l'édition en cours).
  2. *La réponse à une demande* — elle part **par courriel**, à l'admission comme au refus (FR-025, FR-028, SC-011). Écart assumé avec l'écran d'attente de la maquette, qui annonce une notification : à inscrire aux écarts de `05-design.md`.
- La section « Ce que le modèle ne porte pas encore » liste ce qui s'ajoute à `docs/database/` avant tout code, conformément à la règle d'or du projet.

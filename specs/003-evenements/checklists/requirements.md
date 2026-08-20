# Specification Quality Checklist: Événements (B3)

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

**Sur le premier point de la première section.** Les 99 exigences ne nomment **aucune** table, colonne,
contrainte ni fichier : elles énoncent des règles (« une édition dont le pavillon est tenu porte un
sigle », « un seul canal par défaut », « le seul contrôle bloquant est celui de la publication »). Les
noms du modèle vivent dans la section des entités et dans les vérifications, où ils ont leur place —
c'est la règle de `CLAUDE.md` sur les questions posées en mots simples, appliquée au corps normatif.
Vérifié mécaniquement sur la section « Requirements » : aucune occurrence d'identifiant technique.

**Quatre réserves assumées, qui ne bloquent pas la planification.**

- **L'écart n° 9 tient l'option A d'un arbitrage encore en attente** (question n° 9 des points bloqués,
  posée le 16/08). Le prompt la retient explicitement. Trancher autrement ne coûterait que FR-027 à
  FR-030, toutes portées par le service et aucune par le modèle.
- **La granularité du calendrier d'une série de webinaires reste à arbitrer** (écart n° 2 d'A10). La
  spécification ne code aucune règle implicite : le plan annonce le nombre de journées avant d'écrire, et
  rien ne s'écrit sans geste explicite. C'est le comportement le moins engageant tant que la réponse
  manque.
- **US8 (publication de la programmation) ne se démontre pleinement qu'après B5.** Le contrôle préalable,
  le refus bloquant et la date posée sur l'édition sont vérifiables dès ce jalon ; le nombre de séances
  rendues publiques ne l'est qu'une fois les séances existantes. D'où sa priorité P3.
- **FR-078 et FR-079 décrivent un effet qui traverse deux périmètres de données.** La frontière retenue —
  annonce par événement de domaine, consommation par le module Programmation — est la seule qui respecte
  le principe de frontière ; le mécanisme exact (transaction, garde de rejeu, forme de l'événement) est
  une décision de plan, pas de spécification.

**Une divergence relevée avec la documentation existante, corrigée dans la spécification.** L'écart n° 5
d'A10 supposait qu'un chargé de programmation pouvait composer les journées spéciales. Vérification faite
dans le semis des rôles, ce rôle ne détient **aucune** permission du module des événements. La
spécification ne corrige pas le semis — ce serait modifier le modèle — et consigne l'écart.

# Specification Quality Checklist: Guide Négo — thématiques, « Ma journée » et profil (étape 0c)

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

**Les deux questions ont été tranchées par le commanditaire le 22/09**, et la spécification réécrite en conséquence :

1. **Les thématiques de négociation sont un vocabulaire à elles** — `negotiation_theme`, semé des dix
   thématiques de la maquette, comme `activity_theme` l'est pour les activités du Pavillon. Les sessions
   (3a) et les alertes (3b) s'y rattacheront ; le Pavillon n'est jamais filtré par « mes thématiques ».
   Aucun écran d'administration des vocabulaires : un terme s'ajoute par le SQL, comme sur le site.
2. **Les textes qui engagent ont une source unique** — un fichier `fr`/`en` par texte, portant sa version,
   embarqué dans l'API et servi par une route publique pour le site comme pour l'application ; la version
   servie remplace le réglage `PRIVACY_POLICY_VERSION`, et un contrôle refuse un texte modifié sans
   nouvelle version. Un éditeur pourra venir plus tard derrière la même route.

**Trois divergences avec la maquette, assumées et inscrites** :

- Les thématiques suivent le compte, là où l'écran « 12 À propos » écrit qu'elles restent sur le téléphone.
- Les trois interrupteurs de consentement ne sont pas livrés — aucun n'a d'effet à cette étape, et un
  interrupteur sans effet trompe (constitution, XII). **Écart 40 de [05-design.md](../../../docs/AppNego/05-design.md).**
- La cloche des notifications et le groupe « Notifications par thématique » attendent l'étape 3b.

**Un point laissé au plan, délibérément** : où vit le lien entre une personne et ses thématiques —
les spécialisations déjà portées par le profil de négociateur, ou une table propre. Une vérité, pas deux ;
le plan tranche et le justifie.

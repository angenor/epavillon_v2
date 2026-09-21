# Specification Quality Checklist: Guide Négo — coquille et système de design (étape 0a)

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

## Notes

- La spécification cite des fichiers du dépôt (maquette, dossier de passation, icône, nom proposé du drapeau) : ce sont les références imposées par le prompt de l'étape, pas des choix d'implémentation. Le service worker, IndexedDB, le manifeste et l'arborescence du client sont laissés au plan.
- Révisée le 21/09/2026 après relecture du commanditaire : quatre hypothèses confirmées ; cinquième onglet porté par `negotiation.channels`, sans nouveau drapeau ; thème dans « Profil et réglages », rien de provisoire ; FR-028 réduite aux composants de la coquille et à ceux que toute étape emploie, fondations entières ; ajouts sur le drapeau (panne d'API ≠ éteint, allumé = activé et 100 %) et sur l'en-tête (ni notifications ni avatar). Les seize points revalidés, aucun marqueur de clarification ; `/speckit-clarify` écarté par le commanditaire.
- Révisée une seconde fois le 21/09/2026, après relecture du plan : l'interface reste en français quel que soit le téléphone, sans toucher au cookie de langue du site (cas limite, FR-034) ; ouverture sur la version gardée sous réseau lent, mise à jour en arrière-plan, arrêt par le drapeau indépendant (récit 2 scénarios 8 et 9, FR-016 bis, SC-002). Les seize points revalidés.
- Révisée le 21/09/2026 après `/speckit-analyze` : FR-012 bis (« Prête hors connexion »), FR-007 et cas limite (la barre seule défile ; 320 px), FR-014 et deux cas limites (heure du téléphone sans fuseau ; « hier à », « le 11 nov. à » — écart 32), cas limite de la page d'installation réduit à l'observable, SC-002 précisé. Les seize points revalidés.

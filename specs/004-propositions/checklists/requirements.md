# Specification Quality Checklist: Propositions (B4)

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

**Sur le premier point de la première section.** Les 103 exigences ne nomment **aucune** table, colonne,
contrainte, fonction ni fichier : elles énoncent des règles (« le dossier est créé en brouillon, quel que
soit l'état demandé », « une demande de correction est forcée en visibilité partagée », « le renvoi n'est
pas soumis à la fenêtre de l'appel »). Les noms du modèle vivent dans le contexte, dans la section des
entités, dans le tableau des frontières et dans celui des écarts, où ils ont leur place. Vérifié
mécaniquement sur la section « Requirements » : aucune occurrence d'identifiant technique.

**Cinq réserves assumées, qui ne bloquent pas la planification.**

- **La question n° 8 des points bloqués n'a jamais reçu de réponse** (posée le 16/08 : le déposant
  voit-il sa note et son rang ?). L'écran A5 a tenu l'option A — état, corrections et décision seulement
  — et la spécification la reprend (FR-077). Trancher autrement n'ouvrirait que deux champs de la
  composition de l'espace organisation ; rien d'autre ne bouge.
- **La question ouverte par l'écart n° 35 reste ouverte** : une résolution posée par le déposant vaut-elle
  clôture pour le comité, ou seulement déclaration ? La spécification retient la **déclaration**, le comité
  conservant la faculté de la retirer (FR-081, FR-082). C'est la lecture la moins engageante : elle ne
  retire rien au comité et n'oblige le déposant à rien.
- **L'écart n° 30 tient une règle par défaut, pas un arbitrage** : le contact du dossier vaut le déposant
  (FR-032), en attendant que le commanditaire dise s'il faut le demander à l'étape des organisations.
- **La création d'une personne inconnue écrit hors du périmètre de données du module** (intervenant dont
  l'adresse n'est pas connue). La spécification pose l'exigence (FR-025, FR-026) et **laisse la frontière
  au plan** : dérogation bornée, ou contrat d'événement — la seconde voie interdisant la création
  synchrone dont le formulaire a besoin. Le dire ici évite de le subir en écrivant le code.
- **US5 et US7 ne se démontrent complètement qu'avec B5 et B6.** Les séances programmées et leurs
  décomptes viennent de B5 ; le téléversement d'une pièce vient de B6. Ce qui est vérifiable dès ce jalon
  — la composition, le filtre du fil, le rattachement d'un objet déjà stocké — l'est entièrement, d'où les
  priorités P2.

**Huit écarts nouveaux, tous vérifiés dans le SQL et non supposés.** Deux d'entre eux auraient cassé à
l'exécution dès le premier appel : l'adresse d'URL obligatoire que le formulaire ne porte pas (n° 95, qui
fait échouer le tout premier enregistrement d'un brouillon), et l'absence de tout appel à la consolidation
des notes (n° 98, qui laisse le classement du comité figé sans qu'aucune erreur ne le signale). Un
troisième aurait produit des courriels en double en production : le déclencheur d'état émet déjà
l'événement de domaine (n° 93) — c'est **l'inverse** de ce que B3 avait constaté sur son propre module, et
c'est pourquoi il fallait le vérifier plutôt que de le déduire du précédent.

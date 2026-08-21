# Specification Quality Checklist: Sessions (B5)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-21
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

**Sur le premier point de la première section.** Les 114 exigences et les 29 critères ne nomment **aucune**
table, colonne, contrainte, fonction ni fichier — vérifié mécaniquement sur les deux sections : zéro
identifiant technique. Elles énoncent des règles (« une salle nulle renvoie la séance au panneau, sans
rien supprimer », « la promotion depuis la liste d'attente porte exactement le nombre de places
libérées »). Les noms du modèle vivent dans le contexte, la section des entités, le tableau des
frontières et celui des écarts, où ils ont leur place.

**Une seule question au commanditaire, et elle ne bloque pas la planification.** Que devient la séance
d'un dossier annulé après acceptation (écart n° 123) ? L'option tenue provisoirement — annulation d'office
avec le même motif — est la moins silencieuse, et en changer ne toucherait qu'un seul embranchement du
service. La question est posée en mots simples aux points bloqués.

**Quatre réserves assumées.**

- **La forme d'une réponse « pays » est fixée par cette spécification** (écart n° 11, ouvert depuis le
  16/08 et explicitement renvoyé à ce prompt) : le code ISO à deux lettres, comme les données simulées.
  Le commanditaire n'a rien à trancher là — c'est une décision d'API —, mais la fixer maintenant est ce
  qui empêche un export mêlant deux formes.
- **La preuve du consentement à une réponse sensible s'écrit hors du périmètre de données du module.**
  La spécification pose l'exigence (FR-079 à FR-081) et **laisse la frontière au plan** : dérogation
  bornée, ou contrat d'événement — la seconde voie interdisant de refuser l'inscription faute de
  consentement, puisque la preuve serait écrite après coup. Le dire ici évite de le subir en écrivant le
  code.
- **US6 ne se démontre qu'avec le module Événements en fonctionnement.** La publication est une chaîne à
  deux modules : B3 contrôle, estampille et annonce ; B5 reçoit et rend publiques les séances désignées.
  Le test de bout en bout qui compare l'annonce à l'effet (SC-016) est le seul qui prouve la chaîne
  entière, et il est explicitement demandé aux points bloqués depuis le 20/08.
- **Cinq parties du fichier `075` ne sont pas livrées par ce jalon** et le disent : les questions du
  public, l'annulation et le report d'une séance, la création d'une séance sans dossier, l'écriture du
  compte rendu, et les rappels — ces derniers appartenant à B6. Aucune n'est nommée par le prompt, aucune
  n'a d'écran, et le périmètre le dit plutôt que de le laisser deviner.

**Treize écarts nouveaux, n° 111 à 123, tous vérifiés dans le SQL et non supposés.** Trois auraient
produit un défaut silencieux dès le premier usage réel :

- **n° 113** — la journée de rattachement ne se recalcule pas quand on déplace une séance, alors que
  déplacer est le geste le plus fréquent de tout l'écran. La séance serait rangée au mauvais jour, sans
  qu'aucune erreur ne le signale.
- **n° 114** — le déclencheur d'inscription ne valide **rien** lorsque la séance ne porte pas de
  formulaire attaché, alors que le formulaire applicable peut venir de l'édition ou de la plateforme.
  Une inscription sans aucune réponse obligatoire passerait.
- **n° 116** — la promotion depuis la liste d'attente ne vérifie pas la jauge, le contrôle de capacité ne
  portant que sur l'insertion. Le dépassement ne se verrait que le jour de l'activité.

**Et un quatrième qui corrige la consigne elle-même** : l'écart n° 7, recopié dans le prompt, demande de
refuser le canal de diffusion à l'écriture. C'est **faux** — le déclencheur ne le pose que lorsqu'il est
nul, et n'écrase jamais un canal choisi. Le refuser aurait cassé une fonctionnalité livrée du
planificateur. La consigne est traitée dans son intention, pas à la lettre (écart n° 111).

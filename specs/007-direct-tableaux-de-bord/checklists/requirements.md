# Specification Quality Checklist: Direct + Tableaux de bord (B9)

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-27
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

**6 histoires, 12 cas limites, 71 exigences, 14 critères, 4 décisions tranchées, aucun marqueur de
clarification.**

**Deux exigences ont été amendées le 27/08, en écrivant le plan, et les amendements sont datés dans le
fichier.** FR-018 : le tableau de bord se garde par `analytics.dashboard.read` — le modèle porte cette
permission et `GET /health` la teste depuis B1, mais elle n'est pas attribuée au rôle qui pilote une
édition ([research.md](../research.md) R10). FR-057 et FR-059 : **la page publique d'une activité
n'existe pas**, l'exposition du bandeau se fait donc sur la page des programmations, à l'échelle de
l'édition (R26). Les deux se sont vues en confrontant la spécification au dépôt, pas à l'exécution —
c'est ce que la phase de plan existe pour produire.

**Sur le premier point de la première section, la même réserve qu'en B6, et pour les mêmes raisons.**
Relevé mécaniquement, les seuls identifiants techniques des sections « Requirements » et « Success
Criteria » sont : les **huit chemins de routes** et leurs verbes, **deux noms de schéma**, **un chemin de
dossier**, **un code de permission**, **trois valeurs de portée** et **deux cibles du `Makefile`.**

Chacun y est parce qu'il **est** le livrable ou la contrainte, et non parce que la règle a été écrite en
jargon :

- Les huit routes sont nommées par le prompt, écrites par le site depuis les 17 et 18/08, et **comptées
  une par une** par la vérification du contrat. Une exigence qui les décrirait sans les nommer ne serait
  pas vérifiable.
- Les deux schémas et le dossier de lectures hors schéma sont une contrainte de **structure** : un module
  = un schéma = un crate, et l'absence d'arête entre deux modules est une vérification mécanique
  bloquante du principe II. L'écrire ailleurs qu'en exigence reviendrait à ne pas la tenir.
- Le code de permission est nommé parce que le principe V impose de tester une **permission** et jamais un
  rôle : c'est le nom qui est la règle.
- Les trois valeurs de portée sont nommées dans une exigence dont le sujet **est** qu'elles n'ont aucune
  colonne d'édition, et qu'un filtre écrit à la main les laisserait fuir.

Partout ailleurs, les exigences énoncent des règles sans nommer une table, une colonne, une contrainte ni
une fonction : « le rattachement d'un message à une édition passe par la fonction du modèle qui le
calcule », « une famille sans élément ne produit pas de ligne », « un indicateur dont la donnée n'existe
pas est nul, jamais zéro ».

**Quatre points étaient laissés à trancher par le prompt ; les quatre le sont, dans la spécification et
non en chemin** — section « Décisions tranchées » : où vit la composition du tableau de bord et pourquoi
elle ne viole pas la frontière (D1), où vit le seuil qui rend un dossier urgent (D2), si une portée
globale se retire depuis une édition (D3), et si l'exposition publique entre dans ce jalon (D4).

**Une seule question au commanditaire, posée en mots simples, et elle ne bloque pas la planification.**
Quand un message d'entretien s'affiche sur tout le site, l'équipe d'une seule COP doit-elle pouvoir
l'enlever elle-même, ou faut-il passer par l'équipe centrale ? La position tenue — passer par l'équipe
centrale — est la **réversible** : l'ouvrir plus tard ne casse rien, la fermer plus tard casserait une
habitude prise. En changer ne toucherait qu'un embranchement du contrôle d'autorisation.

**Deux modifications du modèle après le plan, et aucune n'est un changement de schéma** — ni table, ni
colonne, ni type, ni fonction. Une ligne de réglage pour le seuil de l'écart n° 43, ouvert depuis le
17/08 et aujourd'hui écrit dans le code du site, ce que le principe I interdit ; et une attribution de
permission, sans laquelle le tableau de bord serait refusé au rôle qui pilote une édition. La
spécification n'en annonçait qu'une : la seconde vient du plan (R10).

**Le prompt demande une chose déjà faite, et la spécification le dit plutôt que de la refaire.** La
lecture isolée de la santé opérationnelle, que le site appelle en dehors de la composition pour
rafraîchir la seule zone qui se rafraîchit sans recharger le reste, **est servie depuis B1**. Elle n'est
ni refaite ni comptée dans les huit routes.

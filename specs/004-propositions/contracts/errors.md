# Contrat — Erreurs

**Fonctionnalité** : Propositions (B4) · **Date** : 2026-08-20

> Le catalogue vit dans `backend/crates/kernel/src/error.rs` et l'OpenAPI est **engendrée** depuis lui. Ce fichier dit ce que B4 y ajoute, et comment les refus de PostgreSQL se traduisent.

---

## La règle, inchangée depuis B1

Un refus **exprimé par le contrat du front** sort en **200**, avec son discriminant. Un refus **non exprimé** sort en erreur HTTP, avec un code stable et un message français.

Ce module est celui où la première ligne l'emporte le plus largement. Sept refus métier sont déjà des membres d'union côté front :

| Refus | Où il vit dans le contrat | Statut |
|---|---|---|
| Appel clos | `SubmitProposalResult` — `{ status: 'call_closed', deadline }` | 200 |
| Plafond atteint | `SubmitProposalResult` — `{ status: 'quota_reached', max }` | 200 |
| Transition impossible | `DecisionResult` — `{ status: 'transition_not_allowed' }` | 200 |
| Motif exigé | `DecisionResult` — `{ status: 'reason_required' }` | 200 |
| Dossier déjà confié | `BulkSkip` — `already_assigned` | 200 |
| Membre déporté | `BulkSkip` — `recused` | 200 |
| Dossier introuvable **dans une action groupée** | `BulkSkip` — `not_found` | 200 |

**Un dossier introuvable hors action groupée reste un 404** : la nuance n'est pas cosmétique. Dans une sélection de douze, l'introuvable est un **écart** qu'on montre à côté des onze autres ; seul, c'est une ressource qui n'existe pas.

---

## Les six codes ajoutés au catalogue

| Code | Statut | Quand | Ce que l'écran en fait |
|---|---|---|---|
| `PROPOSAL_NOT_EDITABLE` | 422 | dossier rejeté, retiré, annulé, ou édition terminée | affiche pourquoi la modification est close, et propose de déposer un nouveau dossier |
| `PROPOSAL_SPEAKER_IDENTITY_LOCKED` | 422 | tentative de modifier l'identité d'une personne qui possède un compte | reverrouille le champ et nomme la personne |
| `PROPOSAL_REVIEW_NOT_ASSIGNED` | 403 | noter sans affectation, ou après un déport | masque la grille, laisse la lecture |
| `PROPOSAL_UNKNOWN_TERM` | 422 | un code de thématique hors de la taxonomie attendue | nomme le code refusé |
| `PROPOSAL_TEXT_TOO_LONG` | 422 | un texte au-delà de sa borne | nomme le champ et la limite |
| `PROPOSAL_UNKNOWN_REFERENCE` | 422 | édition, appel, organisation, personne, critère ou objet stocké inconnu | nomme le champ, comme `ORG_*` et `EVENT_*` le font déjà |

**Six, et pas plus** : tout le reste est déjà exprimé par le contrat, ou couvert par les codes du noyau — `NOT_FOUND`, `FORBIDDEN`, `FORBIDDEN_SCOPE`, `VALIDATION_FAILED`, `UNAUTHENTICATED`.

**Aucun code n'est ajouté pour la recevabilité** : ses trois refus sont des réponses, pas des erreurs.

---

## Traduction des refus de PostgreSQL

| SQLSTATE | Contrainte ou origine | Traduction |
|---|---|---|
| `23001` | déclencheur de la machine à états | `DecisionResult.transition_not_allowed` (200), **message français du déclencheur repris mot pour mot** par l'outil du noyau |
| `23001` | déclencheur de recevabilité | **ne devrait pas remonter** : les trois causes sont classées avant (R9). S'il remonte, c'est une course → `transition_not_allowed` avec son message |
| `23502` | déclencheur de la machine à états, motif manquant | `DecisionResult.reason_required` (200). Sûr : la transaction n'écrit que deux colonnes nullables (R8) |
| `23505` · `ux_proposals_slug` | adresse d'URL déjà prise dans l'édition | **rattrapé, jamais rendu** : le service réessaie avec un suffixe (R5) |
| `23505` · `ux_proposal_speakers` | même personne, même rôle, deux fois | 422 `VALIDATION_FAILED` sur le champ des intervenants, en nommant la personne |
| `23505` · `ux_reviews` | deux revues d'une même personne | **rattrapé** : l'écriture est un `INSERT … ON CONFLICT DO UPDATE`, une personne n'a qu'une revue |
| `23505` · clé de `proposal_organizations` | organisation déjà associée | 422, en nommant l'organisation |
| `23514` · déclencheur de plafond de note | note supérieure au maximum du critère | 422, **en nommant le critère et sa borne** |
| `23514` · `ck_proposals_preferred_period` | fin avant début | 422 sur le champ du créneau |
| `23514` · `proposals_duration_minutes_check` | durée hors 15–600 | 422 — mais les bornes **de l'appel** sont plus serrées et refusent avant |
| `23514` · `ck_proposals_submitted_at` | dépôt sans date | ne peut pas remonter : la date est posée par le déclencheur |
| `23503` · toute clé `xmod_fk_*` | référence inconnue | `PROPOSAL_UNKNOWN_REFERENCE`, en nommant le champ d'après le nom de la contrainte |
| `23514` · domaine `i18n_text` | français manquant sur un texte multilingue | 422, champ déduit du **nom du domaine** (`violated_domain` du noyau) |
| `22001` | dépassement de longueur en base | ne peut pas remonter : les bornes de l'API sont plus serrées (R15) |

**Ce que le tableau interdit** : reconnaître une cause au **texte** d'un message. Trois messages français, dont deux interpolent des valeurs, changeraient à la première reformulation du SQL (R8).

---

## Ce qu'aucune erreur ne divulgue

Un dossier **hors périmètre** se refuse exactement comme un dossier inexistant : même code, même forme, même absence de détail. Le principe IX l'exige, et la résolution d'ascendance (R13) est écrite pour que le contrôle passe **avant** que quoi que ce soit ne soit rendu.

Un message de visibilité « comité » ou « privée » n'apparaît dans **aucune** erreur : les refus de résolution nomment l'identifiant du message, jamais son corps.

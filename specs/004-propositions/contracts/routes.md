# Contrat — Routes

**Fonctionnalité** : Propositions (B4) · **Date** : 2026-08-20

> **Ce document ne définit aucune forme de réponse.** Les formes vivent dans `frontend/app/types/` et n'y ont qu'une seule source. On indique ici, pour chaque route : le verbe, le chemin, l'autorisation exigée, le type de requête et de réponse **par son nom TypeScript**, et la politique de statut.
>
> La documentation OpenAPI est **engendrée depuis le code**. Ce fichier est la carte, pas la documentation.

---

## Préfixe, transport et politique de statut

Rien ne change depuis B1, B2 et B3 : préfixe `/api`, `Accept-Language` sur chaque requête, `X-Request-Id` sur chaque réponse, session par cookies, vérification de l'origine sur toute écriture, en-têtes CORS posés. Les chemins sont donnés **tels que le front les écrit**, sans le préfixe.

**La règle de statut est celle de B1** :

| Le refus est… | Réponse |
|---|---|
| **exprimé par le contrat** comme membre d'union ou champ de résultat | **200**, avec son discriminant |
| **non exprimé** par le contrat | statut d'erreur HTTP + corps d'erreur ([`errors.md`](errors.md)) |

**Ce module penche fortement vers la première ligne**, plus encore que B3. `SubmitProposalResult` porte ses deux refus (`call_closed`, `quota_reached`), `DecisionResult` porte les siens (`transition_not_allowed`, `reason_required`), et `BulkResult` porte ses écarts dossier par dossier. Les refus de la machine à états et de la recevabilité sortent donc en **200**.

**Les paramètres que le front passe encore et que l'API ignore** : `actorId`, le périmètre d'administration passé en argument, et `organization_id` dans les corps. **L'API lit sa propre session et remonte elle-même à l'édition** (R13). Ils disparaîtront au raccordement (B7). Motif éprouvé en B1, B2 et B3.

---

## Les permissions consommées

| Permission | Ce qu'elle garde ici | Portée exigée |
|---|---|---|
| `programme.proposal.submit` | déposer, renvoyer, retirer — côté organisation | aucune portée : c'est le droit du membre d'organisation, et **l'adhésion active** est le vrai contrôle |
| `programme.proposal.read_all` | la liste du back-office, la fiche, l'historique, les pièces | l'édition visée, ou globale |
| `programme.review.write` | noter, se déporter, écrire un message, demander des corrections | l'édition visée |
| `programme.proposal.decide` | retenir, rejeter, remettre en évaluation, annuler | l'édition visée |
| `event.call.manage` | **affecter** un membre du comité — écart n° 48, tranché par A7 | l'édition visée |

Détenir l'une n'accorde aucune autre : les combinaisons sont testées. **Le rôle d'administration ne détient pas `programme.review.write` par défaut** : il ne peut donc pas demander de corrections. C'est l'écart n° 50, tranché en A8 — une ligne de la table des droits, pas une fatalité, et l'écran de gestion des rôles permet de l'accorder sans toucher au code.

`event.call.manage` vient d'un autre module ; comme en B3, le garde vit dans le noyau et **aucune arête entre crates n'en découle**.

---

## Deux préfixes partagés, composés dans `api`

`/people` et `/organizations` sont déjà occupés (R18). Deux `web::scope` du même préfixe **ne se complètent pas** : Actix retient le premier et rend 404 sur les routes du second — défaut déjà payé en B2, trois routes muettes sur vingt et une.

| Préfixe | Contributeurs après B4 | État |
|---|---|---|
| `/people` | identité, organisations, **propositions** | composé dans `api` depuis B1 — on s'y ajoute |
| `/organizations` | organisations, **propositions** | **à refactoriser** sur le même patron : `org` expose ses routes de ce préfixe séparément, `api` compose une seule fois |
| `/admin/planner` | événements, puis B5 | composé dans `api` depuis B3 — ce module n'y touche pas |

---

## Côté organisation — le dépôt (US1, US6)

| # | Verbe | Chemin | Autorisation | Requête → Réponse |
|---|---|---|---|---|
| 1 | GET | `/proposals/form-context` | session, adhésion active | `?organization_ids` → `ProposalFormContext` |
| 2 | GET | `/proposals/draft` | session | → brouillon en cours, ou `null` |
| 3 | POST | `/proposals` | `programme.proposal.submit`, adhésion active à l'organisation porteuse | `SaveDraftPayload` → `SaveDraftResult` |
| 4 | PUT | `/proposals/{id}` | idem, **et** le dossier appartient à l'organisation | `SaveDraftPayload` → `SaveDraftResult` |
| 5 | GET | `/proposals/{id}/draft` | idem | → `EditableProposal` |
| 6 | POST | `/proposals/{id}/submit` | idem | `SubmitProposalPayload` → `SubmitProposalResult` |
| 7 | POST | `/proposals/{id}/resubmit` | idem | `SaveDraftPayload` → `SubmitProposalResult` |
| 8 | GET | `/people/lookup` | session | `?email` → `PersonLookup` ou `null` |

**Le n° 3 crée en brouillon**, quel que soit l'état demandé (écart n° 96), et rend le numéro attribué par le déclencheur. L'adresse d'URL est dérivée par le service (R5).
**Le n° 6 et le n° 7 sont distincts, et c'est l'écart n° 38** : la fenêtre de l'appel ne s'applique qu'au premier, le plafond aux deux.
**Le n° 5 recompose** : français, heure murale du fuseau de l'édition, verrouillage d'identité (R6, écart n° 39).
**Le n° 8 ne rend jamais l'annuaire** : la clé est l'adresse, et rien d'autre.

---

## Côté organisation — l'espace (US5)

| # | Verbe | Chemin | Autorisation | Requête → Réponse |
|---|---|---|---|---|
| 9 | GET | `/organizations/{id}/workspace` | adhésion **active** | → `WorkspaceOverview` ou `null` |
| 10 | GET | `/organizations/{id}/editions` | adhésion active | → `EventEdition[]` |
| 11 | GET | `/proposals/{id}/file` | adhésion active à l'organisation porteuse | → `ProposalFile` ou `null` |
| 12 | POST | `/proposals/{id}/comments` | adhésion active **ou** `programme.review.write` | `ReplyToCommentPayload` \| `PostCommentPayload` → `ProposalComment` |
| 13 | POST | `/proposal-comments/{id}/resolution` | déposant **ou** `programme.review.write` | `ResolveCommentPayload` → `ProposalComment` |
| 14 | DELETE | `/proposal-comments/{id}/resolution` | idem | `ResolveCommentPayload` → `ProposalComment` |

**Les n° 9, 10 et 11 ne portent jamais** une note, un rang, le nom d'un membre du comité ni un inscrit nommé (FR-077). Vérifié par un test qui balaie la charge utile entière, pas par relecture.
**Le n° 12 sert deux appelants** : une réponse du déposant est **toujours** partagée et **jamais** une demande de correction ; un message du comité porte sa visibilité, et une demande de correction est forcée en partagé (écart n° 99).
**Les n° 13 et 14 portent l'écart n° 35** : le déposant pose et retire, le comité aussi de son côté — par permission, pas par formulaire.

---

## Côté comité — la liste (US3)

| # | Verbe | Chemin | Autorisation | Requête → Réponse |
|---|---|---|---|---|
| 15 | GET | `/proposals/list` | `programme.proposal.read_all` + périmètre | `?event_id` → `ProposalListScreen` |
| 16 | GET | `/proposals/dashboard` | idem | `?event_id` → `ProposalDashboardRow[]` |
| 17 | GET | `/proposals/committee` | idem | `?event_id` → `ProposalFacet[]` |
| 18 | GET | `/proposals` | adhésion active, ou `read_all` + périmètre | `?organization_id` → `Proposal[]` |
| 19 | GET | `/proposals/transitions` | session | → `ProposalTransitionRule[]` |
| 20 | POST | `/proposals/assignments` | `event.call.manage` + périmètre | `AssignReviewerPayload` → `BulkResult` |
| 21 | POST | `/proposals/status` | selon la transition visée, **dossier par dossier** | `ChangeStatusPayload` → `BulkResult` |

**Le n° 15 rend tout l'écran en une réponse** : lignes, sept facettes comptées **sur les mêmes lignes** (R16), non-lus, fuseau, ville, échéance effective, revues attendues.
**Le n° 19 rend la table de règles telle quelle** — contrat existant, **global**, sans dossier. Les transitions offertes pour un dossier et une personne sont ailleurs (n° 27).
**Le n° 21 évalue l'autorisation dossier par dossier** : une sélection peut traverser deux éditions, et le périmètre s'applique à chacune.
**Périmètre vide → refus explicite**, jamais liste vide (principe V).

---

## Côté comité — la fiche (US4)

| # | Verbe | Chemin | Autorisation | Requête → Réponse |
|---|---|---|---|---|
| 22 | GET | `/proposals/{id}/review-desk` | `programme.proposal.read_all` + périmètre | → `ReviewDeskScreen` ou `null` |
| 23 | PUT | `/proposals/{id}/reviews` | `programme.review.write` + périmètre + **affectation non déportée** | `SaveReviewPayload` → `SaveReviewResult` |
| 24 | POST | `/proposals/{id}/recusal` | `programme.review.write` + affectation | `RecusalPayload` → `ReviewAssignment` |
| 25 | POST | `/proposals/{id}/decision` | selon la transition visée | `DecisionPayload` → `DecisionResult` |
| 26 | GET | `/proposals/{id}` | `read_all` + périmètre, ou adhésion | → `Proposal` ou `null` |
| 27 | GET | `/proposals/{id}/available-transitions` | session + accès au dossier | → les transitions offertes **pour ce lecteur** |

**Le n° 22 applique le voile à la source** (R4) et **pose l'accusé de lecture** (R3) — une lecture qui écrit, assumée par le modèle.
**Le n° 23 appelle la consolidation** dans la même transaction et rend les agrégats **relus** (R10, écart n° 98).
**Le n° 27 existe parce que `/proposals/{id}/transitions` est déjà pris par le journal** (R19, écart n° 101). La fiche porte les mêmes données dans son champ `available_transitions`, pour ne pas doubler l'appel à l'affichage.

---

## Détail, historique, pièces et reprise (US7, US8)

| # | Verbe | Chemin | Autorisation | Requête → Réponse |
|---|---|---|---|---|
| 28 | GET | `/proposals/{id}/organizations` | accès au dossier | → `ProposalOrganization[]` |
| 29 | GET | `/proposals/{id}/speakers` | accès au dossier | → `ProposalSpeaker[]` |
| 30 | GET | `/proposals/{id}/documents` | accès au dossier | → `ProposalDocument[]` |
| 31 | GET | `/proposals/{id}/comments` | accès au dossier, **filtré par visibilité** | → `ProposalComment[]` |
| 32 | GET | `/proposals/{id}/transitions` | accès au dossier | → `ProposalTransition[]` — **le journal** |
| 33 | GET | `/proposals/{id}/history` | `read_all` + périmètre | → `ProposalHistoryEntry[]` |
| 34 | GET | `/proposals/{id}/themes` | accès au dossier | → les termes de thématique du dossier |
| 35 | POST | `/proposals/{id}/documents` | adhésion active, ou `read_all` + périmètre | rattachement d'un objet stocké → `ProposalDocument` |
| 36 | DELETE | `/proposals/{id}/documents/{document_id}` | idem | → confirmation du détachement |
| 37 | POST | `/admin/proposals/transitions-backfill` | `programme.proposal.read_all` en portée **globale** | → nombre de dossiers traités et de lignes semées |

**« Accès au dossier »** signifie : adhésion active à l'organisation porteuse, **ou** lecture générale dans le périmètre de l'édition. Les deux voies sont distinctes et testées séparément.
**Le n° 31 filtre à la source** : ce qui n'est pas envoyé ne peut pas fuiter.
**Les n° 35 et 36 sont additifs** — l'étape des documents est masquée côté front depuis le 17/08 ; la table et le rôle de téléversement existent depuis l'origine. Le dépôt du fichier appartient à B6.
**Le n° 37 est l'opération de déduction** (R20) : synchrone, rejouable, portée globale.

---

## Ordre d'enregistrement des chemins

Trois chemins littéraux doivent être déclarés **avant** leur homologue paramétré, sans quoi ils seraient capturés :

- `/proposals/list`, `/proposals/dashboard`, `/proposals/committee`, `/proposals/transitions`, `/proposals/form-context`, `/proposals/draft` — tous avant `/proposals/{id}`.
- `/proposals/{id}/transitions` (le journal) est un chemin **enfant**, il ne concurrence pas `/proposals/transitions` : le préfixe diffère au premier segment.

C'est le même avertissement qu'en B3, où trois chemins étaient concernés.

---

## Récapitulatif

**37 routes** : 8 pour le dépôt, 6 pour l'espace organisation, 7 pour la liste, 6 pour la fiche, 10 pour le détail, l'historique, les pièces et la reprise.

**Deux ajouts additifs au contrat du front**, livrés côté API et ignorés jusqu'à B7 : le champ `available_transitions` de la fiche, et les deux routes de pièces jointes.

**Un chemin que le prompt proposait et que le contrat occupait déjà** : `/proposals/{id}/transitions`. Le journal le garde ; les transitions offertes prennent `available-transitions` (écart n° 101).

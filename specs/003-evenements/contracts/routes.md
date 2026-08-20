# Contrat — Routes

**Fonctionnalité** : Événements (B3) · **Date** : 2026-08-20

> **Ce document ne définit aucune forme de réponse.** Les formes sont dans `frontend/app/types/` et n'y ont qu'une seule source. On indique ici, pour chaque route : le verbe, le chemin, l'autorisation exigée, le type de requête et de réponse **par son nom TypeScript**, et la politique de statut HTTP.
>
> La documentation OpenAPI est **engendrée depuis le code**. Ce fichier est la carte, pas la documentation.

---

## Préfixe, transport et politique de statut

Rien ne change depuis B1 et B2 : préfixe `/api`, `Accept-Language` sur chaque requête, `X-Request-Id` sur chaque réponse, session par cookies, vérification de l'origine sur toute écriture, en-têtes CORS posés. Les chemins ci-dessous sont donnés **tels que le front les écrit**, sans le préfixe.

**La règle de statut est celle de B1, reprise sans changement** :

| Le refus est… | Réponse |
|---|---|
| **exprimé par le contrat** comme membre d'union ou champ de résultat | **200**, avec `ok: false` et son code |
| **non exprimé** par le contrat | statut d'erreur HTTP + corps d'erreur ([`errors.md`](errors.md)) |

**Ce module penche fortement vers la première ligne.** `EditionSaveResult`, `CallSaveResult` et `EditionTabResult` portent tous leur propre forme de refus : la quasi-totalité des contraintes de `060_events.sql` sortent donc en **200**. Seuls trois refus n'ont aucune place dans le contrat et sortent en erreur HTTP.

**Les paramètres que le front passe encore et que l'API ignore.** Les données simulées transmettent `actorId`, le périmètre d'administration et, dans les corps de suppression, `event_id`. **L'API lit sa propre session et remonte elle-même à l'édition** (R2) : ces valeurs sont ignorées, jamais lues, et disparaîtront au raccordement (B7). C'est le motif éprouvé en B1 et B2.

---

## Les deux permissions, et pourquoi elles se testent séparément

| | `event.event.manage` | `event.call.manage` |
|---|---|---|
| **Couvre** | l'édition, ses journées, ses fils, ses lieux, ses salles, ses canaux | l'appel, sa grille, son comité |
| **Portée exigée** | l'édition visée, ou globale — **globale seule à la création** (FR-011) | l'édition visée, ou globale |
| **Détenue par** | `admin` (attribuable global ou sur un événement) et `super_admin` | les mêmes |

Détenir l'une **n'accorde pas** l'autre : un test le vérifie dans les deux sens. Aujourd'hui aucun rôle ne porte l'une sans l'autre — c'est l'écart n° 88, consigné et non corrigé.

Une **troisième** permission est consommée, et elle vient d'un autre module : `programme.session.schedule` garde les deux routes de publication (R12).

---

## Lectures publiques — aucune session

`event.v_public_editions` porte le critère de publicité : **ni brouillon, ni annulée**. Il n'est pas recopié côté code (FR-084, écart n° 26).

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `GET` | `/events/public` | — | `EventEdition[]` | 200 | **À déclarer AVANT `/events/{slug}`**, sinon `public` est capturé comme adresse d'URL (R11). Trié par date de début décroissante |
| `GET` | `/events/{slug}` | — | `EventEdition \| null` | 200 | **Une requête, deux vues** jointes par la gauche (R16). Embarque les **trois** déclinaisons d'image et l'appel résolu — l'écart n° 25 se referme ici. `null` pour un brouillon, une annulée ou une adresse inconnue : les trois sont indiscernables |
| `GET` | `/event-series` | — | `EventSeries[]` | 200 | Les quatre séries du semis, avec leur genre et leur décompte d'éditions |
| `GET` | `/events/{id}/days` | — | `EventDay[]` | 200 | Le calendrier d'une édition publique |
| `GET` | `/events/{id}/tracks` | — | `ProgrammeTrack[]` | 200 | Les fils **publiés** seulement |
| `GET` | `/events/{id}/venues` | — | `Venue[]` | 200 | |
| `GET` | `/events/{id}/rooms` | — | `Room[]` | 200 | Les salles de tous les lieux de l'édition |
| `GET` | `/events/{id}/channels` | — | `BroadcastChannel[]` | 200 | Les canaux de l'édition **et** les canaux généraux de la plateforme, comme le front les compose déjà |
| `GET` | `/events/{id}/call` | — | `CallForProposals \| null` | 200 | **Zéro ou un**, jamais un tableau. L'annulé est exclu |
| `GET` | `/events/{id}/images` | — | `Record<EditionImageRole, AttachedImage \| null>` | 200 | **Livrée pour ne pas casser l'écran, et vouée à disparaître** : `GET /events/{slug}` porte désormais les trois images. À retirer en B7 |

## Lecture bornée par le périmètre — session requise

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `GET` | `/events` | — | `EventEdition[]` | 200 | Le sélecteur d'édition du back-office. **Filtrée** par le périmètre, pas refusée : un périmètre vide rend une liste vide, et c'est le store qui décide de l'écran. C'est la seule route de ce module où périmètre vide ≠ refus, parce que le contrat du front le veut ainsi |

---

## Back-office — les éditions

Permission `event.event.manage`. **Périmètre vérifié avant toute lecture**, et refus explicite sur périmètre vide (FR-008).

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `GET` | `/admin/events` | — | `EditionListScreen` | 200 · 403 | Lignes, séries et millésimes **en une réponse**, facettes comptées sur le même jeu de lignes. `is_global_scope` dit si l'appelant administre tout. **Périmètre vide → 403**, jamais une liste vide |
| `GET` | `/admin/events/form-options` | — | `EditionFormOptions` | 200 · 403 | **À déclarer AVANT `/admin/events/{id}`** (R11). Séries, pays, fuseaux, statuts. Servie à part de la liste : le référentiel des pays ne repart pas à chaque affichage du tableau |
| `GET` | `/admin/events/{id}` | — | `EditionDetail \| null` | 200 · 403 · 404 | **Les six onglets en une réponse** (R3). `404` pour une édition inexistante **ou** hors périmètre — indiscernables |
| `POST` | `/admin/events` | `EditionFormPayload` | `EditionSaveResult` | 200 · 403 | **Portée GLOBALE exigée** : une édition qui n'existe pas encore n'offre aucune portée. `EVENT_GLOBAL_SCOPE_REQUIRED` sinon |
| `PUT` | `/admin/events/{id}` | `EditionFormPayload` | `EditionSaveResult` | 200 · 403 · 404 | Écriture **totale** (R13). `programme_published_at` n'est jamais touchée ici |

**Les refus d'écriture d'une édition sortent en 200**, dans `errors: EditionFormError[]` : `period`, `physical_location`, `slug_taken`, `edition_taken`, `year_range`, `coordinates`, `required`. **Le sigle manquant emprunte cette forme** — `{ code: 'required', field: 'acronym' }` —, et la réponse porte en plus `suggested_acronym` (R1, champ additif).

`days_created` compte ce que la nouvelle période a ajouté ; `days_removed` et `sessions_detached` valent **toujours zéro** ici : un enregistrement d'édition ne supprime aucune journée (FR-033).

---

## Back-office — les six onglets

Toutes ces routes rendent `EditionTabResult`, dont `detail` porte **la composition entière recalculée** (FR-024). Leurs refus sortent en 200, dans `error_code` : `not_found`, `required`, `period`, `code_taken`, `slug_taken`, `capacity`, `platform_channel`, `deactivated`.

> **`deactivated` n'est pas un refus.** Il accompagne `ok: true` et dit qu'un canal a été désactivé plutôt que supprimé (R7). L'annotation OpenAPI le dit à l'endroit où l'on serait tenté de croire l'inverse.

### Journées du calendrier — `event.event.manage`

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `GET` | `/admin/events/{id}/days/plan` | — | `DayGenerationPlan \| null` | 200 · 403 · 404 | **Lecture seule, rien ne s'écrit.** Dates à créer, journées hors période avec leurs séances, journées inchangées |
| `POST` | `/admin/events/{id}/days` | `{ remove_outside_period: boolean }` | `EditionTabResult` | 200 · 403 · 404 | **Le plan est recalculé dans la transaction** (R4). Sans le drapeau, aucune journée n'est retirée. `sessions_detached` compté **avant** (R8) |
| `PUT` | `/admin/events/{id}/days/{dayId}` | `EditionDayPayload` | `EditionTabResult` | 200 · 403 · 404 | Contenu **éditorial** seul. La date ne se modifie pas |

### Journées spéciales — `event.event.manage`

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `POST` | `/admin/tracks` | `EditionTrackPayload` | `EditionTabResult` | 200 · 403 · 404 | L'édition vient de la charge utile **et est vérifiée** ; les thématiques sont écrites dans le même geste |
| `PUT` | `/admin/tracks/{id}` | `EditionTrackPayload` | `EditionTabResult` | 200 · 403 · 404 | L'édition vient de **l'ascendance du fil**, jamais du corps (R2) |
| `DELETE` | `/admin/tracks/{id}` | *(corps ignoré)* | `EditionTabResult` | 200 · 403 · 404 | La seule suppression du module qui cascade sur un rattachement éditorial. `sessions_detached` = rattachements perdus, comptés avant |

### Lieux et salles — `event.event.manage`

| Verbe | Chemin | Requête | Réponse | Statut |
|---|---|---|---|---|
| `POST` | `/admin/venues` | `EditionVenuePayload` | `EditionTabResult` | 200 · 403 · 404 |
| `PUT` | `/admin/venues/{id}` | `EditionVenuePayload` | `EditionTabResult` | 200 · 403 · 404 |
| `DELETE` | `/admin/venues/{id}` | *(corps ignoré)* | `EditionTabResult` | 200 · 403 · 404 |
| `POST` | `/admin/rooms` | `EditionRoomPayload` | `EditionTabResult` | 200 · 403 · 404 |
| `PUT` | `/admin/rooms/{id}` | `EditionRoomPayload` | `EditionTabResult` | 200 · 403 · 404 |
| `DELETE` | `/admin/rooms/{id}` | *(corps ignoré)* | `EditionTabResult` | 200 · 403 · 404 |

Retirer un lieu emporte ses salles ; `sessions_detached` compte les séances de **toutes** ses salles, mesurées avant (R8). `is_virtual` est écrit tel quel et jamais déduit du mode de participation.

### Canaux de diffusion — `event.event.manage`

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `POST` | `/admin/channels` | `EditionChannelPayload` | `EditionTabResult` | 200 · 403 · 404 | Poser le défaut **retire le précédent dans la même transaction** (R6) |
| `PUT` | `/admin/channels/{id}` | `EditionChannelPayload` | `EditionTabResult` | 200 · 403 · 404 | Un canal **général** (sans édition) → `platform_channel` |
| `DELETE` | `/admin/channels/{id}` | *(corps ignoré)* | `EditionTabResult` | 200 · 403 · 404 | Désactivé s'il a servi (`deactivated`, `ok: true`), supprimé sinon (R7) |

### Appel à propositions et grille — `event.call.manage`

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `GET` | `/admin/calls/default-criteria` | — | `EditionCriterion[]` | 200 · 403 | **Lue en base, jamais recopiée.** Les six critères, leurs libellés bilingues, leurs poids et l'éliminatoire viennent de `event.seed_default_criteria()` |
| `POST` | `/admin/calls` | `EditionCallPayload` | `CallSaveResult` | 200 · 403 · 404 · 422 | **L'appel et la grille en une transaction** (R9) |
| `PUT` | `/admin/calls/{id}` | `EditionCallPayload` | `CallSaveResult` | 200 · 403 · 404 · 422 | L'édition vient de **l'ascendance de l'appel** |

Les refus sortent en 200, dans `errors: CallFormError[]` : `window`, `extension`, `speakers`, `duration_bounds`, `daily_window`, `already_exists`, `code_taken`, `criteria_empty`, `criterion_code_duplicate`, `required`. Un refus portant sur une ligne de grille désigne son **rang** (`criterion_index`).

**Un seul refus sort en 422** : la suppression d'un critère porteur de notes (`EVENT_CRITERION_HAS_SCORES`), faute de variante dans le contrat (R9).

`scores_affected` prévient qu'un barème modifié va déplacer des moyennes déjà calculées.

### Comité de sélection — `event.call.manage`

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `PUT` | `/admin/calls/{id}/reviewers` | `CommitteePayload` | `CommitteeSaveResult` | 200 · 403 · 404 · 422 | **Ajouts, retraits et plafonds d'un seul geste**, une transaction. Doublons de charge utile dédoublonnés par le service. `removed_with_assignments` nomme les membres retirés portant encore des dossiers |

Ajouter quelqu'un **n'accorde aucun droit** : la réponse porte `has_review_permission` et se contente de le dire. Une personne inconnue → `EVENT_UNKNOWN_REFERENCE` (422).

---

## Publication de la programmation — `programme.session.schedule`

**Ces deux routes vivent sous un préfixe partagé.** `/admin/planner` sera aussi celui de B5 : le scope est composé **une seule fois dans `api`**, chaque module y versant ses routes — le patron de `/people` posé en B1, appliqué avant que le défaut ne se reproduise (R11).

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `GET` | `/admin/planner/readiness` | `event_id` | `PublicationReadinessIssue[]` | 200 · 403 · 404 | **Lecture seule**, consultable avant toute tentative. Rend `occurs_at`, un **instant** — jamais un intervalle mis en forme |
| `POST` | `/admin/planner/publish` | `{ event_id }` | `PublishProgrammeResult` | 200 · 403 · 404 | **Le seul contrôle bloquant du module.** Un point `blocking` → `blocked: true`, rien n'est écrit. Les avertissements accompagnent sans retenir |

Une publication qui aboutit estampille l'édition, **annonce** par `event.programme.published` et rend `published_count` — un décompte de **désignation** (R10). Republier est inoffensif : la date d'origine ne s'écrase pas et aucun second événement n'est émis.

---

## Récapitulatif

| Famille | Routes |
|---|---|
| Lectures publiques | 10 |
| Lecture bornée par le périmètre | 1 |
| Back-office — éditions | 5 |
| Back-office — journées | 3 |
| Back-office — fils | 3 |
| Back-office — lieux et salles | 6 |
| Back-office — canaux | 3 |
| Back-office — appel et grille | 3 |
| Back-office — comité | 1 |
| Publication | 2 |
| **Total** | **37** |

**Un test frappe les trente-sept sur la vraie application** (R18). C'est la leçon de B2, où trois routes sur vingt et une étaient muettes sans que rien ne le dise.

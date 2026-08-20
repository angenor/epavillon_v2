# Phase 1 — Le modèle, et ce que le code en fait

**Fonctionnalité** : Événements (B3) · **Date** : 2026-08-20

**Aucune table, aucune colonne, aucun type n'est créé ni modifié.** Ce document ne redéfinit pas le modèle : il dit, table par table, ce que le module lit, ce qu'il écrit, ce qu'il laisse à la base, et d'où vient chaque forme que le front attend. La source reste [`docs/database/060_events.sql`](../../docs/database/060_events.sql), qui porte sa propre documentation.

---

## 1. Ce que le code lit, écrit, et ne touche pas

### `event.event_series` — la série

**Lecture seule dans ce jalon.** Aucun écran n'offre de créer une série ; le semis en pose quatre. Le module rend le genre, le nom, l'état d'activité et le **nombre d'éditions rattachées** — décompte joint, jamais une colonne.

`kind` est un énuméré du modèle et le reste : la distinction « c'est une COP » / « c'est un cycle de webinaires » vient de là, jamais d'une liste d'adresses recopiée dans un composant.

`organizer_organization_id` est déclarée au registre des références des organisations (`060` § 8, stratégie `reassign`) : une fusion d'organisations la déplace toute seule. **Le module n'a rien à faire pour cela.**

### `event.events` — l'édition

| Colonne | Ce que le module en fait |
|---|---|
| `series_id`, `edition_label`, `edition_year` | Écrits par le formulaire. Leur unicité conjointe est tenue par `ux_events_series_edition` et **traduite**, jamais anticipée |
| `title`, `description`, `highlights` | Textes multilingues `platform.i18n_text`. Écrits tels quels ; **jamais résolus côté serveur** — la résolution est un geste d'affichage |
| `acronym` | **Facultatif en base, et il le reste.** La seule règle que le service ajoute au modèle : exigé quand `has_pavilion` vaut vrai (R1). Écrit tel qu'il est saisi ; c'est le déclencheur d'affectation du numéro de dossier qui le met en majuscules |
| `slug` | Écrit par le formulaire. `ux_events_slug` porte sur **toute la plateforme** — traduit sur son champ |
| `status` | Écrit par le formulaire. Aucune machine à états en base : le modèle n'en pose pas pour l'édition, et le code n'en invente pas |
| `participation_mode`, `timezone` | Écrits. `platform.timezone_name` valide l'identifiant contre la base de fuseaux de PostgreSQL — **c'est elle qui fait foi** (R5) |
| `starts_at`, `ends_at` | Instants complets, écrits tels que reçus. `ck_events_period` traduite |
| `country_id`, `city`, `address` | Écrits. `ck_events_physical_location` traduite : hors ligne, pays **et** ville exigés |
| `latitude`, `longitude` | Écrits. `ck_events_coordinates` traduite : les deux ou aucun |
| `has_pavilion` | Écrit. **Commande la règle du sigle**, et rien d'autre côté code |
| `programme_published_at` | **Jamais écrite par le formulaire.** Posée par la publication seule, une fois, sans écraser une date existante (R10) |
| `created_by` | Posé à la création depuis le contexte d'écriture |
| `created_at`, `updated_at` | Posés par la base ; `updated_at` par `tg_events_updated_at` |

**Cette table est auditée** (`tg_events_audit`). C'est l'une des deux seules du module : toute écriture doit donc passer par l'unique porte du noyau, sans quoi l'historique de l'édition devient anonyme.

Les vérifications que le code **ne redouble pas** : période, lieu physique, coordonnées, bornes du millésime, unicité de l'adresse d'URL, unicité série–millésime–libellé, validité du fuseau. Toutes traduites — [`contracts/errors.md`](contracts/errors.md).

### `event.event_days` — le calendrier

**Aucun déclencheur de dérivation.** Rien en base ne crée une journée quand une édition change de période, rien n'en supprime quand elle se resserre. La génération est donc un comportement d'application, et un **geste explicite** (R4).

| Colonne | Ce que le module en fait |
|---|---|
| `day_date` | Écrite par la génération, calculée **en base dans le fuseau de l'édition** (R5). Jamais modifiée ensuite |
| `title`, `slug`, `description`, `is_featured`, `color_hex` | **Éditoriales.** Écrites par la route d'habillage, jamais touchées par la génération |
| `sort_order` | Rang du jour dans la période, posé à la génération |

`ux_event_days_date` et `ux_event_days_slug` portent toutes deux sur l'édition : traduites sur leur champ.

Une journée générée porte **sa date et rien d'autre**. Inventer « Jour 3 » produirait un titre que personne n'a écrit et qui s'afficherait tel quel sur la page publique.

### `event.programme_tracks` — les journées spéciales

Écriture complète : création, modification, suppression. Ce que le module **n'écrit pas** : la composition. Le rattachement d'une séance à un fil vit dans `programme.session_tracks` et se décide au planificateur.

`starts_on` / `ends_on` sont **indicatives** — le commentaire du modèle le dit — et ne contraignent aucun rattachement. Seule `ck_programme_tracks_period` (fin après début) est vérifiée par la base et traduite.

Les **thématiques** passent par `reference.entity_terms`, clé `('event', 'programme_tracks', <id>)`. Elles sont écrites par le module et rendues à l'affichage par `reference.term_badges()`, avec libellé traduit et couleur. **Jamais recopiées dans un fichier i18n** — c'est le défaut n° 1 de la v1 appliqué à une grille de journées.

`published_at` porte l'ouverture de la page publique du fil : posée ou remise à nul par le même enregistrement que le reste, jamais par un geste séparé.

### `event.venues` et `event.rooms` — le stand

Écriture complète. Deux points méritent d'être dits :

- **`rooms.is_virtual` n'est pas un détail d'inventaire.** Une salle virtuelle accepte les créneaux simultanés, et `programme.detect_conflicts()` n'y signale aucune double réservation. Le service l'écrit **tel quel** et ne le déduit jamais du mode de participation : c'est le **lieu** qui dit l'occupation.
- **`rooms.enforce_room_exclusivity` n'existe pas ici.** C'est une colonne de `programme.sessions`, dénormalisée depuis `is_virtual` par un déclencheur de l'autre module. Le nôtre n'y touche pas.

`ux_rooms_code` porte sur le lieu. Ni `venues` ni `rooms` ne portent `updated_at` ni déclencheur d'audit : **il n'y a pas d'historique champ par champ pour un lieu ou une salle**, et on ne le compense pas (écart n° 92).

### `event.broadcast_channels` — le direct

Ressource **réservable**, au même titre qu'une salle. Deux index gouvernent l'écriture, et tous deux ont une asymétrie qu'il faut avoir en tête (R6) :

- `ux_broadcast_channels_code` est `NULLS NOT DISTINCT` : deux canaux **généraux** de la plateforme ne peuvent pas partager un code, un canal général et un canal d'édition le peuvent.
- `ux_broadcast_channels_default` regroupe les canaux généraux sous un identifiant de substitution et ne porte que sur les canaux **actifs**. Poser un défaut d'édition ne déloge donc pas le canal général semé.

Un canal **général** (`event_id IS NULL`) n'est pas modifiable depuis une édition : refus `platform_channel`, dans le contrat, en 200.

Le module **n'affecte aucun canal à une séance** : le modèle le fait par déclencheur dès qu'une séance devient diffusée.

### `event.calls_for_proposals` — l'appel

**Cardinalité 0..1**, tenue par `ux_calls_one_per_event`, qui **exclut les appels annulés**. Un appel annulé n'empêche donc pas d'en créer un nouveau — c'est écrit dans le SQL, et le refus « appel déjà existant » ne doit pas se déclencher là.

| Colonne | Ce que le module en fait |
|---|---|
| `code`, `title`, `description`, `status` | Écrits par le formulaire. `ux_calls_code` traduite |
| `opens_at`, `closes_at`, `extended_until` | Écrits. `ck_calls_window` et `ck_calls_extension` traduites. **La prolongation reste à part** : la trace de ce qui a été annoncé aux organisations ne se perd pas |
| `results_expected_at` | Écrite |
| règles de recevabilité | Écrites telles quelles. Elles sont appliquées par **B4**, à la soumission, jamais ici |
| `daily_start_time`, `daily_end_time` | Écrites **en heure locale de l'édition**, jamais converties. `ck_calls_daily_window` traduite |
| `min/max/default_duration_minutes` | Écrits. `ck_calls_duration_bounds` traduite d'un bloc — elle porte trois conditions et un seul nom |
| `required_reviews`, `blind_review` | Écrits tels que le formulaire les porte. **Aucune valeur par défaut imposée par le service** : un seul mot dans `blind_review` tranchera l'arbitrage encore ouvert, sans toucher au code |
| `created_by` | Posé à la création depuis le contexte d'écriture |

**Cette table est auditée** (`tg_calls_audit`) — la seconde et dernière du module.

Trois fonctions du modèle sont **appelées, jamais recalculées** : `event.effective_deadline()`, `event.is_call_open()` (statut **et** fenêtre, pas le seul statut), `event.max_weighted_score()`.

### `event.review_criteria` — la grille

Écriture par **diff sur le code**, dans la transaction de l'appel (R9).

`event.seed_default_criteria()` est **lue et jamais recopiée** : la route qui rend la grille par défaut l'exécute sur un appel jetable dans une transaction annulée, ou lit ses six lignes par un appel dédié — les six libellés bilingues, les poids et le critère éliminatoire viennent de la base.

**Le piège de cette table** : `programme.review_scores` la référence en `ON DELETE CASCADE`. Supprimer un critère **détruit les notes**, sans erreur et sans trace. Le service compte et refuse (R9) — c'est l'entorse assumée du plan.

`score_count`, que le contrat du front porte sur chaque critère, est un décompte joint depuis `programme.review_scores` : jamais une colonne.

### `event.call_reviewers` — le comité

Écriture d'un seul geste : ajouts, retraits et plafonds ensemble, dans une transaction. Clé primaire `(call_id, person_id)` : une charge utile portant deux fois la même personne est **dédoublonnée par le service**, jamais remontée comme erreur de base.

**Cette table dit la composition, pas le droit d'accès.** Le commentaire du modèle le dit en toutes lettres : l'autorisation reste portée par `identity.role_assignments` sur la portée de l'édition. Le service **n'attribue aucun rôle** en ajoutant un membre ; il **signale** que la personne ne détient pas la permission d'évaluer.

`workload_cap` est un plafond **indicatif** : rien ne l'applique, et le module ne l'applique pas non plus.

---

## 2. Ce que le module lit hors de son schéma, et pourquoi

Toutes ces lectures sont réunies dans un fichier (R14). Aucune n'écrit.

| Schéma | Lecture | Question de **ce** module à laquelle elle répond |
|---|---|---|
| `programme` | dossiers déposés par édition, brouillons exclus | « combien de dossiers cette édition a-t-elle reçus ? » |
| `programme` | séances par édition, et séances placées en salle | « où en est le placement de cette édition ? » |
| `programme` | séances par journée, salle, lieu, canal | « que détacherait ce retrait ? » (R8) |
| `programme` | rattachements séance–fil | « combien de séances ce fil porte-t-il ? » |
| `programme` | notes posées par critère | « ce critère peut-il être supprimé ? » (R9) |
| `programme` | `publication_readiness(event_id)` | « cette édition peut-elle être publiée ? » (R10) |
| `programme` | séances désignées par la publication | « combien de séances la publication rend-elle publiques ? » |
| `programme` | dossiers confiés et revues rendues par membre | « quelle est la charge de ce membre du comité ? » |
| `programme` | `v_edition_stats` | « quel volume de programme cette édition publie-t-elle ? » |
| `identity` | personnes assignables et candidats au comité | « qui peut curer un fil, qui peut siéger ? » |
| `identity` | `has_permission` sur l'édition | « ce membre détient-il bien le droit d'évaluer ? » |
| `identity` | `administered_events` | le périmètre — par le garde du noyau, jamais par une requête d'ici |
| `reference` | `terms_of`, `term_badges`, pays, langues | thématiques d'un fil, listes du formulaire |
| `media` | `attached_image` par la vue publique | les trois déclinaisons d'une édition (R17) |

**Aucune écriture hors du schéma `event`.** La seule qu'on serait tenté d'ajouter — la visibilité des séances à la publication — passe par l'outbox (R10).

---

## 3. Ce que le front attend, et d'où chaque forme vient

Les formes vivent dans `frontend/app/types/admin-events.ts` et `frontend/app/types/event/`. Elles ne sont pas redéfinies ici. Ce tableau dit **d'où vient ce qui n'est pas une colonne**.

| Champ attendu | Origine |
|---|---|
| `EditionListRow.series_name`, `series_kind` | jointure sur la série |
| `EditionListRow.country_name` | jointure sur `reference.countries` |
| `EditionListRow.proposal_count` | décompte joint, brouillons exclus |
| `EditionListRow.session_count`, `scheduled_session_count` | décomptes joints |
| `EditionListRow.day_count` | décompte joint, schéma propre |
| `EditionListRow.call_status`, `call_deadline` | l'appel non annulé, et `effective_deadline()` |
| `EditionListScreen.series`, `years` | facettes calculées **sur le même jeu de lignes** que la liste |
| `EditionListScreen.is_global_scope` | le périmètre, tel que le garde du noyau le rend |
| `EditionDetail.period` | `generate_series` dans le fuseau de l'édition (R5) |
| `EditionDetail.images` | `media.attached_image()`, trois rôles |
| `EditionDay.session_count`, `is_outside_period` | décompte joint, et comparaison à la période |
| `EditionTrack.curator_name` | jointure sur la personne |
| `EditionTrack.themes` | `reference.term_badges()` |
| `EditionTrack.session_count` | décompte joint — **lecture seule** |
| `EditionRoom.session_count`, `EditionChannel.session_count` | décomptes joints |
| `EditionCall.effective_deadline`, `is_open`, `max_weighted_score` | les trois fonctions du modèle |
| `EditionCall.proposal_count` | décompte joint |
| `EditionCriterion.score_count` | décompte joint |
| `EditionCommitteeMember.assigned_count`, `submitted_count` | décomptes joints |
| `EditionCommitteeMember.has_review_permission` | `identity.has_permission` sur l'édition |
| `EditionSaveResult.days_created` | ce que la génération a créé, mesuré |
| `EditionSaveResult.days_removed`, `sessions_detached` | **toujours zéro** ici : un enregistrement d'édition ne supprime aucune journée (FR-033) |
| `EditionTabResult.sessions_detached` | compté **avant** le détachement (R8) |
| `CallSaveResult.scores_affected` | un critère conservé dont le barème change **et** qui porte des notes (R9) |
| `CommitteeSaveResult.removed_with_assignments` | membres retirés portant encore des dossiers |
| `PublishProgrammeResult.published_count` | décompte de **désignation**, sous l'instantané de la transaction (R10) |

**Deux champs sont ajoutés au contrat, de façon additive**, et ignorés par le front jusqu'à B7 :

| Champ | Sur | Pourquoi |
|---|---|---|
| `suggested_acronym: string \| null` | `EditionSaveResult` | Porter la valeur proposée avec le refus. La forme actuelle exprime déjà le refus (`{ code: 'required', field: 'acronym' }`) mais n'a pas de place pour une suggestion (R1) |
| `EVENT_CRITERION_HAS_SCORES` | erreur HTTP 422 | `CallErrorCode` n'a pas de variante pour ce refus. En attendant B7, il sort en erreur HTTP plutôt qu'en membre d'union (R9) |

---

## 4. Ce que le modèle porte et que ce jalon ne livre pas

- **La suppression d'une édition.** Elle cascade sur journées, fils, lieux, salles, canaux et appel. Aucun écran ne l'offre ; le retrait passe par le statut.
- **La dé-publication d'une programmation.** Aucun contrat ne la porte.
- **L'écriture d'une série.** Lecture seule dans ce jalon.
- **Le rattachement des images.** À B6 (R17).
- **Les rappels d'échéance.** Les règles vivent dans `engagement.reminder_rules` ; à B6 (R15).
- **`event.venues.map_url` et `kind`** sont écrits, mais aucune vérification de forme au-delà du domaine `platform.url` et du `CHECK` de la base.

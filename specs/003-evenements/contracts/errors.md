# Contrat — Erreurs

**Fonctionnalité** : Événements (B3) · **Date** : 2026-08-20

Principe IX : **toute erreur porte un code stable et un message français.** La forme du corps, les codes transverses du noyau et la règle de statut sont ceux de B1 et ne sont pas rejoués ici — voir [`../../001-socle-identite/contracts/errors.md`](../../001-socle-identite/contracts/errors.md).

**Rappel de politique** : un refus prévu par le contrat du front n'est pas une erreur HTTP. Ce module penche fortement de ce côté — `EditionSaveResult`, `CallSaveResult` et `EditionTabResult` portent chacun leur forme de refus. **La quasi-totalité des contraintes de `060_events.sql` sortent donc en 200.**

---

## Ce qui sort en 200, et n'est pas une erreur

### Refus d'écriture d'une édition — `EditionSaveResult.errors`

| Code | Champ désigné | Contrainte traduite |
|---|---|---|
| `period` | `ends_at` | `ck_events_period` |
| `physical_location` | `country_id` puis `city` | `ck_events_physical_location` |
| `slug_taken` | `slug` | `ux_events_slug` |
| `edition_taken` | `edition_label` | `ux_events_series_edition` |
| `year_range` | `edition_year` | `events_edition_year_check` |
| `coordinates` | `latitude` ou `longitude` | `ck_events_coordinates` |
| `required` | le champ manquant | `NOT NULL`, **et la règle du sigle** |

**Le sigle emprunte `required` sur le champ `acronym`.** La forme actuelle du contrat n'a pas de variante dédiée, et n'en a pas besoin : le champ nommé suffit à désigner le fautif, et le sigle étant facultatif en base, il n'y a aucune ambiguïté avec un `NOT NULL`. La **valeur proposée** voyage à part, dans `suggested_acronym` (champ additif, R1).

### Refus d'écriture d'un onglet — `EditionTabResult.error_code`

| Code | Quand |
|---|---|
| `not_found` | objet inexistant **ou hors périmètre** — indiscernables |
| `required` | champ obligatoire manquant |
| `period` | `ck_programme_tracks_period` |
| `code_taken` | `ux_programme_tracks_code`, `ux_rooms_code`, `ux_broadcast_channels_code` |
| `slug_taken` | `ux_programme_tracks_slug`, `ux_event_days_slug` |
| `capacity` | `rooms_capacity_check` |
| `platform_channel` | modification d'un canal général de la plateforme depuis une édition |
| `deactivated` | **succès** : le canal a servi, il est désactivé et non supprimé (R7) |

### Refus d'écriture d'un appel — `CallSaveResult.errors`

| Code | Champ désigné | Contrainte traduite |
|---|---|---|
| `window` | `closes_at` | `ck_calls_window` |
| `extension` | `extended_until` | `ck_calls_extension` |
| `speakers` | `max_speakers` | `ck_calls_speakers` |
| `duration_bounds` | `default_duration_minutes` ou la borne fautive | `ck_calls_duration_bounds` |
| `daily_window` | `daily_end_time` | `ck_calls_daily_window` |
| `already_exists` | — | `ux_calls_one_per_event` |
| `code_taken` | `code` | `ux_calls_code` |
| `criteria_empty` | — | règle du service : une grille vide n'évalue rien |
| `criterion_code_duplicate` | rang de la ligne (`criterion_index`) | `ux_review_criteria`, anticipée pour nommer le rang |

`ck_calls_duration_bounds` porte **trois conditions sous un seul nom**. Le service compare les trois pour désigner le champ le plus probablement fautif — la valeur par défaut hors bornes en premier, c'est le cas courant — mais ne réimplémente pas la contrainte : c'est la base qui refuse.

### Refus de publication — `PublishProgrammeResult.blocked`

Un point de gravité `blocking` retient la publication : `blocked: true`, `published_count: 0`, et `issues` dit quoi régler. **Ce n'est pas une erreur HTTP** — c'est le résultat attendu d'un contrôle.

---

## Catalogue des codes ajoutés par ce module

Trois seulement, et chacun parce que le contrat du front n'a **aucune** place pour l'exprimer.

| Code | Statut | Message | Quand |
|---|---|---|---|
| `EVENT_GLOBAL_SCOPE_REQUIRED` | 403 | La création d'une édition exige des droits sur l'ensemble de la plateforme. | Création d'une édition par un compte détaché sur une ou plusieurs éditions. **Distinct de `FORBIDDEN`** parce que l'écran sait dire *pourquoi* : une édition qui n'existe pas encore n'offre aucune portée où vérifier un droit (FR-011) |
| `EVENT_CRITERION_HAS_SCORES` | 422 | Ce critère porte déjà des notes : le retirer effacerait l'argumentaire des évaluations rendues. | Suppression d'un critère référencé par `programme.review_scores`. **Champ** : `criteria`, avec le rang de la ligne et le nombre de notes. Sans ce refus, la cascade détruirait les notes en silence (R9) |
| `EVENT_UNKNOWN_REFERENCE` | 422 | La valeur choisie n'existe pas. | Série, pays, langue, fuseau ou personne inconnus. Le champ fautif est déduit du nom de la contrainte ou du **nom du domaine** |

Ils sont **engendrés dans l'OpenAPI depuis le catalogue du noyau**, comme les vingt de B1 et les onze de B2 : un code ajouté apparaît au prochain démarrage, un code oublié n'existe pas.

**Une dette pour B7, inscrite ici pour ne pas se perdre** : `EVENT_CRITERION_HAS_SCORES` mériterait une variante de `CallErrorCode` pour que le message se pose sur la ligne de grille plutôt que dans un bandeau. C'est un ajout au contrat du front, à faire au raccordement.

---

## Traduction des erreurs PostgreSQL

Principe VIII : **le code ne redouble pas une contrainte de la base — il traduit son refus.** La correspondance vit dans le noyau ; ce tableau ajoute les entrées de ce module.

### Violations d'unicité (`23505`)

| Contrainte | Devient |
|---|---|
| `ux_events_slug` | `slug_taken`, champ `slug` |
| `ux_events_series_edition` | `edition_taken`, champ `edition_label` |
| `ux_event_days_date` | **Ne doit jamais remonter** : la génération ne crée que les dates absentes, calculées dans la même transaction (R4). S'il remonte, c'est un défaut de code → `INTERNAL` |
| `ux_event_days_slug` | `slug_taken` |
| `ux_programme_tracks_code` / `_slug` | `code_taken` / `slug_taken` |
| `ux_rooms_code` | `code_taken` |
| `ux_broadcast_channels_code` | `code_taken`. **Attention à l'asymétrie** : `NULLS NOT DISTINCT` fait que deux canaux généraux ne peuvent pas partager un code |
| `ux_broadcast_channels_default` | **Ne doit jamais remonter** : le service retire le défaut précédent avant de poser (R6). S'il remonte, c'est que l'ordre a été inversé → `INTERNAL` |
| `ux_calls_one_per_event` | `already_exists`. **N'est levée que par un appel non annulé** — l'index exclut les annulés |
| `ux_calls_code` | `code_taken` |
| `ux_review_criteria` | `criterion_code_duplicate` avec le rang. Le service dédoublonne avant, donc ce refus vise le cas où deux lignes de la charge utile portent le même code |
| clé primaire de `call_reviewers` | **Ne doit jamais remonter** : la charge utile est dédoublonnée par le service → `INTERNAL` |

### Violations de vérification (`23514`)

| Contrainte | Devient |
|---|---|
| `ck_events_period` | `period` |
| `ck_events_coordinates` | `coordinates` |
| `ck_events_physical_location` | `physical_location` |
| `events_edition_year_check` | `year_range` |
| `ck_programme_tracks_period` | `period` |
| `rooms_capacity_check` | `capacity` |
| `ck_calls_window` · `ck_calls_extension` · `ck_calls_speakers` · `ck_calls_duration_bounds` · `ck_calls_daily_window` | leur code homonyme |
| checks de forme de code (`programme_tracks_code_check`, `broadcast_channels_code_check`, `calls_for_proposals_code_check`, `review_criteria_code_check`) | `required` sur le champ `code`, message précisant la forme attendue |
| checks de couleur (`event_days_color_hex_check`, `programme_tracks_color_hex_check`) | `required` sur `color_hex` |
| checks de vocabulaire (`venues_kind_check`, `broadcast_channels_provider_check`) | `EVENT_UNKNOWN_REFERENCE` |
| **violation de domaine** — `platform.slug`, `platform.url`, `platform.timezone_name`, `platform.email` | Le nom de la contrainte est celui du domaine, pas de la colonne : **on se sert du nom de type** (`PG_DIAG_DATATYPE_NAME`), qui est fiable. `timezone_name` → `EVENT_UNKNOWN_REFERENCE` champ `timezone` ; `slug` → `required` champ `slug` ; `url` → `required` champ `map_url` ou `guidelines_url` selon la route |

### Violations de clé étrangère (`23503`)

| Contrainte | Devient |
|---|---|
| `events_series_id_fkey` · `events_country_id_fkey` | `EVENT_UNKNOWN_REFERENCE`, champ déduit |
| `broadcast_channels_locale_fkey` | `EVENT_UNKNOWN_REFERENCE`, champ `locale` |
| `xmod_fk_programme_tracks_curator` | `EVENT_UNKNOWN_REFERENCE`, champ `curated_by` |
| `xmod_fk_call_reviewers_person` | `EVENT_UNKNOWN_REFERENCE`, champ `person_id` |
| `xmod_fk_events_creator` · `xmod_fk_calls_creator` | **Ne doivent jamais remonter** : l'acteur vient de la session → `INTERNAL` |

### Ce que la base **ne** refuse **pas**, et que le service refuse

Deux règles seulement, et il faut savoir laquelle est une entorse.

| Règle | Est-ce une entorse au principe VIII ? |
|---|---|
| **Sigle exigé quand le pavillon est tenu** | **Non.** Le modèle ne porte pas cette règle et ne doit pas la porter (R1). Rien n'est redoublé |
| **Critère porteur de notes non supprimable** | **Oui**, et elle est justifiée dans le plan : la clé est `ON DELETE CASCADE`, la base détruirait les notes sans rien dire (R9) |

### Ce que la base refuse et que le service **ne refuse pas**

**Les chevauchements de créneaux.** Aucune contrainte d'exclusion n'existe sur les créneaux et le service n'en invente pas : le système détecte et signale, il ne bloque jamais (règle métier n° 2). Le seul contrôle bloquant du module est celui de la publication.

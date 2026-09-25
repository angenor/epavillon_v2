# Data Model — Guide Négo, sessions de négociation (étape 3a)

**Source de vérité** : `docs/database/` — ce fichier dit ce qui change et pourquoi ; le SQL s'écrit
d'abord dans `020_reference.sql`, `100_negotiations.sql` et `900_seed.sql`, puis la base locale se
**migre** par [migration.sql](migration.sql), rejouable. Les décisions sont dans [research.md](research.md).

---

## 1. Ce qui existe et sert tel quel

| Objet | Usage |
|---|---|
| `negotiation.meetings` | **La session de négociation est une ligne de `kind = 'negotiation_session'`** (ADR-008). Rien ne la lit encore dans le code : l'étendre ne casse rien |
| `negotiation.spaces` (`climat`) | L'espace de rattachement de toutes les sessions importées |
| `event.events` (`cop31`, `Asia/Istanbul`) | L'édition visée par l'import ; son `timezone` est **le fuseau de la COP** |
| `reference.taxonomy_terms` et `metadata` | Vocabulaires ; `metadata.denominations` porte les formes sous lesquelles la source nomme un terme ([R4](research.md)) |
| `negotiation.theme_subscriptions` | « Mes thématiques » ; patron recopié pour les groupes |
| `negotiation.tg_check_term_taxonomy` | Garde le vocabulaire d'une colonne de terme |
| `platform.jobs`, `kernel::jobs` | Le travail d'import et celui de traduction, replanifiés par eux-mêmes |
| `platform.settings` | Le modèle de rédaction d'ADR-005 |

## 2. Deux vocabulaires nouveaux (`020_reference.sql`)

Inscrits dans `reference.taxonomies` (`is_system`), puis leurs termes.

| Taxonomie | Termes semés | `metadata.denominations` |
|---|---|---|
| `negotiation_meeting_type` | `plenary`, `contact_group`, `informal_consultations`, `informal_informals` (« Aparté »), `heads_of_delegation`, `presidency_consultation`, `mandated_event`, `group_coordination`, `negotiation_other` | `denominations` (fragments de titre), `source_categories` (catégories de la source qui l'admettent), `requires_title_match`, `default_for` — [R4](research.md). **`label.en` est le terme anglais passé au lexique** |
| `negotiation_group` | `african_group`, `ldc`, `g77_china`, `aosis`, `arab_group`, `lmdc`, `ailac`, `eu`, `eig`, `basic`, `umbrella_group`, `alba`, `grulac` | Les noms et sigles lus dans la source (« African Group », « AGN », « G77 & China », « LDCs », « SIDs »…) |
| `negotiation_theme` | *(existe, dix termes)* | — |

`is_system` ; aucun écran ne les modifie. Les libellés `fr`/`en` sont des **données** : jamais dans
un fichier i18n.

## 3. `negotiation.meetings` — colonnes ajoutées

Toutes **nulles** pour les réunions qui ne viennent pas de la source (étape 4) ; un `CHECK` exige les
cinq premières dès que `source_key` est posé.

| Colonne | Type | Rôle |
|---|---|---|
| `source_key` | `text` | Identifiant de la session **à la source**. `UNIQUE (event_id, source_key)` |
| `source_url` | `platform.url` | « Voir l'original » |
| `title_original` | `text` | Titre anglais tel que lu — **fait foi** (FR-022). `title` reçoit `{"en": title_original}` |
| `first_read_at` | `timestamptz` | Première lecture |
| `last_read_at` | `timestamptz` | Dernière lecture **où elle figurait** |
| `meeting_type_term_id` | `uuid` → `taxonomy_terms` | Gardé `negotiation_meeting_type` |
| `group_term_id` | `uuid` → `taxonomy_terms` | Gardé `negotiation_group` ; seulement pour une coordination |
| `agenda_item_id` | `uuid` → `negotiation.agenda_items` | Point de l'ordre du jour ; nul = « Hors ordre du jour officiel » |
| `is_open_access` | `boolean` | Ouverte / Accès limité |
| `absent_reads` | `smallint NOT NULL DEFAULT 0` | Lectures réussies de suite où elle manquait ; à **2** elle passe `cancelled` (FR-015) |
| `cancelled_at` | `timestamptz` | Heure du constat de l'annulation |

**Valeurs posées par l'import** (les colonnes existantes ont des défauts faits pour l'étape 4) :
`kind = 'negotiation_session'`, `space_id` = l'espace `climat` (lu par son slug), `slug =
<slug de l'édition>-<source_key>`, `format = 'onsite'` (sinon `ck_meetings_online_access` refuse),
`is_ifdd_organized = false` (FR-021), `timezone` = celui de l'édition, `status = 'scheduled'`,
`title = {"en": title_original}`. Le `CHECK` des colonnes de source se nomme
`ck_meetings_source_complete`.

**Contraintes retouchées** :

- `ck_meetings_period` devient `end_at IS NULL OR end_at > start_at`, et `end_at` perd `NOT NULL` —
  une session sans fin annoncée existe à la source. `ck_meetings_imported_end` garde `NOT NULL` pour
  toute réunion non importée : l'étape 4 n'en hérite pas.
- `ck_meetings_onsite_venue` exempte une ligne importée : la salle peut manquer à la source, et on
  n'en invente pas.
- `ck_meetings_cancellation` reste : l'import écrit `cancellation_reason` = `'source'` (« CANCELLED »),
  `'postponed'` (« POSTPONED ») ou `'removed'` (disparue) — des codes, non des libellés ;
  `ck_meetings_import_cancellation` borne ces trois valeurs quand `source_key` est posé.
- `tg_meeting_status_event` **n'émet rien pour une ligne importée** (FR-021) : l'événement
  `negotiation.meeting.cancelled` préviendrait des inscrits qui n'existent pas, et la notification de
  changement est de 3b.

**États** : `status` ne prend que `scheduled` ou `cancelled` pour une ligne importée. « En cours » et
« Terminée » se déduisent de l'heure ; « Déplacée » de l'existence d'un changement d'heure ou de salle
([R6](research.md)). **Aucune valeur d'ENUM ajoutée.**

**Index** : `ux_meetings_source (event_id, source_key) WHERE source_key IS NOT NULL` ;
`ix_meetings_event_day (event_id, start_at) WHERE kind = 'negotiation_session'`.

## 4. Tables nouvelles — sept (`100_negotiations.sql`)

### `negotiation.agenda_items` — les points de l'ordre du jour d'une COP

| Colonne | Type | Note |
|---|---|---|
| `id` | `uuid` PK | |
| `event_id` | `uuid` NOT NULL | `xmod_fk_agenda_items_event` → `event.events` |
| `code` | `text` NOT NULL | « SBI 12 », tel que lu. `UNIQUE (event_id, code)` |
| `title` | `text` NOT NULL | Intitulé anglais, extrait du titre de la première session qui le cite |
| `theme_term_id` | `uuid` | Gardé `negotiation_theme` ; posé au back-office (Q2) |
| `theme_set_by`, `theme_set_at` | | Qui l'a rattaché, quand |
| `first_read_at` | `timestamptz` NOT NULL | |

Un point **n'est jamais supprimé** par l'import : son rattachement est un travail de l'IFDD.

### `negotiation.meeting_changes` — l'historique des écarts

| Colonne | Type | Note |
|---|---|---|
| `id` | `uuid` PK | |
| `meeting_id` | `uuid` NOT NULL | → `meetings`, `ON DELETE CASCADE` |
| `field` | `text` NOT NULL | `CHECK (field IN ('start','end','venue','title','type','access','agenda_item','status'))` — liste close du code, pas un vocabulaire |
| `old_value`, `new_value` | `jsonb` | |
| `detected_at` | `timestamptz` NOT NULL | Heure de la lecture qui l'a vu |
| `import_run_id` | `uuid` | → `import_runs` |

« Avant → après » : pour chacun de `start`, `end`, `venue`, l'`old_value` de **son** dernier changement, sinon la valeur courante ; l'historique entier reste
pour 3b et le back-office. `ix_meeting_changes_meeting (meeting_id, detected_at DESC)`.

*Pas de table des documents liés : la source n'en donne aucun ([R2](research.md), FR-029).*

### `negotiation.official_imports` — l'import d'une COP

| Colonne | Type | Note |
|---|---|---|
| `id` | `uuid` PK | |
| `event_id` | `uuid` NOT NULL UNIQUE | `xmod_fk_official_imports_event` |
| `is_enabled` | `boolean NOT NULL DEFAULT false` | L'interrupteur (FR-018) |
| `reader` | `text NOT NULL DEFAULT 'archive'` | `CHECK (reader IN ('archive','live'))` |
| `archive_name` | `text` | Le jeu archivé lu ([R3](research.md)) |
| `archive_first_day` | `date` | Pose le premier jour de l'archive sur ce jour — la recette tourne « aujourd'hui » |
| `live_url` | `platform.url` | Le JSON du calendrier de conférence, quand l'accord ouvrira l'accès ([R2](research.md)) |
| `time_correction_minutes` | `smallint NOT NULL DEFAULT 60` | Les heures de la source valent l'heure locale moins une heure ([R2](research.md)) |
| `official_programme_url` | `platform.url NOT NULL` | Le renvoi « Programme officiel de la CCNUCC » |
| `interval_seconds` | `integer NOT NULL DEFAULT 300` | `CHECK (≥ 60)` |
| `missed_threshold` | `smallint NOT NULL DEFAULT 3` | `CHECK (≥ 1)` — FR-017 |
| `missed_reads` | `smallint NOT NULL DEFAULT 0` | Lectures manquées d'affilée |
| `last_success_at`, `last_attempt_at` | `timestamptz` | |
| `last_error` | `text` | Message français de la dernière lecture manquée |
| `last_change_count` | `integer` | Écarts de la dernière lecture réussie |
| `failing_since` | `timestamptz` | Première lecture manquée de la série — « n'a pas répondu depuis 06:40 » |
| `updated_by`, `updated_at` | | |

**`negotiation.import_is_serving(event_id) → boolean`** — la règle de coupure, **en un seul endroit** :
allumé, au moins une lecture réussie, `missed_reads < missed_threshold`, **et** dernière réussite
plus récente que `missed_threshold × interval_seconds + 60 s` ([R5](research.md) — un worker arrêté
ne manque aucune lecture, et ne doit pas laisser l'affichage ouvert).

### `negotiation.import_runs` — le journal

`id`, `import_id`, `started_at`, `finished_at`, `outcome text CHECK (outcome IN ('success','failure'))`,
`error text`, `session_count integer`, `change_count integer`, `is_manual boolean`. **`change_count` = nombre de
sessions touchées** (apparue, changée, absente, reparue), pas de lignes de `meeting_changes`. Purgé au-delà de 30 jours par le
travail d'import lui-même.

### `negotiation.title_translations` — une traduction par titre

| Colonne | Type | Note |
|---|---|---|
| `source_text` | `text` PK | Le titre anglais exact — deux sessions de même titre partagent une traduction |
| `text_fr` | `text` NOT NULL | |
| `model` | `text` NOT NULL | Le modèle d'OpenRouter qui l'a produite |
| `translated_at` | `timestamptz NOT NULL DEFAULT now()` | |

`COMMENT ON TABLE` inscrit l'**écart au principe XII** décidé le 25/09 : publiée sans relecture,
bornée aux titres de sessions officielles, toujours marquée « Traduction automatique ».

### `negotiation.group_subscriptions` — « Mon groupe »

Copie de `theme_subscriptions` : `person_id` (`xmod_fk_group_subscriptions_person`),
`group_term_id` gardé `negotiation_group`, `followed_at`, `left_at` ;
`ux_group_subscriptions_active (person_id, group_term_id) WHERE left_at IS NULL`.

### `negotiation.agenda_entries` — « Mon agenda »

| Colonne | Type | Note |
|---|---|---|
| `person_id` | `uuid` | `xmod_fk_agenda_entries_person` |
| `meeting_id` | `uuid` | → `meetings`, `ON DELETE CASCADE` |
| `remind_before` | `interval` | Nul = pas de rappel ; `CHECK (remind_before IS NULL OR remind_before = '15 minutes')` |
| `added_at` | `timestamptz NOT NULL DEFAULT now()` | |
| `updated_at` | `timestamptz NOT NULL DEFAULT now()` | |

PK `(person_id, meeting_id)`. **Pas `meeting_registrations`** : une inscription a une capacité, une
liste d'attente et un compteur ; suivre une session officielle n'engage aucune place ([R8](research.md)).
Le retrait **supprime** la ligne. Aucune contrainte de chevauchement (règle n° 2).

## 5. Réglage (`900_seed.sql`)

`ai.drafting_model` — le modèle de rédaction d'ADR-005, lu à chaque traduction. Et la ligne
`negotiation.official_imports` de `cop31`, **éteinte**, lecteur `archive`, `official_programme_url =
https://unfccc.int/cop31/schedule` — posée par `INSERT … SELECT … FROM event.events WHERE slug = 'cop31'
ON CONFLICT DO NOTHING` : sur une base sans cette édition, rien n'est posé.

## 6. Audit et contexte d'écriture

`tg_audit` sur `official_imports`, `agenda_items`, `agenda_entries`, `group_subscriptions` — les
gestes humains. **Pas sur `meeting_changes`, `import_runs`, `title_translations`** : ce sont déjà des
journaux ; `meetings` l'a déjà, et le travail d'import écrit sous `job.context()`.

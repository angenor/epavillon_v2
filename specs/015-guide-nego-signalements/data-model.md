# Data Model — Guide Négo, signalements et notifications (étape 3b)

SQL d'abord dans `docs/database/` (`100_negotiations.sql`, `110_engagement.sql` pour les types,
`030_identity.sql` ou `100_negotiations.sql` pour la permission, selon où vivent les voisines), puis
[migration.sql](migration.sql) rejouable sur `epavillon_dev2`. Décisions : [research.md](research.md).

## 1. `negotiation.report_status` — ENUM (machine à états)

`submitted` → `validated` | `rejected` ; `validated` → `submitted` (annulation dans la fenêtre, R3).

## 2. `negotiation.session_reports`

| Colonne | Type | Note |
|---|---|---|
| `id` | `uuid` PK | |
| `author_id` | `uuid` NOT NULL | `xmod_fk_session_reports_author` → `identity.people` |
| `client_ref` | `uuid` NOT NULL | `ux_session_reports_client_ref (author_id, client_ref)` — R5 |
| `event_id` | `uuid` NOT NULL | `xmod_fk_session_reports_event` — la COP |
| `meeting_id` | `uuid` | → `meetings` ; nul pour `unannounced` |
| `reason` | `text` NOT NULL | `CHECK (reason IN ('cancelled','time','venue','other','unannounced'))` ; `ck_session_reports_target` : `meeting_id` nul **si et seulement si** `unannounced` |
| `proposed_start` | `timestamptz` | `time`, et début d'une `unannounced` |
| `proposed_venue` | `text` | `venue`, et « Où » d'une `unannounced` |
| `what` | `text` | « Quoi » — requis pour `unannounced` (`ck_session_reports_unannounced`) |
| `proposed_day` | `date` | le jour de la liste — requis pour `unannounced` (même contrainte) |
| `theme_term_id` | `uuid` | gardé `negotiation_theme` ; `unannounced` seulement |
| `detail` | `text` | `CHECK (char_length(detail) <= 600)` |
| `status` | `negotiation.report_status` NOT NULL DEFAULT `submitted` | |
| `submitted_at` | `timestamptz` NOT NULL DEFAULT now() | |
| `decided_by`, `decided_at` | | `xmod_fk_session_reports_decider` |
| `reject_reason` | `text` | `CHECK (reject_reason IN ('source_maintains','already_known','not_precise'))`, requis si `rejected` |
| `reject_detail` | `text` | ≤ 600 |
| `source_snapshot` | `jsonb` | ce que disait la source à la décision (ADR-010) |
| `published_at` | `timestamptz` | **seule porte du public** : encart, réunion non annoncée, « Validé », notifications et courriels n'existent qu'une fois posé (R3) |
| `withdrawn_at` | `timestamptz` | |
| `withdrawal` | `text` | `CHECK (withdrawal IN ('caught_up','admin'))` ; la fin de session n'écrit rien (R7) |
| `network_meeting_id` | `uuid` | → `network_meetings`, posé à la validation d'une `unannounced` |

`ck_session_reports_decision` : `submitted` ⇒ `decided_*`, `source_snapshot`, `published_at` nuls ;
`rejected` ⇒ `reject_reason` posé ; `published_at` ⇒ `status = 'validated'`. **Aucun déclencheur
d'outbox sur cette table** : les événements naissent du travail de publication et du refus.
Pas de colonne `updated_at` (les heures métier suffisent).

Index : `ux_session_reports_pending (author_id, meeting_id) WHERE status = 'submitted' AND meeting_id
IS NOT NULL` (R5) ; `ix_session_reports_queue (submitted_at) WHERE status = 'submitted'` ;
`ix_session_reports_meeting (meeting_id) WHERE published_at IS NOT NULL AND withdrawn_at IS NULL`.
Trigger : audit.

## 3. `negotiation.network_meetings`

| Colonne | Type | Note |
|---|---|---|
| `id` | `uuid` PK | |
| `event_id` | `uuid` NOT NULL | `xmod_fk_network_meetings_event` |
| `report_id` | `uuid` NOT NULL UNIQUE | → `session_reports` ; créée par le travail de publication, jamais avant |
| `title` | `text` NOT NULL | « Quoi » |
| `venue` | `text` | « Où » |
| `start_at` | `timestamptz` | nul = sans heure |
| `day` | `date` NOT NULL | le jour de la liste, dans le fuseau de la COP |
| `theme_term_id` | `uuid` | gardé `negotiation_theme` |
| `validated_at` | `timestamptz` NOT NULL | |
| `withdrawn_at` | `timestamptz` | retrait par l'administration ; la fin du jour se calcule |

Aucune colonne de source : ce n'est pas une session officielle. Audit.

## 3 bis. Destinataires — une seule règle

`negotiation.change_recipients(meeting_id uuid) RETURNS SETOF uuid` : personnes de `agenda_entries` sur
la session ∪ personnes dont un suivi actif de la thématique de la session (par son point de l'ordre du
jour) a `notify_changes`. `negotiation.network_recipients(network_meeting_id uuid)` :
`network_agenda_entries` ∪ thématique allumée. STABLE. Appelées **par `negotiation` seul** — l'import, le travail de publication, le travail de
courriel ; `engagement` reçoit la liste dans l'événement (R8).

## 4. Colonnes ajoutées

- **`agenda_entries` ne change pas** (3a reste intact) : la réunion non annoncée a sa table sœur
  `negotiation.network_agenda_entries` (`person_id` `xmod_fk_network_agenda_entries_person`,
  `network_meeting_id` → `network_meetings` `ON DELETE CASCADE`, `remind_before` au même `CHECK` que 3a,
  `added_at`, `updated_at`), PK `(person_id, network_meeting_id)`, audit.
- `negotiation.theme_subscriptions` : `notify_changes boolean NOT NULL DEFAULT false` (R11).

## 5. Permission

`negotiation.report.validate` (portée globale), attribuée au rôle `admin` — à côté des permissions de
`negotiation` semées dans `100_negotiations.sql`.

## 6. Types de notification (`110_engagement.sql`, semis)

`negotiation.meeting.changed`, `negotiation.report.published`, `negotiation.network_meeting.published`,
`negotiation.report.decided` — `module_code = 'negotiation'`, `default_channels = {in_app}`,
`criticality = 'normal'` (R8).

## 7. Accord

`identity.consents`, `purpose = 'guide_nego_notifications'` — aucune table nouvelle ; aucune ligne
= allumé (R10).

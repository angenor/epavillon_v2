# Data Model — Guide Négo, Réunions de la Francophonie (étape 4)

SQL d'abord (`020_reference.sql`, `100_negotiations.sql`, `110_engagement.sql`), puis
[migration.sql](migration.sql) rejouable sur `epavillon_dev2`. Décisions : [research.md](research.md).

## 1. Vocabulaire `francophone_meeting_type` (`020_reference.sql`)

Termes `preparatory_workshop` (« Atelier préparatoire »), `negotiators_consultation` (« Concertation des
négociatrices et négociateurs »), `ministerial_consultation` (« Concertation ministérielle »), FR/EN,
`is_system`.

## 2. `negotiation.meetings` — colonnes ajoutées

| Colonne | Type | Rôle |
|---|---|---|
| `francophone_type_term_id` | `uuid` → `taxonomy_terms` | gardé `francophone_meeting_type` ; requis hors brouillon pour `preparatory_workshop` et `francophone_consultation` saisies (`ck_meetings_francophone_type`) |
| `access_audience` | `platform.i18n_text` | public d'un accès limité ; requis si `is_open_access = false` sur une réunion saisie |
| `pavilion_session_id` | `uuid` | `xmod_fk_negotiation_meetings_pavilion_session` → `programme.sessions(id) ON DELETE SET NULL` |
| `requires_registration` | `boolean NOT NULL DEFAULT true` | sans inscription : ni capacité ni liste d'attente, lien visible des personnes admises |
| `waitlist_enabled` | `boolean NOT NULL DEFAULT true` | |

Contraintes ajoutées (réunions saisies des deux natures) : `ck_meetings_francophone_event` (`event_id`
non nul), `ck_meetings_francophone_kind` (nature ↔ `kind`), `ck_meetings_access_audience`
(`is_open_access` non nul, `access_audience` requis s'il est faux) ; trigger
`tg_check_meeting_pavilion_edition` (activité de la même édition). Valeurs posées par le serveur :
research R9 bis.

Commentaire de `is_open_access` élargi (R3). Index `ix_meetings_francophonie (event_id, start_at) WHERE
kind IN ('preparatory_workshop','francophone_consultation') AND status <> 'draft'`.

## 3. `negotiation.meeting_registrations` — colonnes ajoutées

| Colonne | Type | Rôle |
|---|---|---|
| `waitlist_position` | `integer` | `ck_meeting_registrations_waitlist : (status = 'waitlisted') = (waitlist_position IS NOT NULL)` |
| `client_ref` | `uuid` | `ux_meeting_registrations_client_ref (person_id, client_ref)` |

Triggers ajoutés : `tg_validate_meeting_registration` (BEFORE INSERT OR UPDATE OF status — R4 : contrôles
seulement à l'entrée, comptage par `count(*)` sous verrou, jamais `registered_count`) ; audit.
**Attention** : chaque inscription met à jour `meetings.registered_count` (trigger existant) — l'édition
au back-office ne se garde pas sur `updated_at`.

## 4. Fonction

`negotiation.promote_meeting_waitlist(p_meeting_id uuid) RETURNS SETOF uuid`
— sous verrou de la réunion, promeut `capacity − count(registered)` personnes (toutes si la capacité
est nulle), recompacte les positions, rend les personnes promues. Le paramètre `p_count` disparaît.

## 5. Types de notification (`110_engagement.sql`)

`negotiation.francophone_meeting.changed`, `negotiation.meeting_registration.promoted` —
`module_code = 'negotiation'`, `{in_app}`, `normal`. **Ne pas semer** `negotiation.meeting.published` ni
`.cancelled` (émis par `tg_meeting_status_event`, sans charge `notification`).

## 6. Destinataires

`negotiation.meeting_audience(p_meeting_id uuid) RETURNS SETOF uuid` — inscrites et liste d'attente
(non annulées). STABLE. Appelée par `negotiation` seul.

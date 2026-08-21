# Phase 1 — Le modèle existant, et ce que le code en fait : Sessions (B5)

> **Aucun DDL.** Ce document ne décrit pas des tables à créer : il dit ce que le module lit, ce qu'il écrit, ce que la base garantit déjà, et — le plus important — **ce qu'elle ne garantit pas** et qui incombe donc au service. La source de vérité reste `docs/database/075_programme_sessions.sql`.

---

## 1. Ce que le module écrit dans son schéma

| Table | Écritures de ce jalon | Ce qui l'écrit aussi |
|---|---|---|
| `programme.sessions` | naissance à l'acceptation · créneau et salle · diffusion et canal · journée de rattachement · date de publication et passage en « programmé » | quatre déclencheurs : `updated_at`, audit, dérivation, émission d'événements, plus la synchronisation du porteur |
| `programme.session_speakers` | recopie à la naissance | — |
| `programme.session_organizations` | recopie des co-organisations à la naissance ; **jamais la ligne du porteur** | `tg_sessions_sync_lead_organization()` la pose et la déplace |
| `programme.session_tracks` | remplacement de la liste des journées spéciales | `tg_session_tracks_check_event()` refuse un fil d'une autre édition |
| `programme.registrations` | inscription, annulation, promotion, première présence | `tg_validate_registration()`, `tg_registrations_emit_events()`, `updated_at` |

**Non écrites par ce jalon** : `programme.registration_forms` et `registration_form_fields` (lues seulement — le back-office qui les compose n'existe pas), `session_questions` et ses deux tables satellites (hors périmètre), `sessions.report`, `report_submitted_at`, `attendee_count`, `view_count`, `cancelled_reason` (hors périmètre, aucun écran ne les écrit).

---

## 2. Les quatre valeurs dérivées, et leur régime exact

C'est le cœur de l'écart n° 7, et sa lecture ligne à ligne le corrige (écarts n° 111 et n° 112).

| Colonne | Comment elle est produite | Se déclenche sur | Contrat d'écriture |
|---|---|---|---|
| `time_range` | `GENERATED ALWAYS AS (tstzrange(starts_at, ends_at, '[)')) STORED` | — | **Refusée**, champ nommé. PostgreSQL refuse déjà, mais brutalement |
| `enforce_room_exclusivity` | déclencheur, depuis `event.rooms.is_virtual` ; **faux** quand il n'y a pas de salle | `room_id`, `starts_at`, `event_id`, `is_streamed`, `broadcast_channel_id` — **jamais sur elle-même** | **Refusée**, champ nommé. Une valeur envoyée seule ne serait **pas corrigée** : elle tiendrait |
| `broadcast_channel_id` | déclencheur, **seulement si nul** et diffusion activée : canal par défaut de l'édition, à défaut celui de la plateforme. **Effacé** si la diffusion est retirée | mêmes colonnes | **Acceptée** quand la diffusion est activée · **refusée**, champ nommé, quand elle est retirée |
| `event_day_id` | déclencheur, **seulement si nulle** : le jour civil de `starts_at` dans le fuseau de l'édition | mêmes colonnes | **Acceptée et facultative.** Non fournie, le service la remet à nul pour que la déduction ait lieu (R9) |

**Le piège du déclencheur de dérivation, en une phrase** : il ne complète que ce qui est **nul**, et il ne se réveille que sur **cinq colonnes**. Tout ce qui n'est ni nul ni dans cette liste lui échappe.

---

## 3. Ce que la base **ne** garantit **pas**, et qui incombe au service

C'est la liste qui compte : chaque ligne est une règle que le code doit tenir, et l'oublier ne produit aucune erreur.

| # | Règle | Pourquoi la base ne la porte pas | Écart |
|---|---|---|---|
| 1 | Une séance naît à l'acceptation d'un dossier | Aucun déclencheur, aucune fonction : la table n'est peuplée que par une insertion | n° 57 |
| 2 | La journée de rattachement suit une séance déplacée | Le déclencheur ne déduit que si la colonne est nulle | **n° 113** |
| 3 | Les réponses obligatoires sont vérifiées même sans formulaire **attaché** | Le contrôle est gardé par `IF v_session.registration_form_id IS NOT NULL` | **n° 114** |
| 4 | Le type, les options et les bornes d'une réponse | La base ne connaît que la **présence** d'une réponse obligatoire | n° 6 |
| 5 | Une clé de réponse inconnue est refusée | Le document est libre : toute clé y entre | n° 6 |
| 6 | Une réponse sensible exige un consentement, et le consentement est conservé | `is_sensitive` est une marque, sans effet | — |
| 7 | Les inscriptions ne sont pas ouvertes avant leur date d'ouverture | Le déclencheur ne vérifie que la clôture | **n° 115** |
| 8 | Une séance qui ne prend pas d'inscription en refuse | `registration_required` n'est lu par personne | **n° 115** |
| 9 | La jauge tient sous concurrence | `count(*)` sans verrou, sous `READ COMMITTED` | **n° 124** |
| 10 | Les positions d'attente sont uniques et sans trou | `max(...) + 1` sans verrou, aucun index unique | **n° 124** |
| 11 | La promotion depuis l'attente ne dépasse pas la jauge | Le contrôle de capacité est borné à l'insertion | **n° 116** |
| 12 | Une séance publiée passe de « pressentie » à « programmée » | Rien ne pose l'état ; seule la date était prévue | R12 |
| 13 | La publication s'applique **une seule fois** par annonce | Portée par le noyau, pas par la base | — |
| 14 | Le décompte rendu à l'organisation ne porte aucun nom | Aucune vue ne distingue les deux lectures | n° 36 |
| 15 | Une salle, un canal, un fil désignés appartiennent bien à l'édition | Seul le fil est contrôlé (par déclencheur) ; ni la salle ni le canal | — |

---

## 4. Ce que la base garantit, et que le service **traduit** sans le rejouer

| Garantie | Où | Ce que le service en fait |
|---|---|---|
| `ck_sessions_period` — la fin après le début | contrainte | Traduit sur le champ de fin |
| `ck_sessions_cancelled_reason` — un motif si annulée | contrainte | Hors périmètre : ce jalon n'annule pas de séance |
| `ux_sessions_slug` — adresse unique par édition | contrainte | Le service dérive et suffixe (R7) |
| `ux_sessions_proposal_sequence` — un rang par dossier | contrainte | **Porte l'idempotence de la naissance** (R6) |
| `tg_session_tracks_check_event` — un fil de la même édition | déclencheur | Traduit en code stable et message français |
| `ck_registrations_waitlist` — position ⇔ état d'attente | contrainte | Jamais écrite à la main : la base la pose |
| `ux_registrations_person_session` — une inscription vivante par personne | index partiel | Traduit en refus nommé ; une réinscription après annulation passe |
| Bascule automatique en liste d'attente | déclencheur | **Laissée faire** ; le service lit l'état obtenu et le rend |
| Refus de jauge sans liste d'attente | déclencheur | Traduit, en portant le nombre de places |
| Refus d'inscription à une séance annulée, et après clôture | déclencheur | Traduits ; voir l'écart n° 125 pour ce qu'ils interdisent aussi |
| Émission des événements de séance et d'inscription | déclencheurs | **Rien.** Le service n'émet pas (R2) |
| Synchronisation du porteur de séance | déclencheur | **Rien.** Le service ne pose que les co-organisations |
| Audit champ par champ | déclencheur | Alimenté par le contexte d'écriture du noyau |

---

## 5. Les lectures et écritures hors du schéma `programme`

**Lectures ajoutées à `repo/cross/`** — la règle reste celle de B2 : *un module lit hors de son schéma quand la question porte sur ses propres entités.*

| Lecture | Question de **ce** module |
|---|---|
| `event.events` — **+ `starts_at`, `programme_published_at`** sur la lecture existante | « quand commence l'édition, son programme est-il paru ? » |
| `event.event_days` | « quelles colonnes de jours le planificateur affiche-t-il ? » |
| `event.rooms` | « où cette séance est-elle installée, et occupe-t-elle le stand ? » |
| `event.programme_tracks` | « quelles journées spéciales peut-on lui rattacher ? » |
| `event.broadcast_channels` | « quel canal cette séance occupe-t-elle ? » |
| `event.calls_for_proposals` (déjà autorisée) | « quelle durée et quelle heure d'ouverture pour une séance naissante ? » |
| `org.organizations` + `reference.countries` (déjà autorisées) | « qui porte cette séance, et d'où ? » |
| `identity.people` (déjà autorisée) | « qui intervient, qui est inscrit ? » |
| `reference.taxonomy_terms`, `terms_of()`, `term_badges()` (déjà autorisées) | les thématiques d'une séance, et les options d'un champ adossé à une taxonomie |
| `reference.countries.iso2` | la validation d'une réponse « pays » (R18) |
| `reference.locales` | la langue d'une inscription |

**Écritures hors schéma — la troisième arrive, et elle est bornée**

| Table | Fichier | Statut |
|---|---|---|
| `reference.entity_terms` | `repo/themes.rs` (**modifié** : une seconde entité, `sessions`) | Dérogation déjà assumée en B3 et B4. Le triplet reste écrit **littéralement**, jamais reçu |
| `identity.people` | `repo/people.rs` (**inchangé**) | Précédent livré en B2 et B4. Réutilisé tel quel pour l'inscrit sans compte |
| `identity.consents` | `repo/consents.rs` (**nouveau**) | Troisième dérogation, justifiée en R22 et au « Complexity Tracking » du plan. Une seule finalité, une seule table, jamais rien d'autre |

---

## 6. Les traversées de type SQLx

Les quatre de B1 à B4, inchangées : énumération en `text` (`status::text`, annoté `AS "status!"`), `platform.i18n_text` en `jsonb`, `numeric` en `float8`, domaines (`platform.slug`, `platform.timezone_name`, `platform.email`) par double transtypage.

**Deux ajouts propres à ce jalon**

- **`tstzrange` se lit en `text`** — `time_range::text`, `overlap::text`. Le contrat du front déclare une chaîne, et les données simulées écrivent la représentation textuelle de PostgreSQL. Composer cette chaîne côté Rust depuis un `PgRange` reviendrait à réécrire ce que la base rend déjà.
- **`answers`, `options` et `validation` restent des `serde_json::Value`** — c'est la conséquence voulue du formulaire configurable (écart n° 6), et non une lacune de typage.

**Rappel de B3, qui vaut ici deux fois** : toute colonne lue depuis une **vue** est rendue nullable par SQLx — une vue ne porte aucune contrainte de nullité et le vérificateur le suppose. `v_public_schedule` et `v_edition_stats` s'annotent colonne par colonne.

---

## 7. Les formes rendues, et d'où vient chaque champ

Le contrat du front est la référence ; ce tableau dit seulement d'où chaque valeur non triviale est tirée.

| Forme | Champ | Source |
|---|---|---|
| `PlannerSession` | `room_name` | `event.rooms.name` |
| | `organization_name`, `_acronym`, `_country_code` | `org.organizations` + `reference.countries.iso2` |
| | `reference_code`, `average_score`, `requested_duration_minutes`, `preferred_start_at`, `scheduling_constraints` | `programme.proposals` — même schéma |
| | `track_ids` | `programme.session_tracks` |
| | `themes` | `reference.term_badges('programme', 'sessions', …, 'activity_theme')` |
| | `speaker_count` | décompte sur `programme.session_speakers` |
| `PlannerScreen` | `timezone`, `zone_label`, `programme_published_at` | `event.events.timezone`, `city`, `programme_published_at` |
| `ScheduleConflict` | toutes | `programme.detect_conflicts(event_id)`, telle quelle |
| `PublicScheduleRow` | toutes | `programme.v_public_schedule`, telle quelle |
| `TrackedSession` | `registered_count` | inscriptions `registered` **et** `attended` — le même prédicat que la vue publique et que le déclencheur de jauge |
| | `waitlisted_count` | inscriptions `waitlisted` |
| | `capacity` | `programme.sessions.capacity` |
| | `reminders` | **vide** jusqu'à B6 |
| `RegistrationFormField` | `options` | tel quel, **plus** les codes résolus quand une taxonomie est visée |

**Un décompte, une définition.** `registered_count` compte `registered` et `attended`, exactement comme `v_public_schedule` et comme le contrôle de jauge du déclencheur. Trois définitions différentes du même mot produiraient trois chiffres, et c'est l'organisation qui s'en apercevrait.

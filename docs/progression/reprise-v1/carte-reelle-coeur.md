# Carte réelle de Supabase : le cœur (comptes, organisations, événements, activités)

> Relevé du 27/10/2026 sur `sauvegardes/reprise-v1/supabase-v1-20261027-0419.dump` (étape 2 du [plan](plan.md)). Périmètre : `auth.users`, `auth.identities`, `users`, `user_roles`, `countries`, `organizations`, `organization_validations`, `events`, `activities` et ses tables filles. Les autres tables sont relevées à part. Aucune donnée personnelle ici : des comptes, des taux, des valeurs de vocabulaire. « nul » = `NULL` ; « vide » = chaîne vide, que la v1 laisse souvent à la place d'un `NULL`.

## 0. Ce qui change le plan

| Constat | Effet sur le plan |
|---|---|
| **4 100 comptes**, pas 2 556 : 2 020 importés de Laravel le 28/09/2025, 639 créés par le formulaire PACO en mars 2026, 1 441 autres | D1 et D9 à recouper : que deviennent les comptes PACO ? |
| Les 2 020 comptes Laravel sont **tous marqués vérifiés à la date de l'import**, sans lien cliqué ; 97 se sont connectés depuis | le chiffre « vérifiés » de D1 (3 635) en compte 2 020 de fait |
| La vraie date d'inscription vit dans `public.users.created_at` (2021 à 2026), celle d'`auth.users` est celle de l'import | `people.created_at` vient de `public.users` |
| Une seule identité par compte, **toutes `email`** : ni Google ni Microsoft | aucun compte fédéré à reprendre |
| `activities.activity_status` vaut `draft` sur les 259 lignes : colonne morte. **Tout l'état est dans `validation_status`** | plus de « deux statuts mêlés » : un seul, qui mêle décision et déroulé (`live`, `completed`) |
| Aucune ancienne thématique (`climate_resilience`…) : les 17 valeurs de l'ENUM actuel, dont 4 sans terme v2 et `finance` ≠ `climate_finance` | D6 réduit à cinq termes |
| **D8 non confirmé** : les heures COP31, lues à Istanbul, tombent sur la même grille que la COP30 lue à Belém (9 h à 17 h 45) ; aucun décalage de 3 h dans l'historique | aucune correction des activités Supabase |
| COP31 : **aucune activité acceptée**, aucune évaluation, aucune inscription ; 78 dossiers en évaluation ont déjà un créneau | les créneaux doivent vivre sans proposition acceptée |
| **14 241 des 14 346 inscriptions sont au webinaire PACO** (D9, non repris) ; il en reste 105, COP30 | lot 7 minuscule ; Zoom sans objet pour la reprise |
| Intervenants : **aucun sans adresse** dans Supabase, mais 43 adresses portées par plusieurs personnes ; 173 lignes seraient rattachées à la mauvaise personne par l'adresse | D16 change de forme : rapprocher par adresse **et** nom |
| `organization_requests`, `event_registrations`, `activity_panelists` n'existent pas en production ; `organization_aliases`, `activity_speaker_connections` sont vides | rien à reprendre |
| `organization_validations` n'est pas la validation par l'IFDD (portée par `is_verified`) mais une attestation par des membres | une table v2 à créer |

## 1. Comptes : `auth.users`, `auth.identities`, `public.users`

4 100 lignes dans chacune, mêmes identifiants des deux côtés : aucun compte d'un seul côté, aucune identité orpheline. Aucun doublon d'adresse sans tenir compte de la casse (toutes en minuscules, `public.users.email` = `auth.users.email` partout). L'adresse du commanditaire est présente : **oui**.

### Origine des comptes

| Origine | Comptes | `email_confirmed_at` | Connectés une fois au moins |
|---|---|---|---|
| Import Laravel (`is_migrate`, `laravel_user_id`) | 2 020 | 2 020, tous datés de l'import (28/09/2025, un au 03/08/2025) | 97 |
| Formulaire PACO (`raw_user_meta_data.source = 'paco'`), mars 2026 | 639 | 300 | 300 |
| Inscription sur la v1 | 1 441 | 1 315 | 1 315 |
| **Total** | **4 100** | **3 635** (465 non vérifiés, dont 339 PACO) | **1 712** |

### `created_at` par année

| Année | `auth.users` | `public.users` |
|---|---|---|
| 2021 | | 266 |
| 2022 | | 471 |
| 2023 | | 438 |
| 2024 | | 771 |
| 2025 | 2 386 | 440 (dont 74 Laravel) |
| 2026 | 1 714 | 1 714 |

Pour les comptes non Laravel, les deux dates coïncident à la minute (2 080 sur 2 080).

### `auth.users` (34 colonnes)

| Colonne v1 | Type, nul | Remplie | v2 | Règle |
|---|---|---|---|---|
| `id` | uuid | 4 100 | `legacy.id_map` | nouvel `identity.people.id` |
| `email` | varchar | 4 100 | `identity.people.primary_email` | tel quel (déjà en minuscules) |
| `encrypted_password` | varchar | 4 100, toutes bcrypt `$2a$` | aucune | non reprise (D1) : `accounts.password_hash` nul, `password_reset_required_at` posé |
| `email_confirmed_at` | timestamptz, nul | 3 635 | `identity.people.email_verified_at` | tel quel (D1) ; voir écart « vérification Laravel » |
| `last_sign_in_at` | timestamptz, nul | 1 712 | `identity.accounts.last_login_at` | tel quel |
| `created_at` | timestamptz | 4 100 | `identity.accounts.created_at` | tel quel (date d'ouverture du compte Supabase) |
| `raw_user_meta_data` | jsonb | 4 100 | `legacy.id_map` | `source = 'paco'` (639) et `is_migrated` servent au tri, rien d'autre à reprendre |
| `raw_app_meta_data` | jsonb | 4 100 | aucune | `provider = email` partout |
| `confirmation_token`, `confirmation_sent_at`, `recovery_*`, `email_change*`, `reauthentication_*` | varchar, timestamptz | jetons non vides : 465 (les non vérifiés) | aucune | jetons techniques, périmés : non repris |
| `phone`, `phone_confirmed_at`, `banned_until`, `deleted_at`, `invited_at`, `is_super_admin` | | 0 | aucune | toujours nuls |
| `aud`, `role`, `instance_id`, `is_sso_user`, `is_anonymous` | | constantes (`authenticated`, faux) | aucune | sans information |

`auth.identities` : 4 100 lignes, une par compte, `provider = 'email'` partout ; aucun compte à plusieurs fournisseurs. Rien d'autre à reprendre : `identity.accounts.provider = 'password'`, `provider_subject` nul.

### `public.users` (29 colonnes)

| Colonne v1 | Type, nul | Remplie | v2 | Règle |
|---|---|---|---|---|
| `email` | text NOT NULL | 4 100 | `people.primary_email` | identique à `auth.users.email` |
| `email_pro` | text, nul | 64 (39 identiques à `email`) | `identity.person_emails` (`label = 'professional'`) | les 25 différentes ; aucune n'est l'adresse d'un autre compte |
| `first_name`, `last_name` | text NOT NULL | 4 100, aucun vide | `people.first_name`, `last_name` | `btrim` |
| `phone` | text, nul | 86 (61 au format international) | `people.phone` | tel quel |
| `biography` | text, nul | 70 | `people.biography` (`i18n_text`) | `{"fr": …}` |
| `profile_photo_url`, `_thumbnail_url` | text, nul | 22, Supabase | `media.assets` + attachement `avatar` | la vignette se recalcule |
| `country_id` | uuid, nul | 135 | `people.country_id` | par code ISO (§ 3) |
| `organization_id` | uuid, nul | 317 | `org.memberships` + `people.primary_organization_id` | § 4 ; fonction exigée (D5) |
| `address` | text, nul | 77 | **aucune** | extension |
| `is_organization_verified` | bool | faux partout | aucune | sans information |
| `display_mode` | text | `light` 4 096, `dark` 4 | **aucune** | extension |
| `notification_preferences` | jsonb | 4 082 au défaut court, 17 tout vrai, 1 avec des refus | `engagement.notification_preferences` | une ligne par refus, pour le seul compte qui en a |
| `networking_visibility` | bool | faux : 1 | `people.is_directory_visible` | tel quel |
| `is_blocked`, `is_suspended`, `blocked_*`, `suspended_*` | | 0 | `people.status` | `active` partout |
| `created_at` | timestamptz | 4 100 | `people.created_at` | **la vraie date d'inscription**, Laravel compris |
| `updated_at` | timestamptz | 4 100 | `people.updated_at` | tel quel |
| `is_migrate`, `laravel_user_id` | bool, bigint | 2 020 | `legacy.id_map` | garder le lien pour la future reprise Laravel (2021 à 2024) |

Colonnes v2 sans source : `civility`, `job_title`, `city` (nuls), `preferred_locale` (`fr`), `timezone` (`UTC`).

## 2. Rôles : `user_roles`

18 lignes, 10 personnes ; 4 090 comptes n'ont aucun rôle (`standard` implicite).

| Rôle v1 | Lignes | Actives, échues | v2 (`identity.role_assignments`) |
|---|---|---|---|
| `revisionniste` | 8 | 6 bornées dans le temps (2 échues : 25/11/2025, 25/01/2026), 2 sans fin | `reviewer`, portée globale |
| `admin` | 3 | toutes valides jusqu'en 2028 à 2040 | `admin`, globale |
| `super_admin` | 3 | idem | `super_admin`, globale |
| `paco` | 3 | valides jusqu'en 2027 et 2028 | aucune (D9) ; voir écart PACO |
| `negotiator` | 1 | `is_active = false`, échu le 25/08/2025 | `negotiator`, globale, `revoked_at` posé |

Les trois `admin` sont les trois `super_admin` et trois des `revisionniste`. Règles : `assigned_at` → `granted_at` et `valid_from` ; `valid_until` tel quel (toujours postérieur) ; `assigned_by` → `granted_by` ; `is_active = false` → `revoked_at = valid_until`.

## 3. Pays : `countries`

209 lignes, `code` ISO alpha-2 en majuscules, unique. Non reprise : `reference.countries` est semée (249 codes) ; on fait correspondre par `code` = `iso2`. Seul absent de la v2 : `XK` (Kosovo), utilisé nulle part. 87 pays utilisés (personnes, organisations, activités, invités, événements), tous présents. `continent` (`Africa`…) et `is_francophone` (47) ne sont pas repris : la v2 porte `continent` et `oif_status` semés.

## 4. Organisations : `organizations`, `organization_validations`

### `organizations` (19 colonnes, 236 lignes)

| Colonne v1 | Type, nul | Remplie | v2 (`org.organizations`) | Règle |
|---|---|---|---|---|
| `name` | text NOT NULL | 236 | `legal_name` + `organization_names` (`legal`) | `btrim` |
| `acronym` | text, nul | 201 (13 nuls, 22 vides, 1 d'une lettre) | `acronym` + `organization_names` (`acronym`) | vide → nul ; une lettre refusée par `CHECK` (2 à 32) → nul, noté dans `id_map` |
| `email` | text NOT NULL, unique | 236 (128 sur domaine public, 144 égales à l'adresse d'un compte) | `contact_email` ; domaine non public → `organization_domains` non vérifié | |
| `email_verified` | bool | 40 | aucune | identique à `is_verified` ligne à ligne |
| `organization_type` | ENUM NOT NULL | 236 | `organization_type_code` | § 4.3 |
| `logo_url` | text, nul | 122 Supabase (4 nuls, 110 vides) | `media.assets` + attachement `logo` | |
| `website` | text, nul | 173 en `http(s)` (6 nuls, 57 vides, 3 avec espaces) | `website` (`platform.url`) | vide → nul ; espaces retirés, sinon nul |
| `country_id` | uuid, nul | **236**, aucun nul | `country_id` | par code ISO |
| `description` | text, nul | 213 (7 nuls, 16 vides) | `description` (`{"fr": …}`) | vide → nul |
| `is_verified`, `verified_by`, `verified_at` | | 40 (vérifiées par les comptes administrateurs) | `status = 'active'`, `verified_by`, `verified_at` | les 196 autres : `candidate` |
| `is_active`, `is_duplicate`, `duplicate_of` | | vrai, faux, nul partout | aucune | sans information |
| `created_by`, `created_at`, `updated_at` | | 236 | idem | `created_by` par `id_map` |

Créées de août à novembre 2025 (105) et d'août à octobre 2026 (131). 231 ont au moins un membre, 5 aucun ; 168 portent une activité (COP31 115, COP30 86), 68 aucune. Le créateur est membre de sa fiche dans 230 cas : rôle `manager` proposé pour lui, `member` pour les autres.

### Doublons probables

| Critère | Résultat |
|---|---|
| Même nom normalisé (`normalize_label`), avec ou sans pays | 0 |
| Paires candidates (même domaine non public, même sigle, nom proche à 85 %) | 12 paires, 15 fiches : domaine 7, sigle 2, nom proche 2, sigle et nom 1 ; 4 dans le même pays |

Peu de doublons internes : le risque principal est le rapprochement avec les fiches que la v2 aura (production purgée, D2) et entre éditions, à l'étape 5.

### Types (`organization_type`)

| v1 | Fiches | v2 (`organization_type`) |
|---|---|---|
| `ngo_association` | 129 | `ngo_association` |
| `private_sector` | 48 | `private_sector` |
| `public_national_institution` | 28 | `public_national_institution` |
| `international_organization` | 23 | `international_organization` |
| `regional_organization` | 8 | `regional_organization` |

### `organization_validations` (77 lignes)

`organization_id`, `validated_by`, `validated_at`, unique par (organisation, validateur). 56 organisations, 48 validateurs ; 70 lignes par des comptes sans rôle, 18 par un membre de l'organisation, 16 par son créateur. Sans lien avec `is_verified` (51 organisations attestées non vérifiées, 35 vérifiées sans attestation). C'est une **attestation par des pairs**, absente de la v2 (`compute_trust_score` ne l'a pas) : extension.

## 5. Événements : `events`

| | CdP 30 | CdP31 | PACO |
|---|---|---|---|
| Intitulé | 30ème Conférence Des Parties | 31e conférence des Nations Unies sur le climat | PACO - Webinaire |
| `year` | 2025 | 2026 | 2026 |
| Mode | `in_person` | `in_person` | `online` |
| Dates (`in_person_*`, type `date`) | 10/11 au 21/11/2025 | 09/11 au 20/11/2026 | aucune (`online_*` nuls aussi) |
| `timezone` | `America/Belem` | `Europe/Istanbul` | `UTC` |
| Lieu | Belém, BR | Antalya, TR, adresse « Bientôt disponible » | DZ, « En ligne » (imposé par un `CHECK` v1) |
| `event_status` | `ongoing` (périmé) | `upcoming` | `upcoming` |
| `submission_status`, `submission_deadline` | `open`, 21/11/2025 23:59 UTC | `open`, 26/10/2026 20:59 UTC (23:59 à Istanbul) | `open`, 31/12/2026 |
| `is_programmation_available` | vrai | faux | faux |
| Images | une bannière 16:9 basse qualité et le logo, sur l'ancien serveur | bannière 32:9 et logo sur Supabase | aucune |
| Activités | 118 | 140 | 1 |

| Colonne v1 | v2 (`event.events`) | Règle |
|---|---|---|
| `title`, `description` | `title`, `description` (`{"fr": …}`) | |
| `year`, `acronym` | `edition_year`, `acronym` | `series_id` : série COP climat semée ; `slug` calculé |
| `participation_mode`, `timezone`, `country_id`, `city` | idem | |
| `address` | `address` | « Bientôt disponible » → nul |
| `in_person_start_date`, `_end_date` | `starts_at`, `ends_at` | 00:00 du premier jour et 24:00 du dernier, dans le fuseau de l'événement |
| `event_status` | `status` | `ongoing` d'un événement passé → `completed` ; `upcoming` → `announced` |
| `submission_deadline`, `submission_status` | `event.calls_for_proposals` (`closes_at`, `status`) | un appel par édition ; COP30 `published`, COP31 `under_review` ; `opens_at` sans source (écart) |
| `is_programmation_available` | `programme_published_at` | date sans source (écart) |
| `banner_*_url` (six colonnes) | attachements `banner` (32:9), `cover` (16:9), `thumbnail` (1:1) | haute qualité, sinon basse ; la v2 recalcule les déclinaisons |
| `logo_url` | **aucune** | `media.attachable_roles` n'a pas `('event','events','logo')` : extension |
| `online_*_datetime` | aucune | toujours nuls |

`has_pavilion` vrai pour les deux CdP. PACO n'est pas repris (D9).

## 6. Activités : `activities` (36 colonnes, 259 lignes)

Toutes les dates sont des `timestamptz` stockés en UTC (`+00`). Aucune activité sans organisation (`organization_id NOT NULL`, aucune orpheline) ; 9 déposées par une personne qui n'est pas membre de l'organisation porteuse (COP31 6, COP30 3). Une seule activité supprimée (COP30, rejetée, avec motif).

### Statuts réels

`activity_status` vaut `draft` partout. `validation_status` :

| `validation_status` | CdP 30 | CdP31 | PACO | v2 `proposal_status` | v2 séance |
|---|---|---|---|---|---|
| `submitted` | 2 | 7 | | `submitted` | |
| `under_review` | 56 (27 avec créneau) | 130 (78 avec créneau) | | `under_review` | écart « créneaux sans acceptation » |
| `approved` | 8 | | 1 | `accepted` | `completed` (CdP 30 passée) |
| `live` | 1 (resté bloqué) | | | `accepted` | `completed` |
| `completed` | 38 | | | `accepted` | `completed` |
| `rejected` | 13 (dont 9 acceptées puis rejetées, 1 supprimée) | 3 | | `rejected`, ou `cancelled` si acceptée avant (écart) | |
| `draft`, `cancelled` | 0 | 0 | | | |

L'historique (`activity_modifications`, § 10) montre des allers-retours nombreux (jusqu'à 30 changements pour une activité). Seul l'état final fait le statut ; l'historique va dans `proposal_transitions`.

### D8 : le décalage de 3 h n'est pas dans les données

Créneaux arbitrés (`final_*`), heures UTC converties dans le fuseau de l'événement :

| | Débuts arbitrés, heure locale | Fins |
|---|---|---|
| CdP 30 (UTC-3) | 9:00, 10:15, 11:30, 13:15, 14:15, 15:30, 16:45 | jusqu'à 17:45 |
| CdP31 (UTC+3) | 9:00, 10:15, 11:30, 13:00, 14:15, 15:30, 16:45 | jusqu'à 17:45 |

Même grille aux deux éditions, chacune dans son fuseau. Créneaux souhaités COP31 : 9:00 à 16:00 à Istanbul (fins 10:00 à 17:00). Avec 3 h de plus ou de moins, la grille commencerait à 6:00 ou finirait à 20:45. Dans `activity_modifications`, aucun déplacement de ±3 h en série parmi les 271 changements de `final_start_date` (quatre de 3 h exactement à la CdP 30, isolés et dans les deux sens ; aucun à la CdP31). **Aucune correction à appliquer aux activités de Supabase.** Le décalage soupçonné vient sans doute de l'affichage ou des heures écrites dans le code des journées spéciales, à vérifier sur celles-ci.

### Correspondance

| Colonne v1 | Type, nul | Remplie | v2 | Règle |
|---|---|---|---|---|
| `event_id` | uuid NOT NULL | 259 | `programme.proposals.event_id`, `call_id` | par `id_map` |
| `organization_id` | uuid NOT NULL | 259 | `proposals.organization_id` + `proposal_organizations` (`lead`) | un seul porteur en v1 |
| `submitted_by` | uuid NOT NULL | 259 (192 personnes) | `proposals.submitted_by` | |
| `title`, `objectives`, `detailed_presentation` | text NOT NULL | 259, aucun vide | `title`, `objectives`, `detailed_presentation` (`{"fr": …}`) | 3 titres en double dans une même édition : suffixe au `slug` |
| `country_id` | uuid, nul | 258 (240 = pays de l'événement) | `proposals.country_id` | tel quel |
| `format` | ENUM NOT NULL | 259 | `format` (`event.participation_mode`) | identique : `in_person` 140, `hybrid` 118, `online` 1 |
| `main_themes` | `activity_theme[]` NOT NULL | 259, 1 à 17 par activité | `reference.entity_terms` (`activity_theme`) | § 6.4 |
| `categories` | `activity_categories[]` NOT NULL | 259, 1 à 7 | `entity_terms` (`activity_category`) | codes identiques |
| `activity_type` | ENUM NOT NULL | `side_event` 237, `country_day` 22 | **aucune** (colonne supprimée le 15/09) | extension |
| `proposed_start_date`, `_end_date` | timestamptz NOT NULL | 259 | `preferred_start_at`, `preferred_end_at` | 1 fin avant le début (refusée par `ck_proposals_preferred_period`) ; 2 durées de plus de 600 min, `duration_minutes` laissé nul |
| `final_start_date`, `_end_date` | timestamptz, nul | 167 (CdP 30 85, CdP31 81, PACO 1) | `programme.sessions.starts_at`, `ends_at`, `timezone` de l'événement | durées : 60 min 131, 480 min 17 (CdP 30 non retenues), 45, 90, 120, 135, 210 |
| `cover_image_high_url`, `_low_url` | text, nul | 52, Supabase | attachement `cover` de la proposition et de la séance | haute qualité seule |
| `banner_url`, `completion_report` | text, nul | 0 | aucune | toujours nuls |
| `zoom_meeting_id` | uuid, nul | 48 | aucune | réunions expirées, écartées (D3) |
| `youtube_link` | text, nul | 33, toutes un identifiant de 11 caractères | `live.streams` (`replay`) | un identifiant sert deux activités : une vidéo, une séance |
| `tags` | text[], nul | 12 activités CdP 30, 76 mots-clés | **aucune** | extension |
| `is_deleted`, `deleted_*` | | 1 | `deleted_at`, `deleted_by`, `deleted_reason` | tel quel |
| `activites_view_count` | bigint | 57 non nuls | `proposals.view_count` (et `sessions.view_count`) | tel quel |
| `activites_dashboard_view_count`, `last_viewed_at`, `send_activites_recu_email_count` | | 165, 175, 236 non nuls | aucune | compteurs techniques : écart |
| `created_at`, `updated_at` | | 259 | idem | |
| (absent) | | | `submitted_at`, `decided_at`, `decided_by` | tirés de `activity_modifications` (première entrée en `submitted` ou `under_review`, dernière décision), sinon `created_at` |

### Thématiques et catégories

| `activity_theme` v1 | Activités | v2 (`activity_theme`) |
|---|---|---|
| `adaptation` | 177 | `adaptation` |
| `mitigation` | 129 | `mitigation` |
| `gender_youth_and_education` | 113 | **absent** (la v2 n'a que `gender`) |
| `climate_ambition_ndc` | 86 | `climate_ambition_ndc` |
| `finance` | 81 | `climate_finance` (même notion, code différent) |
| `climate_justice_indigenous` | 79 | `climate_justice_indigenous` |
| `forests_nature_based_solutions` | 76 | **absent** |
| `agriculture_food` | 71 | `agriculture_food` |
| `just_energy_transition` | 57 | **absent** |
| `loss_and_damage` | 52 | `loss_and_damage` |
| `renewable_energy_land` | 48 | `renewable_energy_land` |
| `water_fisheries` | 44 | `water_fisheries` |
| `industry_transition_and_technology` | 40 | `industry_transition_and_technology` |
| `health_solidarity` | 36 | `health_solidarity` |
| `other` | 27 | **absent** |
| `transport_urbanization` | 26 | `transport_urbanization` |
| `sustainable_livestock` | 19 | `sustainable_livestock` |

`activity_categories` : `best_practices` 183, `results_sharing` 161, `awareness` 139, `capacity_building` 133, `field_project` 128, `technological_innovation` 76, `concertation` 76 ; les sept codes existent tels quels en v2.

## 7. Intervenants : `activity_speakers` (14 colonnes, 816 lignes)

CdP31 459, CdP 30 357. Deux activités sans intervenant (CdP 30 1, PACO 1), de 1 à 9 par activité.

| Colonne v1 | Type, nul | Remplie | v2 | Règle |
|---|---|---|---|---|
| `first_name`, `last_name` | text NOT NULL | 816, aucun vide | `identity.people` | rapprochement, voir plus bas |
| `email` | text NOT NULL | 816, **aucune vide ni mal formée** (46 avec majuscules) | `people.primary_email` | |
| `civility` | text, nul | 809 | `people.civility` | `mr`, `M.`, `Mr`, `M`, `M. v2` → `m` ; `mrs`, `Mme`, `ms` → `mme` ; `dr`, `Dr`, `Dr.` → `dr` ; `prof` → `pr` ; vide (3) et nul (4) → nul |
| `position` | text, nul | 814 | `proposal_speakers.job_title_snapshot` | |
| `organization` | text, nul | 816 | `organization_snapshot` | texte libre, sans lien vers une fiche |
| `is_available_for_questions` | bool | faux : 156 | idem | |
| `has_confirmed_by_email` | bool | faux partout | `confirmed_at` | nul |
| `confirmation_email_sent_at` | timestamptz, nul | 49 | `confirmation_sent_at` | |
| `photo_url`, `photo_thumbnail_url` | text, nul | 116, Supabase | attachement `avatar` de la personne | si elle n'en a pas déjà |
| `created_at` | | 816 | `created_at` | |

Séance : les intervenants d'une activité acceptée sont aussi écrits dans `session_speakers`.

**D16 dans Supabase.** Zéro intervenant sans adresse. Le problème est ailleurs : 573 adresses pour environ 689 personnes distinctes (adresse et nom). 43 adresses sont portées par des personnes de noms de famille différents (195 lignes, jusqu'à 12 personnes par adresse) ; 226 lignes portent l'adresse du déposant ; 99 lignes répètent une adresse déjà utilisée dans la même activité (34 activités). Rapprocher par l'adresse seule rattacherait **173 lignes** (CdP 30 79, CdP31 94) à une autre personne. 179 adresses d'intervenants sont celles d'un compte, dont 49 avec un nom différent de celui du compte.

## 8. Documents : `activity_documents` (17 lignes)

Tous CdP 30, sur des activités acceptées (13 `completed`, 4 `approved`) ; tous dans le compartiment `epavillonv`, déposés par le déposant de l'activité. PDF 14, DOCX 2, PPTX 1.

| Colonne v1 | v2 | Règle |
|---|---|---|
| `file_url`, `file_type` | `media.assets` | |
| `title` | `proposal_documents.title` (`{"fr": …}`) | |
| `types` (text[]) | `document_type_code` | `presentation` (9) → `presentation` ; `autre` (5) → `other` ; `report` (1) → `report` ; `additional_resource` (2) → **absent** |
| `uploaded_by`, `uploaded_at` | idem | |
| (absent) | `is_public` | sans source : écart |

## 9. Évaluation : notes, commentaires, lectures

### `activity_ratings` (110 lignes)

CdP 30 seulement (aucune note COP31), 107 activités (3 notées deux fois), **3 évaluateurs** (102, 5 et 3 notes). Une note globale sur 20 par pas de 0,5 (de 1,5 à 20, médiane 14), **aucun critère** ; 19 avec commentaire ; unique par (activité, évaluateur).

| v1 | v2 (`programme.reviews`) |
|---|---|
| `activity_id`, `revisionniste_id` | `proposal_id`, `reviewer_id` |
| `rating` | `score_out_of_20`, `mode = 'quick'` |
| `comment` | `comment` |
| `created_at`, `updated_at` | `created_at`, `submitted_at`, `updated_at` |
| (absent) | `recommendation = 'neutral'` ; les trois évaluateurs dans `event.call_reviewers` de l'appel COP30 ; `refresh_proposal_score` ensuite |

### `revision_comments` (181 lignes)

CdP 30 164, CdP31 17 ; 25 activités, 4 auteurs (dont un sans rôle, 11 commentaires). `shared_with_submitter` vrai : 17. `shared_with_revisionists` (uuid[]) : nul 12, sinon une liste de 2 à 11 évaluateurs, toujours un sous-ensemble des évaluateurs (6 listes distinctes, l'auteur y figure 158 fois).

| v1 | v2 (`programme.proposal_comments`) |
|---|---|
| `activity_id`, `created_by`, `comment_text` | `proposal_id`, `author_id`, `body` |
| `shared_with_submitter` vrai | `visibility = 'submitter'` |
| sinon | `visibility = 'committee'` (la liste nominative n'a pas de place : écart) |
| `created_at`, `updated_at` | `created_at`, `edited_at` (1 modifié) |

### `revision_comment_reads` (323 lignes) et `revisionniste_activity_views` (622 lignes)

Lectures par commentaire, 5 lecteurs : **aucune table v2** (`proposal_reads` est par dossier) : extension. Vues par dossier, 6 évaluateurs (CdP31 385, CdP 30 236, PACO 1), 4 975 vues cumulées : `programme.proposal_reads` à l'identique (`first_viewed_at` → `first_read_at`, `last_viewed_at` → `last_read_at`, `view_count` → `read_count`).

## 10. Historique : `activity_modifications` (1 560 lignes)

Du 21/09/2025 au 26/10/2026 ; CdP 30 1 082, CdP31 478. Champs : `validation_status` 602, `final_end_date` 273, `final_start_date` 271, `tags` 100, `cover_image_*` 146, `detailed_presentation` 41, `proposed_*` 68, `title` 28, `objectives` 26, `format` 3, `organization_id` 2. Valeurs en JSON doublement encodé (une chaîne JSON dans une chaîne). `old_value` est **toujours nul** pour `validation_status` : l'état précédent se reconstitue par l'ordre chronologique. `modified_by` nul : 4.

| v1 | v2 |
|---|---|
| lignes `validation_status` | `programme.proposal_transitions` : `from_status` reconstitué, statuts traduits comme au § 6, répétitions consécutives fusionnées, `actor_id`, `occurred_at` |
| toutes les lignes | `platform.audit_log` (`entity_table = 'proposals'`, ou `'sessions'` pour les créneaux, ce que lit `session_schedule_history`), `actor_label` dénormalisé |

## 11. Inscriptions : `activity_registrations` (22 colonnes, 14 346 lignes)

| | CdP 30 | PACO (D9, non repris) |
|---|---|---|
| Lignes | 105, sur 39 activités | 14 241, sur l'unique activité PACO |
| Avec compte, invités | 102, 3 | 308, 13 933 |
| Personnes distinctes (adresse) | 70 | 11 166 |
| `session_edition` | 1 partout | 1 à 10 |
| Doublons (même activité, même adresse) | 1 (invité puis compte) | 0 dans une même édition ; 3 075 réinscriptions d'une édition à l'autre |
| `attended`, `joined_at` | faux, 0 | faux, 3 650 |
| Zoom (`zoom_registrant_id`, `personal_join_url`) | 104, 105 | 0 |
| `referral_source` | nul partout | `email_newsletter` 5 158, `ifdd_linkedin` 3 173, `ifdd_facebook` 1 790, `ifdd_website` 1 659, `other` 1 436, `ifdd_x` 54, nul 971 |
| Adresses d'invité mal formées | 0 | 19 |

CdP31 : aucune inscription. Les 105 de la CdP 30 visent des activités `completed` (88), `approved` (10) et `rejected` (7, acceptées puis rejetées).

| Colonne v1 | v2 (`programme.registrations`) | Règle |
|---|---|---|
| `activity_id`, `session_edition` | `session_id` | la séance de l'activité (`sequence_number = 1`) |
| `user_id` / `guest_email`, `guest_first_name`, `guest_last_name` | `person_id` | invité rapproché par adresse, sinon personne créée sans compte ; doublon invité et compte fusionné |
| `guest_country_id` | `people.country_id` | si vide |
| `guest_organization` | `answers.organization` | champ du formulaire par défaut |
| `referral_source`, `_other` | `answers.referral_source` (vocabulaire `referral_source`, mêmes codes) | sans objet pour la CdP 30 |
| `registration_date` | `created_at` | |
| `attended`, `joined_at`, `attendance_duration` | `status`, `joined_at`, `attendance_minutes` | `registered` partout (personne n'est noté présent) |
| `zoom_*`, `personal_join_url`, `fallback_*`, `recovered_at` | aucune | Zoom expiré et journal technique (D3) |
| (absent) | `source = 'import'` | |

## 12. Questions du public : `activity_questions` (8), `activity_question_answers` (1)

CdP 30 seulement : 7 sur des activités `completed`, 1 sur une activité rejetée (donc sans séance). Toutes visibles, aucune désactivée ; `is_answered` faux partout alors qu'une réponse existe. `target_speakers` : 1 à 5 intervenants, tous existants.

| v1 | v2 |
|---|---|
| `activity_questions` | `programme.session_questions` : `person_id`, `body`, `target_speaker_ids` (intervenants de séance par `id_map`), `is_visible`, `answered_at` (date de la réponse) |
| `disabled_*` | `moderated_*` (aucun) |
| `activity_question_answers` | `session_question_answers` : `answered_by` = personne de l'intervenant, `is_official` vrai |

## 13. Écarts nouveaux à trancher

- **Comptes nés de PACO** (639, dont 339 jamais vérifiés, 280 inscrits au webinaire) et les 3 rôles `paco`. Proposition : reprendre les 300 vérifiés comme des comptes ordinaires sans rôle (ce sont des personnes réelles qui se connectent) ; laisser dans Supabase les 339 jamais confirmés, que PACO garde (D9).
- **Vérification des comptes Laravel** : 2 020 comptes « vérifiés » le jour de l'import, sans lien cliqué. Proposition : reprendre la date telle quelle (D1) et marquer l'origine dans `id_map`, pour qu'un compte d'audit puisse séparer les deux populations.
- **Statut unique `validation_status` mêlant décision et déroulé.** Proposition : la décision va sur la proposition, le déroulé sur la séance (`approved`, `live`, `completed` → proposition `accepted`, séance `completed` pour la CdP 30) ; l'historique complet dans `proposal_transitions`.
- **Acceptées puis rejetées** (9 à la CdP 30, dont 4 avec des inscrits). Proposition : `rejected` si le dernier état l'est, comme la v1 l'affiche ; pas de séance ; les 7 inscriptions gardées en `cancelled`.
- **CdP 30 : 58 dossiers jamais décidés** (56 en évaluation, 2 déposés) pour une édition close. Proposition : les garder dans leur état, sans rien décider à la place de l'IFDD ; l'appel passe `published`, ils sortent des listes de travail.
- **Créneaux sans acceptation** (CdP31 78, CdP 30 27). Proposition : séance `planned`, non publiée, liée à la proposition en évaluation, que le planificateur v2 sait afficher ; à la CdP 30, aucune séance, le créneau reste dans le journal d'audit.
- **Intervenants à adresse partagée** (43 adresses, 173 lignes mal rattachables). Proposition : rapprocher par adresse **et** nom ; une ligne dont le nom diffère du titulaire de l'adresse devient une personne sans adresse propre (la forme retenue pour D16), l'adresse partagée gardée comme contact sur sa ligne d'intervenant (`proposal_speakers`, colonne à créer).
- **Thématiques sans terme v2** (D6) : `gender_youth_and_education` (113 activités), `forests_nature_based_solutions` (76), `just_energy_transition` (57), `other` (27). Proposition : créer les quatre termes, couleur comprise ; `finance` (81) → `climate_finance`.
- **Type d'activité** (`side_event` 237, `country_day` 22), sans place depuis le 15/09. Proposition : un vocabulaire `activity_type` dans `reference.taxonomy_terms`, rattaché par `entity_terms` comme les thématiques ; aucune colonne.
- **Mots-clés libres** (`tags`, 12 activités, 76 mots). Proposition : `programme.proposals.keywords text[]`.
- **Logo d'un événement** : la v2 ne prévoit que bannière, couverture, vignette, galerie. Proposition : le rôle `('event','events','logo')` dans `media.attachable_roles`.
- **Attestations d'organisation par des pairs** (77). Proposition : `org.organization_endorsements` (organisation, personne, date, unique), lue par `compute_trust_score`.
- **Adresse postale d'une personne** (77). Proposition : `identity.people.postal_address text`.
- **Thème d'affichage** (4 en sombre). Proposition : `identity.people.color_scheme` (`light`, `dark`, `system`), le site le lisant à la connexion.
- **Lecture des commentaires de révision** (323). Proposition : `programme.proposal_comment_reads` (commentaire, personne, date).
- **Liste nominative d'évaluateurs sur un commentaire** (169 commentaires, toujours des évaluateurs). Proposition : visibilité `committee` sans colonne nouvelle, la v2 partageant tout au comité ; la liste reste dans `id_map`.
- **`additional_resource`**, type de document absent (2). Proposition : créer le terme `additional_resource` dans `document_type`.
- **Visibilité des documents** (`is_public`, sans source). Proposition : vrai pour les 17, tous joints à des activités tenues, comme la v1 les montrait sur la page de l'activité (à vérifier dans le code v1).
- **Compteurs techniques** (`activites_dashboard_view_count`, `last_viewed_at`, `send_activites_recu_email_count`). Proposition : journaux techniques (D3), gardés dans `id_map` seulement.
- **Dates sans source** : ouverture des appels (`opens_at`) et publication du programme CdP 30 (`programme_published_at`). Proposition : création de l'événement pour l'ouverture, dernière modification de l'événement pour la publication, toutes deux signalées dans `id_map`.
- **Données refusées par les contraintes v2** : 1 créneau souhaité qui finit avant de commencer, 1 sigle d'une lettre, 3 sites avec espaces, 1 question sur une activité sans séance. Proposition : valeur mise à nul, rattachement à la proposition pour la question (colonne `proposal_id` nullable sur `session_questions`, ou question écartée avec raison), chaque cas noté dans `id_map`.

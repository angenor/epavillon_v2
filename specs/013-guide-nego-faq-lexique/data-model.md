# Modèle de données — étape 2 : FAQ, parcours et lexique

Tout s'écrit d'abord dans `docs/database/`, section **§10 Savoir** de `100_negotiations.sql`, puis dans `migration.sql`, recopié mot pour mot. Conventions du schéma : `id uuid PRIMARY KEY DEFAULT platform.uuid_v7()`, textes métier en `platform.i18n_text`, `updated_at` par `platform.tg_set_updated_at()`, audit par `platform.tg_audit()` sur toute table éditée au back-office, `xmod_fk_<table>_<rôle>` vers `identity.people`, `COMMENT ON` en français.

## Vocabulaires — `020_reference.sql`

| Taxonomie | Termes semés | Colonnes employées |
|---|---|---|
| `faq_section` | `first_cop` « Ma première COP », `process` « Le processus », `negotiating_groups` « Les groupes de négociation », `on_site` « Sur place » | `label`, `icon` (nom du pictogramme de Guide Négo), `sort_order` |
| `glossary_family` | `meetings` « Réunions », `texts` « Textes », `themes` « Thématiques » | `label`, `sort_order` |

Semés dans `020_reference.sql` et dans la migration (`ON CONFLICT DO NOTHING`). **Référencés par identifiant**, comme `theme_subscriptions.theme_term_id` : `uuid REFERENCES reference.taxonomy_terms(id)`, gardé par `negotiation.tg_check_term_taxonomy('<colonne>', '<taxonomie>')`, qui lit un `uuid`. L'API parle en codes (`section_code`, `family_code`, `theme_code`) et fait la correspondance, comme `repo/themes.rs`.

## ENUM — machines à états

```text
negotiation.knowledge_status  : draft | published | to_review
negotiation.question_status   : pending | answered | added_to_faq
negotiation.proposal_status   : pending | accepted | rejected
negotiation.report_status     : open | closed
```

Transitions de `knowledge_status` : `draft → published` (vérification datée exigée pour la FAQ) ; `published ⇄ to_review` ; `published | to_review → draft` (dépublier). `to_review` reste visible (tranché le 25/09).

## Tables

### `faq_entries`

| Colonne | Type | Règle |
|---|---|---|
| `section_term_id` | uuid | → `reference.taxonomy_terms`, vocabulaire `faq_section` |
| `question`, `answer` | i18n_text | `fr` requis ; `answer` NULL en brouillon, requise dès `published` — `ck_faq_entries_answer` |
| `status` | knowledge_status | défaut `draft` |
| `verified_on` | date | facultatif, même publiée — `ck_faq_entries_verified` retirée le 05/10 (publication sur instruction de la hiérarchie) |
| `verified_by` | uuid | `xmod_fk_faq_entries_verifier`, requis avec `verified_on` |
| `editorial_rank` | smallint | ordre des « plus lues » à défaut de lectures ; null = hors classement |
| `origin_question_id` | uuid | → `expert_questions`, null si rédigée |
| `first_published_at` | timestamptz | posé au premier `published`, jamais effacé ; interdit la suppression (`tg_faq_entries_undeletable`) |
| `question_norm` | text GENERATED | `platform.normalize_label(question->>'fr')`, index `gin_trgm_ops` |
| `created_at`, `updated_at`, `created_by` | | `xmod_fk_faq_entries_author` |

### `faq_related` — questions liées

`entry_id`, `related_id` (→ `faq_entries`, `ON DELETE CASCADE`), `sort_order` ; PK `(entry_id, related_id)` ; `ck_faq_related_not_self`. Déclencheur : touche `updated_at` de `entry_id`.

### `knowledge_sources` — sources d'une entrée de FAQ ou du lexique

| Colonne | Type | Règle |
|---|---|---|
| `faq_entry_id` / `glossary_entry_id` | uuid | **exactement un** — `ck_knowledge_sources_owner` |
| `document_id` | uuid | → `negotiation.documents` ; ou bien |
| `external_title`, `external_url` | text | une référence extérieure — `ck_knowledge_sources_target` : document ou titre extérieur, pas les deux |
| `section_label` | text | « annexe A.3 », « Le contexte de négociation » |
| `page_from`, `page_to` | smallint | facultatifs, `page_to ≥ page_from` |
| `quote` | text | citation facultative |
| `sort_order` | smallint | |

Déclencheur : touche `updated_at` du parent.

### `faq_reads` — lectures comptées, anonymes

`entry_id`, `day date`, `count int` ; PK `(entry_id, day)` ; incrément par `INSERT … ON CONFLICT DO UPDATE`. Ni auteur ni appareil. Purgé au-delà de 90 jours par le travail `purge` existant.

### `faq_feedback` — « Cette réponse vous a-t-elle aidée ? »

`entry_id`, `person_id` (`xmod_fk_faq_feedback_person`), `helpful boolean`, `missing_reason text` (`too_vague | off_topic | outdated`, `ck_faq_feedback_reason` : null si `helpful`), `created_at`, `updated_at` ; PK `(entry_id, person_id)`. La personne n'est jamais rendue par une route (R9).

### `faq_reports` — « Dépassé ou faux »

| Colonne | Type | Règle |
|---|---|---|
| `entry_id` | uuid | → `faq_entries` |
| `reporter_id` | uuid | `xmod_fk_faq_reports_reporter` — plafond et une voix, jamais rendu |
| `client_ref` | uuid | `ux_faq_reports_client_ref (reporter_id, client_ref)` |
| `reasons` | text[] | ⊂ `{rule_changed, wrong, source_mismatch}` ; non vide sauf `from_feedback` — `ck_faq_reports_reasons` |
| `from_feedback` | boolean | venu de « Non » → « Dépassée ou fausse » ; `ux_faq_reports_from_feedback (entry_id, reporter_id) WHERE from_feedback` |
| `details` | text | ≤ 600 caractères |
| `status` | report_status | `open` |
| `outcome` | text | à la clôture : `revised | confirmed | dismissed` |
| `handled_by`, `handled_at` | | `xmod_fk_faq_reports_handler` ; `ck_faq_reports_closed` : les trois posés ensemble avec `closed` |

### `expert_questions`

| Colonne | Type | Règle |
|---|---|---|
| `asker_id` | uuid | `xmod_fk_expert_questions_asker` ; jamais rendu au back-office |
| `client_ref` | uuid | `ux_expert_questions_client_ref (asker_id, client_ref)` |
| `theme_term_id` | uuid | → `reference.taxonomy_terms`, vocabulaire `negotiation_theme` |
| `body` | text | 1 à 600 caractères |
| `consent_to_faq` | boolean | |
| `status` | question_status | `pending` |
| `answer` | text | requis dès `answered` |
| `answered_by`, `answered_at` | | `xmod_fk_expert_questions_answerer` |
| `faq_entry_id` | uuid | → `faq_entries`, posé à la promotion ; `ck_expert_questions_promotion` : seulement si `consent_to_faq` |
| `created_at`, `updated_at` | | |

Déclencheur AFTER : passage à `answered` → `platform.emit_event('negotiation','expert_question',…,'negotiation.expert_question.answered')` pour l'audit ; le courriel, lui, est un travail mis en file par le service (R11).

### `pathway_groups` et `pathway_steps` — « Ma première COP »

`pathway_groups` : `label i18n_text`, `sort_order`, `is_published`, `updated_at`.

`pathway_steps` :

| Colonne | Type | Règle |
|---|---|---|
| `group_id` | uuid | → `pathway_groups` |
| `label` | i18n_text | |
| `detail`, `origin_label` | i18n_text | facultatifs |
| `link_kind` | text | `document | faq | glossary`, null sans lien |
| `link_document_id`, `link_page`, `link_section` | | lien vers un document, page ou section |
| `link_faq_id`, `link_glossary_id` | uuid | lien vers une entrée |
| `link_label` | i18n_text | « Guide, chapitre 3 » |
| `sort_order`, `is_published`, `updated_at` | | |

`ck_pathway_steps_link` : `link_kind` s'accorde avec la seule cible posée.

### `pathway_checks` — coches d'un compte

`person_id`, `step_id` (`ON DELETE CASCADE`), `checked_at` ; PK `(person_id, step_id)`. Une coche d'étape non publiée n'est pas comptée à l'affichage.

### `glossary_entries`

| Colonne | Type | Règle |
|---|---|---|
| `slug` | text | **désignation stable** ; `ux_glossary_entries_slug` ; `ck_glossary_entries_slug` `^[a-z0-9]+(-[a-z0-9]+)*$` ; posé à l'insertion par `platform.slugify(term)` (existe) s'il est vide, jamais recalculé |
| `family_term_id` | uuid | → `reference.taxonomy_terms`, vocabulaire `glossary_family` |
| `term` | text | terme anglais, tel qu'il s'écrit |
| `acronym` | text | « GGA », facultatif |
| `variants` | text[] | écritures admises : « bracketed », « brackets » |
| `translation`, `definition` | i18n_text | `fr` requis |
| `heard_in_room` | text | la phrase entendue, en anglais |
| `status` | knowledge_status | |
| `first_published_at` | timestamptz | comme la FAQ : interdit la suppression |
| `term_norm` | text GENERATED | `platform.normalize_label(term)`, index `gin_trgm_ops` et index unique `ux_glossary_entries_term_norm` (résolution sans ambiguïté) |
| `acronym_norm` | text GENERATED | `platform.normalize_label(acronym)` |
| `variants_norm` | text[] | tenu par déclencheur (une colonne générée ne prend pas de fonction sur tableau), index `gin` |
| `created_at`, `updated_at`, `created_by` | | |

### `glossary_related` — termes liés

Comme `faq_related`, sur `glossary_entries`.

### `glossary_favorites`

`person_id`, `entry_id` (`ON DELETE CASCADE`), `created_at` ; PK `(person_id, entry_id)`.

### `glossary_proposals` et `glossary_proposal_authors`

`glossary_proposals` : `term`, `term_norm` GENERATED, `status proposal_status`, `glossary_entry_id` (posé à `accepted`), `rejection_reason` (requis à `rejected`), `handled_by`, `handled_at`, `created_at`, `updated_at` ; `ux_glossary_proposals_pending (term_norm) WHERE status = 'pending'` — une proposition du même terme s'y ajoute.

`glossary_proposal_authors` : `proposal_id`, `person_id`, `client_ref` (`ux … (person_id, client_ref)`), `context text ≤ 600`, `created_at` ; PK `(proposal_id, person_id)`.

## Fonctions

- `negotiation.glossary_resolve(p_text text) RETURNS uuid`, `STABLE` — entrées `status <> 'draft'` ; ordre : `term_norm`, `acronym_norm`, `variants_norm`, `slug` (R6).
- `negotiation.knowledge_fingerprint() RETURNS text` — `max(updated_at)` et nombre de lignes de chaque table du paquet, et le jour courant.
- `negotiation.tg_touch_knowledge_parent()` — déclencheur générique des tables enfants.
- `negotiation.tg_glossary_slug()` — pose `slug` et `variants_norm`.

## Permissions et rôles — §9

- `negotiation.knowledge.publish` → `admin`, `expert`.
- `negotiation.knowledge.review` → `expert`.

## Ce qui ne s'ajoute pas au modèle

Les derniers termes consultés, les coches et favoris sans compte, le jour de la dernière lecture comptée : propres au téléphone.

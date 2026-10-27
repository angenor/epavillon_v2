# Reprise v1 : carte réelle des tables annexes et des fichiers

> Relevé du 27/10/2026 sur l'extraction `supabase-v1-20261027-0419.dump`. Périmètre : tables hors cœur (le cœur, comptes à évaluations, a sa propre carte), tables vides, fichiers, liens YouTube. Aucune donnée personnelle ici : comptes et formes seulement. PACO (`paco_demographic_data`) ignoré (D9).

## 1. Tables annexes

### Synthèse

| Table v1 | Lignes | Cible v2 | Verdict |
|---|---|---|---|
| `negotiation_documents` | 43 | `negotiation.documents` | 21 documents réels : un import Laravel joué deux fois (§ 1.1) |
| `user_favorite_documents` | 2 | `negotiation.document_bookmarks` | direct |
| `messages` | 73 | `engagement.conversations`, `conversation_participants`, `direct_messages` | ressemble à des essais (§ 1.3) |
| `connections` | 13 | `engagement.connection_requests` | direct |
| `notifications` | 79 | `engagement.notifications` | un type manque en v2 |
| `message_templates` | 5 | `engagement.message_templates` + `template_versions` | modèles semés à la création du schéma v1, jamais retouchés |
| `incident_messages` | 2 | `live.incidents` | direct, portée `event_day` |
| `media_gallery` | 11 | `media.assets` + `media.attachments` | rôle `gallery` absent sur la séance (D7) |
| `user_testimonials` | 1 | `content.highlights` | titre obligatoire en v2, absent en v1 |
| `zoom_meetings` | 50 | `live.meetings` | à l'écart selon D3, à confirmer (§ 1.10) |
| `ai_chat_sessions` | 10 | `tool.conversations` | une seule personne, quatre jours : essais |
| `ai_chat_messages` | 24 | `tool.conversation_messages` | idem |
| `ai_chat_feedback` | 2 | `tool.message_feedback` | idem |
| `migration_logs` | 2 047 | aucune (journal technique) | mais porte des données Laravel jamais reprises (§ 1.14) |

### 1.1 `negotiation_documents` (43)

| v1 | Type v1 | v2 | Note |
|---|---|---|---|
| `id` | uuid | `legacy.id_map` | |
| `title` | text | `title` (`{fr}`) | |
| `description` | text | `summary` (`{fr}`) | 21 valeurs distinctes |
| `document_type` | enum `document_type` | `document_type_term_id` | `other` 19, `negotiation_guide` 17, `technical_note` 7 ; les trois codes existent en v2 |
| `category` | enum `session_category` | `track_term_id` (taxonomie `negotiation_track`) | `climate` 23, `biodiversity` 18, `desertification` 2 ; codes identiques en v2 |
| `cover_image_url` | text | `cover_asset_id` | 42 sur l'ancien serveur, 1 Supabase (§ 3) |
| `file_url` | text | `asset_id` | 42 PDF sur l'ancien serveur, 1 Supabase ; `external_url` reste nul |
| `uploaded_by` | uuid | `uploaded_by` | une seule personne (super administrateur) |
| `created_at` | timestamptz | `created_at`, `published_at` | pas de brouillon en v1 : tout est publié |
| `is_migrate` | boolean | `migrated_from_v1` | 42 vrais (venus de Laravel), 1 faux (saisi dans Supabase) |
| (absent) | | `slug`, `version` | à dériver du titre ; `version = '1'` |

**Un import joué deux fois** : 21 lignes le 27/09/2025 à 17:14 (toutes typées `other`), 21 lignes à 17:18 (typées), plus 1 document saisi le 26/09. Les deux favoris pointent sur le second passage. Documents réels : 1 + 21 du second passage, dont **un même document rangé dans deux filières** (climat et biodiversité), soit 21 documents. La v2 n'a qu'une `track_term_id`.

**Inversion type/catégorie annoncée par le plan : non observée** sur le second passage, les types concordent avec les titres. En revanche cinq « Résumé… » sont typés `negotiation_guide` alors que la v2 a un type `summary`, et le résumé « désertification » partage son fichier PDF avec un guide biodiversité (lien Laravel probablement faux).

**Sécurité v1** : table sans RLS et `GRANT ALL` au rôle `anon` (§ 5).

### 1.2 `user_favorite_documents` (2)

`user_id` → `person_id` · `document_id` → `document_id` (via `id_map`, second passage) · `favorited_at` → `created_at` · `id` abandonné (la clé v2 est le couple). `note` reste nul.

### 1.3 `messages` (73)

| v1 | v2 |
|---|---|
| `sender_id`, `recipient_id` | `conversations` (`kind = direct`, `direct_key` = paire triée) + deux `conversation_participants` |
| `content` | `direct_messages.body` |
| `is_read` | `conversation_participants.last_read_at` = dernier message (tous lus) |
| `created_at` | `direct_messages.created_at` |
| `updated_at` | `edited_at` : à laisser nul, `updated_at` diffère de `created_at` sur les 73 lignes sans qu'aucun contenu n'ait changé de façon traçable |

Constat : 2 expéditeurs, 3 paires, **11 messages d'une personne à elle-même**, 48 caractères au plus, tous sur le 22 au 24/08/2025, entre un super administrateur et un compte négociateur-évaluateur. Profil d'essai. Un message à soi-même n'a pas de forme en v2 (conversation à un seul participant).

### 1.4 `connections` (13)

`requester_id`, `recipient_id`, `created_at`, `updated_at` → mêmes colonnes de `engagement.connection_requests`. `status` (enum `connection_status_v2`) : `pending` 11, `accepted` 1, `cancelled` 1 ; valeurs présentes en v2. `responded_at` (exigé hors `pending`) ← `updated_at`. `message`, `expires_at` nuls. Aucune paire en double, aucune demande à soi-même.

### 1.5 `notifications` (79)

| v1 | v2 |
|---|---|
| `user_id` | `person_id` |
| `notification_type` | `type_code` : `connection_request` (13) → `engagement.connection.requested` ; `revision_comment` (66) → **aucun type v2** |
| `title`, `content` | `title`, `body` (`{fr}`) ; deux titres fixes, 15 contenus |
| `related_entity_id` | `subject_*` : 66 visent une activité (→ `programme.proposals`), 13 une connexion (→ `engagement.connection_requests`) |
| `is_read` | `read_at` : 68 lues, 11 non lues ; la v1 n'a pas de date de lecture |
| `created_at` | `created_at` |

### 1.6 `message_templates` (5)

`activity_approved`, `activity_rejected`, `connection_request`, `newsletter_subscription`, `activity_reminder`. Texte brut (pas de HTML), variables à accolade simple (`{first_name}`), toutes créées à la même seconde le 03/08/2025 : graines du schéma, jamais modifiées. La v1 envoyait ses courriels depuis des fonctions Deno, pas depuis cette table. Correspondance si reprise : `name` → `key` (slug), `subject`/`body` → `template_versions.subject`/`body_text`, `variables` → `variables`.

### 1.7 `incident_messages` (2)

| v1 | v2 (`live.incidents`) |
|---|---|
| `event_id` | portée via `event_day_id` (CdP 30) |
| `day_date` | `event_day_id` (10 et 11/11/2025) ; `scope = 'event_day'` |
| `organization_id` | nul sur les 2 |
| `message_fr`, `message_en` | `message` (`{fr, en}`) |
| `severity` | `severity` : `error` sur les 2 (valeur v2 identique) |
| `is_active` | faux sur les 2 : `published_at` ← `created_at`, `unpublished_at` ← `updated_at` |
| `created_at`, `updated_at` | idem ; `display_from` ← `created_at` |

`incident_kind_code` prend le défaut `technical_issue`.

### 1.8 `media_gallery` (11)

`media_type` : `photo` 11 · `context_type` : `activity` 11, sur 4 activités, toutes `validation_status = completed` (donc séances en v2). `media_url` = `thumbnail_url` sur les 11 lignes (aucune vignette distincte). `description`, `author` nuls. `title` → `media.assets.caption` ou `alt_text` ; `uploaded_by` → `owner_person_id`. Rattachement : rôle `gallery` sur `programme.sessions`, absent du modèle (D7, extension à écrire).

### 1.9 `user_testimonials` (1)

`testimonial_text` → `quote` (et `body`) · `user_id` → `person_id` · `featured` (vrai) → `status = 'published'` · `background_color` (`#10B981`) → `background_color_hex` · `photo_url` → attachement `cover` ou `banner` · `context_type` `{training}` : aucune formation en base (table vide), rien à rattacher · `thematique_type` vide · `nature_code = 'testimonial'`. **`title` est obligatoire en v2, la v1 n'en a pas.**

### 1.10 `zoom_meetings` (50)

| v1 | v2 (`live.meetings`) | Secret |
|---|---|---|
| `meeting_id` (11 chiffres) | `external_id` | |
| `topic` | `topic` | |
| `start_time` | `scheduled_start_at` (28/10 au 21/11/2025) | |
| `duration` (60 : 41, 45 : 5, 120 : 2, 90 : 1, 210 : 1) | `duration_minutes` | |
| `timezone` (`UTC` 49, `Europe/Paris` 1) | `timezone` | |
| `registration_url` (49) | `registration_url` | |
| `join_url` (50, porte `?pwd=`) | `join_url` sans le paramètre `pwd` | **oui** (code d'accès chiffré dans l'URL) |
| `password` (50, en clair, 6 à 8 caractères) | `passcode_encrypted` | **oui** |
| `start_url` (50, porte `?zak=`) | `start_url_encrypted` | **oui** (jeton d'hôte) |
| `host_email` (2) | `host_email` | |
| `created_by`, `created_at` | idem | |
| `banner_url` | (vide sur les 50) | |
| (via `activities.zoom_meeting_id`, 48) | `session_id` | |

Rattachement : 48 activités désignent une réunion par `zoom_meeting_id` (= `zoom_meetings.id`, jamais `meeting_id`), une chacune : 38 `completed`, 8 `approved`, 1 `live`, **1 `rejected`** (pas de séance en v2) ; 2 réunions autonomes. Toutes passées : `status = 'ended'`, `sync_status = 'not_required'`.

Autres secrets Zoom hors de cette table : `activity_registrations.personal_join_url` et `zoom_join_url` (105 chacune, `?tk=…&pwd=…`, jeton personnel d'inscrit) ; 5 réponses de `ai_chat_messages.content` recopient un lien de réunion.

### 1.11 à 1.13 Conversations avec l'assistant IA

| v1 | v2 |
|---|---|
| `ai_chat_sessions.user_id` | `tool.conversations.person_id` |
| `title` | `title` |
| `feature_type` (`negotiation_documents` 10) | `assistant_id` → assistant semé `negotiation_helper` |
| `category` (enum `session_category_v2`) | nul sur les 10 ; `context_*` restent nuls |
| `is_active` (vrai 10) | `is_archived = false` |
| `ai_chat_messages.role` (`user` 12, `assistant` 12) | `role` (mêmes valeurs) |
| `content` | `content` |
| `source_documents` (`[]` sur les 24) | `citations = '[]'` |
| `metadata` : `model` (`qwen/qwen3-8b` 8, `qwen/qwen3-14b` 1), `tokens`, `responseTime`, `toolCallsExecuted`, `error` (3) | `model`, `output_tokens`, `latency_ms`, `finish_reason` ; le reste dans aucune colonne |
| `ai_chat_feedback.feedback_type` (`positive` 1, `negative` 1) | `rating` (`up`, `down`) ; `comment` nul |

Constat : les 10 conversations appartiennent au même super administrateur, du 26 au 29/10/2025, une sans message. Essais de mise au point de l'assistant.

### 1.14 `migration_logs` (2 047)

**Trace de l'import des comptes Laravel vers Supabase**, le 28/09/2025 entre 02:01 et 03:00 UTC : `old_user_id` (entier Laravel), `new_user_id` (uuid Supabase, présent dans `auth.users` et `users` pour les 2 047), `action` (`created` 2 019, `updated` 28 ; jamais `failed`), `old_user_data` (la ligne Laravel complète en JSON), `notes` et `error_message` vides. Les 2 020 `users.laravel_user_id` se retrouvent tous ici.

Pas de cible v2 (journal technique, D3), mais `old_user_data` porte ce que l'import de 2025 **n'a pas recopié** dans Supabase :

| Champ Laravel | Rempli | Absent de Supabase pour la même personne | Place v2 |
|---|---|---|---|
| `titre` (fonction) | 2 047 | 2 047 (aucune colonne v1) | `org.memberships.job_title` : répond à D5 |
| `photo_url` (`images_uploades/<fichier>`) | 1 047 (1 046 fichiers) | 1 041 | `identity.people` avatar (`media.attachments`) |
| `telephone` | 138 | 136 | téléphone de `identity.people` |
| `biographie` | 38 | 37 | biographie de `identity.people` |
| `organisation_id` (Laravel) | 1 051 | 1 008 | `org.memberships`, si la table Laravel des organisations est relue |
| `is_admin` | 3 | | à recouper avec `user_roles` |
| `profile_photo_url` | 2 047 | | `ui-avatars.com` : avatar généré, rien à reprendre |
| `token_peerjs`, `apprenant_id`, `intervenant_id` | | | techniques ou formations Laravel, rien à reprendre |

## 2. Tables vides (42)

| Domaine | Tables | Ce qu'elles devaient porter |
|---|---|---|
| Négociation | `negotiators`, `negotiator_consultations`, `negotiation_sessions`, `francophonie_meetings`, `francophonie_meeting_registrations` | profils et désignations des négociateurs, consultations, sessions de négociation, réunions de la Francophonie et inscriptions |
| Assistant IA | `document_embeddings` | vecteurs des documents de négociation |
| Messagerie | `message_groups`, `group_members`, `group_messages`, `user_blocks` | conversations de groupe, blocages |
| Mise en relation | `appointments` | rendez-vous entre membres |
| Infolettres | `newsletter_lists`, `newsletter_subscriptions`, `newsletter_campaigns`, `email_history` | listes, abonnés, campagnes, historique des courriels |
| Contenus | `innovations_practices`, `innovation_reactions`, `ratings`, `technical_documents`, `video_testimonials`, `comments` | innovations et bonnes pratiques, réactions, notes, fiches techniques, témoignages vidéo, commentaires publics |
| Formations | `trainings`, `training_chapters`, `training_participants`, `lesson_contents`, `participant_chapter_progress`, `participant_lesson_progress`, `evaluations`, `evaluation_questions`, `evaluation_answers`, `evaluation_results` | formations, chapitres, leçons, progression, évaluations et certificats |
| Quiz et sondages | `live_quizzes`, `quiz_questions`, `quiz_subscriptions`, `quiz_responses`, `quiz_results`, `polls`, `poll_responses` | quiz en direct, sondages |
| Programme | `session_registrations`, `activity_speaker_connections` | inscriptions à une séance d'un événement multi-séances, liens Zoom personnels des intervenants |
| Organisations | `organization_aliases` | sigles et autres dénominations |
| Technique | `rls_monitoring` | anomalies de politiques RLS |

Conséquences pour le plan : les lignes `negotiators` (ordre 2), `negotiation_sessions` et `francophonie_meetings` (ordre 9), `video_testimonials` et `innovations_practices` (ordre 10) n'ont rien à reprendre ; D10 est sans objet ; l'exclusion des « vecteurs du chatbot » (D3) aussi ; le § 4 « tout le reste est repris » ne concerne en fait ni rendez-vous, ni sondages, ni infolettres, ni historique des courriels, ni formations.

## 3. Fichiers

### 3.1 Stockage Supabase

| Bucket | Public | Limite | Objets | Volume | Types MIME |
|---|---|---|---|---|---|
| `epavillonp` | oui | 1 000 Ko, images et PDF | 604 | 48,7 Mo | JPEG 476, PNG 117, WebP 5, SVG 3, PDF 3 |
| `epavillonv` | oui | 50 Mo | 22 | 54,3 Mo | PDF 16, MP4 2, DOCX 2, PPTX 1, `octet-stream` 1 |
| `activity-media` | oui | 1 Mo, images | 11 | 4,5 Mo | JPEG 7, PNG 4 |
| `documents` | oui | 50 Mo | 0 | | |
| **Total** | | | **637** | **107,5 Mo** | |

Le bucket `video-thumbnails` cité au plan **n'existe pas**, `documents` est vide.

Dossiers : `epavillonp/` : `activities_banner/` 179, `intervenants/` 270, `logo/` 98, `profil/<uuid>/` 44, `temoignage/<uuid>/` 4, `formation/` 3, `doc_nego/` 4, `temp/` 2 · `epavillonv/` : `activities_document/<uuid>/` 19, `temoignage/<uuid>/` 2 (vidéos), `nego_doc/` 1 (fichier témoin de dossier vide) · `activity-media/activity-media/<uuid>/` 11.

**Tous les objets cités par une table existent** (535 sur 535). **102 objets ne sont cités par aucune ligne courante** (24,3 Mo) :

| Orphelins | Nombre | Explication |
|---|---|---|
| anciennes couvertures d'activité, citées par l'historique `activity_modifications` | 36 | remplacées depuis |
| `intervenants/` | 38 | intervenants supprimés ou photo remplacée |
| `logo/` | 8 | logos remplacés |
| `activities_banner/` | 5 | |
| `temoignage/` (3 photos, 2 vidéos) | 5 | `video_testimonials` vide |
| `formation/` | 3 | `trainings` vide |
| `doc_nego/` | 2 | |
| `temp/` | 2 | |
| `activities_document/` | 2 | |
| `nego_doc/` (fichier témoin) | 1 | |

Paires original et vignette : les couvertures d'activité (52), les photos d'intervenants (116) et de profil (22) existent chacune en deux objets distincts (« high/low », « photo/thumbnail »). La v2 fabrique ses déclinaisons (`media.renditions`) : seul l'original a besoin d'être repris.

### 3.2 Colonnes qui désignent un fichier

| Colonne | Non nulles | Supabase (présent) | Ancien serveur | Autre |
|---|---|---|---|---|
| `activities.cover_image_high_url` | 52 | 52 (`epavillonp/activities_banner/`) | | |
| `activities.cover_image_low_url` | 52 | 52 | | |
| `activities.banner_url` | 0 | | | |
| `activities.completion_report` | 0 | | | |
| `activity_documents.file_url` | 17 | 17 (`epavillonv/activities_document/<uuid>/`) | | |
| `activity_speakers.photo_url` | 116 | 116 (`epavillonp/intervenants/`) | | |
| `activity_speakers.photo_thumbnail_url` | 116 | 116 | | |
| `users.profile_photo_url` | 22 | 22 (`epavillonp/profil/<uuid>/`) | | |
| `users.profile_photo_thumbnail_url` | 22 | 22 | | |
| `organizations.logo_url` | 232 | 122 (`epavillonp/logo/`) | | 110 chaînes vides |
| `events.banner_high_quality_32_9_url` | 1 | 1 (CdP31) | | |
| `events.banner_low_quality_16_9_url` | 1 | | 1 (CdP 30, `/public/images_uploades/`) | |
| `events.logo_url` | 2 | 1 (CdP31) | 1 (CdP 30, `/public/images_uploades/`) | |
| `events` : 4 autres colonnes de bandeau | 0 | | | |
| `negotiation_documents.file_url` | 43 | 1 (`epavillonp/doc_nego/`) | 42 (`/images_uploades/` 22, `/documents_uploades/` 20) | |
| `negotiation_documents.cover_image_url` | 43 | 1 | 42 (`/images_uploades/` 40, `/documents_uploades/` 2) | |
| `media_gallery.media_url`, `thumbnail_url` | 11 | 11 (`activity-media/`) | | |
| `user_testimonials.photo_url` | 1 | 1 (`epavillonp/temoignage/<uuid>/`) | | |
| `zoom_meetings.banner_url` | 0 | | | |
| `migration_logs.old_user_data.photo_url` | 1 047 | | 1 047 chemins relatifs `images_uploades/<fichier>` | |
| `organizations.website` | 230 | | | 173 sites web, 57 chaînes vides (pas des fichiers) |

Toutes les URL Supabase ont la forme `https://jzkuvulxfhtcelpvrxgf.supabase.co/storage/v1/object/public/<bucket>/<chemin>`. Toutes les adresses de l'ancien serveur sont en `https://epavillonclimatique.francophonie.org/` ; aucune n'utilise l'IP `68.168.118.201`. Les textes libres (`activities.detailed_presentation` 16, `tags` 1) contiennent des liens vers des sites tiers (Google Drive 4, sites d'organisations), pas des fichiers de la plateforme : ils restent du texte.

### 3.3 Ancien serveur : fichiers à récupérer

| Source | Dossier | Fichiers distincts | Dans `storage.objects` |
|---|---|---|---|
| `negotiation_documents` (PDF) | `/images_uploades/`, `/documents_uploades/` | 19 | non (aucun nom ne correspond) |
| `negotiation_documents` (couvertures PNG, JPG) | `/images_uploades/`, `/documents_uploades/` | 20 | non |
| `events` CdP 30 (bandeau, logo) | `/public/images_uploades/` | 2 | non |
| `migration_logs` (photos de profil Laravel) | `images_uploades/` (relatif) | 1 046 (JPG 966, PNG 76, GIF 2, SVG, BMP, WebP) | non |
| **Total** | | **41 + 1 046** | |

Depuis la bascule DNS du 26/10, le domaine ne pointe plus sur l'ancien serveur : lecture par l'IP avec l'en-tête `Host`, comme prévu à l'étape 1.

## 4. Liens YouTube

Seule colonne remplie : `activities.youtube_link`, **33 valeurs, toutes l'identifiant seul** (11 caractères, ni URL ni `youtu.be`), toutes sur la CdP 30, sur des activités `completed` (32) et `live` (1). **Un identifiant sert deux activités** distinctes, alors qu'une vidéo ne sert qu'une séance en v2. Les autres colonnes vidéo (`innovations_practices.youtube_video_url`, `lesson_contents.youtube_url`, `video_testimonials.video_url`) sont dans des tables vides.

## 5. Constat de sécurité en passant

35 tables `public` ont la RLS active ; 39 ne l'ont pas, dont `user_roles`, `messages`, `negotiation_documents`, `activity_speakers`, `activity_documents`, toutes avec `GRANT ALL` au rôle `anon` (la clé publique du site). Quiconque lit la clé dans le code du site peut y lire et écrire, **y compris s'attribuer un rôle**. Supabase restant ouvert pour PACO (D9), c'est actif aujourd'hui. Pour la reprise : les rôles extraits ne sont pas garantis, à faire confirmer.

## 6. Écarts nouveaux à trancher

| Écart | Proposition |
|---|---|
| Documents de négociation importés deux fois de Laravel (43 lignes, 21 documents) | Reprendre le second passage (17:18) et le document saisi dans Supabase ; écarter le premier passage dans `id_map`, raison « import en double » |
| Un document rangé dans deux filières, la v2 n'en porte qu'une | Filière principale climat, la seconde en thématique par `reference.entity_terms` ; sinon étendre le modèle (filières multiples) |
| Cinq résumés typés « guide » ; un résumé désertification pointant sur le PDF d'un guide biodiversité | Typer les résumés `summary` ; faire vérifier le fichier du résumé désertification par le commanditaire avant reprise |
| Messages directs (73) : profil d'essai, 11 messages à soi-même | Écarter comme essais, raison dans `id_map` ; à défaut reprendre les seuls échanges entre deux personnes |
| Conversations avec l'assistant IA (10, 24, 2) : une seule personne, quatre jours de mise au point | Écarter comme essais ; à défaut reprendre sous l'assistant `negotiation_helper` |
| Type de notification `revision_comment` sans équivalent v2 | Étendre le catalogue : `programme.proposal.commented` (une ligne de `engagement.notification_types`, pas de colonne) |
| Notifications lues sans date de lecture | `read_at` ← `created_at`, approximation consignée dans `id_map` |
| Modèles de courriel v1 (5) : graines jamais modifiées, jamais lues par l'envoi | Non repris : la v2 porte déjà ses modèles (exception du principe 5) |
| Témoignage sans titre, exigé par `content.highlights` | Titre « Témoignage » suivi du nom affiché, modifiable au back-office ; statut `archived` si l'IFDD ne veut pas le voir revenir à l'accueil |
| Réunions Zoom (50) : D3 les écarte pour leurs secrets, mais elles datent l'activité et 48 activités y renvoient | Reprendre sans secret (`password`, `start_url`, `pwd` retiré de `join_url`), état `ended` ; l'activité refusée garde sa réunion par `event_id` seul |
| Liens Zoom personnels des inscrits (`tk=`, `pwd=`, 105) | Ne pas les reprendre : jetons d'accès, réunions closes |
| Galerie d'activité (11 photos, 4 activités) : `media_url` = `thumbnail_url` | Rôle `gallery` sur `programme.sessions` (D7), un seul objet par photo, `title` → `caption`, texte alternatif à écrire |
| `migration_logs` porte la fonction de 2 047 personnes, 1 041 photos, 136 téléphones, 37 biographies jamais recopiés dans Supabase | Lire `old_user_data` comme source de la carte des comptes : fonction → `job_title` (règle D5), photo, téléphone, biographie ; la table elle-même reste à l'écart (journal technique) |
| 1 046 photos de profil Laravel sur l'ancien serveur | Les récupérer avec les 41 autres fichiers à l'étape 1 ; une photo introuvable laisse la personne sans avatar |
| 102 objets orphelins (24,3 Mo), dont 2 vidéos de témoignage et 3 images de formation sans ligne | Ne pas reprendre ; garder la liste dans `id_map` (raison « non cité ») ; montrer les 2 vidéos au commanditaire avant de trancher |
| Vignettes en double (couvertures, intervenants, profils) | Ne reprendre que l'original, la v2 fabrique les déclinaisons |
| Bucket `video-thumbnails` absent, `documents` vide | Corriger la liste des buckets du plan (étape 1) |
| Identifiant YouTube partagé par deux activités | Le rattacher à la séance dont le titre correspond, l'autre sans rediffusion ; faire valider |
| Tables v1 sans RLS, écrivables avec la clé publique, dont `user_roles` | Faire relire les rôles par le commanditaire avant reprise ; fermer ces droits côté Supabase tant que PACO y vit |

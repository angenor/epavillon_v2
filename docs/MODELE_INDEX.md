# Index du modèle de données — quoi lire pour quoi

Ce fichier existe pour une raison précise : **une session Claude Code n'a pas la mémoire de la précédente**. Plutôt que de recharger tout le modèle à chaque fois — 14 143 lignes de SQL réparties sur 18 fichiers — on lit ici quels fichiers concernent la tâche du jour, et on ne lit que ceux-là.

Chaque fichier SQL porte sa propre documentation : en-tête expliquant les décisions de conception, `COMMENT ON` sur les tables et colonnes non évidentes. Les lire suffit ; il n'est pas nécessaire de consulter le cadrage pour coder.

---

## Par écran

| Écran / tâche | Fichiers à lire | Tables et fonctions clés |
|---------------|-----------------|--------------------------|
| **Authentification, profil** | `030_identity.sql` | `identity.people`, `accounts`, `sessions`, `one_time_tokens`, `person_emails` |
| **Rôles et permissions** | `030_identity.sql` | `roles`, `permissions`, `role_permissions`, `role_assignments` · `has_permission()`, `effective_permissions()`, `administered_events()` |
| **RGPD** | `030_identity.sql` | `consents`, `current_consents`, `privacy_requests` · `anonymize_person()` |
| **Rattachement / recherche d'organisation** | `040_organizations.sql` | `org.organizations`, `organization_names`, `organization_domains`, `memberships` · `find_similar_organizations()`, `public_email_domains` |
| **Fusion des doublons** | `040_organizations.sql` | `duplicate_candidates`, `merge_log`, `organization_references` · `merge_organizations()`, `resolve_organization()`, `compute_trust_score()` |
| **Gestion d'un événement** | `060_events.sql` | `event.event_series`, `events`, `event_days`, `venues`, `rooms`, `broadcast_channels` |
| **Journées spéciales** | `060_events.sql` + `075_programme_sessions.sql` | `event.programme_tracks` · `programme.session_tracks` |
| **Appel à propositions, grille de critères** | `060_events.sql` | `calls_for_proposals`, `review_criteria`, `call_reviewers` · `is_call_open()`, `effective_deadline()`, `seed_default_criteria()`, `max_weighted_score()` |
| **Formulaire de soumission** | `070_programme_proposals.sql` | `programme.proposals`, `proposal_organizations`, `proposal_speakers`, `proposal_documents` · `proposal_transitions_allowed` |
| **Suivi d'un dossier (espace organisation)** | `070_programme_proposals.sql` | `proposal_transitions`, `proposal_comments`, `proposal_reads` · `proposal_history()` |
| **Liste des propositions (back-office)** | `070_programme_proposals.sql` | `v_proposal_dashboard` (vue prête à l'emploi) · `proposal_transitions_allowed` (les actions offertes) · `unread_proposals_for()` (dossiers jamais ouverts par une personne) |
| **Fiche d'évaluation** | `070_programme_proposals.sql` + `060_events.sql` | `reviews`, `review_scores`, `review_assignments`, `event.review_criteria` · `refresh_proposal_score()` |
| **Planificateur de créneaux** | `075_programme_sessions.sql` | `programme.sessions`, `session_speakers`, `session_organizations`, `session_tracks` · `detect_conflicts()`, `publication_readiness()`, `session_schedule_history()` |
| **Programmation publique** | `075_programme_sessions.sql` | `v_public_schedule` (vue prête à l'emploi, état temporel calculé) |
| **Inscriptions** | `075_programme_sessions.sql` | `registration_forms`, `registration_form_fields`, `registrations` · `record_join()`, `promote_from_waitlist()` |
| **Questions du public** | `075_programme_sessions.sql` | `session_questions`, `session_question_answers`, `session_question_votes` |
| **Visio, diffusion, incidents** | `080_live.sql` | `live.meetings`, `meeting_participants`, `streams`, `incidents`, `provider_webhook_events` · `active_incidents()`, `build_embed_url()` |
| **Téléversement, images, quotas de stockage** | `050_media.sql` | `media.assets`, `renditions`, `attachments`, `attachable_roles`, `storage_quotas` · `object_url()`, `find_orphan_assets()` |
| **Notifications, courriels, rappels** | `110_engagement.sql` | `notification_types`, `notifications`, `notification_preferences`, `message_templates`, `email_messages`, `reminder_rules`, `scheduled_reminders` · `schedule_session_reminders()` |
| **Commentaires publics, réactions** | `110_engagement.sql` | `comments`, `commentable_subjects`, `reactions` |
| **Publications** | `090_publications.sql` | `publication.articles`, `publishing_policies`, `article_revisions`, `editorial_reviews` · `remaining_quota()` |
| **Négociations** | `100_negotiations.sql` | `negotiation.spaces`, `meetings`, `documents`, `channels`, `channel_messages` · `unread_message_counts()` |
| **Formations** | `125_training.sql` | `training.trainings`, `chapters`, `chapter_resources`, `enrollments`, `quizzes`, `quiz_questions`, `certificates` · `compute_progress()`, `score_attempt()`, `issue_certificate()` |
| **Sondages, assistants IA** | `120_tools.sql` | `tool.surveys`, `survey_questions`, `assistants`, `knowledge_chunks` · `search_chunks()` |
| **Tableaux de bord** | `130_analytics.sql` | `mv_proposal_funnel`, `mv_organization_scorecard`, `mv_session_attendance`, `mv_reviewer_workload`, `v_platform_overview`, `v_operational_health` |
| **Reprise des données v1** | `910_migration_v1.sql` | `legacy.id_map`, `organization_resolution`, `v_reconciliation` |

---

## Toujours utile, quel que soit l'écran

| Besoin | Fichier | Élément |
|--------|---------|---------|
| Types de base | `000_bootstrap.sql` | `platform.i18n_text`, `platform.email`, `platform.slug`, `platform.url`, `platform.timezone_name` |
| Résoudre un texte multilingue | `000_bootstrap.sql` | `platform.t(champ, locale)` — repli sur le français |
| Identifiants | `000_bootstrap.sql` | `platform.uuid_v7()` |
| Normalisation d'un libellé | `000_bootstrap.sql` | `platform.normalize_label()`, `slugify()`, `extract_domain()` |
| Publier un événement de domaine | `010_platform.sql` | `platform.emit_event()` |
| Différer un travail | `010_platform.sql` | `platform.jobs`, `claim_jobs()`, `fail_job()` |
| Historique d'une entité | `010_platform.sql` | `platform.entity_history()` |
| Activer / désactiver une fonctionnalité | `010_platform.sql` | `platform.feature_flags`, `is_feature_enabled()` |
| Pays, langues, thématiques, catégories | `020_reference.sql` | `reference.countries`, `locales`, `taxonomies`, `taxonomy_terms` |
| Rattacher une entité à des thématiques | `020_reference.sql` | `reference.entity_terms` · `terms_of()` pour **filtrer** (codes), `term_badges()` pour **afficher** (libellé, couleur) |

---

## Vues prêtes à l'emploi

Trois vues répondent à des écrans entiers en une requête. Les utiliser plutôt que de recomposer la jointure à la main :

- **`programme.v_public_schedule`** — la programmation publique, avec l'état temporel calculé (à venir / en cours / passé / reporté / annulé), le nom de la salle, l'organisation **et son pays**, les thématiques **avec leur libellé et leur couleur**, les journées spéciales et l'image de couverture, toutes déjà résolues. Elle répond à l'écran **en une requête** : une colonne qui manque coûte une requête par écran, ou un renoncement d'affichage.
- **`programme.v_proposal_dashboard`** — la liste des propositions du back-office : avancement des revues et **qui les doit**, revues **en retard**, revues manquantes, demandes de correction ouvertes, rang dans l'événement, et de quoi identifier un dossier dans un tableau dense — format, pays de l'organisation porteuse, thématiques **avec leur libellé et leur couleur**, nombre de co-organisateurs, nombre de lectures. Elle répond à l'écran **en une requête**. Ce qu'elle ne peut pas porter : « ce dossier, MOI, l'ai-je ouvert ? » dépend du lecteur — c'est `programme.unread_proposals_for(personne, édition)`.
- **`analytics.v_operational_health`** — l'état de santé du système : outbox en retard, travaux en échec, courriels en rebond, partitions manquantes.

---

## Ordre de chargement des fichiers

Les 18 fichiers sont numérotés dans leur ordre de dépendance. **Ne pas les réordonner.** Le catalogue détaillé — numéro, schéma, contenu de chacun — est le tableau « Ordre d'exécution » de [README.md](README.md) ; il n'est pas repris ici, ce document répondant à la question « quoi lire pour quoi » et non « que contient chaque fichier ».

---

## Vérifier le modèle

```sql
-- Frontières de modules : doit renvoyer zéro ligne
SELECT * FROM platform.cross_module_fk_report WHERE NOT is_compliant;

-- Ce qu'il faudrait couper pour extraire un module en service autonome
SELECT * FROM platform.generate_module_decoupling_script('tool');

-- Santé du système
SELECT * FROM analytics.v_operational_health;
```

---

## Si le modèle doit changer

1. Modifier le fichier SQL concerné dans `docs/database/`.
2. Recharger une base propre : `docker compose -f ops/docker-compose.dev.yml down -v && up -d`.
3. Vérifier que la chaîne complète passe et que le rapport de frontières reste vide.
4. Répercuter dans les types TypeScript et les structures Rust.
5. Noter le changement dans `docs/PROGRESSION.md`.

En production, ces fichiers sont découpés en migrations incrémentales tracées dans `platform.schema_migrations` — mais tant que la base n'est pas déployée, on modifie directement le fichier de référence.

# Tasks: Guide Négo — Réunions de la Francophonie (étape 4)

**Input**: [spec.md](spec.md) · [plan.md](plan.md) · [research.md](research.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

**Dossier** : `/Users/mac/Documents/projets/IFDD/epavillon_v2-dev2` — base `epavillon_dev2`, API 8096, site 3005.
**Règles** : CLAUDE.md, constitution, `/Users/mac/Documents/projets/IFDD/epavillon_v2/.orchestration/protocole.md` — seuls ses propres PID ; aucune boucle sur `pgrep -f` ; `cargo test --workspace` et `make check-safe` seulement sous verrou ; entre les phases, `SQLX_OFFLINE=true cargo test -p <crates touchés>`, `npm run typecheck`, `check:guide-nego`, `test:guide-nego`, `make check-api-contract`. `.sqlx` par `SQLX_OFFLINE_DIR`.
**Une phase = un commit**, dernière ligne `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`. `neg/` = `backend/crates/modules/negotiation/`, `fe/` = `frontend/`.

## Phase 1 — Le modèle

- [x] T001 Vocabulaire `francophone_meeting_type` (trois termes FR/EN) dans `docs/database/020_reference.sql` — data-model § 1
- [x] T002 `docs/database/100_negotiations.sql` : colonnes de `meetings` (`francophone_type_term_id` et sa garde, `access_audience`, `pavilion_session_id` en `xmod_fk_negotiation_meetings_pavilion_session` → `programme.sessions` `ON DELETE SET NULL`, `requires_registration`, `waitlist_enabled`, `ck_meetings_francophone_type`, index) ; `programme` ajouté aux dépendances d'en-tête ; commentaire de `is_open_access` élargi ; contraintes `ck_meetings_francophone_event`, `ck_meetings_francophone_kind`, `ck_meetings_access_audience`, trigger `tg_check_meeting_pavilion_edition` ; colonnes de `meeting_registrations` (`waitlist_position`, `client_ref`, contrainte, unicité) ; `tg_validate_meeting_registration` **selon research R4** (contrôles à l'entrée seulement, désinscription libre avant le début, promotion sans contrôle, comptage `count(*)` sous `FOR UPDATE`, liste d'attente non vide → attente) ; audit des inscriptions ; `promote_meeting_waitlist(p_meeting_id)` (places libres calculées sous verrou) ; `meeting_audience()` — data-model § 2 à 4, § 6
- [x] T003 Deux types de notification dans `docs/database/110_engagement.sql` (et le compte attendu par `backend/crates/modules/engagement/tests/harnais.rs`)
- [x] T004 `specs/016-guide-nego-reunions/migration.sql` rejouable ; sauvegarde de `epavillon_dev2` ; migration deux fois ; comparaison avec une base jetable chargée depuis `docs/database/` puis supprimée ; contrôles de comportement (capacité, attente, promotion, promotion après fermeture de la fenêtre, désinscription tardive, capacité abaissée puis relevée, fenêtre nulle = ouverte, réunion annulée, activité d'une autre édition refusée) ; `cargo build -p negotiation -p engagement`
- [x] T005 Une ligne dans `docs/progression/modele.md`

## Phase 2 — Lire et s'inscrire (API) — US1, US2

- [x] T006 [US1] Six codes au catalogue `backend/crates/kernel/src/error.rs`
- [x] T007 [US1] `GET /negotiation/meetings?edition=` (publiques, publiées, ETag, **jamais le lien visio**) dans `neg/src/{domain,repo,service,routes}/meetings.rs`
- [x] T008 [US2] `GET /negotiation/me/meeting-registrations` (`Cache-Control: private, no-store`, empreinte par personne), `PUT` et `DELETE /negotiation/me/meeting-registrations/{id}` dans `neg/src/{domain,repo,service,routes}/meeting_registrations.rs` : garde `negotiation.space.access` sur la portée globale ; même `client_ref` → état courant ; ligne désinscrite + `client_ref` neuf → réinscription ; refus de la base traduits ; désinscription → `promote_meeting_waitlist()` dans la même transaction ; lien visio = `external_url` pour les inscrites, et pour toute admise si la réunion ne demande pas d'inscription (research R6)
- [x] T009 OpenAPI, `make openapi`, formes TS dans `fe/app/types/negotiation-meetings.ts` (ré-exportées)
- [x] T010 [P] [US1] Tests `neg/tests/reunions_lecture.rs` : brouillon absent, publiée présente, **aucun lien visio dans la réponse publique**, `304`
- [x] T011 [P] [US2] Tests `neg/tests/reunions_inscription.rs` : inscrite, complet → attente avec position, complet sans attente `409`, fenêtre close, annulée, rejeu `client_ref` (une ligne), **concurrence sur la dernière place** (SC-003), désinscription → promotion, désinscription puis réinscription (nouveau `client_ref`), nouvelle venue derrière une liste d'attente non vide, lien visio servi à l'inscrite seulement, `403`/`401`

## Phase 3 — Back-office — US3

- [x] T012 [US3] Routes `/admin/negotiation/meetings…` (liste, créer, modifier, publier, annuler, lier au Pavillon, inscrites) et `/admin/negotiation/pavilion-activities` dans `neg/src/{domain,repo,service,routes}/admin_meetings.rs` ; `MEETING_MANAGE` ajoutée à `neg/src/domain/permissions.rs` ; garde `Requires<MeetingManage>` **sur la portée globale** (research R9 ; jamais `RequiresAnyScope`) ; ajouter les nouvelles routes à la relecture de `neg/tests/perimetre_url_forgee.rs` (liste des fichiers de routes seulement : il est proche de 1000 lignes) ; le serveur pose espace, slug, édition, fuseau, `kind` (R9 bis) ; organisateur (IFDD ou organisation lue en SQL dans `org`) ; relever/retirer la capacité promeut ; invariants traduits en `400 NEGOTIATION_MEETING_INVALID`, y compris à la publication ; OpenAPI, `make openapi`
- [x] T013 [P] [US3] Tests `neg/tests/reunions_admin.rs` : garde (sans permission refusé, adresse forgée refusée, **administrateur d'une seule édition refusé**, `space_lead` refusé), publier en ligne sans lien refusé, capacité relevée → promotion, deux réunions qui se chevauchent acceptées (FR-019), saisie, publication, annulation avec motif, lien vers une activité de l'édition (autre édition refusée), activité supprimée → lien nul, aucune écriture dans `programme`
- [x] T014 [US3] Écrans `fe/app/pages/admin/negociations/reunions/{index,nouvelle,[id]}.vue` et `fe/app/components/admin/negotiation/Meeting*.vue` (composants `ui/` du site : `UiDatePicker` dans le fuseau de l'édition — patron `components/admin/incidents/Form.vue` —, `UiCombobox` pour l'activité du Pavillon), méthodes dans `fe/app/composables/api/admin-negotiations.ts`, textes `i18n/locales/{fr,en}/pages/admin.negociations.reunions.json`, entrée de menu dans `fe/app/layouts/admin.vue` avec `permissions: ['negotiation.meeting.manage']` et sa clé dans `i18n/locales/{fr,en}/_nav.json` ; champ « Organisée par l'IFDD » (et organisation sinon, par le composant de choix d'organisation du site s'il existe) ; quatre états
- [x] T015 [US3] Vérifier au navigateur (version construite) : quickstart § 1

## Phase 4 — Prévenir — US2, US3

- [ ] T016 [US3] Constantes des deux événements dans `backend/crates/contracts/src/negotiation.rs` ; annulation et changement d'heure ou de lieu d'une réunion publiée (ancienne et nouvelle valeur comparées par le service à l'écriture) → destinataires par `meeting_audience()`, avis par `neg/src/notifications/avis.rs`, événement `negotiation.francophone_meeting.changed` avec la charge `notification`, courriels par une **cible nouvelle `FrancophoneMeeting`** du travail `neg/src/jobs/change_email.rs` avec sa propre composition dans `mail.rs` (lien `/guide-nego/francophonie/reunions/<id>`, titre i18n, heure, lieu, motif) — research R8
- [ ] T017 [US2] Place obtenue depuis la liste d'attente (désinscription ou capacité relevée) → `negotiation.meeting_registration.promoted` pour chaque personne promue, courriel par un **travail distinct** (clé `promotion:<meeting>:<personne>`) selon l'accord
- [ ] T018 [P] Tests `neg/tests/reunions_avis.rs` : destinataires, charge émise, courriel posé, accord éteint, aucun avis pour un brouillon

## Phase 5 — Plomberie client

- [ ] T019 `fe/app/composables/api/negotiation-meetings.ts` et une ligne de montage dans `useApi.ts`
- [ ] T020 [P] Règles `fe/app/utils/guide-nego/reunions.ts` et tests `fe/tests/guide-nego/reunions.test.ts` — contracts/client.md
- [ ] T021 `useGnReunions` (garde `reunions:<slug>`) et `useGnInscriptionsReunions` (file, clé `inscription-reunion:<id>`, **`client_ref` neuf à chaque geste**, `409` → abandon et message ; garde `mes-inscriptions-reunions` : le lien visio en sort dès l'intention de désinscription, la clé entre dans la liste d'effacement de `useGnSession` à la déconnexion et au retrait d'accès) dans `fe/app/composables/guide-nego/` ; jeux d'exemple `fe/app/mocks/`

## Phase 6 — US1 : l'onglet et la fiche

- [ ] T022 [US1] `pages/guide-nego/francophonie/index.vue` (remplace `francophonie.vue`) : `GnSegmente` deux segments, `?section=`, titre = nom complet, `GnSectionReunions` / `GnSectionPavillon` (état vide seulement)
- [ ] T023 [US1] `GnLigneReunion`, `GnEtiquettePavillon`, `GnSectionReunions` (liste, une marque par ligne, accès limité, fuseau dit une fois au-dessus des heures et dans le nom accessible, pied de liste, états chargement / vide / hors connexion) ; planche
- [ ] T024 [US1] `pages/guide-nego/francophonie/reunions/[id].vue` : heure avec jour et fuseau, lieu ou « En ligne », visioconférence (lien pour l'inscrite, sinon « Réservé aux personnes inscrites »), organisateur, nature, accès limité, description, étiquette Pavillon ; textes
- [ ] T025 [US1] Vérifier au navigateur : quickstart § 2

## Phase 7 — US2 : s'inscrire

- [ ] T026 [US2] Bouton d'inscription sur la fiche (« M'inscrire » → « Inscrite », « Rejoindre la liste d'attente », « Liste d'attente — position N », « Inscriptions closes », désinscription) ; sans compte ou sans accès → connexion ou accès ; refus au retour dits (R5)
- [ ] T027 [US2] Vérifier au navigateur : quickstart § 3 et § 4 (Mailpit)

## Phase 8 — US4 : Ma journée, recherche

- [ ] T028 [US4] Bloc « trois agendas » de `pages/guide-nego/index.vue` : `journee/GnJourneeLigneSessions.vue`, `GnJourneeLigneReunions.vue`, `GnJourneeLignePavillon.vue` (vide) ; `fe/tests/guide-nego/ma-journee.test.ts` vert
- [ ] T029 [US4] Si `main` porte l'étape 2 : `git merge main` (conflits : `progress.md` garde les deux, `api.ts` par `make openapi`, SQL garde les deux sections), puis `reunionsTrouvees` dans `utils/guide-nego/recherche-globale.ts`, le groupe dans la page de recherche, la clé `pas-encore` ne nomme plus que le Pavillon ; sinon, noter la tâche pour la fusion
- [ ] T030 [US4] Vérifier au navigateur : quickstart § 5

## Phase 9 — Recette

- [ ] T031 Quickstart entier, version construite, 360 px, clair et sombre ; corriger
- [ ] T032 [P] Écarts de l'étape dans `docs/AppNego/05-design.md` (à la suite du dernier numéro)
- [ ] T033 [P] § 15 de `docs/DEPLOIEMENT.md` : migration 016 dans l'ordre ; essais sur appareil réel (s'inscrire en mode avion, lien visio hors connexion, liste d'attente)
- [ ] T034 i18n `en` contre `fr`, clé pour clé
- [ ] T035 Une ligne au journal de `docs/AppNego/progress.md` (la ligne d'état à la clôture)

## Dependencies

1 → 2 → 3 → 4 (même crate). 5 après 2. 6 → 7 → 8 (mêmes écrans). 3 et 5 peuvent se suivre dans n'importe quel ordre.

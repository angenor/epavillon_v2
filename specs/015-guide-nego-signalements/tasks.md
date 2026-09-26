# Tasks: Guide Négo — signalements et notifications (étape 3b)

**Input**: [spec.md](spec.md) · [plan.md](plan.md) · [research.md](research.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

**Dossier** : `/Users/mac/Documents/projets/IFDD/epavillon_v2-dev2` — base `epavillon_dev2`, API 8096, site 3005.
**Règles** : CLAUDE.md, la constitution, `/Users/mac/Documents/projets/IFDD/epavillon_v2/.orchestration/protocole.md` — on n'arrête que ses propres PID ; jamais `make check`, `down -v`, `pkill`.
**Une phase = un commit**, message en français, dernière ligne `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`. Contrôles ciblés : `SQLX_OFFLINE=true cargo test -p negotiation -p engagement`, `npm run typecheck`, `check:guide-nego`, `test:guide-nego`, `make check-api-contract`. `backend/.sqlx` se met à jour par `SQLX_OFFLINE_DIR` (voir les phases de 3a).

`neg/` = `backend/crates/modules/negotiation/`, `eng/` = `backend/crates/modules/engagement/`, `fe/` = `frontend/`.

---

## Phase 1 — Le modèle (bloquant)

- [x] T001 Écrire dans `docs/database/100_negotiations.sql` : ENUM `negotiation.report_status`, tables `session_reports` (dont `proposed_day`, `published_at`, `ck_session_reports_decision`, aucun déclencheur d'outbox), `network_meetings`, `network_agenda_entries`, colonne `theme_subscriptions.notify_changes`, fonctions `negotiation.change_recipients()` et `negotiation.network_recipients()`, contraintes, index, `xmod_fk_*`, gardes de vocabulaire, audit, commentaires français — data-model § 1 à 4
- [x] T002 Semer la permission `negotiation.report.validate` (portée globale) et l'attribuer au rôle `admin`, à côté des permissions de `negotiation` dans `docs/database/100_negotiations.sql` — data-model § 5
- [x] T003 Semer les quatre `engagement.notification_types` dans `docs/database/110_engagement.sql` — data-model § 6
- [x] T004 Écrire `specs/015-guide-nego-signalements/migration.sql`, rejouable, dans une transaction (patron `specs/014-guide-nego-sessions-agenda/migration.sql`)
- [x] T005 Sauvegarder `epavillon_dev2`, jouer la migration deux fois, comparer à une base jetable chargée depuis `docs/database/` (puis la supprimer) ; `cargo build -p negotiation -p engagement`
- [x] T006 Une ligne dans `docs/progression/modele.md`

**Commit** : « feat(guide-nego): étape 3b, phase 1 — le modèle des signalements ».

## Phase 2 — Signaler, côté API (US1)

- [x] T007 [US1] Ajouter les six codes au catalogue `backend/crates/kernel/src/error.rs`
- [x] T008 [US1] Écrire `POST /negotiation/reports` (garde `negotiation.space.access` globale ; `client_ref` rejoué → `200` même signalement ; doublon en attente → `409` ; validation des champs par motif) et `GET /negotiation/me/reports` (ETag) dans `neg/src/{domain,repo,service,routes}/reports.rs`
- [x] T009 [US1] Ajouter `can_validate_reports` à `GET /negotiation/me/access` (`neg/src/domain/access.rs` et son service) ; déclarer la permission dans `neg/src/domain/permissions.rs`
- [x] T010 [US1] Monter, déclarer à l'OpenAPI, `make openapi` ; formes TS dans `fe/app/types/negotiation-reports.ts` (ré-exportées par `fe/app/types/index.ts`)
- [x] T011 [P] [US1] Tests sur base réelle `neg/tests/signalements.rs` : nominal par motif, réunion non annoncée, rejeu `client_ref` (une ligne, `200`), doublon `409`, sans accès `403`, sans compte `401`, précision > 600 refusée, audit

**Commit** : « feat(guide-nego): étape 3b, phase 2 — signaler, côté API ».

## Phase 3 — Valider et afficher, côté API (US2, US3)

- [x] T011a [US2] (fait en phase 2) Déclarer d'abord les constantes d'événements et de travaux dans `backend/crates/contracts/src/negotiation.rs` (quatre événements, `negotiation.report.publish`, `negotiation.session_change_email`)
- [x] T012 [US2] Écrire la file et les décisions (`GET /admin/negotiation/reports`, `POST …/validate`, `…/undo`, `…/reject`, `…/withdraw`), montées dans `admin_routes()`, sous `Requires<ReportValidate>` globale, dans `neg/src/{domain,repo,service,routes}/admin_reports.rs` ; `source_now`, `source_snapshot`, `decided_by` ; valider **ne rend rien public** et pose la publication (T013) ; annuler = `UPDATE … WHERE status='validated' AND published_at IS NULL` sans borne de temps, sinon `409` ; retirer — research R3
- [x] T013 [US2] Écrire le travail `negotiation.report.publish` dans `neg/src/jobs/publish.rs` : `run_at = now() + 30 s` calculé par la base, clé `publish:<report>:<decided_at>` ; dans une transaction, `SELECT … FOR UPDATE`, n'agit que si `validated`, même `decided_at`, `published_at` nul, non retiré ; pose `published_at`, crée la `network_meeting`, émet `report.published` / `network_meeting.published` et `report.decided`, pose les courriels (T021) ; rejoué, ne fait rien ; inscrit dans `job_handlers()`. Le refus émet `report.decided` directement
- [x] T014 [US3] Étendre `GET /negotiation/sessions` : `network_reports` par session (validés, non retirés, session non terminée) et `network_meetings` (**publiées**, non retirées, jour non passé) **servies aussi quand l'affichage est coupé** ; `network_reports` : **publiés** seulement ; « Mes signalements » montre `submitted` tant que rien n'est publié — `neg/src/repo/sessions.rs`, `domain/sessions.rs` ; aucun nom d'autrice
- [x] T015 [US3] Écrire le rapprochement pur `neg/src/import/rattrapage.rs` (tests unitaires) et l'appeler dans `neg/src/jobs/import.rs::ecrire` **après** l'écriture de la lecture, sur l'état en base, absences comprises : retrait `caught_up` dans la transaction de lecture ; salles comparées par `denominations::normaliser`
- [x] T015a [US3] Écrire `PUT`/`DELETE /negotiation/me/agenda/network/{id}` et `network_entries` dans `GET /negotiation/me/agenda` (`neg/src/{repo,service,routes}/agenda.rs`) — contracts/api-signalements.md § Agenda
- [x] T016 OpenAPI, `make openapi`, formes TS de la file dans `fe/app/types/negotiation-reports.ts`
- [x] T017 [P] [US2] Tests `neg/tests/validation.rs` : garde (un administrateur d'une seule édition refusé), file et `source_now`, valider → rien de public avant la publication, puis encart servi ; **valider puis annuler → aucun événement d'outbox ni travail de courriel après exécution du travail (SC-004)** ; **concurrence** : annulation lancée pendant que le travail tient la ligne → l'un ou l'autre, jamais les deux ; travail rejoué après publication → aucun second événement ; annuler après publication `409`, refus → motif chez l'autrice, retrait
- [x] T018 [P] [US3] Tests `neg/tests/par_dessus.rs` : ligne de `meetings` et `meeting_changes` identiques avant/après validation (SC-003), rattrapage par l'import (salle, heure, annulation ; « Autre » jamais), encart d'une session terminée non servi, réunion non annoncée servie quand coupé, aucun nom d'autrice dans la réponse publique (SC-008)

**Commit** : « feat(guide-nego): étape 3b, phase 3 — valider et afficher par-dessus ».

## Phase 4 — Notifier, côté API (US4, US5)

- [x] T019 [US4] (après T021) Dans `neg/src/jobs/import.rs`, pour les changements `start`, `venue` et les annulations d'une session importée : calculer les destinataires par `negotiation.change_recipients()`, composer l'avis (FR/EN, `neg/src/notifications/avis.rs` partagé avec la publication) et émettre `negotiation.meeting.changed` avec la charge `notification` ; poser les courriels dans la même transaction. Le travail de publication (T013) fait de même pour ses trois événements
- [x] T020 [US4] Ajouter au consommateur `eng/src/consumers/notifications.rs` une **branche générique** : tout événement dont la charge porte `notification` (forme dans `backend/crates/contracts/`) et dont le type est actif → un avis `in_app` par destinataire reçu, après `canal_autorise` ; option `replace` ajoutée à `eng/src/repo/notifications.rs::ecrire` (titre, corps, variables remplacés sur la ligne non lue) ; **aucun appel ni lecture de `negotiation`** ; filtre `module` (par `notification_types.module_code`) ajouté à `GET /notifications` (`eng/src/routes/notifications.rs`) ; les branches de `programme` inchangées — research R8, contracts/notifications.md
- [x] T021 [US4] Écrire le travail `negotiation.session_change_email` (un travail par destinataire, clé `email:<cible>:<tranche de 10 min>:<personne>`, relit l'état et les seuls signalements publiés, vérifie l'accord, heures avec le fuseau de la COP) dans `neg/src/jobs/change_email.rs` et ses gabarits dans `neg/src/mail.rs` ; l'inscrire dans `job_handlers()`
- [x] T022 [US5] Écrire `GET`/`PUT /negotiation/me/notifications` (accord dans `identity.consents`, `guide_nego_notifications`, version servie, aucune ligne = allumé) et `PUT /negotiation/me/themes/notifications` + `notify` dans `GET /negotiation/me/themes` (**l'empreinte l'inclut**), dans `neg/src/{domain,repo,service,routes}/notifications.rs` et `themes.rs` ; OpenAPI, `make openapi`, formes TS
- [x] T023 [P] [US4] Tests — branche générique, regroupement avec `replace`, filtre `module` et non-régression de `programme` dans `eng/tests/notifications_generiques.rs` ; destinataires calculés, charge émise, courriels et accord dans `neg/tests/notifications.rs` : destinataires (agenda toujours, thématique seulement allumée, autrice pour la décision), regroupement par session et jour, un courriel pour deux changements dans la fenêtre (SC-006), accord éteint → aucun courriel mais la notification, aucun courriel pour une validation annulée
- [x] T024 [P] [US5] Tests des réglages dans `neg/tests/reglages_notifications.rs` : accord (bascule, version, défaut allumé), thématiques de notification limitées aux suivies, quitter une thématique l'éteint

**Commit** : « feat(guide-nego): étape 3b, phase 4 — notifier ».

## Phase 5 — Plomberie client

- [ ] T025 Client `fe/app/composables/api/negotiation-reports.ts` (signaler, mes signalements, file, décisions, réglages) et `fe/app/composables/api/notifications.ts` (liste, marquer lu) ; lignes de montage dans `fe/app/composables/useApi.ts`
- [ ] T026 [P] Règles pures `fe/app/utils/guide-nego/signalements.ts` (encart affiché ou non : terminé, retiré ; repère ; libellé lecteur d'écran ; réunion non annoncée du jour et fin du jour dans le fuseau ; état local « en attente d'envoi ») et tests `fe/tests/guide-nego/signalements.test.ts`
- [ ] T027 Composables `useGnSignalements` (file avec clé `signalement:<client_ref>`, garde `mes-signalements`, `409` abandonne et le dit), `useGnValidation` (en ligne seulement), `useGnNotifications` (garde `notifications`, filtre `module=negotiation`, marquer lu en file **une intention par notification** `lu:<id>`), `useGnReglageNotifications` dans `fe/app/composables/guide-nego/` ; `useGnSessions` expose `network_reports` et `network_meetings`, y compris en coupure

**Commit** : « feat(guide-nego): étape 3b, phase 5 — la plomberie des signalements côté application ».

## Phase 6 — US1 : signaler (écrans)

- [ ] T027a [US1] Créer `GnFeuilleSignaler` (quatre motifs à pictogramme, précision facultative 600, valeur proposée pour l'heure et la salle, bouclier, « Envoyer » actif d'emblée, « Annuler ») dans `fe/app/components/guide-nego/GnFeuilleSignaler.vue`, textes `gn-feuille-signaler.json`, planche
- [ ] T028 [US1] Fiche `fe/app/pages/guide-nego/negociations/[id].vue` : « Signaler un changement » (sans accès → accès), message éphémère « Signalement envoyé. Vérification en cours. » + « Voir », ligne « Votre signalement — envoyé à … »
- [ ] T029 [US1] Écran `fe/app/pages/guide-nego/negociations/non-annoncee.vue` (Quoi requis, Où, Quand, thématique) et lien en bas de la liste `negociations/index.vue`
- [ ] T030 [US1] Écran « Mes signalements » `fe/app/pages/guide-nego/negociations/signalements.vue` (trois états + en attente d'envoi, motif de refus, vide, hors connexion) ; lien depuis le profil
- [ ] T031 [US1] Vérifier au navigateur, version construite, 360 px, clair et sombre : quickstart § 1

**Commit** : « feat(guide-nego): étape 3b, phase 6 — signaler ».

## Phase 7 — US2, US3 : valider et afficher (écrans)

- [ ] T032 [US3] Créer `GnEncartSignalement` (violet, losange, « Signalé par le réseau — validé par l'IFDD à … », motif, valeur, précision ; jamais de nom) et la prop `signale` de `GnLigneSession` (losange + « Signalé » + libellé complet) ; rendu d'une réunion non annoncée dans `GnLigneSession` ; planche
- [ ] T033 [US3] Poser l'encart sur la fiche, le repère et les réunions non annoncées dans la liste (y compris coupée, avec la phrase de la maquette) et dans « Mon agenda » (ajout d'une réunion non annoncée à l'agenda, T015a) ; fiche `fe/app/pages/guide-nego/negociations/reseau/[id].vue`
- [ ] T034 [US2] Écran `fe/app/pages/guide-nego/validation/signalements.vue` (cartes, source du moment, « Valider » → « Validé. Affiché dans une minute au plus. » + `GnMessageEphemere` « Annuler » six secondes, feuille « Ne pas retenir » à trois motifs, « Retirer », traités du jour, vide, sans réseau) ; entrée « Signalements » dans Ressources si `can_validate_reports`
- [ ] T035 Vérifier au navigateur : quickstart § 2 et § 3

**Commit** : « feat(guide-nego): étape 3b, phase 7 — valider et afficher par-dessus ».

## Phase 8 — US4, US5 : notifier (écrans)

- [ ] T036 [US4] Créer `GnCloche` (compteur jaune, 48 px, libellé) dans l'emplacement `action` de `GnEntete` (accueil et onglets) et `GnLigneNotification` ; planche
- [ ] T037 [US4] Écran `fe/app/pages/guide-nego/notifications.vue` (par jour dans le fuseau de la COP, non lue en 600 + carré jaune, toucher → lu + fiche, « Tout marquer comme lu », vide, hors connexion)
- [ ] T038 [US5] Profil `ressources/reglages.vue` : « Notifications par thématique » (une ligne par thématique suivie, éteinte par défaut) ; « À propos » : l'interrupteur « Notifications » (courriel) et sa phrase
- [ ] T039 Vérifier au navigateur : quickstart § 4, § 5 (Mailpit)

**Commit** : « feat(guide-nego): étape 3b, phase 8 — notifier ».

## Phase 9 — Recette

- [ ] T040 Dérouler `quickstart.md` en entier, version construite, 360 px, clair et sombre ; corriger
- [ ] T041 [P] `docs/AppNego/05-design.md` : mettre à jour l'écart 40 (« Notifications » livré en 3b, les deux autres non) ; inscrire les écarts de 3b (« validé par l'IFDD » partout, entrée « Mes signalements » dans le profil, repère « Signalé » dans les listes)
- [ ] T042 [P] § 15 de `docs/DEPLOIEMENT.md` : migration 015 dans l'ordre ; essais sur appareil réel (signaler en mode avion, valider au doigt avec « Annuler », cloche et centre, courriel reçu)
- [ ] T043 i18n `en` contre `fr`, clé pour clé, sur les fichiers de 3b
- [ ] T044 Une ligne au journal de `docs/AppNego/progress.md` ; la ligne d'état « 3b » est posée à la clôture ; écart pour « Validé. Affiché dans une minute au plus. »

**Commit** : « feat(guide-nego): étape 3b, phase 9 — recette et finitions ».

---

## Dependencies

Phase 1 bloque tout. 2 → 3 → 4 (même crate, même fichier d'import) ; dans la phase 4, T021 avant T019. 5 dépend des formes de 2 à 4 ; 6 → 7 → 8 (mêmes écrans). La phase 5 peut commencer après la phase 3 si les formes de la phase 4 sont posées d'abord.

## Implementation Strategy

MVP = phases 1 à 3 et 5 à 7 : signaler, valider, afficher par-dessus. Les notifications (4, 8) complètent le critère de l'étape.

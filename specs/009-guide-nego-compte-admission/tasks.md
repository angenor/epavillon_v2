---

description: "Tâches — Guide Négo 0b, compte et admission"
---

# Tasks: Guide Négo — compte et admission (étape 0b)

**Input**: `specs/009-guide-nego-compte-admission/` — [spec](spec.md) · [plan](plan.md) ·
[research](research.md) · [data-model](data-model.md) · [migration.sql](migration.sql) ·
[contracts/](contracts/) · [quickstart](quickstart.md)

**Tests** : **obligatoires**, et ce n'est pas un choix de style — le principe X de la constitution les
exige sur base réelle et jetable, avec quatre points par module : chemin nominal, refus par périmètre
en URL forgée, traduction d'un invariant de base, écriture de l'outbox.

**Organisation** : par récit utilisateur, pour que chaque tranche se livre et s'éprouve seule.

## Format : `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichiers différents, aucune dépendance en attente
- **[Story]** : US1 à US5, selon [spec.md](spec.md)
- Les identifiants **« bis », « ter », « quater »** sont les tâches ajoutées après `/speckit-analyze`
  du 21/09. Elles gardent le numéro de la tâche qu'elles suivent plutôt que de renuméroter les 113
  autres et d'invalider toutes les références croisées.

## Ce qu'il ne faut jamais lancer

`make check`, `make check-db` et `docker compose … down -v` **détruisent la base locale**, qui n'a pas
de sauvegarde. La porte est `make check-safe`, et elle se lance **en fin de cycle**, pas à chaque
phase.

---

## Phase 1 : Le modèle, et sa migration

**Objet** : le SQL d'abord (principe I), puis le chemin qui l'amène dans une base en service.

**⚠️ Bloquant** : SQLx ne compile pas sans la base migrée. Rien ne commence avant.

- [X] T001 Ajouter le type `identity.session_client` et les quatre colonnes d'appareil à `identity.sessions`, avec `ix_sessions_client` et leurs `COMMENT ON`, dans `docs/database/030_identity.sql` § sessions
- [X] T002 [P] Ajouter la taxonomie `negotiation_network` (`is_system = true`) et le terme `women_negotiators` dans `docs/database/020_reference.sql`
- [X] T003 Ajouter `negotiation.invitation_codes` — contraintes `ck_invitation_codes_scope`, `_uses`, **`_quota`**, `_period`, index `ux_invitation_codes_normalized` (unicité **totale**, révoqués compris) — dans `docs/database/100_negotiations.sql` après le § 2
- [X] T004 Ajouter `negotiation.invitation_code_uses` et son trigger `tg_invitation_code_uses_count` dans `docs/database/100_negotiations.sql`
- [X] T005 Ajouter `negotiation.network_memberships` et ses triggers dans `docs/database/100_negotiations.sql`
- [X] T006 Ajouter `negotiation.access_requests`, le type `access_request_status`, les deux index partiels d'unicité et les triggers `tg_access_request_transition` et `tg_access_request_event` dans `docs/database/100_negotiations.sql`
- [X] T007 Ajouter `negotiation.invitation_code_attempts`, le type `invitation_attempt_outcome` et la fonction `invitation_attempts_recent(uuid, interval)` — **sans argument d'appareil** — dans `docs/database/100_negotiations.sql`
- [X] T008 Ajouter les vues `negotiation.v_invitation_codes` et `negotiation.v_invitation_code_uses` dans `docs/database/100_negotiations.sql`
- [X] T009 [P] Semer `negotiation.admission_mode` (`"code"`) et `negotiation.invitation_attempts` dans `docs/database/900_seed.sql`, plus un espace et deux codes d'exemple pour le développement local
- [X] T010 **Relire et compléter** `specs/009-guide-nego-compte-admission/migration.sql` — écrit avec le plan — contre les fichiers de `docs/database/` modifiés en T001 à T009 : type et colonnes de session, taxonomie, cinq tables, vues, fonctions, triggers, réglages. **Rejouable sans dégât**, chaque objet créé sous condition
- [X] T011 Sauvegarder la base locale (`pg_dump`), puis **jouer `specs/009-guide-nego-compte-admission/migration.sql`, puis le rejouer** : la seconde passe ne doit rien changer ni rien faire échouer
- [X] T012 Comparer les schémas (§ 13 de `docs/DEPLOIEMENT.md`, étape 6) : `pg_dump --schema-only` de la base migrée contre celui de la base modèle du harnais, triés — **aucun écart** hors partitions `engagement.email_messages_AAAAMM`
- [X] T013 `make check-db-safe` au vert, puis consigner le changement de modèle dans `docs/progression/modele.md` — seule écriture hors `docs/AppNego/` autorisée (ADR-017)

**Point de contrôle** : la base locale porte le nouveau schéma, **sans avoir été détruite**, et la migration est prouvée rejouable.

---

## Phase 2 : Le crate `negotiation` existe

**Objet** : le module naît, monté et vide. Toutes les routes suivantes s'y posent.

**⚠️ Bloquant** : aucun récit ne commence avant.

- [X] T014 Créer `backend/crates/modules/negotiation/Cargo.toml`, calqué sur celui d'`identity`, **dépendances limitées à `kernel` et `contracts`** (principe II), et l'inscrire aux membres et alias de `backend/Cargo.toml`
- [X] T015 Créer `backend/crates/modules/negotiation/src/lib.rs` : `NegotiationState`, `routes(cfg)`, `admin_routes(cfg)`, `job_handlers(...)`
- [X] T016 [P] Créer `backend/crates/contracts/src/negotiation.rs` (constantes d'agrégat et les six types d'événements) et l'exporter depuis `backend/crates/contracts/src/lib.rs`
- [X] T017 [P] Déclarer les permissions typées dans `backend/crates/modules/negotiation/src/domain/permissions.rs` : `negotiation.space.access`, `negotiation.space.manage` — **jamais un nom de rôle**
- [X] T018 Monter le crate dans l'API : dépendance de `backend/crates/api/Cargo.toml`, champ et construction dans `src/state.rs`, bloc `is_mounted("negotiation")` et `app_data` dans `src/lib.rs`, code dans la liste figée de `src/modules.rs`
- [X] T019 Ajouter `doc.merge(negotiation::routes::openapi::NegotiationApi::openapi())` dans `backend/crates/api/src/openapi.rs` et créer `negotiation/src/routes/openapi.rs` — **après T018**, qui pose la dépendance de crate
- [X] T020 [P] Monter `negotiation::job_handlers(...)` dans `backend/crates/worker/src/main.rs`, avec `backend/crates/worker/Cargo.toml` — **la chaîne de purge s'y branche en T091**, quand `jobs/purge.rs` existe
- [X] T021 [P] Ajouter les cinq codes d'erreur à la macro `codes!` de `backend/crates/kernel/src/error.rs` : `NegotiationAccessRequestPending`, `NegotiationAccessRequestDecided`, `NegotiationInvitationCodeDuplicate`, `NegotiationAdmissionModeInvalid`, `NegotiationSpaceUnknown`
- [X] T022 Créer `backend/crates/modules/negotiation/tests/commun/mod.rs` sur le modèle de celui d'`identity` : base jetable, état, semis d'un espace, d'un code et d'un compte
- [X] T023 Vérifier dans `backend/crates/modules/negotiation/Cargo.toml` et le graphe du workspace qu'**aucune arête ne relie deux crates de modules** — contrôle mécanique de la constitution

**Point de contrôle** : `cargo run -p api` démarre, le module est monté, `make openapi` passe.

---

## Phase 3 : US1 — Entrer avec son compte (P1) 🎯 MVP

**But** : créer un compte ou se connecter depuis l'application, avec **une session qui dit d'où elle
vient, qui dure une COP, et des courriels qui ramènent dans l'application**.

**Épreuve indépendante** : créer un compte depuis l'application, confirmer son adresse depuis le
courriel, revenir dans l'application, se déconnecter, se reconnecter — puis se connecter au site avec
la même adresse, sans second compte.

### Tests

- [X] T024 [P] [US1] Test : une session ouverte avec `client.kind = "app"` porte `client_kind = 'app'` et son appareil — `backend/crates/modules/identity/tests/session_client.rs`
- [X] T025 [P] [US1] Test : **la rotation recopie le client** — après `refresh`, `client_kind` vaut toujours `'app'`. C'est le piège n° 1 : sans ce test, rien n'échoue et les chiffres mentent — `identity/tests/session_rotation.rs` (étendu)
- [X] T026 [P] [US1] Test : une session `app` dure la durée longue **sans case cochée** ; le site garde 12 h, et 30 j avec « se souvenir de moi » ; la durée **repart** à chaque rotation — `identity/tests/session_duree_app.rs`
- [X] T027 [P] [US1] Test : le lien du courriel de vérification mène à `/guide-nego/…` quand la demande vient de l'application, et à l'écran du site sinon — `identity/tests/courriel_retour_app.rs`
- [X] T028 [P] [US1] Test : un appel sans objet `client` se comporte exactement comme avant — **non-régression du site** — `backend/crates/api/tests/routes_auth.rs` (étendu)
- [X] T028 bis [P] [US1] Test : une personne inscrite **sur le site** se connecte depuis l'application sans second compte, et l'inverse (FR-003, SC-004) — `identity/tests/compte_unique_deux_clients.rs`
- [X] T028 ter [P] [US1] Test : une inscription depuis l'application avec une adresse **déjà connue** rend la **même réponse** qu'avec une adresse inconnue (FR-002) — `identity/tests/inscription_reponse_invariable.rs` (étendu)
- [X] T028 quater [P] [US1] Test : la déconnexion d'un appareil **laisse les autres sessions ouvertes** (FR-006) — `identity/tests/deconnexion_un_seul_appareil.rs`

### Back

- [X] T029 [US1] Étendre `NewSession` et l'INSERT de `backend/crates/modules/identity/src/repo/sessions.rs` aux quatre colonnes
- [X] T030 [US1] Étendre `Device` et `session::open` dans `identity/src/service/session.rs` ; **`expiry()` choisit la durée d'après le client** ; la rotation recopie `client_kind`, `device_id`, `device_label`, `device_platform` de la session remplacée
- [X] T031 [P] [US1] Ajouter `AUTH_SESSION_TTL_APP` (défaut `90d`) à `backend/crates/kernel/src/config.rs` et à `.env.example`
- [X] T032 [US1] Accepter l'objet `client` facultatif dans les corps de `login` et `register` — `identity/src/routes/auth.rs` —, le valider (un `kind` inconnu désigne son champ), le passer à `identity/src/service/auth.rs`
- [X] T033 [US1] Faire rendre par `GET /auth/me` le `client_kind`, le `device_label` et l'`issued_at` de la session courante — `identity/src/routes/auth.rs`
- [X] T034 [US1] Retenir le client de la demande avec le jeton (`one_time_tokens.payload`) dans `identity/src/service/registration.rs` et `service/password_reset.rs`
- [X] T035 [US1] Composer le lien d'après ce client dans `identity/src/mail.rs` : variantes `/guide-nego/verification-adresse` et `/guide-nego/nouveau-mot-de-passe`, **non localisées** — Guide Négo est en français quel que soit le téléphone
- [X] T036 [US1] Étendre `identity/tests/cohabitation_sous_prefixe.rs` au préfixe `/v2` pour les liens de courriel et le `Path` du cookie de rafraîchissement
- [X] T037 [US1] `make openapi` et vérifier que les deux routes portent leur nouveau corps

### Client

- [X] T038 [P] [US1] Créer `frontend/app/utils/guide-nego/appareil.ts` : `device_id` engendré une fois et gardé en stockage local (`gn.appareil`), libellé et plateforme composés — **jamais un cookie**
- [X] T039 [P] [US1] Créer `frontend/app/composables/api/guide-nego.ts` et brancher le bloc `negotiation` dans `frontend/app/composables/useApi.ts`
- [X] T040 [P] [US1] Créer `frontend/app/composables/guide-nego/useGnSession.ts` — **à ne pas confondre avec `useGnConnexion` (0a), qui dit l'état du réseau** : enveloppe le store `auth` du site, expose l'état du compte lisible hors connexion, relit au retour au premier plan (`visibilitychange`), **ne nomme aucun cookie**. Session expirée **sans réseau** : garder ce qui a été lu avec son heure, ne réclamer la reconnexion qu'au retour du réseau, **ne jamais vider l'écran** (FR-006 ter)
- [X] T041 [US1] Créer `frontend/app/pages/guide-nego/compte.vue` — étape 1 sur 3, prénom et nom, adresse, pays, mot de passe, l'aide sur le compte ePavillon, la sortie « J'ai déjà un compte », et l'**attente de confirmation** avec « J'ai confirmé mon adresse » et « Renvoyer le courriel »
- [X] T042 [P] [US1] Créer `frontend/app/pages/guide-nego/connexion.vue` et `frontend/app/pages/guide-nego/mot-de-passe-oublie.vue`, avec la feuille basse du mot de passe oublié
- [X] T043 [P] [US1] Créer `frontend/app/pages/guide-nego/verification-adresse.vue` — quatre états du jeton, et sur iPhone « Adresse confirmée — retournez dans Guide Négo »
- [X] T044 [P] [US1] Créer `frontend/app/pages/guide-nego/nouveau-mot-de-passe.vue`, mêmes exigences de mot de passe que le site
- [X] T045 [US1] Ajouter la déconnexion à `frontend/app/pages/guide-nego/ressources/reglages.vue`, avec la mention de ce qu'elle laisse sur le téléphone
- [X] T046 [P] [US1] Poser les fichiers i18n `fr` et `en` : `pages/guide-nego.{compte,connexion,mot-de-passe-oublie,verification-adresse,nouveau-mot-de-passe}.json`
- [X] T047 [P] [US1] Test `node --test` de l'identifiant d'appareil et de la session hors connexion — `frontend/tests/guide-nego/appareil.test.ts`

**Point de contrôle** : § 1 du [quickstart](quickstart.md) déroulé en entier, **T025 et T026 comprises**.

---

## Phase 4 : US2 — Ouvrir les modules réservés avec le code (P1)

**But** : le code reçu sur WhatsApp ouvre l'accès, et chacune de ses neuf issues dit quoi faire ensuite.

**Épreuve indépendante** : avec un code actif et le mode « code », saisir successivement un code juste,
inconnu, révoqué, épuisé, terminé, puis répéter les essais faux jusqu'au blocage.

### Tests

- [X] T048 [P] [US2] Test : les neuf issues de `redeem`, chacune en **200** avec son message — `backend/crates/modules/negotiation/tests/redeem_issues.rs`
- [X] T049 [P] [US2] Test : **cinq échecs, puis un sixième essai avec un `device_id` différent → `throttled`**. C'est la preuve que le compte est par personne — `negotiation/tests/essais_par_personne.rs`
- [X] T050 [P] [US2] Test : **deux entrées simultanées sur le dernier usage d'un code** — une seule passe, l'autre reçoit `exhausted` ; `used_count` n'excède jamais `max_uses` — `negotiation/tests/quota_concurrent.rs`
- [X] T051 [P] [US2] Test : le code du réseau accorde l'appartenance ; un code général ne l'accorde pas ; une personne déjà admise gagne l'appartenance **sans second accès** — `negotiation/tests/reseau.rs`
- [X] T052 [P] [US2] Test : `redeem` écrit l'usage, l'attribution de rôle et l'événement d'outbox **dans une seule transaction** — `negotiation/tests/redeem_transaction.rs`
- [X] T053 [P] [US2] Test : `nego-024`, `NEGO 024` et `Nego024` désignent le même code, et le code engendré fait **huit caractères, tirets compris** — `negotiation/tests/code_normalise.rs`

### Back

- [X] T054 [P] [US2] `negotiation/src/repo/settings.rs` : lire `negotiation.admission_mode` et `negotiation.invitation_attempts` — **relus à chaque tentative, sans cache**
- [X] T055 [P] [US2] `negotiation/src/repo/codes.rs` : retrouver un code par sa forme normalisée, via `v_invitation_codes` pour son état
- [X] T056 [P] [US2] `negotiation/src/repo/attempts.rs` : enregistrer un essai, compter par `invitation_attempts_recent(person, window)`
- [X] T057 [US2] `negotiation/src/service/redeem.rs` : les neuf issues, l'attribution de rôle avec sa portée, l'adhésion `space_members`, l'appartenance au réseau, l'événement — une transaction ouverte par `Db::write(&ctx)`
- [X] T058 [US2] `negotiation/src/routes/acces.rs` : `POST /api/negotiation/invitation-codes/redeem`, union en 200, message français composé par l'API
- [X] T059 [US2] `negotiation/src/repo/access.rs` et `routes/acces.rs` : `GET /api/negotiation/me/access` — état dérivé du RBAC, portée, réseau, demande, mode ; `ETag` et `If-None-Match` — **`?since=` écarté**, voir le journal du 21/09 : l'état vient de quatre tables et de `now()`, aucune colonne ne porte l'instant où il a changé, et un 304 fautif laisserait ouverts les modules d'un accès retiré
- [X] T060 [US2] Traduire les erreurs d'invariant de la base en codes français dans `backend/crates/modules/negotiation/src/service/redeem.rs` : usage en double, quota dépassé, portée invalide, terme hors taxonomie — **aucune vérification préalable en Rust**
- [X] T061 [US2] `make openapi` et `make check-api-contract`

### Client

- [X] T062 [US2] Créer `frontend/app/pages/guide-nego/code.vue` — étape 2 sur 3, la phrase sur le groupe WhatsApp, le champ, les neuf issues affichées **telles que l'API les formule**, les sorties de chaque cas
- [X] T063 [P] [US2] Créer `frontend/app/composables/guide-nego/useGnAcces.ts` : lit `me/access` à travers `useGnLecture` — hors connexion, l'état lu avec son heure
- [X] T064 [US2] Hors connexion, dans `frontend/app/pages/guide-nego/code.vue` (après T062) : dire que la saisie demande le réseau, **n'annoncer aucun accès**, et ne rien mettre en file
- [X] T065 [P] [US2] i18n `fr` et `en` : `pages/guide-nego.code.json` — titres, aides et boutons seulement, **aucun message de refus**
- [X] T066 [P] [US2] Test `node --test` de l'état d'accès hors connexion — `frontend/tests/guide-nego/acces.test.ts`

**Point de contrôle** : § 2 du quickstart déroulé. **Le critère de sortie est à moitié tenu.**

---

## Phase 5 : US3 — L'IFDD tient ses codes (P2)

**But** : créer, lister, révoquer un code ; voir qui est entré avec lequel ; retirer des accès.

**Épreuve indépendante** : créer un code, entrer avec, le voir dans les usages, le révoquer, vérifier
qu'il n'ouvre plus **et que les accès tiennent**, puis retirer un accès.

### Tests

- [ ] T067 [P] [US3] Test : **un administrateur d'ÉVÉNEMENT ne voit rien du back-office de Guide Négo**, sur les DOUZE routes de `contracts/api-admin.md` — sept sur les codes, deux sur le mode d'admission, trois sur les demandes —, URL forgée comprise : le refus est le même que pour un identifiant inexistant (SC-008). La garde est `negotiation.space.manage` **sur la portée globale** (`Requires<SpaceManage>`), **jamais `RequiresAnyScope`** — le rôle `admin` porte cette permission et s'attribue aussi sur un événement, et la route paraîtrait gardée — `negotiation/tests/perimetre_url_forgee.rs`
- [ ] T068 [P] [US3] Test : **révoquer un code ne retire aucun accès déjà accordé** (ADR-006) — `negotiation/tests/revocation_ne_retire_pas.rs`
- [ ] T069 [P] [US3] Test : un accès retiré cesse d'ouvrir dès la lecture suivante de `me/access` — `negotiation/tests/retrait_acces.rs`
- [ ] T070 [P] [US3] Test : **aucune permission sur la portée globale donne un refus explicite**, jamais une liste vide — `negotiation/tests/perimetre_vide_refuse.rs`. *`administered_events()` n'est pas appelée par ce module* : elle ne rend que des portées `event`, quand un code porte `global` ou `negotiation_space`
- [ ] T070 bis [P] [US3] Test : **toute écriture de `negotiation` laisse son auteur** — création, révocation, retrait d'accès et décision retrouvés dans `platform.audit_log` avec leur `actor_id` (FR-046, principe VII) — `negotiation/tests/toute_ecriture_laisse_son_auteur.rs`

### Back

- [ ] T071 [P] [US3] `negotiation/src/repo/uses.rs` : usages d'un code par `v_invitation_code_uses`, avec l'état réel de l'accès
- [ ] T072 [US3] `negotiation/src/service/admin_codes.rs` : engendrer le code (**huit caractères, tirets compris**, exemple `NEGO-024`, alphabet sans `0/O` ni `1/I/L`), créer, révoquer, retirer un accès, retirer tous les accès d'un code — **chaque écriture ouverte par `Db::write(&ctx)`**, jamais une connexion nue
- [ ] T073 [US3] `negotiation/src/routes/admin_codes.rs` : les sept routes sous `/api/admin/negotiation/invitation-codes`, toutes derrière **`Requires<SpaceManage>`** — portée globale, et **ni `RequiresAnyScope` ni `Perimeter`**
- [ ] T074 [US3] `make openapi`

### Client

- [ ] T075 [P] [US3] Créer `frontend/app/pages/admin/negociations/codes/index.vue` — liste, filtres d'URL en français (`etat`, `espace`, `q`), pagination, **quatre états d'écran**, sur le modèle de `admin/incidents/index.vue`
- [ ] T076 [P] [US3] Créer `frontend/app/pages/admin/negociations/codes/nouveau.vue` — libellé, **portée : une COP précise ou Guide Négo en entier**, quota, validité, appartenance au réseau ; le code engendré est montré
- [ ] T077 [US3] Créer `frontend/app/pages/admin/negociations/codes/[id].vue` — détail, usages, révocation et retraits, chacun derrière une confirmation
- [ ] T078 [P] [US3] i18n `fr` et `en` : `pages/admin.negociations.codes.json`
- [ ] T079 [P] [US3] Ajouter l'entrée de navigation du back-office et sa clé dans `_nav.json`

**Point de contrôle** : § 3 du quickstart. **Le critère de sortie est tenu.**

---

## Phase 6 : US4 — Durcir l'admission sans redéployer (P2)

**But** : trois modes, des demandes, une file, deux courriels.

**Épreuve indépendante** : basculer sur « approbation » sans mise en ligne, demander, traiter,
recevoir le courriel ; recommencer en mode « les deux » avec un code juste.

### Tests

- [ ] T080 [P] [US4] Test : **le changement de mode prend effet à la tentative suivante**, sans redémarrage — `negotiation/tests/mode_admission.rs`
- [ ] T081 [P] [US4] Test : une seule demande en attente — deux envois simultanés ne font qu'une ligne, et le conflit sort traduit en français — `negotiation/tests/demande_unique.rs`
- [ ] T082 [P] [US4] Test : une demande tranchée ne se retranche pas ; la transition est refusée **par le trigger** — `negotiation/tests/transition_demande.rs`
- [ ] T083 [P] [US4] Test : admettre écrit l'état, l'accès, l'événement **et met le courriel en file, dans une seule transaction** ; rien ne part si elle échoue — `negotiation/tests/decision_transaction.rs`
- [ ] T084 [P] [US4] Test : en mode `code_and_approval`, un code juste **n'ouvre pas** — il ouvre une demande portant ce code — `negotiation/tests/mode_les_deux.rs`
- [ ] T084 bis [P] [US4] Test : **une demande en attente survit à un changement de mode** et reste traitable (FR-029) — `negotiation/tests/mode_change_demande_survit.rs`

### Back

- [ ] T085 [P] [US4] `negotiation/src/repo/requests.rs` : créer, lire, lister par périmètre, trancher
- [ ] T086 [US4] `negotiation/src/service/requests.rs` : demander, **annuler** ; et `service/admin_requests.rs` : admettre, refuser — accès, réseau, événement et courriel dans **une transaction ouverte par `Db::write(&ctx)`**
- [ ] T087 [P] [US4] `negotiation/src/mail.rs` : les deux modèles — demande admise, demande refusée avec son motif — **dans ce crate**, jamais un appel vers `identity`
- [ ] T088 [P] [US4] `negotiation/src/jobs/emails.rs` : les deux `JobHandler`, montés par `job_handlers()`
- [ ] T089 [US4] `negotiation/src/routes/acces.rs` : `POST` et `DELETE /api/negotiation/access-requests`
- [ ] T090 [US4] `negotiation/src/routes/admin_requests.rs` et `admin_admission.rs` : la file, les deux décisions, `GET` et `PUT` du mode
- [ ] T091 [US4] `negotiation/src/jobs/purge.rs` : purge des essais au-delà de 90 jours, **et brancher sa chaîne récurrente** dans `backend/crates/worker/src/main.rs` (laissée ouverte par T020)
- [ ] T092 [US4] `make openapi`

### Client

- [ ] T093 bis [US4] En mode `approval`, **faire disparaître la saisie de code du parcours** — `frontend/app/pages/guide-nego/code.vue` et l'enchaînement depuis `GnVerrou.vue` proposent la demande à la place (FR-022)
- [ ] T093 [US4] Créer `frontend/app/pages/guide-nego/demande.vue` — nom, pays, **heure d'envoi avec son fuseau**, « En attente », l'annonce du courriel, le rappel de ce qui reste lisible, et les deux sorties
- [ ] T094 [P] [US4] Créer `frontend/app/pages/admin/negociations/demandes/index.vue` — la file, admettre et refuser avec motif
- [ ] T095 [P] [US4] Créer `frontend/app/pages/admin/negociations/admission.vue` — les trois modes, et ce que chacun produit pour la personne qui entre
- [ ] T096 [P] [US4] i18n `fr` et `en` : `pages/guide-nego.demande.json`, `admin.negociations.demandes.json`, `admin.negociations.admission.json`

**Point de contrôle** : § 4 du quickstart.

---

## Phase 7 : US5 — Le verrou et « Mon accès » (P3)

**But** : comprendre ce qu'ouvre le code, et savoir où l'on en est.

**Épreuve indépendante** : sans accès, ouvrir chaque module réservé ; puis lire « Mon accès » dans
chacun des cinq états.

- [ ] T097 [P] [US5] Créer `frontend/app/components/guide-nego/GnVerrou.vue` — titre du module, mention réservée, liste de ce qui s'y trouve, rappel de ce qui reste ouvert, deux sorties. **Le seul composant nouveau de l'étape**
- [ ] T098 [US5] Poser le verrou dans `frontend/app/pages/guide-nego/echanges.vue`, d'après la **permission effective** lue de `me/access` par `useGnAcces`, jamais d'un état retenu côté client
- [ ] T099 [US5] Sans compte, faire mener le verrou d'abord à `compte.vue` ou `connexion.vue`, **puis** à `code.vue` — enchaînement dans `frontend/app/components/guide-nego/GnVerrou.vue` (après T097)
- [ ] T100 [P] [US5] Créer `frontend/app/pages/guide-nego/ressources/acces.vue` — les cinq états, la date, **ce que l'accès ouvre** (une COP nommée ou tout Guide Négo), l'appartenance au réseau ; hors connexion, l'état lu avec son heure
- [ ] T101 [US5] Ajouter la ligne « Mon accès » à `frontend/app/pages/guide-nego/ressources/reglages.vue`
- [ ] T102 [P] [US5] i18n `fr` et `en` : `pages/guide-nego.acces.json`, `components/gn-verrou.json`
- [ ] T103 [P] [US5] Ajouter `GnVerrou` à `frontend/app/components/guide-nego/planche/PlancheComposantsSurfaces.vue` et **le retirer des absents** de `frontend/i18n/locales/{fr,en}/components/gn-planche-composants.json`
- [ ] T103 bis [P] [US5] **Prouver FR-008 et SC-006** : aucun champ, libellé ni réponse d'API de ce périmètre ne porte le genre — balayage de `frontend/app/pages/guide-nego/`, `frontend/i18n/locales/*/pages/guide-nego.*.json` et des réponses de `contracts/` ; le résultat s'écrit dans le journal de `docs/AppNego/progress.md`
- [ ] T103 ter [P] [US5] Test : un client qui **prétend** avoir l'accès ne l'obtient pas — le verrou suit `me/access`, jamais un état retenu côté client (FR-032) — `frontend/tests/guide-nego/verrou.test.ts`

**Point de contrôle** : § 5 du quickstart. **Vérifier qu'aucun écran de ce parcours ne demande ni n'affiche un genre.**

---

## Phase 8 : Recette

- [ ] T104 [P] Vérifier que les fichiers de `frontend/i18n/locales/en/` portent **exactement** les clés de leurs jumeaux `fr/`, aucune manquante ni en trop ; toute chaîne restée en dur dans `app/pages/guide-nego/` est assumée et justifiée une par une
- [ ] T105 **Répéter `specs/009-guide-nego-compte-admission/migration.sql` sur une copie de la base**, données comprises, puis **recomparer les schémas** (T012) : c'est ce contrôle qui a rattrapé le seul oubli du 16/09
- [ ] T106 [P] Mesurer au navigateur à 320, 360 et 390 px, en clair et en sombre, les huit écrans de `frontend/app/pages/guide-nego/` contre `docs/AppNego/design/ecrans/02-socle.html` : aucun débordement horizontal, cibles de 48 px, anneau de focus visible
- [ ] T107 [P] Vérifier les mots de `design/lexique.md` : « Code d'invitation » — jamais « clé », « jeton » ni « mot de passe » —, « Demande en attente », « Réservé aux négociatrices et négociateurs »
- [ ] T108 **Non-régression du site, sur la version construite** : accueil, back-office et `/negociations` inchangés ; `frontend/app/pages/auth/*.vue` fonctionnent **sans** objet `client` ; aucun service worker ni manifeste après une visite du site seul
- [ ] T109 `npm run check:guide-nego`, `test:guide-nego`, `typecheck`, `cargo test -p negotiation`, `make check-api-contract`, `npm run verifier-garde:guide-nego`
- [ ] T110 `make check-safe` au vert — **API arrêtée** : les tests d'`identity` se sont montrés sensibles à une activité concurrente sur la base le 21/09
- [ ] T111 Dérouler le [quickstart](quickstart.md) au navigateur, § 1 à § 8, **chronomètre en main sur le parcours compte → code → entrée** : le relevé va au journal de `docs/AppNego/progress.md` (SC-001)
- [ ] T111 bis Vérifier que le back-office restitue **le compte des appartenances au réseau**, et pas seulement les usages par code (SC-007) — `frontend/app/pages/admin/negociations/codes/index.vue`
- [ ] T111 ter [P] Test : **un envoi de courriel qui échoue n'annule pas la décision** — la demande reste admise, le travail se rejoue (SC-011) — `negotiation/tests/courriel_echec_decision_tient.rs`
- [ ] T112 Dérouler § 9 **sur un Android réel puis un iPhone** : l'installation, le retour du courriel sur les deux systèmes, `client_kind` après une nuit, et § 2 en 3G lente
- [ ] T113 Mettre à jour `docs/AppNego/progress.md` — la ligne d'état 0b, le journal du jour, et un ADR si quelque chose s'est tranché en chemin

---

## Dépendances et ordre

### Entre phases

- **Phase 1 (modèle et migration)** : bloque tout. SQLx ne compile pas sans elle
- **Phase 2 (le crate)** : dépend de la phase 1, bloque tous les récits
- **Phase 3 (US1)** : ne touche que `identity` et le client — **peut avancer en parallèle des phases 4 et 5**
- **Phase 4 (US2)** : dépend de la phase 2
- **Phase 5 (US3)** : dépend de la phase 2 ; s'éprouve mieux après la phase 4, qui lui donne des usages à montrer
- **Phase 6 (US4)** : se pose sur ce que 4 et 5 ont bâti
- **Phase 7 (US5)** : dépend de `me/access` (T059)
- **Phase 8** : après tout le reste

### À l'intérieur d'un récit

Tests → dépôt → service → routes → contrat engendré → écrans. Le SQL n'est jamais après le code.

### Ce qui peut avancer ensemble

- T001, T002 et T009 — trois fichiers SQL différents. **T003 à T008 écrivent tous `100_negotiations.sql` : elles se suivent**
- T016, T017, T021 — pièces indépendantes du crate. **T019 attend T018** (dépendance de crate), **T020 attend T015**
- Toutes les tâches de test d'un même récit
- Les phases 3, 4 et 5 par trois personnes, une fois la phase 2 finie
- Les écrans de back-office (T075, T076, T078) entre eux

---

## Stratégie de livraison

**Le plus petit livrable utile** : phases 1, 2 et 3. Une personne entre dans l'application avec son
compte, y reste une COP entière, et les courriels la ramènent là où elle était. Rien n'est réservé
encore, mais le compte tient.

**Le critère de sortie de l'étape** demande les phases 4 et 5 : la négociatrice entre avec son code,
et l'administrateur le révoque. La phase 6 ajoute la bascule en approbation, qui en est la seconde
moitié.

**Un commit par phase**, comme en 0a, avec `npm run check:guide-nego` et `cargo test -p negotiation`
entre-temps — `make check-safe` en fin de cycle seulement, la porte complète coûtant plusieurs minutes.

---

## Notes

- `[P]` = fichiers différents, aucune dépendance en attente
- Aucune tâche ne redessine un composant livré en 0a : le seul nouveau est `GnVerrou`
- Aucun fichier de `backend/` ni de `frontend/` ne dépasse 1000 lignes
- Les tests ne sont pas optionnels : le principe X les exige sur base réelle
- S'arrêter à chaque point de contrôle et éprouver le récit seul

---

description: "Task list — Socle technique et Identité (B1)"
---

# Tasks: Socle technique et Identité (B1)

**Input**: Design documents from `/specs/001-socle-identite/`

**Prerequisites**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: **OBLIGATOIRES.** Le principe X de la constitution impose des tests d'intégration sur base réelle et jetable, sans aucun mock de base — « la moitié des invariants de ce projet vit dans la base ». Les tâches de test ne sont donc pas optionnelles ici.

**Organization**: par histoire utilisateur, pour que chaque tranche soit implémentable, éprouvable et livrable seule.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichier différent, aucune dépendance en cours
- **[Story]** : `US1` … `US8`, correspondant aux histoires de `spec.md`

## Conventions de chemin

Tout vit dans `backend/`, workspace Cargo symétrique de `frontend/` — emplacements imposés par le principe II. **Un seul fichier de ce jalon vit hors de `backend/`** : la route privée d'envoi de courriel, que la contrainte d'hébergement du 20/08 place côté site.

---

## ⚠️ Une décision d'architecture est apparue en découpant : lire avant T011

**Le garde d'autorisation vit dans le NOYAU, pas dans le module `identity`.**

L'autorisation se teste par `identity.has_permission()` et `identity.administered_events()` — deux fonctions **SQL**. Tous les modules à venir (B2 à B6) doivent les appeler, et **aucun n'a le droit de dépendre du crate `identity`**. Placer le garde dans `identity` créerait donc, dès B2, exactement l'arête que le principe II interdit.

Le garde va donc dans `kernel`, qui appelle les deux fonctions en SQL. Ce n'est pas une dépendance de crate : le noyau connaît le schéma `identity` comme il connaît `platform` — l'autorisation est une préoccupation de plateforme dans ce modèle, et le SQL l'a déjà tranché en la posant comme fonction plutôt que comme service.

**Conséquence pour les tâches** : T024 à T026 créent le garde dans `kernel`, et le module `identity` s'en sert comme n'importe quel autre module le fera. C'est ce qui permet à B2 de ne rien réinventer.

---

## Phase 1 — Mise en place

**Objectif** : l'arborescence existe, elle compile, et `make check-back` cesse d'être inerte.

- [X] T001 Créer le workspace Cargo et ses membres dans `backend/Cargo.toml` (dépendances communes en `[workspace.dependencies]` : actix-web, sqlx, serde, uuid, time, tracing, thiserror)
- [X] T002 [P] Épingler la chaîne d'outils dans `backend/rust-toolchain.toml` et le profil de compilation dans `backend/.cargo/config.toml`
- [X] T003 [P] Créer `backend/.gitignore` (cible de compilation, `.env` local) et vérifier que `.gitignore` racine n'exclut pas `backend/.sqlx/`
- [X] T004 [P] Créer le squelette du crate noyau : `backend/crates/kernel/Cargo.toml` et `backend/crates/kernel/src/lib.rs`
- [X] T005 [P] Créer le squelette du crate de contrats : `backend/crates/contracts/Cargo.toml` et `backend/crates/contracts/src/lib.rs`
- [X] T006 [P] Créer le squelette du crate de module : `backend/crates/modules/identity/Cargo.toml` et `backend/crates/modules/identity/src/lib.rs`
- [X] T007 [P] Créer le squelette du binaire HTTP : `backend/crates/api/Cargo.toml` et `backend/crates/api/src/main.rs`
- [X] T008 [P] Créer le squelette du binaire différé : `backend/crates/worker/Cargo.toml` et `backend/crates/worker/src/main.rs`
- [X] T009 Ajouter à `.env.example` les clés de la section « Authentification » et « Relais de courriel » listées dans research.md § R11 et § R13, **et corriger le commentaire des trois clés SMTP** : elles sont lues par le site, plus par l'API
- [X] T010 Vérifier que `cargo fmt --check`, `cargo clippy -- -D warnings` et `cargo build` passent sur l'arborescence vide (base démarrée), et que `make check-back` cesse d'afficher « backend/ absent » — cible `check-back` du `Makefile`

---

## Phase 2 — Fondations (bloquantes)

**Objectif** : le noyau, les contrats, les deux binaires et le harnais de test. **Aucune histoire ne peut commencer avant.**

### Le noyau

- [X] T011 [P] Implémenter la configuration typée et sa validation au démarrage dans `backend/crates/kernel/src/config.rs` — seuil et durée de verrouillage, durées de jeton **par finalité**, durées de session, clé de signature, relais de courriel (research.md § R11)
- [X] T012 [P] Implémenter le contexte de requête (identifiant de requête, acteur, locale) dans `backend/crates/kernel/src/context.rs`
- [X] T013 [P] Implémenter le type d'erreur unique — code stable, message français, champ fautif — dans `backend/crates/kernel/src/error.rs`, avec le catalogue de `contracts/errors.md`
- [X] T014 Implémenter la traduction `(SQLSTATE, nom de contrainte) → erreur d'API` dans `backend/crates/kernel/src/pg_error.rs`, **y compris le passage tel quel du message français levé par un trigger du modèle** (`restrict_violation`)
- [X] T015 Implémenter le pool et **l'unique façon d'ouvrir une transaction en écriture** dans `backend/crates/kernel/src/db.rs` : elle pose `SET LOCAL app.actor_id` et `app.request_id` avant de rendre la main (principe VII, research.md § R14)
- [X] T016 [P] Implémenter la négociation de locale contre `reference.locales`, repli sur le français, dans `backend/crates/kernel/src/i18n.rs`
- [X] T017 [P] Implémenter Argon2id (19 MiB, 2 itérations), l'**empreinte factice calculée au démarrage**, la génération de jetons aléatoires et leur empreinte SHA-256 dans `backend/crates/kernel/src/crypto.rs`
- [X] T018 [P] Implémenter les traces, la journalisation et la propagation de `X-Request-Id` dans `backend/crates/kernel/src/telemetry.rs`
- [X] T019 Implémenter l'émission d'événement par `platform.emit_event()` dans `backend/crates/kernel/src/events.rs` — **jamais d'INSERT direct** — plus le registre de consommateurs et la garde `platform.inbox_events`
- [X] T020 Implémenter la mise en file, la réservation par `platform.claim_jobs()` et l'échec par `platform.fail_job()` dans `backend/crates/kernel/src/jobs.rs` — **la reprise d'essai n'est pas réécrite**, la base la porte
- [X] T021 Définir le **contrat d'envoi de courriel** (destinataire, sujet, corps déjà composés) dans `backend/crates/kernel/src/mail.rs`, avec deux implémentations sélectionnées par configuration : remise HTTP au serveur du site, et SMTP direct **laissé non branché** pour le jour de l'autorisation (research.md § R13)
- [X] T022 [P] Implémenter le harnais de base jetable — base modèle chargée une fois depuis `docs/database/`, recopie par test — dans `backend/crates/kernel/src/testing.rs`, derrière une caractéristique de compilation
- [X] T023 [P] Déclarer les charges utiles des événements `identity.*` dans `backend/crates/contracts/src/identity.rs`, d'après `contracts/events.md`

### Le garde d'autorisation — dans le noyau (voir l'avertissement plus haut)

- [X] T024 Implémenter l'appel de `identity.has_permission(personne, permission, type_de_portée, portée)` dans `backend/crates/kernel/src/auth.rs`
- [X] T025 Implémenter l'appel de `identity.administered_events(personne)` dans le même fichier, en **gardant les trois cas distincts** : global, éditions listées, **aucun droit → refus explicite**
- [X] T026 Implémenter l'extracteur de route qui exige une permission **de portée globale** (`Requires<P>`), et celui qui exige un périmètre (`Perimeter`), dans `backend/crates/kernel/src/auth.rs`. Une portée **ciblée** dépend du chemin et se vérifie dans le gestionnaire, par `require_permission(…, Scope::Event(id))` — arbitrage, non contrainte d'Actix

### Les deux binaires

- [X] T027 Implémenter l'état partagé et le démarrage de l'API dans `backend/crates/api/src/main.rs` et `backend/crates/api/src/state.rs`
- [X] T028 [P] Implémenter l'intergiciel de contexte de requête dans `backend/crates/api/src/middleware/request_context.rs`
- [X] T029 [P] Implémenter l'intergiciel de vérification d'origine sur toute écriture dans `backend/crates/api/src/middleware/origin.rs`
- [X] T030 Implémenter la lecture de `platform.modules` au démarrage et le montage conditionnel des routes dans `backend/crates/api/src/modules.rs` — un module désactivé répond **404, pas 403**
- [X] T031 Implémenter le relais d'outbox — écoute de `platform_outbox`, balayage de secours toutes les 10 s, réservation `FOR UPDATE SKIP LOCKED` — dans `backend/crates/worker/src/outbox.rs`
- [X] T032 Implémenter la boucle de travaux différés et le registre de tâches dans `backend/crates/worker/src/jobs.rs` et `backend/crates/worker/src/registry.rs`
- [X] T033 [P] Implémenter le consommateur de télémétrie — **une trace par événement** ; le compteur attend l'ouverture d'un pipeline de métriques, qu'aucune tâche ne porte — dans `backend/crates/worker/src/consumers/telemetry.rs` (voir `contracts/events.md` : c'est le seul consommateur de ce jalon, et il **exerce la garde d'idempotence** en attendant B2)

### Vérification des fondations

- [X] T034 Écrire le test du harnais lui-même dans `backend/crates/kernel/tests/harness.rs` : une base est créée, porte les seize schémas, et disparaît en sortant
- [X] T035 Écrire `backend/crates/kernel/tests/contexte_ecriture.rs` : une transaction ouverte par le noyau pose bien l'acteur et l'identifiant de requête, et `platform.audit_log` les retrouve

**✅ Point de contrôle** : `cargo build`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` et les deux tests passent. Aucune arête vers un autre module :

```bash
cargo tree -p identity | tail -n +2 | grep -c "crates/modules/"   # doit valoir 0
```

`tail -n +2` écarte la ligne racine, qui porte elle-même `crates/modules/identity` : sans elle, le compte vaut 1 et jamais 0.

---

## Phase 3 — US1 : Se connecter sans rien apprendre à un inconnu (P1) 🎯 MVP

**Objectif** : les six issues de connexion, dans l'ordre imposé, avec la discrétion tenue **en message et en temps**.

**Éprouvable seule** : exercer les six issues sur un jeu de comptes couvrant chaque cas, et mesurer l'écart de temps entre une adresse inconnue et un mot de passe faux.

- [X] T036 [P] [US1] Déclarer les identifiants typés par agrégat (`PersonId`, `AccountId`, `SessionId`…) dans `backend/crates/modules/identity/src/domain/ids.rs`
- [X] T037 [P] [US1] Déclarer les six issues de connexion et les énumérations du modèle (`person_status`, `auth_provider`) dans `backend/crates/modules/identity/src/domain/login.rs`
- [X] T038 [P] [US1] Implémenter les règles de robustesse du mot de passe — 8 signes, une majuscule, une minuscule, **les mêmes que le front applique déjà** — dans `backend/crates/modules/identity/src/domain/password.rs`
- [X] T039 [US1] Implémenter la lecture d'une personne par adresse — **sans mise en minuscules côté service**, `platform.email` étant un `citext` — dans `backend/crates/modules/identity/src/repo/people.rs`
- [X] T040 [US1] Implémenter la lecture du compte mot de passe, l'incrément du compteur d'échecs, la pose et la remise à zéro du verrou dans `backend/crates/modules/identity/src/repo/accounts.rs`
- [X] T041 [US1] Implémenter l'ordre des contrôles dans `backend/crates/modules/identity/src/service/auth.rs` : **mot de passe d'abord**, puis verrou, suspension, adresse non vérifiée, second facteur (FR-019 à FR-021, FR-024)
- [X] T042 [US1] Implémenter dans le même fichier la vérification contre l'**empreinte factice** quand l'adresse est inconnue ou sans compte mot de passe (FR-020, research.md § R5)
- [X] T043 [US1] Implémenter l'ouverture de session à la connexion réussie — création de la ligne, pose des deux cookies — dans `backend/crates/modules/identity/src/repo/sessions.rs` et `service/session.rs`
- [X] T044 [US1] Implémenter `POST /auth/login` dans `backend/crates/modules/identity/src/routes/auth.rs`, **rendant 200 pour les six issues** (`contracts/routes.md`)
- [X] T045 [US1] Émettre `identity.account.locked` quand le seuil est atteint, depuis `backend/crates/modules/identity/src/service/auth.rs`
- [X] T046 [US1] Écrire `backend/crates/modules/identity/tests/connexion_issues.rs` : les six issues, une par une
- [X] T047 [US1] Écrire `backend/crates/modules/identity/tests/discretion_temps_de_reponse.rs` : 100 tentatives de chaque sorte, **écart de temps sous 10 %** (SC-001)

**✅ Point de contrôle** : quelqu'un peut se connecter, et le formulaire ne révèle plus quels comptes existent.

---

## Phase 4 — US2 : Rester connecté, et pouvoir couper une session (P1)

**Objectif** : renouvellement avec rotation, déconnexion, détection de rejeu, coupure immédiate.

**Éprouvable seule** : se connecter, laisser le jeton d'accès expirer, constater le renouvellement ; se déconnecter, constater que le jeton ne vaut plus rien.

- [X] T048 [P] [US2] Implémenter la fabrication et la vérification du jeton d'accès signé — **aucune permission dedans** — dans `backend/crates/modules/identity/src/domain/access_token.rs` (research.md § R1)
- [X] T049 [US2] Implémenter la rotation — révocation de la session courante, ouverture d'une nouvelle — dans `backend/crates/modules/identity/src/service/session.rs` (research.md § R3)
- [X] T050 [US2] Implémenter la **détection de rejeu** dans le même fichier : un jeton dont la session est révoquée pour rotation fait révoquer **toutes** les sessions de la personne (FR-031)
- [X] T051 [US2] Implémenter la révocation individuelle et en masse, avec leur motif, dans `backend/crates/modules/identity/src/repo/sessions.rs`
- [X] T052 [US2] Implémenter la coupure des sessions sur suspension, blocage et changement de mot de passe, **dans la même transaction que le changement** (FR-033), dans `service/session.rs`
- [X] T053 [US2] Implémenter `GET /auth/me` dans `backend/crates/modules/identity/src/routes/auth.rs` — **200 avec un corps nul** quand il n'y a pas de session, jamais 401
- [X] T054 [US2] Implémenter `POST /auth/refresh` et `POST /auth/logout` dans le même fichier ; la déconnexion **réussit même sans session**
- [X] T055 [US2] Implémenter l'intergiciel qui résout la session et remplit l'acteur du contexte dans `backend/crates/api/src/middleware/auth.rs`
- [X] T056 [US2] Écrire `backend/crates/modules/identity/tests/session_rotation.rs` : renouvellement, rotation, déconnexion
- [X] T057 [P] [US2] Écrire `backend/crates/modules/identity/tests/rejeu_du_jeton.rs` : un jeton rejoué révoque toutes les sessions
- [X] T058 [P] [US2] Écrire `backend/crates/modules/identity/tests/suspension_coupe_les_sessions.rs`

**✅ Point de contrôle** : une session vit, se renouvelle, se coupe — et un jeton volé ne survit pas à son second usage.

---

## Phase 5 — US3 : Ne voir que le périmètre qu'on m'a confié (P1)

**Objectif** : la règle métier n° 8, tenue par l'API et non plus seulement par l'écran.

**Éprouvable seule** : avec un compte détaché sur une édition, appeler chaque lecture, puis forger l'identifiant d'une autre édition.

- [X] T059 [P] [US3] Implémenter les lectures RBAC — attributions en cours, permissions effectives — dans `backend/crates/modules/identity/src/repo/rbac.rs`
- [X] T060 [US3] Implémenter le service de lecture des droits dans `backend/crates/modules/identity/src/service/rbac.rs`
- [X] T061 [US3] Implémenter `GET /people`, `/people/{id}` dans `backend/crates/modules/identity/src/routes/people.rs`
- [X] T062 [US3] Implémenter `GET /people/{id}/roles`, `/permissions`, `/administered-events` dans le même fichier — « soi-même » est décidé **par la session**, jamais par un paramètre
- [X] T063 [US3] Implémenter la composition de la liste des utilisateurs, **bornée par le périmètre**, dans `backend/crates/modules/identity/src/repo/admin_users.rs`
- [X] T064 [US3] Implémenter `GET /admin/users` et `GET /admin/users/{id}` dans `backend/crates/modules/identity/src/routes/admin_users.rs` — hors périmètre → **200 avec `in_scope: false`**, en lecture seule
- [X] T065 [US3] Implémenter `GET /admin/users/{id}/effective-permissions` — permissions **enrichies de leur origine**, quel rôle apporte quoi — dans le même fichier
- [X] T066 [US3] Écrire `backend/crates/modules/identity/tests/perimetre_url_forgee.rs` : sur **chaque** route paramétrée, un compte détaché ne peut atteindre une autre édition *(obligation n° 2 du principe X)*
- [X] T067 [P] [US3] Écrire `backend/crates/modules/identity/tests/perimetre_vide_refuse.rs` : aucun droit → **refus**, jamais une liste vide (FR-048)

**✅ Point de contrôle** : le back-office se partage sans se dupliquer, et une URL forgée ne mène nulle part.

---

## Phase 6 — US4 : Créer un compte et prouver son adresse (P2)

**Objectif** : l'inscription, le courriel, la vérification — et la réponse invariable.

**Éprouvable seule** : s'inscrire avec une adresse libre puis avec une adresse prise, comparer les deux réponses, suivre le lien reçu.

- [X] T068 [P] [US4] Déclarer les trois refus de jeton et leur **ordre** — « déjà utilisé » avant « périmé » — dans `backend/crates/modules/identity/src/domain/token.rs`
- [X] T069 [US4] Implémenter la création d'un jeton dont **l'expiration se dérive de la finalité** (FR-017, FR-018) et sa consommation **atomique** (`WHERE consumed_at IS NULL`, FR-041) dans `backend/crates/modules/identity/src/repo/tokens.rs`
- [X] T070 [US4] Implémenter l'invalidation des jetons non consommés de la même finalité pour la même personne (FR-040) dans le même fichier
- [X] T071 [US4] Implémenter la composition des messages — sujet et corps, en français et en anglais selon `people.preferred_locale` — dans `backend/crates/modules/identity/src/mail.rs`
- [X] T072 [US4] Implémenter l'inscription dans `backend/crates/modules/identity/src/service/registration.rs` : création de la personne et du compte, jeton, **mise en file du courriel dans la même transaction**, et émission de `identity.person.registered`
- [X] T073 [US4] Implémenter dans le même fichier la **réponse invariable** sur adresse déjà connue, avec mise en file du rappel de compte existant (FR-035)
- [X] T074 [US4] Implémenter la vérification d'adresse et le renvoi de lien dans le même fichier ; émettre `identity.person.email_verified`
- [X] T075 [US4] Implémenter `POST /auth/register`, `/auth/verify-email`, `/auth/verify-email/resend` dans `backend/crates/modules/identity/src/routes/auth.rs`
- [X] T076 [US4] Implémenter les tâches d'envoi dans `backend/crates/modules/identity/src/jobs/emails.rs`, **vidant la charge utile dès l'envoi réussi** (research.md § R8)
- [X] T077 [US4] Implémenter la remise HTTP au serveur du site dans `backend/crates/kernel/src/mail.rs` — secret partagé en en-tête, identifiant de message pour le dédoublonnage
- [X] T078 [US4] Implémenter la route privée d'envoi dans `frontend/server/api/internal/mail.post.ts` et `frontend/server/utils/mailer.ts` : secret comparé à temps constant, **404 si absent ou faux**, mémoire courte des identifiants déjà envoyés, envoi SMTP. **Générique — elle sert tous les courriels de la plateforme**
- [X] T079 [US4] Écrire `backend/crates/modules/identity/tests/inscription_reponse_invariable.rs` : adresse libre et adresse prise rendent la même réponse
- [X] T080 [P] [US4] Écrire `backend/crates/modules/identity/tests/jeton_verification.rs` : les trois refus, leur ordre, et la consommation concurrente

**✅ Point de contrôle** : quelqu'un s'inscrit, reçoit son courriel dans Mailpit, vérifie son adresse et se connecte.

---

## Phase 7 — US5 : Retrouver l'accès à un compte (P2)

**Objectif** : la réinitialisation complète, jeton revérifié à l'envoi.

**Éprouvable seule** : demander une réinitialisation pour une adresse inconnue puis connue, mener un cycle complet, vérifier que l'ancien mot de passe ne vaut plus rien.

- [X] T081 [US5] Implémenter la demande de réinitialisation — **réponse invariable** (FR-036) — dans `backend/crates/modules/identity/src/service/password_reset.rs`
- [X] T082 [US5] Implémenter le contrôle préalable du jeton dans le même fichier
- [X] T083 [US5] Implémenter l'enregistrement du nouveau mot de passe : **jeton revérifié** (FR-042), compteur d'échecs et verrou remis à zéro, **toutes les sessions révoquées** (FR-043)
- [X] T084 [US5] Émettre `identity.account.password_changed` depuis `backend/crates/modules/identity/src/service/password_reset.rs`
- [X] T085 [US5] Implémenter `POST /auth/password-reset`, `GET /auth/password-reset/check`, `POST /auth/password-reset/confirm` dans `backend/crates/modules/identity/src/routes/auth.rs`
- [X] T086 [US5] Implémenter la tâche d'envoi du lien de réinitialisation dans `backend/crates/modules/identity/src/jobs/emails.rs`
- [X] T087 [US5] Écrire `backend/crates/modules/identity/tests/reinitialisation.rs` : cycle complet, jeton périmé entre l'affichage et l'envoi, sessions coupées
- [X] T088 [P] [US5] Écrire `backend/crates/modules/identity/tests/mot_de_passe_refuse.rs` : l'erreur **désigne le champ fautif** et porte un message français

**✅ Point de contrôle** : un compte perdu se récupère sans passer par l'assistance.

---

## Phase 8 — US6 : Confier un rôle, le retirer, sur la bonne portée (P2)

**Objectif** : l'attribution scopée, ses cinq issues, et la symétrie attribuer/retirer.

**Éprouvable seule** : avec un compte détaché sur une édition, attribuer sur cette édition (accepté), sur une autre et globalement (refusés), retirer un rôle global (refusé).

- [X] T089 [P] [US6] Déclarer les portées et les issues d'écriture de rôle dans `backend/crates/modules/identity/src/domain/scope.rs`
- [X] T090 [US6] Implémenter l'attribution et le retrait dans `backend/crates/modules/identity/src/repo/rbac.rs` — le retrait **pose** `revoked_at`, `revoked_by`, `revoked_reason`, il ne supprime jamais
- [X] T091 [US6] Implémenter la vérification de `identity.role.assign` **sur la portée visée** dans `backend/crates/modules/identity/src/service/admin_users.rs`
- [X] T092 [US6] Implémenter dans le même fichier la symétrie du retrait : **même permission, sur la portée de l'attribution visée**
- [X] T093 [US6] Implémenter la traduction des refus de la base en issues : `duplicate` (unicité), `scope_not_allowed` (**message du trigger repris tel quel**), et les erreurs de fenêtre et de portée (`contracts/errors.md`)
- [X] T094 [US6] Implémenter la composition des options d'attribution — **restreintes à ce que l'appelant peut réellement accorder** — dans le même fichier
- [X] T095 [US6] Implémenter le changement de statut d'une personne dans le même fichier : motif, auteur, coupure des sessions, refus de `anonymized`
- [X] T096 [US6] Implémenter `POST /admin/users/{id}/roles`, `DELETE /admin/users/roles/{id}`, `PUT /admin/users/{id}/status`, `GET /admin/users/role-options` dans `backend/crates/modules/identity/src/routes/admin_users.rs` — **en ignorant les paramètres de droits envoyés par le client** (FR-055)
- [X] T097 [US6] Émettre `identity.role.granted`, `identity.role.revoked` et `identity.person.status_changed` depuis `backend/crates/modules/identity/src/service/admin_users.rs`
- [X] T098 [US6] Écrire `backend/crates/modules/identity/tests/role_portee.rs` : attribution acceptée sur sa portée, refusée ailleurs ; **le refus du trigger ressort avec son message français** *(obligation n° 3 du principe X)*
- [X] T099 [P] [US6] Écrire `backend/crates/modules/identity/tests/droits_declares_ignores.rs` : les paramètres de droits envoyés par le client n'ont aucun effet

**✅ Point de contrôle** : un webinaire se confie à son responsable sans lui ouvrir le reste de la plateforme.

---

## Phase 9 — US7 : Les effets différés partent une fois, et une seule (P2)

**Objectif** : les garanties de la chaîne différée, éprouvées plutôt que supposées.

**Éprouvable seule** : provoquer une inscription, arrêter et relancer le worker, vérifier qu'un seul courriel est parti.

- [X] T100 [US7] Implémenter la tâche récurrente de purge des jetons périmés et consommés dans `backend/crates/modules/identity/src/jobs/purge.rs` (FR-044)
- [X] T101 [US7] Implémenter la planification de la tâche récurrente au démarrage du worker dans `backend/crates/worker/src/main.rs`
- [X] T102 [US7] Implémenter la remontée des travaux en échec et de l'outbox en retard via `analytics.v_operational_health` dans `backend/crates/api/src/routes/health.rs`
- [X] T103 [US7] Écrire `backend/crates/modules/identity/tests/outbox_transactionnel.rs` : un événement par changement d'état, **et zéro si la transaction est annulée** *(obligation n° 4 du principe X)*
- [X] T104 [P] [US7] Écrire `backend/crates/kernel/tests/idempotence_consommateur.rs` : un relais relancé sur des événements déjà traités n'en rejoue **aucun**
- [X] T105 [P] [US7] Écrire `backend/crates/kernel/tests/unicite_des_travaux.rs` : deux demandes du même travail avec la même clé n'en exécutent qu'une
- [X] T106 [P] [US7] Écrire `backend/crates/kernel/tests/reprise_dessai.rs` : un site injoignable fait échouer le travail, le replanifie, puis le met en file morte
- [X] T107 [US7] Écrire `backend/crates/modules/identity/tests/aucun_secret_en_base.rs` : après un cycle complet, ni mot de passe, ni jeton de session, ni jeton de lien en clair — **et la charge utile du travail est vidée** (SC-009)

**✅ Point de contrôle** : la chaîne différée tient debout, et une panne du site se voit sur la route de santé.

---

## Phase 10 — US8 : Honorer une demande RGPD (P3)

**Objectif** : la file des demandes, son échéance, et l'effacement qui ne détruit que ce qu'on lui demande.

**Éprouvable seule** : déposer une demande de chaque type, exécuter un effacement, vérifier que les compteurs de participation ne bougent pas.

- [X] T108 [P] [US8] Implémenter les lectures de consentements et de demandes dans `backend/crates/modules/identity/src/repo/privacy.rs`
- [X] T109 [US8] Implémenter le service de traitement dans `backend/crates/modules/identity/src/service/privacy.rs`, exigeant la **portée globale** (FR-059)
- [X] T110 [US8] Implémenter l'appel de `identity.anonymize_person()` dans le même fichier, **réservé à une demande d'effacement** (FR-060)
- [X] T111 [US8] **Ne pas émettre `identity.person.anonymized`** : la fonction de la base l'émet déjà. Ajouter le commentaire qui explique pourquoi, dans `service/privacy.rs`
- [X] T112 [US8] Implémenter `GET /admin/privacy-requests` et `PUT /admin/privacy-requests/{id}` dans `backend/crates/modules/identity/src/routes/admin_privacy.rs`
- [X] T113 [US8] Émettre `identity.privacy_request.received` au dépôt d'une demande, depuis `backend/crates/modules/identity/src/service/privacy.rs`
- [X] T114 [US8] Écrire `backend/crates/modules/identity/tests/rgpd_portee_globale.rs` : un administrateur d'édition reçoit **403**, jamais une file filtrée
- [X] T115 [P] [US8] Écrire `backend/crates/modules/identity/tests/effacement.rs` : refusé sur une demande d'export, accepté sur un effacement, **un seul événement émis**, compteurs de participation inchangés

**✅ Point de contrôle** : une demande d'effacement s'honore sans détruire les statistiques d'une COP passée.

---

## Phase 11 — Finition et points transverses

- [X] T116 Annoter chaque route pour l'OpenAPI **généré** dans `backend/crates/modules/identity/src/routes/openapi.rs`, en y déclarant **chaque code d'erreur stable** (FR-063)
- [X] T117 Monter la documentation générée et la route de vivacité dans `backend/crates/api/src/openapi.rs` et `backend/crates/api/src/routes/health.rs`
- [X] T118 [P] Écrire `backend/crates/modules/identity/tests/toute_ecriture_laisse_son_auteur.rs` : après un cycle complet, **aucune ligne d'audit sans acteur** — c'est le seul défaut du module qu'aucun mécanisme ne rattrape
- [X] T119 [P] Générer et versionner les requêtes préparées : `cargo sqlx prepare --workspace` → `backend/.sqlx/`
- [X] T120 Vérifier qu'aucun fichier de `backend/` ne dépasse **1000 lignes** (`find backend -name '*.rs' | xargs wc -l | sort -rn | head`) ; découper ce qui s'en approche
- [X] T121 Vérifier que `cargo tree -p identity` ne montre **aucune arête vers `backend/crates/modules/`**
- [X] T122 Passer `make check` en entier depuis la racine du dépôt — schéma rechargé de zéro, rapport de frontières vide, site qui compile, analyse statique sans un seul avertissement
- [X] T123 Éprouver à la main les parcours de `specs/001-socle-identite/quickstart.md` : les six issues, le cycle d'inscription dans Mailpit, le verrouillage, le périmètre, la chaîne différée
- [X] T124 [P] Mettre à jour `docs/ENVIRONNEMENT_LOCAL.md` : les trois commandes à lancer, les nouvelles clés d'environnement, et **qui envoie les courriels**
- [X] T125 [P] Mettre à jour la progression — journal du jour, `docs/progression/ecrans/b1-socle-identite.md`, décisions, ligne de suivi dans `docs/PROGRESSION.md` et `docs/progression/api.md`
- [X] T126 Noter dans `docs/progression/pieges.md` les trois pièges du module : `anonymize_person()` émet déjà son événement, une écriture sans contexte n'échoue pas, et une adresse inconnue trahit par le temps si l'empreinte factice n'est pas calculée

---

## Dépendances

```
Phase 1 — Mise en place
      ▼
Phase 2 — Fondations  ⚠️ bloquante pour tout le reste
      ▼
   ┌──────────────────────────────────────────────┐
   │  US1 connexion (P1)                          │  ← MVP
   │      ▼                                       │
   │  US2 sessions (P1)                           │
   │      ▼                                       │
   │  US3 périmètre (P1)                          │
   └──────────────────────────────────────────────┘
      ▼
   ┌──────────────────────────────────────────────┐
   │  US4 inscription ──► US5 réinitialisation    │   (P2, la seconde réutilise
   │  US6 rôles                    [indépendante] │    la mécanique de jetons)
   │  US7 effets différés          [indépendante] │
   └──────────────────────────────────────────────┘
      ▼
   US8 RGPD (P3)
      ▼
Phase 11 — Finition
```

**Ce qui dépend vraiment de quoi** :

- **US2 après US1** : la connexion ouvre la session ; US1 la crée, US2 la fait vivre. Les séparer autrement produirait une session que rien n'ouvre.
- **US3 après US2** : borner une liste suppose de savoir qui appelle.
- **US5 après US4** : la réinitialisation réutilise la mécanique de jetons que US4 pose. Techniquement séparable, mais la dupliquer serait absurde.
- **US6, US7 et US8 sont indépendantes entre elles** et ne dépendent que du socle P1.

---

## Ce qui peut tourner en parallèle

**Phase 2** — huit fichiers du noyau sans dépendance mutuelle :

```
T011 config · T012 contexte · T013 erreurs · T016 i18n
T017 crypto · T018 télémétrie · T022 harnais · T023 contrats
```

**Phase 3** — le domaine avant les dépôts : `T036 · T037 · T038`

**Phases 6 à 9, une fois le socle P1 fini** — trois chantiers menés de front, sur des fichiers disjoints :

```
US4 + US5   inscription et réinitialisation   service/registration.rs, password_reset.rs, repo/tokens.rs
US6         rôles                             service/admin_users.rs, repo/rbac.rs
US7         effets différés                   jobs/purge.rs, tests du noyau
```

**Phase 11** — `T118 · T119 · T124 · T125` sans ordre imposé.

---

## Stratégie de livraison

**Le plus petit incrément qui vaille** : Phases 1, 2 et 3 — **T001 à T047**. Quelqu'un peut se connecter, et le formulaire ne dit plus quels comptes existent. C'est démontrable, et c'est déjà la moitié de ce que la v1 faisait mal.

**Le socle utile** : jusqu'à la phase 5 — **T001 à T067**. La connexion, la session, le périmètre. À ce point, les modules B2 à B6 ont tout ce dont ils ont besoin pour commencer : un appelant identifié, une permission testable, un périmètre borné. **C'est le vrai jalon de déblocage du projet.**

**Le module complet** : jusqu'à la phase 10 — **T001 à T115**. L'appel à propositions de la COP31 peut s'ouvrir : les organisations créent leur compte, le vérifient, et l'équipe leur confie des rôles.

**Ce qu'on ne livre pas, et c'est écrit** : le second facteur (arbitré hors périmètre le 20/08), la connexion fédérée (retirée le 17/08), la limitation de débit (aucune exigence, `research.md` § R12).

---

## Récapitulatif

| Phase | Histoire | Priorité | Tâches | Dont tests |
|---|---|---|---|---|
| 1 | Mise en place | — | T001–T010 | — |
| 2 | Fondations | — | T011–T035 | 2 |
| 3 | US1 — connexion | **P1** | T036–T047 | 2 |
| 4 | US2 — sessions | **P1** | T048–T058 | 3 |
| 5 | US3 — périmètre | **P1** | T059–T067 | 2 |
| 6 | US4 — inscription | P2 | T068–T080 | 2 |
| 7 | US5 — réinitialisation | P2 | T081–T088 | 2 |
| 8 | US6 — rôles | P2 | T089–T099 | 2 |
| 9 | US7 — effets différés | P2 | T100–T107 | 5 |
| 10 | US8 — RGPD | P3 | T108–T115 | 2 |
| 11 | Finition | — | T116–T126 | 1 |
| | | | **126 tâches** | **23 tests** |

Les quatre obligations minimales du principe X sont couvertes par **T046/T061** (chemin nominal), **T066** (URL forgée), **T098** (invariant traduit) et **T103** (écriture dans l'outbox).

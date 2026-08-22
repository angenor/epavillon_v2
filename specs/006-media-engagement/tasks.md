---

description: "Task list — Média + Engagement (B6)"
---

# Tasks: Média + Engagement (B6)

**Input**: Design documents from `/specs/006-media-engagement/`

**Prerequisites**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: **OBLIGATOIRES.** Le principe X de la constitution impose des tests d'intégration sur base réelle et jetable, sans aucun mock de base. Les tâches de test ne sont pas optionnelles ici.

**Organization**: par histoire utilisateur, pour que chaque tranche soit implémentable, éprouvable et livrable seule.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichier différent, aucune dépendance en cours
- **[Story]** : `US1` … `US9`, correspondant aux histoires de `spec.md`

## Conventions de chemin

Tout vit dans `backend/`, workspace Cargo symétrique de `frontend/`. **Deux crates sont créés** : `backend/crates/modules/media` et `backend/crates/modules/engagement`. Une exception hors `backend/` : `docs/database/110_engagement.sql`, modifié **une fois**, en T002. Et deux fichiers d'outillage : `Makefile`, `.env.example`.

---

## ⚠️ Six choses à lire avant de commencer

Chacune coûterait cher découverte en chemin, et cinq sur six produisent un défaut **silencieux**.

### 1. Les déclencheurs émettent ET enfilent déjà — deux fois dans ce jalon

Insérer une ligne dans `media.assets` déclenche `media.tg_enqueue_processing()`, qui **met en file** `media.process_asset` **et** émet `media.asset.uploaded`. Appeler `engagement.schedule_session_reminders()` insère les rappels, **met en file un travail par rappel**, passe les lignes à `queued` **et** émet `engagement.reminders.scheduled`.

C'est le piège de B1 (`anonymize_person()`), de B2 (`merge_organizations()`), de B4 (`tg_guard_proposal_status()`) et de B5 (les deux déclencheurs de séance et d'inscription). Ici il porte sur **l'émission et sur la file**, ce qui est nouveau : un service zélé produirait **deux traitements par fichier** et **deux courriels par rappel**, et le doublon ne se verrait qu'en production.

**Aucune tâche de ce découpage n'appelle `kernel::events::emit` ni `kernel::jobs::enqueue` pour ces deux gestes.** **T064** compte les lignes d'outbox **et** de `platform.jobs` après un dépôt et exige **une** de chaque. **T150** en compte autant qu'il y a de rappels créés, jamais le double. Et **T201** est un `grep` qui vaut un test.

Les deux seuls événements que le service émet sont `media.asset.purged` (T187) et `engagement.email.suppressed` (T172).

### 2. `programme.registration.confirmed` N'EXISTE PAS

Le commentaire de `engagement.schedule_session_reminders()` annonce que la fonction est appelée « par l'abonné outbox sur `programme.registration.confirmed` ». **Cet événement n'est émis par personne.**

`programme.registration_status` vaut `registered`, `waitlisted`, `cancelled`, `attended`, `no_show` — jamais `confirmed`. Et `tg_registrations_emit_events()` émet `programme.registration.created` à la **création**, avec le statut en charge utile : une inscription ordinaire naît **à l'état inscrit, par une création**.

Un consommateur écrit d'après ce commentaire **ne serait jamais réveillé** : aucun rappel ne partirait, sans erreur, sans trace, et personne ne s'en apercevrait avant le jour de la séance (écart n° 126).

**T131** branche sur `payload->>'status'`, jamais sur le type d'événement. **T140** éprouve exactement le chemin courant : une inscription **créée** directement à l'état « inscrit », et ses rappels qui doivent exister.

### 3. Une inscription reprise doit RÉACTIVER ses rappels, pas en attendre de nouveaux

`ux_scheduled_reminders_once` porte sur `(session_id, person_id, channel, offset_before)` **sans condition d'état**. Les lignes annulées existent toujours ; `ON CONFLICT DO NOTHING` ne les ressuscite pas.

Une personne qui annule son inscription puis se réinscrit **ne recevrait plus jamais rien**. C'est silencieux, et cela se produit au premier désistement suivi d'un retour.

**T134** remet à `pending` les lignes `cancelled` dont l'instant est encore devant, **avant** d'appeler la fonction de matérialisation. **T142** le mesure : annuler, réinscrire, relire l'état des lignes en base.

Le même raisonnement vaut pour un report : **T135** **déplace** les instants d'envoi des lignes existantes, il ne les recrée pas — et **T143** le mesure.

### 4. L'assainisseur des modèles doit laisser passer les variables d'URL

Un gabarit de courriel contient `href="{{lien_participation}}"`. Un assainisseur qui normalise les URL **détruit la variable** et rend le lien **mort** — un défaut qui ne se voit qu'à la réception, c'est-à-dire après l'envoi à tous les destinataires.

**T029** règle la politique d'URL relatives sur le laisser-passer, et le prouve par un test unitaire. **T159** relit le corps enregistré **après écriture en base** et y cherche `{{` : sans ce test, la décision resterait une intention.

La liste blanche est **différente** de celle de B4 : un courriel a besoin de tableaux et de styles en ligne, les clients de messagerie ignorant les feuilles de style.

### 5. Le schéma change UNE fois, et il faut détruire la base

**T002** ajoute `engagement.session_reminder_schedule()` à `docs/database/110_engagement.sql`. Le schéma n'est chargé qu'au **premier** démarrage du conteneur : sans `down -v`, la base garde l'ancien schéma **sans le dire**, et la compilation échoue sur une fonction que SQLx ne trouve pas — sur un message qui n'a rien à voir avec la cause.

**T003 est donc bloquante pour tout le reste**, et **T004** en tire la conséquence sur le stockage : `down -v` efface aussi le layout de Garage.

### 6. Le préfixe `/sessions` appartient déjà à un module livré

`programme` ouvre `web::scope("/sessions")` depuis B5. **Deux `web::scope` du même préfixe ne se complètent pas** : Actix retient le premier et rend 404 sur les routes du second. Le défaut a coûté trois routes sur vingt et une en B2, il est raconté dans `api/src/lib.rs` et dans `org/src/lib.rs`.

**T116 et T117** appliquent le patron déjà employé pour `/people`, `/organizations` et `/admin/planner` : `programme` expose ses routes **sans le préfixe**, `api` compose une seule fois. **Aucune route de B5 ne change de chemin**, et **T118** le vérifie en frappant les dix-sept.

---

## Phase 1 : Amorçage — le schéma, les crates, l'outillage

**Purpose** : de quoi compiler et de quoi éprouver.

- [X] T001 Lire intégralement `docs/database/050_media.sql` et `docs/database/110_engagement.sql` avant d'écrire une ligne — la règle d'or du dépôt, et la seule protection contre un nom de colonne inventé
- [X] T002 Ajouter `engagement.session_reminder_schedule(p_session_id uuid)` à `docs/database/110_engagement.sql` § 6 : fonction `STABLE`, en lecture seule, rendant `(offset_minutes int, channel text, scheduled_for timestamptz, status text, recipient_count bigint, skip_reason text, sent_at timestamptz)`, ordonnée du décalage le plus lointain au plus proche, avec la consolidation d'état de `research.md` § R18 et un `COMMENT ON FUNCTION` en français expliquant **pourquoi une fonction et non deux requêtes** — ni table, ni colonne, ni type
- [X] T003 Détruire et recharger la base (`docker compose -f ops/docker-compose.dev.yml down -v && up -d`), vérifier les seize schémas et la présence de la fonction ajoutée — **bloquante pour tout le reste**
- [X] T004 Modifier `Makefile` : `garage-init` **importe** une clé fixe lue dans `.env` au lieu d'en créer une aléatoire, et `check-db` l'appelle après `up -d` (R31)
- [X] T005 Ajouter les dix variables nouvelles à `.env.example` avec leur commentaire en français : `MEDIA_STORAGE`, `MEDIA_FS_ROOT`, `MEDIA_MAX_UPLOAD_BYTES`, `MEDIA_SCANNER`, `MEDIA_CLAMD_ADDR`, `MEDIA_SCAN_MAX_BYTES`, `MEDIA_PURGE_INTERVAL`, `MEDIA_RECONCILE_INTERVAL`, `ENGAGEMENT_PARTITION_INTERVAL`, `MAIL_WEBHOOK_TOKEN`
- [X] T006 Déclarer `actix-multipart = "0.8"`, `image` (`default-features = false`, features `jpeg`, `png`, `webp`, `gif`) et `hmac = "0.12"` dans `backend/Cargo.toml`, chacun avec le commentaire qui dit pourquoi — **`hmac` en 0.12 et non 0.13** : la 0.13 exige `sha2` 0.11 quand le workspace tient la 0.10
- [X] T007 Créer `backend/crates/modules/media/` — `Cargo.toml` (dépendances : `kernel`, `contracts`, et rien d'autre venant d'un module), `src/lib.rs`, l'arborescence `domain/ repo/ service/ routes/ jobs/ storage/ scan/`, `state.rs` — et l'inscrire aux membres du workspace
- [X] T008 [P] Créer `backend/crates/modules/engagement/` — même structure, sans `storage/` ni `scan/`, plus `consumers/` et `mail.rs` — et l'inscrire aux membres du workspace
- [X] T009 [P] Vérifier mécaniquement l'absence d'arête : `cargo tree -p media` et `cargo tree -p engagement` ne doivent montrer aucun autre crate de module, **l'un vers l'autre compris**
- [X] T010 Consigner les trois dépendances nouvelles dans `docs/progression/decisions/2026-08-21.md`, comme la constitution l'exige

**Checkpoint** : le workspace compile à vide, la base porte la fonction, Garage a une clé stable.

---

## Phase 2 : Fondations — tout ce que les neuf histoires partagent

**Purpose** : les pièces sans lesquelles aucune histoire ne peut commencer.

**⚠️ CRITIQUE** : aucune histoire ne démarre avant la fin de cette phase.

### Le catalogue d'erreurs et la configuration

- [X] T011 Ajouter les onze codes `MEDIA_*` au catalogue de `backend/crates/kernel/src/error.rs`, avec statut et message français, d'après [`contracts/errors.md`](contracts/errors.md)
- [X] T012 [P] Ajouter les cinq codes `ENGAGEMENT_*` au même catalogue
- [X] T013 [P] Ajouter `MediaConfig` et `EngagementConfig` à `backend/crates/kernel/src/config.rs`, avec leurs valeurs par défaut et leur lecture d'environnement
- [X] T014 [P] Ajouter `MAIL_WEBHOOK_TOKEN` à `MailConfig` — **vide vaut « route non montée »**, jamais « route ouverte » (R30)

### Les contrats d'événements

- [X] T015 Ajouter à `backend/crates/contracts/src/programme.rs` les **noms** des événements émis par la base que ce module consomme — les six `programme.registration.*` et les huit `programme.session.*` —, avec un en-tête disant qu'ils sont émis par **déclencheur** et non par le service de `programme` ; sans eux, chacun les écrirait en littéral dans son coin
- [X] T016 [P] Créer `backend/crates/contracts/src/media.rs` : `AGGREGATE_SCHEMA`, `AGGREGATE_ASSET`, `ASSET_PURGED`, et la charge utile `AssetPurged`. **En-tête** : les deux événements que la base émet déjà n'y figurent pas, et il dit pourquoi
- [X] T017 [P] Créer `backend/crates/contracts/src/engagement.rs` : `EMAIL_SUPPRESSED` et sa charge utile — **l'adresse y est hachée**, jamais en clair, l'outbox étant relayée et tracée
- [X] T018 Déclarer les deux modules dans `backend/crates/contracts/src/lib.rs`

### Le domaine pur du module Média

- [X] T019 [P] `media/src/domain/ids.rs` — `AssetId`, `AttachmentId`, `RenditionId`, sur le patron des identifiants typés des modules livrés
- [X] T020 [P] `media/src/domain/asset.rs` — les cinq énumérations du modèle traversées **en texte** (patron de `identity` et `programme`), et les formes de réponse
- [X] T021 [P] `media/src/domain/keys.rs` — la convention de clé d'objet `<année>/<mois>/<uuid>/<nom-normalisé>.<ext>`, **pure**, avec la normalisation du nom de fichier (ni accent, ni espace, ni barre oblique) et ses tests unitaires
- [X] T022 [P] `media/src/domain/rules.rs` — la table blanche telle qu'elle se lit, et les trois prédicats purs : type accepté, poids accepté, forme acceptée avec sa tolérance
- [X] T023 [P] `media/src/domain/variants.rs` — le jeu de déclinaisons depuis la configuration : trois tailles, format JPEG si opaque et PNG sinon (R12)
- [X] T024 **`media/src/domain/guards.rs` — LA TABLE DE GARDES** : pour chacun des six couples (schéma, table) de `media.attachable_roles`, la garde exigée d'après [`contracts/routes.md`](contracts/routes.md). Toute combinaison non associée est **refusée**, jamais autorisée par défaut (R15, écart n° 127)

### Le domaine pur du module Engagement

- [X] T025 [P] `engagement/src/domain/ids.rs` — `ReminderRuleId`, `ScheduledReminderId`, `NotificationId`, `TemplateId`
- [X] T026 [P] `engagement/src/domain/offsets.rs` — conversion minutes ⇄ intervalle **dans les deux sens**, pure, avec ses tests : c'est la forme du contrat du front, et elle évite de traverser `interval[]` (R19)
- [X] T027 [P] `engagement/src/domain/reminder.rs` — les états, la consolidation de groupe **côté Rust pour les tests**, et les formes de réponse dont `ReminderSlot` et `ApplicableReminderRule`
- [X] T028 [P] `engagement/src/domain/render.rs` — la substitution `{{variable}}`, **pure**, avec le refus nommant la variable manquante et ses tests (R25)
- [X] T029 [P] `engagement/src/domain/sanitize.rs` — la liste blanche du courriel, **différente de celle de B4**, avec la politique d'URL relatives réglée sur le laisser-passer et un test unitaire vérifiant qu'un `href="{{lien}}"` survit (R26)
- [X] T030 [P] `engagement/src/domain/notification.rs` et `template.rs` — formes de réponse

### Les lectures hors schéma, réunies

- [X] T031 `media/src/repo/cross.rs` — **le seul fichier du module qui lise hors de son schéma** : organisation propriétaire, adhésion active, édition d'une entité porteuse, dossier et son organisation, personne. En-tête portant la règle de frontière de B2 et la liste exhaustive
- [X] T032 [P] `engagement/src/repo/cross.rs` — même rôle : séance et son créneau, inscriptions, édition, adhésion, langue et adresse d'une personne, module d'un type de notification
- [X] T033 Vérifier par `grep` qu'**aucune écriture** hors du schéma du module n'existe dans les deux crates — le contrôle est plus strict que celui de B3, B4 et B5, et porte aussi sur `reference` et `content`

### Le stockage, l'analyse et la garde d'envoi

- [X] T034 `media/src/storage/mod.rs` — le trait `ObjectStore` : `put_stream`, `get`, `head`, `delete`, `rename` (R7)
- [X] T035 [P] `media/src/storage/filesystem.rs` — l'implémentation sur fichiers, celle des tests
- [X] T036 `media/src/storage/sigv4.rs` — **la signature, dans un fichier**, avec `hmac` et `sha2` : canonicalisation, condensat de charge utile, en-tête d'autorisation. En-tête portant le **critère de bascule** : une demi-journée, puis on prend le SDK (R8)
- [X] T037 `media/src/storage/s3.rs` — les quatre verbes en *path-style*, au-dessus de T036
- [X] T038 [P] Test — signature vérifiée contre les vecteurs d'exemple d'AWS SigV4, en `media/tests/signature.rs` : le seul moyen de séparer une erreur de signature d'une erreur de configuration
- [X] T039 [P] `media/src/scan/` — le trait `Scanner`, l'implémentation `clamd` (INSTREAM sur TCP, aucune dépendance nouvelle) et l'implémentation `none`, qui rend **`unsupported` et jamais `clean`**, en inscrivant son nom dans la trace (R13)
- [X] T040 **`engagement/src/mail.rs` — LE DÉCORATEUR** : `GardedMailer` implémente `kernel::mail::Mailer`, enveloppe l'expéditeur réel, consulte `engagement.is_email_suppressed()`, écrit la trace dans `engagement.email_messages`, délègue, met la trace à jour. **`OutgoingMail` n'est PAS enrichie** — l'enrichir casserait les six sites de construction des modules livrés, ce que cette décision vise à éviter (R24, écart n° 133)
- [X] T041 Composer le décorateur dans `backend/crates/api/src/state.rs` et dans `backend/crates/worker/src/main.rs` — **aucun module livré n'est modifié**

### Le montage et le harnais

- [X] T042 `media/src/state.rs` et `media/src/lib.rs` — `MediaState` portant base, configuration, stockage et analyseur ; `routes()`, `job_handlers()`
- [X] T043 [P] `engagement/src/state.rs` et `engagement/src/lib.rs` — `EngagementState` ; `routes()`, `session_routes()` **sans préfixe**, `job_handlers()`, `event_consumers()`
- [X] T044 Monter les deux modules dans `backend/crates/api/src/lib.rs`, gardés par `platform.modules` — les codes `media` et `engagement` y sont déjà semés
- [X] T045 Enregistrer les cinq travaux différés et les deux consommateurs dans `backend/crates/worker/src/main.rs`
- [X] T046 [P] Ajouter les deux étiquettes OpenAPI dans `backend/crates/api/src/openapi.rs`
- [X] T047 `media/tests/commun/mod.rs` — harnais : base jetable, stockage **sur fichiers**, analyseur `none`, une organisation, une personne, une édition, et un jeu d'images réelles aux trois formes (32:9, 16:9, 1:1) plus un carré et un PDF
- [X] T048 [P] `engagement/tests/commun/mod.rs` — harnais : édition, séance **datée dans le futur**, inscrits, et de quoi poser une règle. **Le semis ne fournit ni règle, ni modèle de message** : chaque test les pose
- [X] T049 [P] Test — les deux crates montent dans la vraie application et leurs préfixes répondent, en `backend/crates/api/tests/routes_media_engagement.rs` (squelette, complété à chaque histoire)

**Checkpoint** : les deux modules existent, ne dépendent de personne, n'écrivent nulle part hors de leur schéma, et le décorateur d'envoi est en place.

---

> **⚠️ T116 et T117 ONT ÉTÉ FAITES EN PHASE 2, le 21/08, et ce n'est pas une avance de confort.**
> T044 monte les deux modules dans `api/src/lib.rs`. Y composer le scope `/sessions` sans que
> `programme` cesse de l'ouvrir aurait rendu **muettes les dix-sept routes de B5** — l'état
> intermédiaire que le sixième avertissement dit de ne pas laisser derrière soi. Les deux tâches ont
> donc été menées d'un trait avec T044. **T118 reste à faire** : le test livré
> (`crates/api/tests/routes_media_engagement.rs`) frappe les routes **gardées** du préfixe ; les deux
> routes publiques rendent 404 sur une séance inconnue — légitimement, et indiscernable d'une route non
> montée —, et c'est `routes_programme_sessions.rs` qui les éprouve avec une vraie séance.

---

## Phase 3 : US1 — Un fichier arrive, et il n'occupe la place qu'une fois (P1) 🎯 MVP

**Goal** : le dépôt, la déduplication par empreinte, le quota opposable.

**Independent Test** : déposer une image de 2 Mio et vérifier qu'elle est réellement sur le stockage à la clé annoncée ; redéposer le même contenu sous un autre nom et vérifier qu'aucun second objet n'est écrit ; remplir un quota et obtenir un refus nommant l'espace restant.

### Implémentation

- [X] T050 [US1] `media/src/repo/assets.rs` — écriture d'un objet, lecture par identifiant, **recherche par empreinte** via `media.find_by_checksum()`, et lecture de l'adresse composée par `media.object_url()`
- [X] T051 [US1] `media/src/repo/quotas.rs` — capacité par `media.has_storage_capacity()`, et lecture des trois chiffres (plafond, consommé, restant) pour le message de refus
- [X] T052 [US1] `media/src/service/upload.rs` — la pré-vérification : garde, table blanche, capacité, recherche d'empreinte si elle est fournie. **N'écrit rien, ne réserve rien** (FR-016)
- [X] T053 [US1] `media/src/service/upload.rs` — le dépôt : lecture du flux vers une clé **temporaire**, empreinte calculée **au passage**, poids réel comparé au poids annoncé, puis déduplication ou renommage vers la clé définitive (R6, R10)
- [X] T054 [US1] Refus du texte alternatif manquant sur une image, sous `MEDIA_ALT_TEXT_REQUIRED`, **sur le champ** (R9, écart n° 129)
- [X] T055 [US1] Traduction du refus de quota de la base — `SQLSTATE 53100` — sous **le même code** que le refus préalable, avec les trois chiffres (R14, écart n° 136)
- [X] T056 [US1] Nettoyage du temporaire sur flux rompu, sur écart de poids et sur refus de la base : **aucune description écrite, rien ne traîne**
- [X] T057 [US1] `media/src/routes/uploads.rs` — `POST /media/assets/precheck` et `POST /media/assets` (multipart, métadonnées **avant** le fichier), avec leurs annotations OpenAPI et la limite de corps propre
- [X] T058 [US1] `media/src/service/read.rs` et `media/src/routes/assets.rs` — `GET /media/assets/{id}`, rendant l'adresse composée et **jamais** la clé nue

### Tests

- [X] T059 [P] [US1] Test — un dépôt écrit l'objet, rend son identifiant, et le fichier est **réellement** présent sur le stockage à la clé annoncée, en `media/tests/depot.rs`
- [X] T060 [P] [US1] Test — **le même contenu sous un autre nom n'écrit aucun second objet**, et l'identifiant rendu est celui du premier ; `count(*)` sur `media.assets` ne bouge pas
- [X] T061 [P] [US1] Test — la pré-vérification rend un verdict **sans rien écrire** : aucune ligne, aucune clé sur le stockage, dans les cinq cas (accepté, type, poids, quota, doublon)
- [X] T062 [P] [US1] Test — **le refus de quota porte le même code avant et après l'écriture**, et ses trois chiffres, en `media/tests/quota.rs`
- [X] T063 [P] [US1] Test — une image sans texte alternatif est refusée **sur le champ `alt_text`**, et un document sans texte alternatif est accepté
- [X] T064 [P] [US1] Test — **le service n'émet rien et n'enfile rien** : après un dépôt, `platform.outbox_events` compte **une** ligne et `platform.jobs` **une**, jamais deux (avertissement n° 1)
- [X] T065 [P] [US1] Test — un dépôt sur une entité porteuse hors du périmètre de l'appelant reçoit **le même refus** qu'une entité inexistante, en `media/tests/gardes.rs`
- [X] T066 [P] [US1] Test — **la table blanche n'a aucune ligne sans garde** : le test lit `media.attachable_roles` **en base** et échoue sur toute ligne que `domain/guards.rs` ne connaît pas (R15)
- [X] T067 [US1] Ajouter les trois routes d'US1 au test de montage `backend/crates/api/tests/routes_media_engagement.rs`

**Checkpoint** : un fichier entre dans la plateforme, ne s'écrit qu'une fois, et le quota est opposable.

---

## Phase 4 : US2 — Le fichier devient servable sans faire attendre personne (P1)

**Goal** : mesure, analyse, déclinaisons, passage à l'état servable — en tâche de fond.

**Independent Test** : déposer une photographie, constater qu'elle s'affiche immédiatement, et que ses déclinaisons apparaissent ensuite sans nouveau geste ; déposer un fichier reconnu comme dangereux et constater qu'aucune lecture ne le rend.

### Implémentation

- [X] T068 [US2] `media/src/repo/renditions.rs` — écriture d'une déclinaison avec poids et instant de fabrication **ensemble** (`ck_renditions_ready_shape`), et lecture de l'avancement
- [X] T069 [US2] `media/src/jobs/process.rs` — le travail `media.process_asset` : lire l'objet depuis le stockage, relever dimensions ou durée, faire analyser, fabriquer les déclinaisons, écrire l'état servable
- [X] T070 [US2] Le redimensionnement vit dans une **tâche bloquante dédiée** — quelques centaines de millisecondes sur une photographie, comme le hachage de mot de passe en B1
- [X] T071 [US2] Mise en quarantaine sur verdict positif : l'objet n'est **jamais** servi, et tout rattachement le visant est refusé
- [X] T072 [US2] Reprise sans doublon : une déclinaison déjà fabriquée n'est pas refaite ; l'échec définitif écrit son motif et se distingue d'une absence
- [X] T073 [US2] `media/src/routes/assets.rs` — `GET /media/assets/{id}/status`, rendant état, verdict, et déclinaisons prêtes sur déclinaisons attendues
- [X] T074 [US2] `media/src/service/read.rs` — lecture des déclinaisons prêtes par `media.asset_sources()`, sous la forme exacte qu'un `<picture>` attend

### Tests

- [X] T075 [P] [US2] Test — **l'original est servi avant les déclinaisons** : juste après le dépôt, l'adresse est là et la liste des déclinaisons est **vide mais présente**, en `media/tests/traitement.rs`
- [X] T076 [P] [US2] Test — après passage du worker, dimensions relevées, déclinaisons écrites, objet servable
- [X] T077 [P] [US2] Test — **worker arrêté puis relancé : le traitement se fait une seule fois**, `count(*)` sur `media.renditions` égale le nombre de déclinaisons configurées, jamais le double
- [X] T078 [P] [US2] Test — un document n'a ni dimension ni déclinaison, et devient servable ; un média temporel a sa durée et aucune déclinaison d'image
- [X] T079 [P] [US2] Test — un objet en quarantaine n'est rendu par **aucune** lecture publique, et son rattachement est refusé
- [X] T080 [P] [US2] Test — **le moteur `none` écrit `unsupported` et son nom**, jamais `clean` : l'objet devient servable, et la trace dit ce qui a répondu (R13)
- [X] T081 [P] [US2] Test — l'espace des déclinaisons est compté dans la consommation de l'organisation, mesuré sur `media.storage_quotas.used_bytes`
- [X] T082 [US2] Ajouter la route d'avancement au test de montage

**Checkpoint** : une image déposée s'affiche tout de suite, et devient servable seule.

---

## Phase 5 : US3 — Un fichier se rattache à ce qu'il illustre (P1)

**Goal** : la table blanche, le rôle exclusif, la forme attendue, l'affectation en lot — et **l'obligation que B3 a laissée**.

**Independent Test** : enregistrer les trois visuels d'une édition en un geste, puis relire sa fiche par la route de B3 et constater les trois déclinaisons résolues ; provoquer les quatre refus ; retirer un visuel et vérifier que l'objet stocké existe toujours.

### Implémentation

- [X] T083 [US3] `media/src/repo/attachments.rs` — pose, retrait, lecture par entité et rôle, lecture de la table blanche, **décompte des rattachements d'un objet**
- [X] T084 [US3] `media/src/service/attach.rs` — poser sur un rôle multiple, avec l'ordre de tri déclaré
- [X] T085 [US3] `media/src/service/attach.rs` — **l'écriture de remplacement en lot** : une liste d'affectations, appliquées en **une** transaction ; une valeur nulle **retire** sans toucher aux autres (R16, FR-043)
- [X] T086 [US3] Traduction des cinq refus de `tg_validate_attachment` — que le service distingue **par le contrôle qu'il a lui-même fait en amont**, jamais par le texte du message (`contracts/errors.md`)
- [X] T087 [US3] Le refus de forme cite dimensions reçues, rapport obtenu, rapport attendu et tolérance (FR-037)
- [X] T088 [US3] Le texte alternatif propre à un usage prime sur celui de l'objet, sans le modifier (FR-040)
- [X] T089 [US3] `media/src/routes/attachments.rs` — les quatre routes de rattachement, avec leurs annotations OpenAPI
- [X] T090 [US3] `media/src/routes/roles.rs` — `GET /media/roles`, rendant **aussi** la forme attendue et sa tolérance, que le contrat du front ne porte pas encore (data-model § 7)

### Tests

- [X] T091 [P] [US3] Test — **les trois déclinaisons d'une édition s'enregistrent en un geste**, et `GET /events/{slug}` de B3 les rend résolues, en `media/tests/edition_images.rs`. C'est la route de B3 qui sert de mesure
- [X] T092 [P] [US3] Test — une valeur nulle retire **une** déclinaison sans toucher aux deux autres
- [X] T093 [P] [US3] Test — un carré comme bandeau est refusé **en citant le rapport reçu et le rapport attendu**
- [X] T094 [P] [US3] Test — un objet image **sans dimensions relevées** est accepté sur un rôle imposant une forme : c'est le relevé qui a échoué, pas le cadrage (FR-036)
- [X] T095 [P] [US3] Test — un second objet **ajouté** sur un rôle exclusif est refusé ; **remplacé**, il passe et l'ancien rattachement a disparu
- [X] T096 [P] [US3] Test — une combinaison non déclarée est refusée en nommant l'entité et le rôle, **jamais en 500**
- [X] T097 [P] [US3] Test — **détacher ne détruit pas l'objet** : l'objet est relu **après** le détachement, et il est toujours là
- [X] T098 [P] [US3] Test — un rôle multiple rend ses objets dans l'ordre de tri, et l'ordre est modifiable
- [X] T099 [P] [US3] Test — un compte détaché sur une édition ne peut rattacher à **aucune autre**, six identifiants forgés menant au même refus qu'un identifiant inexistant
- [X] T100 [US3] Ajouter les cinq routes d'US3 au test de montage

**Checkpoint** — **JALON 1 (T001–T100)** : **un fichier entre, devient servable, et se rattache.** Les trois écrans livrés qui téléversent ont enfin une API, et l'obligation que B3 avait laissée est refermée.

---

## Phase 6 : US6 — L'administrateur décide ce qui part (P2)

> **⚠️ Cette phase passe AVANT US4 et US5, contre l'ordre des priorités, et c'est assumé.** Rien ne sème de règle de rappel : sur une base neuve, aucune édition n'en a (écart n° 130). **Sans écriture de règle, ni le calendrier ni les envois ne peuvent être démontrés autrement qu'en posant une ligne à la main en SQL** — ce qui prouverait la lecture sans prouver le chemin réel. La même entorse a été faite en B5, pour la même raison, et dite de la même façon.

**Goal** : écrire une règle, la couper, et savoir **quelle** règle s'applique à une séance et **d'où** elle vient.

**Independent Test** : créer une règle d'édition, demander la règle applicable d'une de ses séances et constater qu'elle vient de l'édition ; poser une règle de séance à deux décalages et constater que la réponse ne porte plus que ces deux-là, en disant qu'elle vient de la séance.

### Implémentation

- [X] T101 [US6] `engagement/src/repo/rules.rs` — lecture et écriture des règles, **décalages en minutes dans les deux sens** (R19), canaux traversés en texte avec cast
- [X] T102 [US6] Résolution de la **règle applicable** : celle de la séance si elle existe, sinon celle de l'édition, **sans cumul** — le même `ORDER BY` que la fonction du modèle, jamais une fusion (FR-075)
- [X] T103 [US6] `engagement/src/service/rules.rs` — l'écriture : une **liste** de décalages, jamais un décalage seul ; défaut `[2880, 1440, 60, 30]` (FR-070)
- [X] T104 [US6] Traduction des refus du modèle sur les décalages et sur la portée, chacun **sur son champ**
- [X] T105 [US6] Unicité par édition et par séance traitée comme une **modification**, jamais comme une erreur (FR-073)
- [X] T106 [US6] Coupure d'une règle : annulation des rappels encore à traiter des séances qu'elle gouvernait, avec le décompte rendu (FR-078)
- [X] T107 [US6] `engagement/src/routes/rules.rs` — les trois routes d'administration, gardées par `engagement.reminder.manage` sur la portée **visée** et bornées par le périmètre

### Tests

- [X] T108 [P] [US6] Test — une règle s'écrit avec ses quatre décalages et se relit **en minutes**, en `engagement/tests/regles.rs`
- [X] T109 [P] [US6] Test — **la règle de séance remplace celle de l'édition, sans s'y ajouter** : deux décalages posés sur la séance, deux rendus, et l'origine dit « séance »
- [X] T110 [P] [US6] Test — une séance sans règle propre rend celle de l'édition, avec l'origine « édition » et l'identifiant de l'édition
- [X] T111 [P] [US6] Test — une séance dont ni elle ni son édition n'ont de règle rend **explicitement** qu'aucune ne s'applique (FR-076)
- [X] T112 [P] [US6] Test — liste vide, décalage négatif, neuf décalages : trois refus **sur le champ des décalages** ; portée double ou absente : refus sur la portée
- [X] T113 [P] [US6] Test — une seconde écriture pour la même édition **modifie** la première, et `count(*)` reste à un
- [X] T114 [P] [US6] Test — un compte détaché sur une édition ne peut pas paramétrer les rappels d'une autre
- [X] T115 [US6] Ajouter les trois routes au test de montage

**Checkpoint** : une édition peut enfin avoir une règle, et l'on sait laquelle s'applique.

---

## Phase 7 : US4 — Le calendrier des rappels, en quatre lignes et sans un nom (P2)

**Goal** : l'écart n° 34, et la fermeture de l'écart n° 108 laissé par B4.

**Independent Test** : sur une séance à quarante inscrits et une règle à quatre décalages, demander le calendrier et compter **quatre** lignes portant chacune quarante destinataires ; balayer la réponse entière et n'y trouver aucun identifiant de personne, aucun nom, aucune adresse.

### Le préfixe partagé — à faire avant les routes

- [X] T116 [US4] Modifier `backend/crates/modules/programme/src/lib.rs` et `routes/sessions.rs` : `/sessions` n'est plus ouvert par le module, ses routes sont exposées **sans le préfixe**, dans l'ordre d'enregistrement d'avant (avertissement n° 6)
- [X] T117 [US4] Composer `/sessions` dans `backend/crates/api/src/lib.rs` à partir de `programme` **et** `engagement`, une seule fois, sur le patron de `/people` et de `/admin/planner`
- [X] T118 [US4] Test — **les dix-sept routes de B5 répondent toujours, aux mêmes chemins**, dans `backend/crates/api/tests/`. Sans elle, la refactorisation est une supposition

### Implémentation

- [X] T119 [US4] `engagement/src/repo/reminders.rs` — appel de `engagement.session_reminder_schedule()`, colonnes annotées **une à une** : une fonction qui rend une table ne porte aucune contrainte de nullité (leçon de B3)
- [X] T120 [US4] `engagement/src/service/schedule.rs` — la lecture du calendrier, avec `has_rule` distinguant « aucune règle » de « tout est parti » (FR-051)
- [X] T121 [US4] Garde : adhésion active à l'organisation qui anime, **ou** `programme.registration.manage` sur l'édition — jamais un périmètre d'administration seul, une organisation n'administrant rien
- [X] T122 [US4] `engagement/src/routes/sessions.rs` — les deux routes, **sans préfixe**, avec leurs annotations OpenAPI
- [X] T123 [US4] Modifier `backend/crates/modules/programme/src/repo/sessions.rs` et `domain/sessions.rs` : `TrackedSession.reminders` cesse d'être une liste vide et appelle **la même fonction** (FR-052, FR-053, écart n° 108)

### Tests

- [X] T124 [P] [US4] Test — quarante inscrits et quatre décalages rendent **quatre** lignes portant chacune quarante destinataires, en `engagement/tests/calendrier.rs`
- [X] T125 [P] [US4] Test — **balayage de la charge utile sérialisée entière** : aucun identifiant de personne, aucun nom, aucune adresse. Champ par champ laisserait passer celui qu'on ajoutera demain (FR-048)
- [X] T126 [P] [US4] Test — une règle à deux canaux rend **huit** lignes et non quatre : deux canaux sont deux envois
- [X] T127 [P] [US4] Test — la consolidation d'état : une ligne encore en attente parmi trente-neuf parties rend un groupe **en attente**, jamais « parti » (R18)
- [X] T128 [P] [US4] Test — les lignes sont ordonnées du décalage le plus lointain au plus proche
- [X] T129 [P] [US4] Test — **l'espace organisation porte le calendrier**, dans la forme exacte du contrat du front, et ne rend plus une liste vide
- [X] T130 [US4] Ajouter les deux routes au test de montage

**Checkpoint** : l'organisation lit ce qui part, sans savoir à qui. L'écart n° 34 est refermé.

---

## Phase 8 : US5 — Le rappel part une fois, et une seule (P2)

**Goal** : consommer, matérialiser, réactiver, décaler, annuler, envoyer.

**Independent Test** : inscrire quarante personnes, faire passer l'heure du premier décalage, constater quarante envois ; rejouer l'événement d'inscription et le travail d'envoi, et constater qu'aucun second courriel n'est écrit ; déplacer la séance et constater que les instants d'envoi ont suivi.

### Implémentation

- [X] T131 [US5] `engagement/src/consumers/reminders.rs` — **branche sur `payload->>'status'`**, jamais sur `programme.registration.confirmed`, qui n'existe pas (avertissement n° 2, écart n° 126)
- [X] T132 [US5] Écoute des huit `programme.session.*` : matérialiser sur programmation, **décaler** sur report, annuler sur annulation, ne rien faire sur direct et terminé
- [X] T133 [US5] `engagement/src/service/schedule.rs` — la matérialisation : appel de la fonction du modèle, **sans émettre ni enfiler** (avertissement n° 1)
- [X] T134 [US5] **La réactivation** : remettre à `pending` les lignes `cancelled` dont l'instant est encore devant, **avant** d'appeler la fonction (avertissement n° 3, R21)
- [X] T135 [US5] **Le décalage** sur report : déplacer les instants d'envoi des lignes encore à traiter, jamais les recréer ; un décalage dont l'instant est désormais passé n'est pas envoyé en rattrapage
- [X] T136 [US5] L'annulation avec son motif : séance annulée, inscription annulée, règle coupée
- [X] T137 [US5] `engagement/src/service/compose.rs` — du type au courriel : modèle publié s'il existe, **texte de secours sinon**, avec la trace disant qu'aucun modèle n'a servi (R27, écart n° 131)
- [X] T138 [US5] `engagement/src/jobs/send_reminder.rs` — le travail `engagement.send_reminder` : préférence consultée par `engagement.is_channel_enabled()`, liste de suppression par la garde d'envoi, composition dans la langue du destinataire, écriture de l'instant d'envoi
- [X] T139 [US5] Un envoi écarté l'est **avec son motif**, jamais en silence (FR-065)

### Tests

- [X] T140 [P] [US5] Test — **une inscription créée directement à l'état « inscrit » matérialise ses rappels** : c'est le chemin courant, et celui qu'une lecture du commentaire du modèle aurait cassé (avertissement n° 2), en `engagement/tests/rappels.rs`
- [X] T141 [P] [US5] Test — une personne en liste d'attente n'a **aucun** rappel ; promue, elle les obtient
- [X] T142 [P] [US5] Test — **annuler puis réinscrire rend les rappels à `pending`** et non `cancelled` : le cas que la clé d'unicité rend piégeux (avertissement n° 3)
- [X] T143 [P] [US5] Test — une séance déplacée de trois heures voit les instants de ses rappels non partis **décalés d'autant**, et ses rappels partis **inchangés**
- [X] T144 [P] [US5] Test — un décalage dont l'instant est déjà passé n'est pas créé
- [X] T145 [P] [US5] Test — **l'annonce d'inscription rejouée dix fois produit zéro rappel et zéro travail supplémentaires** (garde de rejeu, unicité, idempotence de file)
- [X] T146 [P] [US5] Test — l'heure venue, **un** courriel part par destinataire ; le travail rejoué n'en produit pas de second
- [X] T147 [P] [US5] Test — **le relais arrêté, rien ne part et rien n'est perdu** ; relancé, le courriel arrive — le point de contrôle de B1, réemployé
- [X] T148 [P] [US5] Test — une adresse supprimée ne reçoit rien, et le rappel porte son motif ; un canal coupé de même
- [X] T149 [P] [US5] Test — une séance annulée annule les rappels à traiter avec leur motif, et laisse les partis tracés
- [X] T150 [P] [US5] Test — **le service n'émet ni n'enfile** : après une matérialisation, `platform.outbox_events` compte **une** ligne et `platform.jobs` exactement le nombre de rappels créés
- [X] T151 [US5] Vérifier que `engagement.send_reminder` **ne déclare pas** `carries_secret()` : sa charge utile ne porte que des identifiants, et un travail mort en garde le diagnostic

**Checkpoint** — **JALON 2 (T001–T151)** : **les rappels de la COP31 partent, une fois et une seule, et l'organisation lit ce qui part.** Les écarts n° 34 et n° 108 sont refermés.

---

## Phase 9 : US7 — Les textes des courriels se corrigent sans redéploiement (P3)

**Goal** : modèles versionnés, multilingues, publiés et réversibles.

**Independent Test** : publier une révision, envoyer un message de ce type et constater le nouveau texte ; revenir à la précédente et constater l'ancien ; citer une variable non promise et constater le refus.

### Implémentation

- [X] T152 [US7] `engagement/src/repo/templates.rs` — modèles, révisions, version servie, variables promises par le type
- [X] T153 [US7] `engagement/src/service/templates.rs` — écriture d'une révision, **corps assaini à l'écriture** par `domain/sanitize.rs` (avertissement n° 4)
- [X] T154 [US7] Publication : un seul geste, réversible ; refus si le gabarit cite une variable que le type ne promet pas, **en la nommant** (FR-083)
- [X] T155 [US7] Aperçu rendu dans les deux langues avec des valeurs d'exemple, **sans rien envoyer**
- [X] T156 [US7] Repli de langue sur le français, conformément au modèle
- [X] T157 [US7] `engagement/src/routes/templates.rs` — les cinq routes, gardées par `engagement.template.manage`
- [X] T158 [US7] La trace d'expédition porte le modèle **et le numéro de révision** réellement servis (FR-089)

### Tests

- [X] T159 [P] [US7] Test — **un `href="{{lien}}"` survit à l'assainissement**, et un `<script>` disparaît **sans emporter le texte** qu'il contenait, en `engagement/tests/modeles.rs` (avertissement n° 4)
- [X] T160 [P] [US7] Test — une révision non publiée n'est pas servie ; publiée, elle l'est ; la précédente reste republiable
- [X] T161 [P] [US7] Test — une variable non promise fait **refuser la publication**, en la nommant
- [X] T162 [P] [US7] Test — **une variable manquante à l'exécution fait échouer l'envoi** : rien ne part, et le motif la nomme
- [X] T163 [P] [US7] Test — un type **sans modèle publié** envoie tout de même, et la trace porte un modèle nul (R27)
- [X] T164 [P] [US7] Test — l'aperçu n'écrit aucune trace d'expédition et n'appelle pas l'expéditeur
- [X] T165 [US7] Ajouter les cinq routes au test de montage

**Checkpoint** : une faute d'orthographe se corrige sans redéploiement, dans les deux langues.

---

## Phase 10 : US8 — Chacun choisit ce qu'il reçoit, et une adresse morte cesse d'être sollicitée (P3)

**Goal** : notifications in-app, préférences, criticité, délivrabilité.

**Independent Test** : couper un canal pour un type et constater que la notification n'arrive que sur l'autre ; couper un type critique et constater la coupure sans effet ; poser une suppression et constater qu'aucun envoi ne part, **quel que soit le module émetteur**.

### Implémentation

- [X] T166 [US8] `engagement/src/repo/notifications.rs` et `preferences.rs` — écriture, lecture, marquage lu et archivé, préférences par type et canal
- [X] T167 [US8] **Le regroupement** : incrémenter `group_count` sur la notification non lue portant la même clé plutôt qu'en créer une seconde — le modèle le décrit, aucune fonction ne le fait (FR-092)
- [X] T168 [US8] `engagement/src/consumers/notifications.rs` — cherche un type de notification **portant le code de l'événement** ; s'il n'y en a pas, ne fait rien. **La correspondance est une donnée**, pas du code (R22)
- [X] T169 [US8] La résolution des destinataires et des variables pour les **quatre** types servis, et la déclaration explicite que les quatorze autres restent non consommés (R23)
- [X] T170 [US8] `engagement/src/service/notifications.rs` — la lecture rendant lignes **et** compte de non lues dans la même réponse, chaque type disant s'il est **non désactivable** (FR-095)
- [X] T171 [US8] `engagement/src/repo/suppressions.rs` et `service/deliverability.rs` — liste de suppression, levée automatique d'une suppression échue, mise à jour des traces
- [X] T172 [US8] Émission de `engagement.email.suppressed`, **adresse hachée** (contracts/events § 2)
- [X] T173 [US8] `engagement/src/routes/notifications.rs`, `preferences.rs`, `suppressions.rs`, `broadcast.rs` — les dix routes
- [X] T174 [US8] `engagement/src/routes/internal.rs` — `POST /internal/mail-events`, jeton porteur, hors session et hors contrôle d'origine ; **non montée si le jeton n'est pas configuré** (R30)

### Tests

- [X] T175 [P] [US8] Test — **la garde d'envoi s'applique à un module livré** : une adresse supprimée ne reçoit pas l'invitation d'organisation de B2, alors qu'aucune ligne de ce module n'a été modifiée (écart n° 133, R24), en `engagement/tests/delivrabilite.rs`
- [X] T176 [P] [US8] Test — couper un canal pour un type non critique supprime l'envoi sur ce canal et le laisse sur l'autre
- [X] T177 [P] [US8] Test — **couper un type critique est sans effet**, et la lecture le dit
- [X] T178 [P] [US8] Test — un type inconnu vaut **refus d'envoi**, jamais un envoi par défaut
- [X] T179 [P] [US8] Test — trois notifications de même clé forment **une** ligne portant un compte, tant qu'elle n'est pas lue
- [X] T180 [P] [US8] Test — un lien de rebond est **relatif** : aucun nom d'hôte ne fuite dans les données
- [X] T181 [P] [US8] Test — une annonce de délivrabilité **rejouée** ne crée pas de seconde trace, et est comptée dans `ignored`
- [X] T182 [P] [US8] Test — une suppression temporaire échue se lève **sans intervention**
- [X] T183 [P] [US8] Test — la route d'ingestion **n'est pas montée** sans jeton configuré, et rend 404
- [X] T184 [US8] Ajouter les dix routes au test de montage

**Checkpoint** : la plateforme cesse d'écrire à qui ne veut plus la lire, et la réputation du domaine est protégée.

---

## Phase 11 : US9 — Le disque ne se remplit pas tout seul (P3)

**Goal** : orphelins, purge, réconciliation, quotas au back-office.

**Independent Test** : déposer un fichier sans le rattacher, avancer sa date, le voir dans les orphelins ; programmer sa purge, constater qu'il est encore récupérable, puis, la fenêtre échue, constater qu'il a quitté le stockage et que la consommation a baissé d'autant.

### Implémentation

- [X] T185 [US9] `media/src/repo/assets.rs` — lecture des orphelins par `media.find_orphan_assets()`, du plus lourd au plus léger
- [X] T186 [US9] `media/src/service/admin.rs` — **la suppression refusée si l'objet est encore rattaché**, avec le nombre d'entités qui l'utilisent (R11, écart n° 128) ; sinon `media.schedule_asset_purge()`
- [X] T187 [US9] `media/src/jobs/purge.rs` — travail récurrent **qui se replanifie lui-même**, réarmé au démarrage du worker (patron de B1) : efface objets et déclinaisons dont la fenêtre est échue, écrit l'instant de purge, **émet `media.asset.purged`**
- [X] T188 [US9] Une purge dont l'objet a déjà disparu du stockage **aboutit** : l'objectif est atteint (FR-108)
- [X] T189 [US9] [P] `media/src/jobs/reconcile.rs` — travail récurrent appelant `media.reconcile_storage_quotas()` et traçant le nombre de lignes corrigées
- [X] T190 [US9] [P] `engagement/src/jobs/partitions.rs` — travail récurrent appelant `platform.ensure_month_partition()` pour les mois à venir (écart n° 137)
- [X] T191 [US9] `media/src/routes/admin.rs` — les trois routes de back-office, gardées par `org.organization.manage` sur la portée globale, avec **refus explicite** pour un périmètre vide

### Tests

- [X] T192 [P] [US9] Test — un objet non rattaché depuis plus d'un mois apparaît dans les orphelins ; un objet rattaché n'y apparaît **jamais**, en `media/tests/purge.rs`
- [X] T193 [P] [US9] Test — **la suppression d'un objet rattaché à deux fiches est refusée, en disant deux**
- [X] T194 [P] [US9] Test — un objet purgé **a quitté le stockage**, vérifié sur le stockage lui-même, et la consommation a baissé de son poids **déclinaisons comprises**
- [X] T195 [P] [US9] Test — un objet dont la fenêtre n'est pas échue n'est pas touché
- [X] T196 [P] [US9] Test — après réconciliation, la consommation enregistrée **égale** la consommation calculée par `media.organization_storage_usage()`
- [X] T197 [P] [US9] Test — les trois travaux récurrents **se replanifient**, et le démarrage du worker **réarme** la chaîne sans créer de doublon
- [X] T198 [US9] Ajouter les trois routes au test de montage

**Checkpoint** — **JALON 3 (T001–T198)** : les neuf histoires sont livrées.

---

## Phase 12 : Polissage et vérifications transverses

- [ ] T199 [P] Test — **les trente-trois routes répondent sur la vraie application**, avec leur statut attendu : le test qui a attrapé trois routes muettes en B2 et un scope non monté en B4
- [ ] T200 [P] Vérifier que les trente-trois opérations figurent dans l'OpenAPI engendrée, et que les seize codes d'erreur y sont — **deux comptes écrits à deux endroits**, comme en B4
- [ ] T201 [P] `grep -rn 'events::emit\|jobs::enqueue' crates/modules/media/src crates/modules/engagement/src` : seuls **T187** et **T172** doivent apparaître pour l'émission ; pour la file, **exactement trois** — T187, T189 et T190, chacune posant sa PROPRE occurrence suivante et rien d'autre. Aucune mise en file d'un travail métier (avertissement n° 1, précisé en phase 11)
- [ ] T202 [P] `grep` d'écriture hors schéma sur les deux crates, **`reference` et `content` compris** — il doit rester vide
- [ ] T203 [P] `cargo tree -p media` et `-p engagement` : aucune arête vers un autre crate de module, **l'un vers l'autre compris**
- [ ] T204 [P] Aucun fichier de `backend/` au-dessus de mille lignes — découper `service/upload.rs` et `consumers/notifications.rs` en priorité si la barre est franchie
- [X] T205 Mettre à jour `docs/progression/modele.md` : la fonction ajoutée, sa date et sa raison — **obligatoire**, un fichier de `docs/database/` a changé
- [ ] T206 [P] Inscrire aux points bloqués les obligations de B7 nées de ce jalon : le champ de description d'image sur les trois écrans qui téléversent, et la forme attendue exposée par `AttachableRoleRule` que le contrat du front ne porte pas
- [ ] T207 Éprouver les sept parcours du [quickstart](quickstart.md) à la main, **sur Garage réel** — c'est le seul moment où le stockage S3 est exercé (R7)
- [ ] T208 `make check` en entier depuis la racine : base détruite et rechargée, seize schémas, rapport de frontières vide, `clippy -D warnings`, `cargo test --workspace`
- [ ] T209 Mettre à jour la progression — journal du jour, `ecrans/b6-media-engagement.md`, décisions, `api.md`, ligne de suivi dans `PROGRESSION.md`

---

## Dependencies & Execution Order

### Dépendances de phase

- **Phase 1 (amorçage)** : aucune dépendance. **T003 bloque tout le reste** — le schéma a changé.
- **Phase 2 (fondations)** : dépend de la phase 1. **Bloque les neuf histoires.**
- **Phase 3 → 5 (US1, US2, US3)** : séquentielles entre elles — on ne traite pas un objet qui n'existe pas, on ne rattache pas un objet qu'on ne sait pas déposer.
- **Phase 6 (US6)** : dépend des fondations seulement. **Passe avant US4 et US5**, et l'encadré de la phase dit pourquoi.
- **Phase 7 (US4)** : dépend de US6 — sans règle, il n'y a rien à agréger. Dépend aussi de **T002**, la fonction ajoutée.
- **Phase 8 (US5)** : dépend de US6, et de la garde d'envoi (T040, fondations).
- **Phase 9 (US7)** et **Phase 10 (US8)** : dépendent des fondations. US7 améliore US5 sans lui être nécessaire — un rappel part avec le texte de secours.
- **Phase 11 (US9)** : dépend de US1 (des objets à purger) et de US3 (des rattachements à compter).
- **Phase 12** : dépend de tout ce qu'on veut livrer.

### La dépendance que le découpage a fait apparaître

**US6 doit précéder US4 et US5**, contre l'ordre des priorités. Rien ne sème de règle de rappel (écart n° 130) : sans l'écriture de règle, ni le calendrier ni les envois ne se démontrent autrement qu'en posant une ligne à la main en SQL — ce qui prouverait la lecture sans prouver le chemin réel. **US6 devient l'instrument de mesure des deux autres.** La même entorse a été faite en B5, pour la même raison.

### Une dépendance qui touche du code livré

**T116, T117 et T118 modifient le module `programme`** et doivent être faites **ensemble**, dans cet ordre. Entre T116 et T117, les dix-sept routes de B5 sont **muettes** : ne pas s'arrêter au milieu.

### Parallélisme

- Tout ce qui est marqué [P] dans une même phase touche des fichiers distincts.
- Les deux crates sont **entièrement indépendants** : `media` (US1, US2, US3, US9) et `engagement` (US4, US5, US6, US7, US8) peuvent être menés en parallèle par deux personnes après la phase 2, à trois exceptions près — T024 et T040 (fondations), et T116–T118 (le préfixe partagé).

---

## Implementation Strategy

### Les trois jalons

| Jalon | Tâches | Ce qui est livré |
|---|---|---|
| **1** | T001–T100 | **Un fichier entre, devient servable, et se rattache.** Les trois écrans qui téléversent ont enfin une API ; l'obligation que B3 avait laissée est refermée |
| **2** | T001–T151 | **Les rappels partent, une fois et une seule, et l'organisation lit ce qui part.** Les écarts n° 34 et n° 108 sont refermés |
| **3** | T001–T198 | Les neuf histoires : modèles administrables, préférences, délivrabilité, purge |

**Le jalon 1 est le MVP.** Il se démontre en trois minutes : déposer une image, la voir s'afficher, enregistrer les trois visuels d'une édition, et relire sa fiche par la route de B3.

### Ordre recommandé

1. Phases 1 et 2 — **T003 d'abord**, sans quoi rien ne compile.
2. Phase 3 → 4 → 5 : **jalon 1**, arrêt et démonstration.
3. Phase 6 → 7 → 8 : **jalon 2**, arrêt et démonstration.
4. Phases 9, 10, 11 : dans n'importe quel ordre, elles ne se touchent pas.
5. Phase 12 : polissage, `make check`, progression.

---

## Notes

- **[P]** = fichiers différents, aucune dépendance en cours.
- Les tests d'intégration tournent sur base **réelle et jetable**, stockage **sur fichiers** ; le stockage S3 réel s'éprouve à la main en T207.
- **Aucune tâche n'émet d'événement ni ne met en file** pour le dépôt d'un objet ou la matérialisation des rappels : la base fait les deux.
- Commit après chaque tâche ou groupe logique ; s'arrêter à un point de contrôle pour valider une histoire seule.
- **209 tâches, dont 73 de test** et **huit vérifications mécaniques** — arêtes de crates, écriture hors schéma, émission et mise en file, taille des fichiers, comptes de routes et d'opérations.
- Répartition : amorçage 10, fondations 39, US1 18, US2 15, US3 18, US6 15, US4 15, US5 21, US7 14, US8 19, US9 14, polissage 11.

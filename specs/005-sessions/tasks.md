---

description: "Task list — Sessions (B5)"
---

# Tasks: Sessions (B5)

**Input**: Design documents from `/specs/005-sessions/`

**Prerequisites**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: **OBLIGATOIRES.** Le principe X de la constitution impose des tests d'intégration sur base réelle et jetable, sans aucun mock de base. Les tâches de test ne sont pas optionnelles ici.

**Organization**: par histoire utilisateur, pour que chaque tranche soit implémentable, éprouvable et livrable seule.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichier différent, aucune dépendance en cours
- **[Story]** : `US1` … `US9`, correspondant aux histoires de `spec.md`

## Conventions de chemin

Tout vit dans `backend/`, workspace Cargo symétrique de `frontend/`. **Aucun crate n'est créé** : `backend/crates/modules/programme` existe depuis B4 et porte tout le schéma. Seule exception documentaire : `docs/progression/`.

---

## ⚠️ Cinq choses à lire avant de commencer

### 1. Les DEUX déclencheurs du fichier émettent déjà — ne rien émettre

`programme.tg_sessions_emit_events()` émet à la création d'une séance, à chaque changement d'état et à chaque report. `programme.tg_registrations_emit_events()` émet à la création d'une inscription et à chaque changement d'état. **Quatorze types à eux deux**, tous dans la transaction.

C'est le piège de B1 (`anonymize_person()`), de B2 (`merge_organizations()`) et de B4 (`tg_guard_proposal_status()`) — cette fois en double. Un service qui émettrait à son tour produirait **deux** courriels de confirmation et **deux** jeux de rappels par inscription, et **le doublon ne se verrait qu'en production**.

**Aucune tâche de ce découpage n'appelle `kernel::events::emit`.** **T041** compte les lignes d'outbox après une acceptation produisant trois séances et exige **trois**, pas six ; **T125** en compte **une** après une inscription. Et **T149** est un `grep` qui vaut un test : `grep -rn 'events::emit' src/` doit ne rien rendre.

### 2. La journée de rattachement ne se recalcule PAS quand on déplace une séance

`tg_sessions_derive_fields()` ne déduit la journée que lorsqu'elle est **nulle**. Une séance déjà rattachée qu'on déplace du 12 au 14 novembre **reste rattachée au 12**, en silence — et déplacer est le geste le plus fréquent de tout l'écran (écart n° 113).

**T063** met la journée à nul quand le créneau change et qu'aucune journée n'est fournie, pour que le déclencheur la redéduise. **T072** le mesure en relisant `event_days.day_date` en base, pas en croyant la réponse.

Sans T063, la programmation publique et le calendrier du back-office rangent la séance au mauvais jour, **et rien ne le signale**.

### 3. Le déclencheur d'inscription ne valide RIEN quand la séance n'a pas de formulaire ATTACHÉ

Le contrôle des réponses obligatoires est gardé par `IF v_session.registration_form_id IS NOT NULL`. Or le formulaire **applicable** peut venir de l'édition ou de la plateforme — c'est le cas le plus courant, et c'est celui du jeu de données (écart n° 114).

Une inscription **sans aucune réponse obligatoire** passerait donc, alors que l'écran en aura posé quatre. **T113** résout le formulaire — séance, puis édition, puis plateforme — et **T126** éprouve précisément ce cas : une séance **sans formulaire attaché**, une inscription **sans le pays**, un refus attendu. Si elle passe, ni la base ni le service n'ont vérifié.

### 4. Le contrôle de jauge de la base ne tient pas sous concurrence

`tg_validate_registration()` exécute un `count(*)` **sans verrou**. Sous `READ COMMITTED`, deux inscriptions simultanées lisent toutes deux neuf places prises sur dix et passent toutes deux. La position en liste d'attente souffre du même défaut — `max(...) + 1` — et **aucun index unique ne la protège** (écart n° 124).

**T116** prend la ligne de la séance en verrou avant toute écriture d'inscription. **T131** le mesure : cent inscriptions concurrentes sur dix places, et une requête qui cherche deux personnes au même rang d'attente — elle doit rendre **zéro ligne**.

Onze inscrits sur dix places ne se verraient **que le jour de l'activité** ; le doublon de rang, jamais.

### 5. Le canal de diffusion EST saisissable — l'écart n° 7 se trompe sur ce point

Le prompt recopie l'écart n° 7 et demande de refuser trois colonnes à l'écriture, dont le canal. **Le déclencheur ne le pose que lorsqu'il est nul** (`IF NEW.is_streamed AND NEW.broadcast_channel_id IS NULL`) : il complète, il n'écrase jamais. Le contrat du front le porte, et son commentaire dit pourquoi — « l'écran laisse le choix quand l'édition en a plusieurs ».

Le refuser **casserait une fonctionnalité livrée** du planificateur. **T086** l'accepte ; **T090** éprouve qu'un canal choisi est bien retenu, et non remplacé par le canal par défaut.

Ce que l'écart veut empêcher existe pourtant, à un seul endroit : la branche `ELSIF NOT NEW.is_streamed` **efface** le canal. **T087** refuse donc un canal désigné quand la diffusion est retirée, et **T091** l'éprouve.

---

## 🔓 Un point resté ouvert, et l'hypothèse tenue en attendant

**Que devient la séance d'un dossier annulé après acceptation ?** La question est posée aux [points bloqués](../../docs/progression/points-bloques.md) et n'a pas reçu de réponse. L'option recommandée — annuler la séance avec le même motif — **n'est pas implémentée par ce découpage** : elle appartiendrait à US1, et l'écrire avant l'arbitrage reviendrait à trancher à la place du commanditaire.

En attendant, **rien ne se propage** : une annulation de dossier laisse la séance intacte, exactement comme une correction de dossier (règle de B4). Si le commanditaire retient l'option A, **une seule fonction s'ajoute** dans `service/birth.rs`, appelée depuis le même hameçon que la naissance — et rien d'autre ne bouge.

---

## Phase 1 — Mise en place

**Objectif** : le noyau porte ce que le module va lui demander, et le harnais sait fabriquer une édition complète.

- [X] T001 Déclarer `regex` dans `[workspace.dependencies]` de `backend/Cargo.toml` et l'ajouter aux dépendances de `backend/crates/modules/programme/Cargo.toml`, avec le commentaire disant qu'elle est déjà dans l'arbre en transitive (research.md § R27)
- [X] T002 Ajouter le réglage `PRIVACY_POLICY_VERSION` (défaut `2026-01`) à `backend/crates/kernel/src/config.rs` et à `.env.example`, sur le patron des réglages d'exploitation de B1
- [X] T003 Ajouter les **huit** codes d'erreur au catalogue de `backend/crates/kernel/src/error.rs` : `SESSION_DERIVED_FIELD`, `SESSION_UNKNOWN_REFERENCE`, `SESSION_TRACK_EVENT_MISMATCH`, `REGISTRATION_NOT_ACCEPTED`, `REGISTRATION_ANSWER_INVALID`, `REGISTRATION_CONSENT_REQUIRED`, `REGISTRATION_ACCOUNT_REQUIRED`, `REGISTRATION_LOCKED` — statuts et messages français de [contracts/errors.md](contracts/errors.md)
- [X] T004 Ajouter `ConsumerRegistry::register_all(impl IntoIterator<Item = Arc<dyn EventConsumer>>)` à `backend/crates/kernel/src/events.rs`, par symétrie exacte avec `JobRegistry::register_all` (research.md § R13)
- [X] T005 [P] Ajouter `SessionId`, `RegistrationId`, `RoomId`, `TrackId`, `ChannelId`, `EventDayId`, `FormId` à `backend/crates/modules/programme/src/domain/ids.rs`
- [X] T006 [P] Étendre `backend/crates/modules/programme/tests/commun/mod.rs` : fabrique d'une **édition complète** — fuseau, période, jours, une salle **physique** et une salle **virtuelle**, un canal de diffusion **par défaut**, un appel ouvert avec ses bornes
- [X] T007 [P] Étendre `tests/commun/mod.rs` : fabrique d'un **dossier prêt à être retenu** — organisation vérifiée, personne membre active, dossier déposé et passé en évaluation, avec intervenants, co-organisations et thématiques
- [X] T008 Vérifier `cargo fmt --check`, `cargo clippy -D warnings` et `cargo test --workspace` au vert avant d'aller plus loin

**Point de contrôle** : le noyau compile avec ses huit codes, et le harnais sait poser une COP31 jouable.

---

## Phase 2 — Fondations (préalable bloquant)

**Objectif** : tout ce que les neuf histoires partagent. **⚠️ Aucune histoire ne peut commencer avant la fin de cette phase.**

### Le domaine — types purs, aucune requête

- [X] T009 [P] Créer `src/domain/sessions.rs` : les formes de séance rendues au planificateur et à l'écran public, l'état d'une séance, et le décompte à trois nombres
- [X] T010 [P] Créer `src/domain/derived.rs` : les **quatre** champs dérivés et leur régime — refus nommé pour l'intervalle et l'exclusivité de salle, acceptation du canal, refus du canal quand la diffusion est retirée, journée facultative (research.md § R8, data-model § 2)
- [X] T011 [P] Créer `src/domain/birth.rs` : le créneau d'une séance naissante — créneau souhaité, repli sur le premier jour de l'édition à l'heure d'ouverture de l'appel, repli sans appel, durée et fin, rangs d'occurrence (research.md § R4, R5)
- [X] T012 [P] Créer `src/domain/registration.rs` : les six issues d'une tentative d'inscription et ce qui les décide
- [X] T013 Créer `src/domain/answers.rs` : la validation dynamique — **présence** (le vide vaut absence, comme le déclencheur) et **types** des onze natures de champ, dont le code ISO à deux lettres pour un pays (research.md § R15, R17, R18)
- [X] T014 Compléter `src/domain/answers.rs` : appartenance aux **options** — liste explicite ou codes de taxonomie déjà résolus — et **cinq** règles de saisie (`minLength`, `maxLength`, `pattern`, `min`, `max`), toute autre clé étant **ignorée avec une trace** (research.md § R16)
- [X] T015 Compléter `src/domain/answers.rs` : refus des **clés inconnues**, et refus d'un choix multiple obligatoire **vide** — divergence stricte assumée avec la base (research.md § R17)
- [X] T016 [P] Écrire les tests unitaires de `src/domain/answers.rs` : les **six** familles de refus, chacune nommant son champ, plus une règle de saisie inconnue ignorée et un motif invalide ignoré
- [X] T017 [P] Écrire les tests unitaires de `src/domain/birth.rs` et `src/domain/derived.rs`, sans base

### Les lectures hors schéma — réunies, comme en B4

- [X] T018 Étendre `src/repo/cross/mod.rs` : ajouter `starts_at` et `programme_published_at` à la lecture d'édition existante, sans en changer la signature d'appel
- [X] T019 [P] Ajouter à `src/repo/cross/mod.rs` la lecture des **salles** d'une édition — nom, code, jauge, caractère virtuel, diffusion, ordre
- [X] T020 [P] Ajouter à `src/repo/cross/mod.rs` la lecture des **jours** d'une édition
- [X] T021 [P] Ajouter à `src/repo/cross/mod.rs` la lecture des **fils de programmation** d'une édition
- [X] T022 [P] Ajouter à `src/repo/cross/mod.rs` la lecture des **canaux de diffusion** applicables à une édition, plateforme comprise
- [X] T023 [P] Ajouter à `src/repo/cross/mod.rs` la lecture des **codes ISO des pays** et des **codes d'une taxonomie**, pour la validation des réponses
- [X] T024 Mettre à jour l'en-tête de `src/repo/cross/mod.rs` : le tableau des lectures autorisées passe de onze à seize, et la phrase « aucune ligne de ce fichier n'écrit » reste vraie

### Les écritures hors schéma — trois fichiers, et pas un de plus

- [X] T025 Étendre `src/repo/themes.rs` : une **seconde entité**, `sessions`, le triplet restant écrit **littéralement** et jamais reçu ; mettre à jour l'en-tête
- [X] T026 Créer `src/repo/consents.rs` — **écriture hors schéma n° 3**, bornée : une ligne de consentement, une seule finalité, jamais rien d'autre. L'en-tête dit pourquoi un contrat d'événement ne conviendrait pas (research.md § R22)

### Les séances, l'ascendance et le montage

- [X] T027 Créer `src/repo/sessions.rs` : lecture d'une séance et **résolution de son édition** — la seule question posée avant toute vérification de périmètre
- [X] T028 Étendre `src/service/perimeter.rs` : accès à une séance et à une inscription, sur le patron des trois niveaux de B4 — **résoudre l'ascendance, PUIS vérifier le périmètre, PUIS agir**
- [X] T029 Étendre `src/lib.rs` : `planner_routes()` (sans préfixe), `session_routes()`, `registration_routes()`, `public_schedule_routes()`, `event_consumers()` — avec le commentaire sur les chemins littéraux avant les chemins paramétrés
- [X] T030 Modifier `backend/crates/api/src/lib.rs` : composer `/admin/planner` **à partir des deux modules**, sur le patron de `/people` et `/organizations` ; monter les scopes `/sessions` et `/registrations` et les routes publiques
- [X] T031 Vérifier que `backend/crates/api/tests/` prouve toujours que les **deux routes de B3** sous `/admin/planner` répondent — c'est ce test qui verrait une route devenue muette
- [X] T032 Étendre `src/routes/openapi.rs` : deux étiquettes nouvelles, « Planificateur » et « Inscriptions » ; les chemins s'ajouteront **au fil des histoires**, jamais d'avance
- [X] T033 Vérifier `make check-back` au vert et `cargo tree -p programme | grep -E 'identity|org|event'` sans résultat

**Point de contrôle** : le crate compile, ne dépend de personne, et les scopes sont montés — vides mais atteignables.

---

## Phase 3 — US1 : une activité retenue devient une séance à placer (P1)

**Objectif** : retenir un dossier fait apparaître ses séances, sans qu'on ait rien ressaisi. C'est l'écart n° 57, ouvert depuis le 18/08.

**Test indépendant** : retenir un dossier demandant une occurrence, puis un demandant trois ; constater une séance puis trois, sans salle, au créneau souhaité, avec le même nombre d'intervenants que le dossier — puis retenir une seconde fois et constater qu'aucune séance de plus n'apparaît.

- [X] T034 [US1] Écrire dans `src/repo/sessions.rs` l'insertion d'une séance, **`ON CONFLICT (proposal_id, sequence_number) DO NOTHING`** — l'idempotence tombe de la contrainte, jamais d'un décompte préalable (research.md § R6)
- [X] T035 [P] [US1] Créer `src/repo/session_parts.rs` : recopie des **intervenants** du dossier — personne, rôle, fonction et organisation déclarées, notice, confirmation, ordre
- [X] T036 [US1] Compléter `src/repo/session_parts.rs` : recopie des **co-organisations** autres que le porteur — la ligne du porteur est posée par déclencheur et **n'est jamais écrite par le service**
- [X] T037 [US1] Créer `src/service/birth.rs` : composer les séances d'un dossier retenu — titre, résumé, format, fuseau, créneau et durée par `domain/birth.rs`, adresse d'URL dérivée par `domain/slug.rs` avec suffixe de rang
- [X] T038 [US1] Compléter `src/service/birth.rs` : recopier intervenants, co-organisations et **thématiques** (par `repo/themes.rs`), dans la même transaction
- [X] T039 [US1] Modifier `src/service/transition.rs` : appeler `service::birth` **dans la transaction**, quand l'état d'arrivée est « retenu ». Écrire au-dessus de l'appel pourquoi il n'y a **qu'un** hameçon — les deux chemins d'acceptation passent ici, et la reprise v1 n'écrit pas l'état (research.md § R3)
- [X] T040 [P] [US1] Test `tests/naissance.rs` : un dossier à une occurrence produit **une** séance — état « pressenti », **sans salle**, non publiée, au créneau souhaité, dans le fuseau de l'édition
- [X] T041 [US1] Test `tests/naissance.rs` : un dossier à **trois** occurrences produit trois séances, rangs 1 à 3, trois adresses distinctes — **et exactement trois lignes d'outbox**, pas six
- [X] T042 [P] [US1] Test `tests/naissance.rs` : les séances portent le **même nombre** d'intervenants, de co-organisations et de thématiques que le dossier, relus en base
- [X] T043 [P] [US1] Test `tests/naissance.rs` : un dossier **sans créneau souhaité** produit une séance datée du premier jour de l'édition à l'heure d'ouverture de l'appel, **relue dans le fuseau de l'édition**
- [X] T044 [P] [US1] Test `tests/naissance.rs` : un dossier **sans durée** prend la durée par défaut de l'appel ; un dossier **sans appel** prend le début de l'édition et soixante minutes
- [X] T045 [US1] Test `tests/naissance.rs` : une acceptation **rejouée** — remise en évaluation puis nouvelle acceptation — ne crée **aucune** séance de plus
- [X] T046 [P] [US1] Test `tests/naissance.rs` : une **action groupée** retenant douze dossiers crée les séances de chacun, et un dossier écarté n'empêche pas les autres
- [X] T047 [P] [US1] Test `tests/naissance.rs` : **la reprise v1 ne crée aucune séance** — elle n'écrit pas l'état, donc l'hameçon ne se déclenche pas
- [X] T048 [US1] Test `tests/naissance.rs` : **corriger un dossier retenu ne touche aucune séance** — titre, créneau, format et durée modifiés, puis la séance relue **champ par champ**. C'est la garantie de B4, et ce module ne l'affaiblit pas

**Point de contrôle** : le panneau « à placer » a de quoi montrer.

---

## Phase 4 — US2 : le planificateur voit tout son écran en une réponse (P1)

**Objectif** : jours, salles, journées spéciales, canaux, séances placées, séances à placer et **conflits**, ensemble.

**Test indépendant** : charger l'écran avec un compte détaché sur une seule édition, vérifier que les six listes et les conflits arrivent ensemble, puis forger l'identifiant d'une autre édition et obtenir le même refus qu'avec un identifiant inexistant.

- [X] T049 [US2] Créer `src/repo/planner.rs` : la lecture des séances du planificateur — salle jointe, organisation, sigle et **code pays**, journées spéciales, thématiques avec libellé et couleur, nombre d'intervenants
- [X] T050 [US2] Compléter `src/repo/planner.rs` : les champs venus du dossier — numéro, note consolidée, durée souhaitée, créneau souhaité, contraintes de programmation
- [X] T051 [P] [US2] Créer `src/repo/conflicts.rs` : `programme.detect_conflicts()`, **telle quelle**, sans filtrer ni requalifier les gravités ; l'intervalle est lu en `text`
- [X] T052 [US2] Créer `src/service/planner.rs` : composer l'écran **dans une transaction en lecture seule, sur une connexion** — l'en-tête dit pourquoi les conflits ne peuvent pas être un second appel (research.md § R10)
- [X] T053 [US2] Créer `src/routes/planner.rs` : `GET /admin/planner`, **sans écrire son préfixe**, gardée par la permission de planifier et bornée par le périmètre
- [X] T054 [P] [US2] Créer `src/routes/sessions.rs` : le scope `/sessions` avec `GET ""` et `GET /conflicts` — **chemins littéraux avant chemins paramétrés**
- [X] T055 [P] [US2] Compléter `src/routes/sessions.rs` : `GET /{id}/speakers`, `GET /{id}/organizations`, `GET /{id}/tracks`, et la lecture correspondante dans `src/repo/session_parts.rs`
- [X] T056 [P] [US2] Test `tests/planificateur.rs` : la réponse porte l'édition, son fuseau, le libellé de sa ville, la date de publication, les jours, les salles, les fils, les canaux, les placées, les à-placer **et les conflits**
- [X] T057 [P] [US2] Test `tests/planificateur.rs` : une séance **sans salle** est dans « à placer » et jamais dans « placées », et réciproquement
- [X] T058 [P] [US2] Test `tests/planificateur.rs` : une séance née d'un dossier porte son numéro, sa note, sa durée souhaitée, son créneau souhaité et ses contraintes — **sans requête supplémentaire**
- [X] T059 [US2] Test `tests/perimetre_seances.rs` : les **trois** cas du périmètre restent distincts — global, éditions listées, et **aucun droit → refus explicite**, jamais une grille vide
- [X] T060 [US2] Test `tests/perimetre_seances.rs` : six identifiants forgés — dont quatre désignant des objets bien réels d'une autre édition — mènent **tous** au même refus qu'un identifiant inexistant
- [X] T061 [P] [US2] Test `tests/planificateur.rs` : une édition **sans aucune séance** répond avec ses listes vides, jamais par une erreur
- [X] T062 [P] [US2] Test `tests/planificateur.rs` : les lectures séparées — liste des séances, conflits — rendent **exactement** ce que l'écran porte

**Point de contrôle** : l'écran d'arbitrage se charge, complet et borné.

---

## Phase 5 — US3 : placer, déplacer, redimensionner, retirer (P1)

**Objectif** : une seule écriture pour les quatre gestes, **jamais refusée pour chevauchement**.

**Test indépendant** : poser une séance sur un créneau déjà occupé par une autre séance de la même édition en salle physique ; l'écriture réussit et la réponse porte le conflit en gravité haute.

- [X] T063 [US3] Écrire dans `src/repo/sessions.rs` l'écriture du créneau — salle, début, fin —, **avec la mise à nul de la journée de rattachement** quand le créneau change et qu'aucune journée n'est fournie. Le commentaire dit pourquoi (écart n° 113, research.md § R9)
- [X] T064 [US3] Compléter `src/service/planner.rs` : refuser les champs dérivés par `domain/derived.rs` **avant** d'écrire, et vérifier que la salle désignée appartient bien à l'édition de la séance
- [X] T065 [US3] Compléter `src/service/planner.rs` : composer la réponse — la séance **et les conflits de toute l'édition**, lus **dans la transaction, après l'écriture** (research.md § R11)
- [X] T066 [US3] Ajouter `PUT /sessions/{id}/schedule` à `src/routes/sessions.rs`, gardée par la permission de planifier **sur l'édition de la séance**, résolue en base
- [X] T067 [US3] Test `tests/placement.rs` : **deux séances de la même édition en salle physique sur des créneaux qui se recouvrent** — l'écriture aboutit, et la réponse porte le conflit de stand unique en gravité bloquante
- [X] T068 [P] [US3] Test `tests/placement.rs` : une séance en **salle virtuelle** face à une séance physique simultanée ne produit **aucun** conflit de stand
- [X] T069 [P] [US3] Test `tests/placement.rs` : une séance **sans salle** ne produit aucun conflit
- [X] T070 [P] [US3] Test `tests/placement.rs` : deux séances dans la **même** salle physique remontent **une seule fois**, par le conflit qui nomme la salle
- [X] T071 [US3] Test `tests/placement.rs` : retirer la salle renvoie la séance au panneau — elle **existe toujours**, son créneau est intact, et rien n'est supprimé
- [X] T072 [US3] Test `tests/placement.rs` : une séance déplacée du 12 au 14 novembre est **rattachée au 14**, vérifié en relisant `event.event_days.day_date` **en base**
- [X] T073 [P] [US3] Test `tests/placement.rs` : une journée **explicitement fournie** est retenue telle quelle
- [X] T074 [P] [US3] Test `tests/placement.rs` : envoyer l'intervalle dérivé, puis l'exclusivité de salle — **deux refus 422 nommant le champ**, et la séance inchangée après chacun
- [X] T075 [P] [US3] Test `tests/placement.rs` : une fin antérieure ou égale au début est refusée sur son champ, en français ; une salle d'une autre édition est refusée
- [X] T076 [US3] Test `tests/placement.rs` : la réponse d'une écriture porte les conflits **de toute l'édition** — un déplacement résout le conflit d'un bloc situé un autre jour, et le bandeau le montre
- [X] T077 [P] [US3] Test `tests/placement.rs` : un intervenant attendu à deux endroits et une organisation programmée deux fois remontent en **avertissement**, jamais en gravité bloquante
- [X] T078 [US3] Annoter les chemins de US2 et US3 dans `src/routes/openapi.rs`

### 🏁 **JALON 1 — T001 à T078 : le planificateur a de quoi placer, et il place**

À ce point, l'écart n° 57 est refermé : retenir un dossier fait naître ses séances, l'écran d'arbitrage se charge en une réponse, et l'équipe déplace ses blocs sans qu'aucun chevauchement ne soit refusé.

---

## Phase 6 — US4 : les journées spéciales sont composées à la main (P2)

**Objectif** : rattacher une séance à des fils, sans jamais le déduire d'une date.

> **Pourquoi ici plutôt qu'après les histoires P1** : cette écriture et la suivante vivent dans `src/service/planner.rs` et rendent la **même** forme de réponse que US3. Les traiter à part rouvrirait deux fois le même fichier, et donnerait deux occasions de diverger sur la composition des conflits — ce que le contrat du front interdit précisément.

**Test indépendant** : rattacher une séance à deux fils, en retirer un, vérifier que la base retient qui a rattaché quoi ; puis tenter un fil d'une autre édition et obtenir un refus nommé.

- [X] T079 [US4] Écrire dans `src/repo/session_parts.rs` le **remplacement** de la liste des rattachements — ce qui n'y figure plus est détaché, l'acteur venant de la session appelante
- [X] T080 [US4] Compléter `src/service/planner.rs` : traduire le refus de `tg_check_session_track_event()` en `SESSION_TRACK_EVENT_MISMATCH`, **par le code d'erreur et jamais par le texte du message**
- [X] T081 [US4] Ajouter `PUT /sessions/{id}/tracks` à `src/routes/sessions.rs`, rendant la même forme que le placement
- [X] T082 [P] [US4] Test `tests/fils.rs` : la liste envoyée **remplace** la précédente, et `added_by` est relu en base
- [X] T083 [P] [US4] Test `tests/fils.rs` : un fil d'une **autre édition** est refusé avec un code stable et un message français, jamais l'exception brute
- [X] T084 [P] [US4] Test `tests/fils.rs` : la même liste envoyée deux fois laisse le même état, sans doublon
- [X] T085 [P] [US4] Test `tests/fils.rs` : la réponse porte la séance **et** les conflits de l'édition, comme les autres écritures

---

## Phase 7 — US5 : la diffusion, et la règle « un seul direct » (P2)

**Objectif** : marquer une séance diffusée, avec ou sans canal choisi — et laisser deux directs simultanés s'écrire.

**Test indépendant** : marquer deux séances diffusées sur le même créneau sans choisir de canal ; le canal par défaut est posé sur les deux et le conflit remonte en gravité haute.

- [X] T086 [US5] Écrire dans `src/repo/sessions.rs` l'écriture de la diffusion — **le canal désigné est écrit tel quel**, et le canal par défaut est laissé au déclencheur (research.md § R8)
- [X] T087 [US5] Compléter `src/service/planner.rs` : refuser un canal désigné **quand la diffusion est retirée** (`SESSION_DERIVED_FIELD`, champ nommé), et refuser un canal inexistant, désactivé, ou n'appartenant ni à l'édition ni à la plateforme
- [X] T088 [US5] Ajouter `PUT /sessions/{id}/broadcast` à `src/routes/sessions.rs`
- [X] T089 [P] [US5] Test `tests/diffusion.rs` : diffusion activée **sans** canal → le canal par défaut de l'édition est posé, à défaut celui de la plateforme
- [X] T090 [US5] Test `tests/diffusion.rs` : diffusion activée **avec** un canal choisi → **c'est ce canal**, relu en base, et non celui par défaut
- [X] T091 [P] [US5] Test `tests/diffusion.rs` : retirer la diffusion **efface** le canal ; retirer la diffusion **en désignant un canal** est refusé sur ce champ
- [X] T092 [P] [US5] Test `tests/diffusion.rs` : un canal d'une autre édition, ou désactivé, est refusé en le disant
- [X] T093 [US5] Test `tests/diffusion.rs` : **deux séances de deux éditions différentes** diffusées sur le même canal au même moment — les deux écritures aboutissent, et le conflit de diffusion remonte en gravité bloquante **depuis l'une comme depuis l'autre**
- [X] T094 [US5] Annoter les chemins de US4 et US5 dans `src/routes/openapi.rs`

### 🏁 **JALON 2 — T001 à T094 : l'écran d'arbitrage est complet**

Placer, déplacer, rattacher, diffuser, et voir tous les conflits. Il reste à rendre le programme public.

---

## Phase 8 — US7 : le public lit le programme (P1)

**Objectif** : la programmation d'une édition, en une requête, sans session.

**Test indépendant** : lire la programmation d'une édition publiée **sans aucune session ouverte**, et vérifier que seules les séances publiées y figurent, avec leur état temporel et leurs thématiques venues de la base.

- [X] T095 [US7] Créer `src/repo/public_schedule.rs` : `programme.v_public_schedule`, **telle quelle**, chaque colonne annotée pour la nullité — une vue ne porte aucune contrainte, et le vérificateur le suppose (leçon de B3)
- [X] T096 [US7] Compléter `src/repo/public_schedule.rs` : le détail d'une séance **publiée** par son adresse d'URL dans son édition, avec ses intervenants et ses organisations
- [X] T097 [US7] Créer `src/service/public_schedule.rs` et `src/routes/public_schedule.rs` : `GET /schedule` et `GET /events/{event_id}/sessions/{slug}`, **sans exiger de session**
- [X] T098 [P] [US7] Test `tests/programmation_publique.rs` : la lecture répond **sans session**, et ne porte que des séances publiées
- [X] T099 [P] [US7] Test `tests/programmation_publique.rs` : chaque ligne porte sa salle, son organisation, son sigle, son pays, ses journées spéciales, ses thématiques **avec libellé et couleur**, sa couverture et son état temporel
- [X] T100 [US7] Test `tests/programmation_publique.rs` : la couverture se replie sur celle du **dossier d'origine** quand la séance n'en a pas — le repli est la règle, pas une commodité
- [X] T101 [P] [US7] Test `tests/programmation_publique.rs` : une adresse d'URL inconnue et une séance **non publiée** rendent **le même** refus
- [X] T102 [P] [US7] Test `tests/programmation_publique.rs` : une édition dont le programme n'est pas publié rend une réponse **vide**, jamais une erreur

---

## Phase 9 — US6 : la programmation devient réellement publique (P1)

**Objectif** : consommer l'annonce de B3 et rendre publiques exactement les séances désignées. C'est l'obligation inscrite aux points bloqués le 20/08.

**Test indépendant** : publier une édition portant trois séances éligibles, comparer le nombre annoncé au nombre devenu public, puis rejouer l'annonce et constater qu'aucune séance de plus n'est publiée.

- [X] T103 [US6] Créer `src/service/publication.rs` : l'ordre unique qui pose la date **et** fait passer « pressenti » à « programmé », avec le **prédicat porté par l'annonce** et pas un autre. L'en-tête cite les trois preuves qui demandent le changement d'état (research.md § R12)
- [X] T104 [US6] Créer `src/consumers/publication.rs` : l'implémentation d'`EventConsumer` — son nom est celui inscrit au registre d'entrée et **ne se renomme pas à la légère** ; il n'écrit **jamais** la date de publication de l'édition
- [X] T105 [US6] Compléter `src/lib.rs` : `event_consumers()`, et modifier `backend/crates/worker/src/main.rs` pour l'enregistrer par `register_all`
- [X] T106 [P] [US6] Test `tests/publication.rs` : les séances de l'édition, aux **états portés par l'annonce**, **non encore publiques**, reçoivent la date **de l'annonce** — et une séance d'une autre édition n'est pas touchée
- [X] T107 [US6] Test `tests/publication.rs` : les séances « pressenties » passent à « programmées », les déjà « programmées » ne bougent pas, et **le nombre d'événements émis** est celui des seules séances dont l'état a changé
- [X] T108 [US6] Test `tests/publication.rs` : la **même annonce livrée deux fois** ne publie aucune séance de plus, et le registre d'entrée porte **une** ligne
- [X] T109 [P] [US6] Test `tests/publication.rs` : le module **n'écrit pas** `event.events.programme_published_at` — relue avant et après, elle est celle de l'émetteur
- [X] T110 [US6] Test `tests/publication.rs` : **de bout en bout** — publier par la route de B3, laisser le relais travailler, et comparer `published_count` au nombre de séances devenues publiques. L'égalité est attendue ; **tout écart est mesuré et consigné**, jamais supposé nul (research.md § R14)
- [X] T111 [P] [US6] Test `tests/publication.rs` : une édition **sans aucune séance** se publie sans erreur et sans rien publier ; une séance annulée reste visible avec son état
- [X] T112 [US6] Annoter les chemins de US7 dans `src/routes/openapi.rs`

### 🏁 **JALON 3 — T001 à T112 : le programme de la COP31 est public**

C'est le jalon que le prompt demande pour tout ce qui n'est pas l'inscription. À ce point : la séance naît, l'équipe l'arbitre, la publication la rend visible, et le public la lit.

---

## Phase 10 — US8 : s'inscrire, avec un formulaire qui n'a pas été écrit en dur (P2)

**Objectif** : le formulaire vient de la base, les réponses sont validées contre **lui**, et la jauge tient.

**Test indépendant** : charger le formulaire applicable à une séance qui n'en porte aucun, s'inscrire en omettant une réponse obligatoire, puis avec une valeur hors options, puis correctement ; remplir la jauge et constater le passage en liste d'attente.

### Le formulaire

- [X] T113 [US8] Créer `src/repo/forms.rs` : résoudre le formulaire **applicable** — séance, à défaut édition, à défaut plateforme — et ses seuls champs **actifs**, triés (écart n° 114)
- [X] T114 [US8] Compléter `src/repo/forms.rs` : résoudre les options d'un champ adossé à une **taxonomie**, avec leur libellé traduit, en **une** lecture pour tout le formulaire — jamais une par champ
- [X] T115 [US8] Créer `src/routes/registrations.rs` et ajouter `GET /sessions/{id}/registration-form` à `src/routes/sessions.rs`, **lecture publique**

### L'inscription

- [X] T116 [US8] Créer `src/repo/registrations.rs` : **prendre la ligne de la séance en verrou** avant toute écriture, puis insérer. L'en-tête dit pourquoi (écart n° 124, research.md § R19)
- [X] T117 [US8] Compléter `src/repo/registrations.rs` : traduire les refus de PostgreSQL selon [contracts/errors.md](contracts/errors.md) — la valeur qui accompagne un refus est **relue en base**, jamais extraite du message français du déclencheur
- [X] T118 [US8] Créer `src/service/registration.rs` : valider les réponses contre le formulaire **résolu** par `domain/answers.rs`, avant toute écriture
- [X] T119 [US8] Compléter `src/service/registration.rs` : le **consentement** aux réponses sensibles — refus nommant le champ sans lui, écriture de la preuve par `repo/consents.rs` avec la version de politique et l'adresse d'appel
- [X] T120 [US8] Compléter `src/service/registration.rs` : l'inscription **sans compte** — identité prise dans des **champs dédiés**, personne retrouvée par son adresse ou créée par `repo/people.rs` ; refus quand le formulaire ne l'admet pas
- [X] T121 [US8] Compléter `src/service/registration.rs` : les quatre fenêtres — séance annulée, séance ne prenant pas d'inscription, avant ouverture, après clôture — **chacune avec son propre motif** (écart n° 115)
- [X] T122 [US8] Compléter `src/service/registration.rs` : l'**annulation**, et la promotion depuis la liste d'attente — **exactement le nombre de places libérées**, dans la même transaction, sous le même verrou (écart n° 116)
- [X] T123 [US8] Compléter `src/service/registration.rs` : la **première présence**, écrite une seule fois par la fonction du modèle, et les deux lectures — liste nominative et « mes inscriptions »
- [X] T124 [US8] Compléter `src/routes/registrations.rs` : `POST /sessions/{id}/registrations`, `GET /registrations`, `GET /registrations/mine`, `POST /registrations/{id}/cancel`, `POST /registrations/{id}/join` — **littéraux avant paramétrés**

### Les épreuves

- [X] T125 [US8] Test `tests/inscription.rs` : une inscription écrit **une** ligne d'outbox, pas deux — le service n'émet rien
- [X] T126 [US8] Test `tests/formulaire.rs` : sur une séance **sans formulaire attaché**, le formulaire de l'édition est rendu, ses champs **inactifs** absents, ses options de taxonomie résolues — et **une inscription sans le pays est refusée** (écart n° 114)
- [X] T127 [P] [US8] Test `tests/formulaire.rs` : les **six** familles de refus — obligatoire absent, type incompatible, hors options, hors bornes, clé inconnue, choix multiple vide — chacune nommant son champ
- [X] T128 [P] [US8] Test `tests/formulaire.rs` : une réponse « pays » en toutes lettres est refusée, le code ISO à deux lettres accepté, un code inconnu refusé
- [X] T129 [US8] Test `tests/consentement.rs` : une réponse à un champ sensible **sans consentement** est refusée en nommant le champ ; avec consentement, l'inscription aboutit et la preuve est **relue** dans le registre des consentements
- [X] T130 [P] [US8] Test `tests/inscription.rs` : jauge atteinte **avec** liste d'attente → position suivante, sans trou ; **sans** liste d'attente → refus portant le nombre de places, **relu sur la séance**
- [X] T131 [US8] Test `tests/inscription_concurrente.rs` : **cent inscriptions concurrentes sur dix places** → exactement dix confirmées, et **aucun** rang d'attente en double (écart n° 124)
- [X] T132 [US8] Test `tests/inscription.rs` : annuler une inscription **confirmée** promeut **exactement une** personne et ne dépasse jamais la jauge ; annuler une inscription **en attente** ne promeut personne
- [X] T133 [P] [US8] Test `tests/inscription.rs` : une seconde inscription vivante est refusée ; une **réinscription après annulation** est acceptée
- [X] T134 [P] [US8] Test `tests/inscription.rs` : les **quatre** fenêtres rendent quatre motifs distincts
- [X] T135 [US8] Test `tests/inscription.rs` : l'inscription **sans compte** aboutit quand le formulaire l'admet — la personne est créée **sans compte**, retrouvée par son adresse à la seconde inscription — et est refusée quand il ne l'admet pas
- [X] T136 [P] [US8] Test `tests/inscription.rs` : la première présence est écrite **une seule fois** ; « mes inscriptions » ne rend que les siennes
- [X] T137 [US8] Test `tests/inscription.rs` : la liste **nominative** exige la permission de gérer les inscriptions **sur l'édition** ; un compte n'ayant que celle de planifier est refusé (écart n° 119), et un compte hors périmètre reçoit le refus d'une séance inexistante
- [X] T138 [P] [US8] Test `tests/inscription.rs` : **l'annulation d'une inscription à une séance annulée est refusée avec un code nommé**, jamais par un 500 — c'est l'écart n° 125, éprouvé plutôt que découvert
- [X] T139 [US8] Annoter les chemins de US8 dans `src/routes/openapi.rs`

---

## Phase 11 — US9 : l'organisation sait combien de personnes viendront, jamais qui (P2)

**Objectif** : refermer les écarts n° 36 et n° 108.

**Test indépendant** : charger le dossier d'une organisation portant une séance à quarante inscrits, **balayer la charge utile entière** à la recherche d'un nom, d'une adresse ou d'une réponse — n'en trouver aucun.

- [X] T140 [US9] Écrire dans `src/repo/sessions.rs` les séances d'un dossier avec leur salle et **trois nombres** — confirmées (`registered` **et** `attended`, le même prédicat que la vue publique), en attente, jauge (data-model § 7)
- [X] T141 [US9] Modifier `src/service/workspace.rs` : remplir la liste des séances d'un dossier retenu ; **la liste des rappels reste vide, jamais absente** (écart n° 108)
- [X] T142 [US9] Compléter `src/service/workspace.rs` : produire l'action « **compte rendu manquant** » pour chaque séance terminée sans compte rendu, en nommant la séance
- [X] T143 [US9] Test `tests/espace_organisation_seances.rs` : les trois nombres sont exacts, relus en base, et un dossier **non retenu** porte une liste **vide**
- [X] T144 [US9] Test `tests/espace_organisation_seances.rs` : **balayage de la charge utile entière** — ni nom d'inscrit, ni adresse, ni valeur de réponse au formulaire n'y figurent, cherchés **dans la réponse sérialisée** et non champ par champ
- [X] T145 [P] [US9] Test `tests/espace_organisation_seances.rs` : une séance **terminée sans compte rendu** produit l'action correspondante ; une séance à venir n'en produit pas

---

## Phase 12 — Finition et contrôles transverses

**Objectif** : ce qui ne se déduit d'aucun test d'histoire, et que seuls des comptes écrits à deux endroits attrapent.

- [X] T146 Vérifier que `src/routes/openapi.rs` porte **dix-sept** chemins et **huit** codes ; le compte est écrit dans le test de contrôle, pas seulement dans le fichier
- [X] T147 Test `backend/crates/api/tests/routes_programme_sessions.rs` : **frapper les dix-sept routes sur la vraie application** — c'est le seul contrôle qui voit une route écrite mais non montée, et il a déjà attrapé ce défaut en B2 et en B4
- [X] T148 Test du même fichier : les **deux routes de B3** sous `/admin/planner` répondent toujours, et `/sessions/publication-readiness` rend bien **404** — un seul chemin sert cette lecture (écart n° 121)
- [X] T149 [P] Contrôle mécanique : `grep -rn 'events::emit' backend/crates/modules/programme/src/` ne rend **rien**
- [X] T150 [P] Contrôle mécanique : les fichiers écrivant hors du schéma `programme` sont **exactement trois** — `repo/themes.rs`, `repo/people.rs`, `repo/consents.rs`
- [X] T151 [P] Contrôle mécanique : aucun fichier de `backend/` ne dépasse **mille lignes** — c'est le module où la marge est la plus mince
- [X] T152 [P] Contrôle mécanique : `cargo tree -p programme | grep -E 'identity|org|event'` ne rend rien
- [X] T153 Vérifier `SELECT * FROM platform.cross_module_fk_report WHERE NOT is_compliant;` sans ligne, et qu'**aucun fichier de `docs/database/` n'a été modifié**
- [X] T154 Exécuter `make check` **en entier depuis la racine** — base détruite, schéma rechargé de zéro, mise en forme, Clippy sans avertissement, `cargo test --workspace`, site compilé
- [X] T155 Éprouver à la main les treize parcours de [quickstart.md](quickstart.md) sur l'API en fonctionnement, **worker démarré puis arrêté** — le symptôme « publié mais rien de public » doit se reproduire, puis disparaître
- [X] T156 Mettre à jour la progression : `docs/progression/journal/2026-08-2x.md`, `docs/progression/ecrans/b5-sessions.md` (livré, écarts, vérifications), `docs/progression/decisions/`, la ligne de suivi de `docs/progression/api.md` et l'état général de `docs/PROGRESSION.md`

### 🏁 **JALON 4 — T001 à T156 : le module Sessions est complet**

---

## Dépendances entre histoires

```text
Phase 1 (mise en place) ─── Phase 2 (fondations) ─── ⚠️ bloquant pour tout
                                     │
        ┌────────────────────────────┼────────────────────────────┐
        │                            │                            │
      US1 ──────────────────────── US2 ─── US3 ─── US4 ─── US5    │
   (la séance naît)            (l'écran)  (placer) (fils) (direct)│
        │                            │                            │
        └──────────── US7 (le public lit) ─── US6 (la publication) ┘
                                     │
                          US8 (inscriptions) ─── US9 (les trois nombres)
```

| Histoire | Dépend de | Pourquoi |
|---|---|---|
| **US1** | Fondations | Rien d'autre : elle crée ce que tout le reste manipule |
| **US2** | US1 | Sans séance, l'écran n'a rien à montrer — il se charge, mais ne prouve rien |
| **US3** | US2 | La réponse d'écriture porte la même forme de séance et les mêmes conflits |
| **US4**, **US5** | US3 | Même fichier de service, **même forme de réponse** : les séparer donnerait deux occasions de diverger |
| **US7** | US1 | Une séance publiée doit exister ; l'histoire se démontre en posant la date à la main |
| **US6** | US7 | C'est US7 qui prouve que la publication a eu un effet visible |
| **US8** | US1 | On s'inscrit à une séance |
| **US9** | US1, US8 | Les trois nombres se comptent sur des inscriptions réelles |

**US7 avant US6, et c'est délibéré** : la lecture publique se démontre en posant la date de publication à la main, et devient alors l'instrument de mesure de US6. L'inverse obligerait à prouver la publication sans pouvoir en constater l'effet.

---

## Parallélisation

**Dans les fondations** : T009 à T017 (le domaine, sept fichiers indépendants) et T019 à T023 (cinq lectures dans le même fichier — à sérialiser si deux personnes y travaillent).

**Dans chaque histoire** : les tâches de test marquées `[P]` visent des fichiers distincts et ne se gênent pas. Les tâches d'écriture d'un même fichier de service ne sont **jamais** marquées `[P]`.

**Entre histoires** : aucune. Le graphe ci-dessus est une chaîne, à une fourche près — US7 et US8 peuvent avancer en parallèle une fois US1 finie, sur des fichiers entièrement disjoints.

---

## Stratégie de livraison

| Jalon | Tâches | Ce qui devient possible |
|---|---|---|
| **1** | T001–T078 | Retenir un dossier fait naître ses séances ; l'équipe compose la grille et voit ses conflits |
| **2** | T001–T094 | L'écran d'arbitrage est complet : journées spéciales et diffusion comprises |
| **3** | T001–T112 | **Le programme de la COP31 est public.** C'est le jalon que le prompt demande hors inscriptions |
| **4** | T001–T156 | Le module est complet : inscriptions, décomptes, contrôles transverses |

**Le plus petit incrément qui vaille la peine d'être livré est le jalon 1.** En deçà, l'écart n° 57 reste ouvert et le planificateur n'a rien à placer — l'écran existe depuis le 18/08 et attend.

---

## Récapitulatif

| | Nombre |
|---|---|
| Tâches | **156** |
| dont tâches de test | **67** |
| dont contrôles mécaniques | **4** |
| Phases | 12 |
| Histoires | 9 |
| Jalons de livraison | 4 |
| Routes livrées | 17 |
| Codes d'erreur ajoutés | 8 |
| Événements émis par le service | **0** |
| Événements consommés | **1** — le premier du dépôt |
| Travaux différés | **0** |
| Fichiers de `docs/database/` modifiés | **0** |

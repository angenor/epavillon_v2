---

description: "Task list — Événements (B3)"
---

# Tasks: Événements (B3)

**Input**: Design documents from `/specs/003-evenements/`

**Prerequisites**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: **OBLIGATOIRES.** Le principe X de la constitution impose des tests d'intégration sur base réelle et jetable, sans aucun mock de base. Les tâches de test ne sont pas optionnelles ici.

**Organization**: par histoire utilisateur, pour que chaque tranche soit implémentable, éprouvable et livrable seule.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichier différent, aucune dépendance en cours
- **[Story]** : `US1` … `US8`, correspondant aux histoires de `spec.md`

## Conventions de chemin

Tout vit dans `backend/`, workspace Cargo symétrique de `frontend/` — emplacements imposés par le principe II. **Aucun fichier de ce jalon ne vit hors de `backend/`**, à deux exceptions documentaires : `.env.example` et `docs/progression/`.

---

## ⚠️ Trois choses à lire avant de commencer

### 1. Supprimer un critère d'évaluation DÉTRUIT les notes, et l'ordre réussit

`xmod_fk_review_scores_criterion` est `ON DELETE CASCADE`. Retirer une ligne de la grille efface toutes les notes posées sur ce critère — **sans erreur, sans trace, sans que l'écran puisse le voir**. Or ces notes sont l'argumentaire d'une décision de sélection, précisément ce que la v1 n'avait pas et qui rendait un refus inexplicable à l'organisation qui le contestait.

**T060 compte et refuse** (`EVENT_CRITERION_HAS_SCORES`), et **T067 vérifie que les notes sont toujours là après le refus**. C'est l'unique entorse au principe VIII de tout le plan (research.md § R9, écart n° 91) : partout ailleurs, le code traduit ce que la base refuse ; ici, il tient ce qu'elle ne tient pas.

### 2. Le préfixe `/admin/planner` sera partagé avec B5 — il se compose MAINTENANT

Deux `web::scope` du même préfixe **ne se complètent pas** : Actix retient le premier et rend 404 sur les routes du second. Le défaut a coûté **trois routes muettes sur vingt et une** en B2, et il a été corrigé pour `/people`.

B3 pose deux routes de publication sous `/admin/planner`, préfixe que B5 reprendra en entier. **T016 et T018** appliquent le patron dès les fondations : `lib.rs` expose `planner_routes(cfg)` **en plus** de `routes(cfg)`, et c'est `api` qui compose le scope. Cinq lignes maintenant, une enquête évitée en B5.

### 3. Trois chemins littéraux doivent être déclarés AVANT leur homologue paramétré

| À déclarer avant | Sinon capturé par | Tâche |
|---|---|---|
| `GET /events/public` | `GET /events/{slug}` | T071 |
| `GET /admin/events/form-options` | `GET /admin/events/{id}` | T038 |
| `GET /admin/calls/default-criteria` | *(déclaré d'abord par principe)* | T064 |

**T077** l'éprouve : `/events/public` doit rendre une liste, jamais un `null` d'adresse d'URL inconnue.

---

## Phase 1 — Mise en place

**Objectif** : le crate existe, il compile, il ne dépend de personne, et la configuration refuse de démarrer si un réglage est mal écrit.

- [X] T001 Créer le crate `backend/crates/modules/event` (`Cargo.toml`, `src/lib.rs`) et l'ajouter aux `members` et aux `[workspace.dependencies]` de `backend/Cargo.toml`
- [X] T002 [P] Créer l'arborescence interne vide du crate — `src/domain/`, `src/repo/`, `src/service/`, `src/jobs/`, `src/routes/`, `tests/commun/` — sur la forme qu'`identity` a inaugurée et qu'`org` a confirmée
- [X] T003 [P] Ajouter à `.env.example` le réglage `EVENT_CALL_AUTOCLOSE_INTERVAL=1h`
- [X] T004 [P] Étendre `backend/crates/kernel/src/config.rs` d'une section `EventConfig` portant cette clé, **validée au démarrage** : une durée illisible arrête le service, jamais une requête
- [X] T005 [P] Vérifier que `platform.modules` porte déjà l'entrée `event` (schéma `event`, dépendant de `org` et `identity`) et que rien n'est à semer
- [X] T006 [P] Relever ce que `900_seed.sql` fournit déjà — **quatre séries et un canal général actif et par défaut, aucune édition** — et l'inscrire en commentaire d'en-tête de `backend/crates/modules/event/tests/commun/seed.rs` (research.md § R19)
- [X] T007 Vérifier `cargo build`, `cargo fmt --check` et `cargo clippy -- -D warnings` sur le crate vide, base démarrée

---

## Phase 2 — Fondations (bloquantes)

**Objectif** : les codes d'erreur, la traduction des contraintes, les contrats d'événements, le garde d'ascendance, le montage dans les deux binaires et le harnais de test. **Aucune histoire ne peut commencer avant.**

### Erreurs et traduction

- [X] T008 Ajouter au catalogue de `backend/crates/kernel/src/error.rs` les **trois** variantes du module : `EVENT_GLOBAL_SCOPE_REQUIRED` (403), `EVENT_CRITERION_HAS_SCORES` (422), `EVENT_UNKNOWN_REFERENCE` (422), avec leur message français par défaut
- [X] T009 [P] Étendre `backend/crates/kernel/src/pg_error.rs` de la correspondance des contraintes de ce module — unicités, vérifications, clés étrangères — et **du cas des domaines**, dont le nom de contrainte est celui du domaine et non de la colonne : on se sert du **nom de type** (`PG_DIAG_DATATYPE_NAME`), qui est fiable ([`contracts/errors.md`](contracts/errors.md))

### Contrats et ossature

- [X] T010 [P] Créer `backend/crates/contracts/src/event.rs` — les **six** charges utiles d'événements, dont celle de `event.programme.published` qui porte le **prédicat exact** des séances à publier
- [X] T011 [P] Créer `backend/crates/modules/event/src/domain/ids.rs` — alias d'identifiants du module
- [X] T012 [P] Créer `src/domain/permissions.rs` — les **trois** permissions consommées, dont `programme.session.schedule` qui vient d'un autre module, avec le commentaire disant pourquoi cela ne crée aucune arête (research.md § R12)
- [X] T013 Créer `src/repo/cross.rs` avec son **en-tête énumérant les neuf lectures hors schéma autorisées** et la règle qui les gouverne, puis y écrire les **six résolutions d'ascendance** (fil, lieu, salle, canal, appel, journée → édition)
- [X] T014 Écrire dans `src/service/mod.rs` l'assistant « résoudre l'ascendance **puis** vérifier le périmètre » : l'`event_id` du corps de requête est **ignoré**, et l'absence de l'objet produit **le même refus** que l'échec du périmètre (research.md § R2)
- [X] T015 Créer `src/routes/openapi.rs` — enregistrement des chemins et engendrement des codes d'erreur depuis le catalogue du noyau, jamais écrits à la main
- [X] T016 Écrire `src/lib.rs` — `routes(cfg)`, **`planner_routes(cfg)` séparément** (research.md § R11) et `jobs()`

### Montage et travail différé

- [X] T017 Monter `event::routes` dans `backend/crates/api/src/lib.rs`, **seulement si `platform.modules` déclare le module actif**
- [X] T018 Composer `web::scope("/admin/planner")` **une seule fois** dans `backend/crates/api/src/lib.rs`, en y versant `event::planner_routes`, avec le commentaire rappelant que deux scopes du même préfixe ne se complètent pas — sur le patron de `/people` posé en B1
- [X] T019 Écrire `src/jobs/autoclose.rs` — clôture d'un appel `open` dont l'échéance effective est passée, émission de `event.call.closed`, et **replanification par lui-même**
- [X] T020 Enregistrer ce travail dans `backend/crates/worker/src/main.rs`, le démarrage ne faisant que **réarmer** la chaîne au cas où sa dernière occurrence serait morte avant d'avoir posé la suivante

### Harnais des tests

- [X] T021 [P] Écrire `tests/commun/mod.rs` — monter la **vraie application** (intergiciels compris), ouvrir une session, et fabriquer les trois comptes de portée : global, détaché sur une édition, sans aucun droit
- [X] T022 [P] Écrire `tests/commun/seed.rs` — l'édition COP31 (série climat, Belém, fuseau `America/Belem`, pavillon tenu, 9 au 20 novembre 2027) et une édition **sans pavillon**, celle qui prouve que la règle du sigle ne casse rien
- [X] T023 [P] Écrire `tests/perimetre_vide_refuse.rs` — un compte sans aucun droit d'administration reçoit un **refus explicite**, jamais une liste vide, sur chacune des lectures du back-office

### Vérification des fondations

- [X] T024 Vérifier `cargo tree -p event | grep -E 'identity|org'` → **rien**, puis `make check-back` au vert

---

## Phase 3 — US1 : Une édition existe, avec un numéro de dossier qu'on peut épeler (P1) 🎯 MVP

**Objectif** : créer et modifier une édition, avec la règle du sigle et son calendrier.

**Éprouvable seule** : créer une édition avec pavillon sans sigle → refus **avec valeur proposée** ; avec sigle → créée, avec ses douze journées **datées dans le fuseau de Belém** ; sans pavillon et sans sigle → créée.

- [X] T025 [P] [US1] Écrire `src/domain/acronym.rs` — `exiger(has_pavilion, acronym)` sur l'**état résultant** de l'écriture, et `proposer(titre_fr)` : accents dépliés, non-alphanumériques retirés, majuscules, tronqué à douze, **rien** en dessous de deux caractères
- [X] T026 [P] [US1] Tests unitaires de `src/domain/acronym.rs` — bornes 2 et 12, espace, accent, jeu de caractères, et le cas « aucune proposition possible »
- [X] T027 [P] [US1] Écrire `src/domain/calendar.rs` — `plan(premier_jour, dernier_jour, journees_existantes)`, **fonction pure**, rendant les dates à créer, les journées hors période et le nombre d'inchangées
- [X] T028 [P] [US1] Tests unitaires de `src/domain/calendar.rs` — période élargie, resserrée, identique, et période d'un an
- [X] T029 [US1] Écrire `src/repo/editions.rs` — insertion, **mise à jour TOTALE** (research.md § R13) et lecture d'une ligne de liste
- [X] T030 [US1] Écrire dans `src/repo/editions.rs` la période en dates civiles par `generate_series` sur `(starts_at AT TIME ZONE timezone)::date` — **jamais en Rust** (research.md § R5)
- [X] T031 [P] [US1] Écrire dans `src/repo/cross.rs` les décomptes joints d'une ligne — dossiers déposés (brouillons exclus), séances, séances placées, journées —, **en jointures par la gauche** pour qu'une édition à zéro dossier reste visible
- [X] T032 [US1] Écrire `src/repo/days.rs` — lire les journées d'une édition, créer celles qui manquent avec leur rang
- [X] T033 [US1] Écrire `src/service/edition_write.rs` — création et modification, **règle du sigle appliquée sur l'état résultant**, réponse portant `suggested_acronym` (champ additif)
- [X] T034 [US1] Traduire dans `src/service/edition_write.rs` les **six** contraintes nommées d'une édition en `errors: EditionFormError[]`, chacune **sur son champ** ([`contracts/errors.md`](contracts/errors.md))
- [X] T035 [US1] Créer les journées manquantes à l'enregistrement, **n'en supprimer aucune**, et mesurer `days_created` ; `days_removed` et `sessions_detached` valent toujours zéro ici
- [X] T036 [US1] Émettre `event.edition.created` et `event.edition.updated` **dans la transaction**, par l'unique porte d'écriture du noyau — aucun déclencheur ne le fait à notre place (écart n° 87)
- [X] T037 [US1] Écrire `src/routes/admin_events.rs` — `POST /admin/events` et `PUT /admin/events/{id}`
- [X] T038 [US1] Ajouter `GET /admin/events/form-options` **déclarée AVANT** le chemin paramétré — séries, pays, fuseaux, statuts
- [X] T039 [P] [US1] Écrire `tests/sigle_obligatoire_avec_pavillon.rs` — les **quatre** chemins d'écriture, les bornes de format, et la valeur proposée réellement utilisable
- [X] T040 [P] [US1] Écrire `tests/contraintes_edition_traduites.rs` — les six contraintes, chacune sur son champ, aucun message technique
- [X] T041 [P] [US1] Écrire `tests/jours_civils_dans_le_fuseau.rs` — l'édition de Belém porte **douze journées du 9 au 20 novembre**. Si la première tombe le 8 ou le 10, le fuseau n'a pas été appliqué

---

## Phase 4 — US2 : Le périmètre d'administration borne aussi les événements (P1)

**Objectif** : lister, ouvrir le détail, et refuser tout ce qui sort du périmètre — URL forgée comprise.

**Éprouvable seule** : un compte détaché sur une édition en voit une ; un compte sans droit reçoit un refus ; six identifiants forgés mènent tous au même refus.

- [X] T042 [US2] Écrire dans `src/repo/editions.rs` la liste **bornée par le périmètre**, avec ses facettes — séries et millésimes comptés **sur le même jeu de lignes** que la liste
- [X] T043 [US2] Écrire `src/service/edition_read.rs` — les **trois** cas du périmètre distincts jusqu'au bout, et `is_global_scope` dans la réponse
- [X] T044 [US2] Ajouter `GET /admin/events` dans `src/routes/admin_events.rs` — périmètre vide → **refus explicite**
- [X] T045 [US2] Ajouter `GET /events` dans `src/routes/public.rs` — le sélecteur du back-office, **filtrée** et non refusée : c'est la seule route du module où périmètre vide rend une liste vide, parce que le contrat du front le veut
- [X] T046 [US2] Écrire `src/service/detail.rs` — la composition des **six onglets**, lue séquentiellement sur **une seule connexion**, dans une transaction `REPEATABLE READ READ ONLY` (research.md § R3)
- [X] T047 [US2] Ajouter `GET /admin/events/{id}` — **après** `form-options` —, rendant l'introuvable et le hors-périmètre de façon **indiscernable**
- [X] T048 [US2] Exiger la portée **GLOBALE** à la création d'une édition (`EVENT_GLOBAL_SCOPE_REQUIRED`) : une édition qui n'existe pas encore n'offre aucune portée où vérifier un droit
- [X] T049 [US2] Appliquer l'assistant d'ascendance de T014 aux **six** routes paramétrées par un identifiant d'enfant — *fermée avec la phase 10 : les six routes existent (fil, lieu, salle, canal, appel, journée) et appellent toutes le garde. `perimetre_edition_url_forgee.rs` les couvrait déjà par leurs cibles.*
- [X] T050 [P] [US2] Écrire `tests/perimetre_edition_url_forgee.rs` — **chaque** route paramétrée, y compris les six qui remontent par un enfant
- [X] T051 [P] [US2] Écrire `tests/perimetre_liste_filtree.rs` — global, détaché, vide : les trois cas et leurs trois réponses différentes
- [X] T052 [P] [US2] Écrire `tests/creation_portee_globale.rs` — refusée à un compte détaché, acceptée à un compte global
- [X] T053 [P] [US2] Écrire `tests/detail_en_une_reponse.rs` — les six onglets présents, et les décomptes cohérents entre eux

---

## Phase 5 — US3 : L'appel unique s'ouvre avec sa grille, et jamais sans (P1)

**Objectif** : ouvrir la campagne de la COP31 avec ses critères pondérés. **C'est ce que tout le jalon attend.**

**Éprouvable seule** : ouvrir un appel avec sa grille, en refuser un second, prolonger sans perdre l'échéance annoncée, et se voir refuser le retrait d'un critère porteur de notes.

- [X] T054 [P] [US3] Écrire `src/domain/call.rs` — le **diff de grille par code** (insertion / mise à jour / suppression) et la détection de `scores_affected`
- [X] T055 [P] [US3] Tests unitaires de `src/domain/call.rs` — ligne nouvelle sans identifiant, code disparu, barème modifié, poids modifié
- [X] T056 [US3] Écrire `src/repo/calls.rs` — lecture, insertion, mise à jour, et **appel** des trois fonctions du modèle (échéance effective, appel ouvert, note maximale), jamais leur recalcul
- [X] T057 [US3] Écrire `src/repo/criteria.rs` — lecture, insertion, mise à jour, suppression
- [X] T058 [US3] Écrire dans `src/repo/cross.rs` les deux lectures de l'appel — notes posées **par critère**, dossiers déposés **par appel**
- [X] T059 [US3] Écrire `src/service/call.rs` — l'appel et sa grille **en une seule transaction** : un échec sur la grille ne laisse aucun appel
- [X] T060 [US3] Refuser la suppression d'un critère porteur de notes (`EVENT_CRITERION_HAS_SCORES`), **en nommant le critère et son nombre de notes**. ⚠️ La clé est `ON DELETE CASCADE` : sans ce refus, la base détruit les notes en silence (research.md § R9)
- [X] T061 [US3] Traduire les **six** contraintes nommées d'un appel, plus `criteria_empty` et `criterion_code_duplicate` — ce dernier désignant le **rang** de la ligne fautive
- [X] T062 [US3] Poser `scores_affected` quand un critère **conservé** voit son barème ou son poids changer **et** porte déjà des notes
- [X] T063 [US3] Émettre `event.call.opened`, `event.call.closed` et `event.call.deadline_extended`, cette dernière portant **l'échéance initiale** avec la nouvelle
- [X] T064 [US3] Écrire `src/routes/admin_call.rs` — `POST /admin/calls`, `PUT /admin/calls/{id}`, et `GET /admin/calls/default-criteria` **lue en base**, jamais recopiée
- [X] T065 [P] [US3] Écrire `tests/appel_unique_par_edition.rs` — second refusé, et **annulé puis recréé accepté** (l'index exclut les annulés)
- [X] T066 [P] [US3] Écrire `tests/contraintes_appel_traduites.rs` — les six contraintes, chacune sur son champ ; grille vide et codes en double avec leur rang
- [X] T067 [P] [US3] Écrire `tests/critere_porteur_de_notes.rs` — le refus, **et les notes toujours présentes après le refus**. Sans cette seconde moitié, le test ne prouve rien
- [X] T068 [P] [US3] Écrire `tests/grille_par_defaut_lue_en_base.rs` — les six critères, leurs libellés bilingues, leurs poids et l'éliminatoire, **identiques** à ce que le modèle sème

---

## Phase 6 — US4 : Le public voit une édition, ses échéances et son visuel (P1)

**Objectif** : servir la page publique et la frise d'historique. **Deux écarts consignés s'y referment.**

**Éprouvable seule** : sans session, demander les éditions publiques et la page d'une édition annoncée ; le brouillon et l'annulée sont absents, les trois images sont là, l'appel est résolu.

- [X] T069 [US4] Écrire `src/repo/public.rs` — `event.v_public_editions` jointe **par la gauche** à `programme.v_edition_stats`, **en une requête** (research.md § R16)
- [X] T070 [US4] Écrire le service des lectures publiques — la liste et la page par adresse d'URL, sans session
- [X] T071 [US4] Écrire `src/routes/public.rs` — `GET /events/public` **déclarée AVANT** `GET /events/{slug}`, sinon `public` est capturé comme adresse d'URL
- [X] T072 [US4] Ajouter `GET /event-series` — les séries avec leur genre et leur **décompte d'éditions**
- [X] T073 [US4] Ajouter les six lectures d'une édition — journées, fils **publiés seulement**, lieux, salles, canaux (**ceux de l'édition et ceux de la plateforme**), appel
- [X] T074 [US4] Ajouter `GET /events/{id}/images`, **livrée pour ne pas casser l'écran et marquée en commentaire comme vouée à disparaître** : la page d'une édition porte désormais ses trois images (écart n° 25)
- [X] T075 [P] [US4] Écrire `tests/editions_publiques_sans_session.rs` — brouillon et annulée absentes, **annoncée présente**, **hors série présente**
- [X] T076 [P] [US4] Écrire `tests/page_edition_une_requete.rs` — les trois déclinaisons d'image embarquées, l'appel résolu, l'échéance **effective** avec prolongation
- [X] T077 [P] [US4] Écrire `tests/ordre_des_routes_publiques.rs` — `/events/public` rend une liste, jamais le `null` d'une adresse inconnue

---

## Phase 7 — US5 : Le stand a ses salles, et le direct son canal (P2)

**Objectif** : déclarer lieux, salles et canaux, pour que B5 puisse **nommer** un conflit.

**Éprouvable seule** : créer un lieu et deux salles dont une virtuelle, poser un canal par défaut puis un second, et retirer un canal qui a servi.

- [X] T078 [US5] Écrire `src/repo/venues.rs` — lieux et salles, lecture et écriture
- [X] T079 [US5] Écrire `src/repo/channels.rs` — canaux, lecture et écriture
- [X] T080 [US5] Écrire dans `src/repo/cross.rs` les décomptes de séances **par salle, par lieu et par canal**
- [X] T081 [US5] Écrire `src/service/venues.rs` — écritures, `is_virtual` écrit **tel quel** et jamais déduit du mode de participation, et **décompte du détachement AVANT** la suppression (research.md § R8)
- [X] T082 [US5] Écrire `src/service/channels.rs` — poser le canal par défaut **retire le précédent d'abord**, dans la même transaction : l'index n'est pas différable et l'ordre inverse échoue (research.md § R6)
- [X] T083 [US5] Retirer un canal : **désactivé s'il a servi** (`ok: true`, `deactivated`), supprimé sinon — c'est un succès, pas un refus (research.md § R7)
- [X] T084 [US5] Refuser la modification d'un canal **général de la plateforme** depuis une édition (`platform_channel`)
- [X] T085 [US5] Écrire dans `src/routes/admin_tabs.rs` les **six** routes de lieux et de salles
- [X] T086 [US5] Écrire dans `src/routes/admin_tabs.rs` les **trois** routes de canaux
- [X] T087 [P] [US5] Écrire `tests/canal_par_defaut_unique.rs` — un second défaut retire le premier, y compris sur deux écritures concurrentes, et **le canal général semé n'est pas délogé**
- [X] T088 [P] [US5] Écrire `tests/canal_desactive_sil_a_servi.rs` — désactivé et non supprimé, réponse en succès
- [X] T089 [P] [US5] Écrire `tests/detachement_salle_et_lieu.rs` — le chiffre annoncé égale le chiffre réel, salle par salle

---

## Phase 8 — US6 : Les journées du calendrier et les journées spéciales ne se confondent pas (P2)

**Objectif** : habiller le calendrier, composer les fils, et ne jamais écrire sans qu'on l'ait demandé.

**Éprouvable seule** : demander le plan sans rien écrire, générer avec et sans retrait, créer un fil et le supprimer en chiffrant ce qu'il emporte.

- [X] T090 [US6] Compléter `src/repo/days.rs` — habillage éditorial et retrait d'une journée
- [X] T091 [US6] Écrire `src/repo/tracks.rs` — fils, lecture et écriture, **thématiques par `reference.entity_terms`** clé `('event','programme_tracks',id)`
- [X] T092 [US6] Écrire dans `src/repo/cross.rs` les décomptes par journée et par fil, et la lecture des thématiques par `reference.term_badges()`
- [X] T093 [US6] Écrire `src/service/days.rs` — le plan **en lecture seule**, et la génération qui **le recalcule dans sa propre transaction** sans jamais faire confiance au plan renvoyé (research.md § R4)
- [X] T094 [US6] Retirer les journées hors période **seulement sur demande explicite**, en comptant le détachement **avant**
- [X] T095 [US6] Garantir que la régénération **n'écrase aucun contenu éditorial**, et qu'une journée générée porte sa date et **rien d'autre**
- [X] T096 [US6] Écrire `src/service/tracks.rs` — création, modification, unicité du code et de l'adresse **au sein de l'édition**, thématiques et page publique **dans le même geste**
- [X] T097 [US6] Supprimer un fil en chiffrant les **rattachements éditoriaux** perdus, sans supprimer aucune séance
- [X] T098 [US6] Écrire dans `src/routes/admin_tabs.rs` les **trois** routes de journées et les **trois** routes de fils
- [X] T099 [P] [US6] Écrire `tests/plan_necrit_rien.rs` — la base est identique avant et après, journée par journée ; et une période d'un an annonce **plus de trois cents journées sans en écrire une**
- [X] T100 [P] [US6] Écrire `tests/regeneration_preserve_editorial.rs` — titre, adresse, couleur et mise en avant comparés champ à champ
- [X] T101 [P] [US6] Écrire `tests/detachement_journee_et_fil.rs` — le chiffre annoncé égale le chiffre réel, dans les deux cas

---

## Phase 9 — US7 : Le comité se compose sans ouvrir de droits (P2)

**Objectif** : désigner qui siège, sans jamais laisser croire que siéger accorde quelque chose.

**Éprouvable seule** : enregistrer une composition d'un geste, retirer un membre qui porte des dossiers et le voir nommé, ajouter quelqu'un sans le rôle d'évaluateur et le voir signalé.

- [X] T102 [US7] Écrire `src/repo/committee.rs` — lecture de la composition, écriture d'un seul geste
- [X] T103 [US7] Écrire dans `src/repo/cross.rs` les lectures du comité — dossiers confiés, revues rendues, détention de la permission d'évaluer sur l'édition, candidats et responsables assignables
- [X] T104 [US7] Écrire `src/service/committee.rs` — **une transaction** pour les ajouts, les retraits et les plafonds ; charge utile **dédoublonnée par le service**, jamais remontée comme erreur de base
- [X] T105 [US7] Nommer dans la réponse les membres retirés portant encore des dossiers (`removed_with_assignments`), sans annuler aucune évaluation rendue
- [X] T106 [US7] Ajouter `PUT /admin/calls/{id}/reviewers` dans `src/routes/admin_call.rs`, gardée par la permission de gestion des **appels**
- [X] T107 [P] [US7] Écrire `tests/comite_un_seul_geste.rs` — ajouts, retraits et plafonds ensemble ; doublon de charge utile ; personne inconnue refusée
- [X] T108 [P] [US7] Écrire `tests/comite_naccorde_aucun_droit.rs` — aucun rôle attribué en base après un ajout, **et** les deux permissions du module éprouvées séparément dans les deux sens

---

## Phase 10 — US8 : La programmation ne se publie pas avec un conflit ouvert (P3)

**Objectif** : le seul contrôle bloquant du module. Il ne se démontre pleinement qu'avec des séances — donc en partie après B5.

**Éprouvable seule** : sur une édition portant un conflit bloquant, voir la liste avant d'essayer, se voir refuser, lever le conflit, publier, et republier sans effet.

- [X] T109 [US8] Écrire dans `src/repo/cross.rs` l'appel à `programme.publication_readiness()` et **le prédicat des séances à publier** avec son décompte
- [X] T110 [US8] Écrire `src/service/publication.rs` — le contrôle préalable **en lecture seule**, rendant `occurs_at` comme **instant** et jamais comme intervalle mis en forme
- [X] T111 [US8] Refuser la publication dès qu'un point de gravité bloquante subsiste : rien n'est écrit, et la liste dit quoi régler. Les **avertissements** ne retiennent pas
- [X] T112 [US8] Estampiller l'édition par un `UPDATE … WHERE programme_published_at IS NULL` — **la republication n'écrase pas la date d'origine** — et émettre `event.programme.published` dans la même transaction
- [X] T113 [US8] Compléter `backend/crates/contracts/src/event.rs` — la charge utile porte le **prédicat exact**, pour que le consommateur de B5 publie ce qui a été annoncé et pas autre chose
- [X] T114 [US8] Écrire `src/routes/planner.rs` — les deux routes **sans leur préfixe** (composé par `api`, T018), gardées par `programme.session.schedule` (research.md § R12)
- [X] T115 [P] [US8] Écrire `tests/publication_bloquee_puis_publiee.rs` — refus puis publication, la date posée, `published_count` juste, **exactement un** événement
- [X] T116 [P] [US8] Écrire `tests/publication_rejouee_inoffensive.rs` — date d'origine intacte, **aucun second événement**
- [X] T117 [P] [US8] Écrire `tests/publication_sans_seance.rs` — une édition sans séance publie, avec zéro séance et une liste vide

---

## Phase 11 — Finition et points transverses

**Objectif** : les garanties que seule une passe d'ensemble peut tenir, et la progression mise à jour.

- [X] T118 Écrire `tests/toutes_les_routes_repondent.rs` — **les 37 routes sur la vraie application**, intergiciels compris. C'est la leçon de B2, où trois routes sur vingt et une étaient muettes sans que rien ne le dise
- [X] T119 Écrire `tests/outbox_evenements_du_module.rs` — les **six** événements attendus, **et l'absence d'événement** pour les journées, fils, lieux, salles, canaux et comité ([`contracts/events.md`](contracts/events.md))
- [X] T120 Vérifier qu'**aucune ligne du module n'écrit hors du schéma `event`** — le contrôle par `grep` de [`quickstart.md`](quickstart.md) doit ne rien rendre
- [X] T121 [P] Compléter les annotations OpenAPI des 37 gestionnaires, les trois codes d'erreur étant **engendrés** depuis le catalogue du noyau
- [X] T122 [P] Régénérer `backend/.sqlx/` avec `--all-targets`, sans quoi les requêtes des tests ne sont pas versionnées
- [X] T123 [P] Vérifier qu'aucun fichier de `backend/` ne dépasse **1000 lignes**, et découper celui qui s'en approche
- [X] T124 [P] Corriger dans `docs/progression/` toute affirmation que le code aurait démentie en chemin — la règle de B1 et B2 : un document faux coûte plus qu'un document absent
- [X] T125 [P] Inscrire dans `docs/progression/points-bloques.md` les **deux ajouts additifs au contrat du front** à refermer en B7 : `suggested_acronym` sur la réponse d'enregistrement, et une variante de `CallErrorCode` pour le critère porteur de notes
- [X] T126 [P] Inscrire l'obligation de **B5** : le consommateur de `event.programme.published`, sa garde de rejeu, et le fait qu'il publie **le prédicat annoncé et pas un autre**
- [X] T127 [P] Inscrire les obligations de **B6** : le rattachement des trois images d'une édition, et le rappel d'échéance aux organisations
- [X] T128 `make check` **en entier depuis la racine** — il détruit le volume et recharge le schéma de zéro
- [X] T129 Mettre à jour la progression : journal du jour, `docs/progression/ecrans/b3-evenements.md`, décisions prises en chemin, et la ligne de suivi dans `docs/PROGRESSION.md`

---

## Dépendances

```
Phase 1  ──►  Phase 2  ──►  ┌─ Phase 3 (US1)  ──►  Phase 4 (US2)  ──►  Phase 5 (US3)
   (mise      (fondations)  │                          │
   en place)   BLOQUANTE    │                          ├──►  Phase 7 (US5)
                            │                          ├──►  Phase 8 (US6)
                            │                          └──►  Phase 9 (US7) ──► après Phase 5
                            └─ Phase 6 (US4) ─────────────────────────────────┐
                                                                              │
                                            Phase 10 (US8) ◄──────────────────┘
                                                   │
                                            Phase 11 (finition)
```

**Ce qui est réellement bloquant, et pourquoi**

| Dépendance | Raison |
|---|---|
| Phase 2 → tout | Sans les codes d'erreur, l'ascendance et le montage, aucune route ne répond |
| Phase 3 → Phase 4 | Il faut une édition **écrivable** avant de pouvoir borner sa lecture |
| Phase 4 → Phases 7, 8, 9 | Toutes les écritures d'onglet rendent `EditionTabResult.detail`, composé par `service/detail.rs` (T046) |
| Phase 5 → Phase 9 | Le comité appartient à un **appel** ; sans appel, rien à composer |
| Phase 4 → Phase 10 | La publication passe par le garde d'ascendance et le périmètre |
| Phase 6 → rien | Les lectures publiques ne dépendent que des fondations : **elles peuvent partir en parallèle de la phase 3** |

**T060 et T067 vont ensemble.** Livrer le refus sans le test qui vérifie que **les notes sont toujours là après** ne prouve rien : c'est le seul endroit du module où l'échec serait invisible.

---

## Ce qui peut tourner en parallèle

**Phase 1** — `T002 · T003 · T004 · T005 · T006` sans ordre imposé.

**Phase 2** — trois grappes disjointes :

```
erreurs      kernel/error.rs, kernel/pg_error.rs        T008 · T009
contrats     contracts/event.rs, domain/{ids,permissions}.rs   T010 · T011 · T012
harnais      tests/commun/                              T021 · T022 · T023
```

**Phases 3 et 6 ensemble.** Les lectures publiques (`repo/public.rs`, `routes/public.rs`) ne partagent aucun fichier avec l'écriture d'une édition. C'est le parallélisme le plus rentable du découpage : deux histoires **P1** menées de front.

**Phases 7, 8 et 9, une fois la phase 5 finie** — trois chantiers sur des fichiers disjoints :

```
US5   stand et direct     repo/{venues,channels}.rs, service/{venues,channels}.rs
US6   calendrier et fils  repo/{days,tracks}.rs, service/{days,tracks}.rs
US7   comité              repo/committee.rs, service/committee.rs
```

Leur seul point de rencontre est `src/repo/cross.rs` (T080, T092, T103) et `src/routes/admin_tabs.rs` (T085, T086, T098) : **les écrire l'un après l'autre**, ou accepter un conflit de fusion.

**Phase 11** — `T121 · T122 · T123 · T124 · T125 · T126 · T127` sans ordre imposé.

---

## Stratégie de livraison

**Le plus petit incrément qui vaille** : phases 1 à 3 — **T001 à T041**. Une édition de COP existe, avec son sigle exigé et son calendrier daté dans le bon fuseau. C'est l'entité dont B4, B5 et B6 dépendent tous, et l'écart n° 9 est traité de bout en bout.

**Le jalon qui ouvre l'appel à propositions de la COP31** : jusqu'à la phase 5 — **T001 à T068**. Édition, périmètre, appel et grille. **C'est ce que le prompt demande** : à ce point, la campagne peut être ouverte et B4 a de quoi recevoir un dossier.

**Le décor complet et la page publique** : jusqu'à la phase 9 — **T001 à T108**. Lieux, salles, canaux, journées, fils, comité, et l'édition visible du public avec ses échéances. À ce point, B5 a tout ce qu'il lui faut pour placer des séances et **nommer** un conflit.

**Le module complet** : jusqu'à la phase 11 — **T001 à T129**. Avec la publication, dont la seconde moitié — la visibilité des séances — arrivera avec B5.

**Ce qu'on ne livre pas, et c'est écrit** : la suppression d'une édition, la dé-publication d'une programmation et l'écriture d'une série (aucun écran ne les offre) ; le rattachement des images (B6) ; les rappels d'échéance (B6) ; et une borne dure sur la génération du calendrier d'une série de webinaires (arbitrage en attente — le plan annonce, il n'impose rien).

---

## Récapitulatif

| Phase | Histoire | Priorité | Tâches | Dont tests |
|---|---|---|---|---|
| 1 | Mise en place | — | T001–T007 | — |
| 2 | Fondations — erreurs, ascendance, montage, travail différé | — | T008–T024 | 1 |
| 3 | US1 — une édition, un sigle épelable, un calendrier | **P1** | T025–T041 | 5 |
| 4 | US2 — le périmètre borne aussi les événements | **P1** | T042–T053 | 4 |
| 5 | US3 — l'appel unique et sa grille | **P1** | T054–T068 | 5 |
| 6 | US4 — les éditions publiques | **P1** | T069–T077 | 3 |
| 7 | US5 — lieux, salles et canal de diffusion | P2 | T078–T089 | 3 |
| 8 | US6 — journées du calendrier et journées spéciales | P2 | T090–T101 | 3 |
| 9 | US7 — comité de sélection | P2 | T102–T108 | 2 |
| 10 | US8 — publication de la programmation | P3 | T109–T117 | 3 |
| 11 | Finition | — | T118–T129 | 2 |
| | | | **129 tâches** | **31 tests** |

**Les quatre obligations minimales du principe X** sont couvertes par **T118** (chemin nominal des 37 routes), **T050 et T023** (refus par périmètre, URL forgée comprise), **T040, T066 et T067** (invariants de la base traduits) et **T119** (écriture dans l'outbox — **et l'absence d'écriture là où rien ne doit être émis**).

**Les critères de réussite qu'aucun autre test ne tiendrait** : **T039** (SC-001 et SC-002, le sigle et le cas sans pavillon), **T041** (le fuseau — le défaut le plus discret du module), **T067** (SC-004 et l'écart n° 91), **T068** (SC-010), **T087** (SC-015), **T089 et T101** (SC-017, le décompte exact), **T099** (SC-012), **T116** (SC-019), **T076** (SC-022, l'écart n° 25 refermé).

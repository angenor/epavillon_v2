---

description: "Task list — Propositions (B4)"
---

# Tasks: Propositions (B4)

**Input**: Design documents from `/specs/004-propositions/`

**Prerequisites**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: **OBLIGATOIRES.** Le principe X de la constitution impose des tests d'intégration sur base réelle et jetable, sans aucun mock de base. Les tâches de test ne sont pas optionnelles ici.

**Organization**: par histoire utilisateur, pour que chaque tranche soit implémentable, éprouvable et livrable seule.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichier différent, aucune dépendance en cours
- **[Story]** : `US1` … `US8`, correspondant aux histoires de `spec.md`

## Conventions de chemin

Tout vit dans `backend/`, workspace Cargo symétrique de `frontend/` — emplacements imposés par le principe II. **Aucun fichier de ce jalon ne vit hors de `backend/`**, à une exception documentaire : `docs/progression/`.

---

## ⚠️ Quatre choses à lire avant de commencer

### 1. Le déclencheur d'état ÉMET DÉJÀ l'événement de domaine

`programme.tg_guard_proposal_status()` appelle `platform.emit_event()` à chaque transition acceptée. **C'est l'inverse de B3**, où aucun déclencheur du module n'émettait rien, et c'est le retour du piège de B1 (`anonymize_person()`) et de B2 (`merge_organizations()`).

Un service qui émettrait à son tour produirait **deux** événements par transition — donc deux courriels, deux notifications — et **le doublon ne se verrait qu'en production**. Aucune tâche de ce découpage n'émet un `programme.proposal.*` ; **T074** compte les lignes d'outbox après une transition et exige **une** ligne, pas deux.

La conclusion d'un module ne se transporte pas au suivant : celle-ci a été vérifiée dans le corps du déclencheur, pas déduite de B3.

### 2. Le préfixe `/organizations` doit être refactorisé — DU CODE LIVRÉ EST MODIFIÉ

Deux routes du contrat de ce module vivent sous `/organizations`, préfixe qu'un `web::scope` **unique** du module Organisations occupe depuis B2. Deux scopes du même préfixe **ne se complètent pas** : Actix retient le premier et rend 404 sur les routes du second, sans essayer.

Le défaut s'est déjà produit — **trois routes muettes sur vingt et une en B2** — et le commentaire qui le raconte est dans `backend/crates/api/src/lib.rs`, à l'endroit exact où l'on écrit.

**T026 et T027** appliquent le patron de `/people` : `org` expose ses routes de ce préfixe séparément, `api` compose une seule fois. **Aucune route ne change de chemin**, et `crates/api/tests/routes_org.rs` doit rester vert — c'est lui qui prouve que rien n'est devenu muet.

### 3. Le tout premier enregistrement d'un brouillon échoue sans la dérivation d'adresse d'URL

`programme.proposals.slug` est **obligatoire** et unique par édition. Le contrat du formulaire ne le porte pas, et le premier enregistrement automatique a lieu **à la première frappe**, quand le titre est encore vide — `platform.slugify('')` rend alors une valeur nulle.

**T031** dérive, replie et suffixe ; **T053** éprouve les deux cas ordinaires : titre vide, et deux dossiers homonymes dans une même édition. Sans T031, la fonctionnalité entière ne démarre pas (écart n° 95).

### 4. Rien n'appelle la consolidation des notes

`programme.refresh_proposal_score()` existe, son commentaire dit « à appeler après toute saisie de note », et **aucun déclencheur ne la déclenche**. Sans appel explicite, la note d'un dossier, sa moyenne, son nombre de revues et son élimination restent aux valeurs de la ligne : **le classement du comité est faux sans qu'aucune erreur ne le signale**.

**T103** l'appelle dans la transaction du dépôt d'une revue ; **T112** compare la valeur rendue à la valeur relue en base. C'est le pire défaut du module parce qu'il est muet (écart n° 98).

---

## 🔓 Un point resté ouvert, et l'hypothèse tenue en attendant

**Qui, dans une organisation, peut agir sur un dossier déjà déposé ?** La question a été posée et n'a pas reçu de réponse. L'hypothèse de la spécification est tenue : **toute personne dont l'adhésion est active** peut corriger, renvoyer et retirer, ce que l'écran suppose déjà en rouvrant un dossier déposé deux mois plus tôt par une collègue.

Elle est isolée dans **une seule fonction** (`domain/ownership.rs`, T021) et non répandue dans douze gardes. Si le commanditaire tranche autrement — seule la déposante, ou la déposante et les référents —, **une fonction change et rien d'autre**.

---

## Phase 1 — Mise en place

**Objectif** : le crate existe, il compile, il ne dépend de personne.

- [X] T001 Créer le crate `backend/crates/modules/programme` (`Cargo.toml`, `src/lib.rs`) et l'ajouter aux `members` et aux `[workspace.dependencies]` de `backend/Cargo.toml`
- [X] T002 [P] Créer l'arborescence interne vide — `src/domain/`, `src/repo/`, `src/service/`, `src/routes/`, `tests/commun/` — sur la forme qu'`identity`, `org` et `event` ont établie
- [X] T003 [P] Ajouter `ammonia` aux `[workspace.dependencies]` de `backend/Cargo.toml` et à ce crate — **seule dépendance nouvelle du module**, consignée dans `docs/progression/decisions/`
- [X] T004 [P] Créer `src/state.rs` — `ProgrammeState` portant l'accès base et la configuration, sur le modèle d'`EventState`
- [X] T005 [P] Créer `src/domain/ids.rs` — les types d'identifiants du module (dossier, revue, affectation, message, pièce)
- [X] T006 Vérifier `cargo build -p programme` et `cargo tree -p programme` — **aucune arête** vers `identity`, `org` ni `event`

---

## Phase 2 — Fondations

**Objectif** : tout ce que les huit histoires partagent — erreurs, contrats d'événements, ascendance, autorisation, règles pures, montage, harnais. **Rien de ceci n'est propre à une histoire, et tout le reste en dépend.**

### Le catalogue d'erreurs et les contrats d'événements

- [X] T007 Ajouter les six variantes `PROPOSAL_*` au catalogue de `backend/crates/kernel/src/error.rs`, avec leur statut HTTP, d'après [contracts/errors.md](contracts/errors.md)
- [X] T008 [P] Créer `backend/crates/contracts/src/programme.rs` — les charges utiles des **trois** événements que le service émet, et un commentaire disant lesquels la **base** émet déjà
- [X] T009 [P] Étendre `backend/crates/modules/programme/src/domain/permissions.rs` — les cinq permissions consommées, dont `event.call.manage` qui vient d'un autre module

### Les règles pures — le domaine

Chacune est testable sans base. C'est ce qui justifie un `domain/` de dix fichiers : ce module porte dix règles que la base ne porte pas.

- [X] T010 [P] `src/domain/slug.rs` — dérivation de l'adresse d'URL, repli quand le titre est vide, suffixe incrémental (R5)
- [X] T011 [P] `src/domain/limits.rs` — les huit longueurs maximales, alignées sur `TEXT_LIMITS` du front (R15, écart n° 28)
- [X] T012 [P] `src/domain/sanitize.rs` — la liste blanche HTML, alignée **exactement** sur la barre d'outils de l'éditeur (R14, écart n° 32)
- [X] T013 [P] `src/domain/transitions.rs` — ce que le service fait des règles **lues** : offrir, exiger un motif. **Jamais le graphe** (R7)
- [X] T014 [P] `src/domain/eligibility.rs` — le classement des trois refus de recevabilité en réponses nommées (R9)
- [X] T015 [P] `src/domain/blind.rs` — les trois conditions du voile, et le cas de qui décide sans noter (R4)
- [X] T016 [P] `src/domain/facets.rs` — les sept facettes comptées sur les lignes déjà lues (R16)
- [X] T017 [P] `src/domain/draft.rs` — la structure de recomposition et ses conversions (R6)
- [X] T018 [P] Tests unitaires du domaine, sans base : `tests/domaine.rs` — adresse d'URL sur titre vide et sur titre accentué, HTML portant un attribut d'événement et un lien `javascript:`, voile dans ses quatre combinaisons, facettes sur un jeu de lignes écrit à la main

### L'ascendance, le périmètre et la qualité de porteur

- [X] T019 `src/repo/cross.rs` — **les seules lectures hors schéma**, réunies ici : édition d'un dossier, fuseau, appel et ses règles, grille, comité, organisation, personne, thématiques, objet stocké, historique de participation. Un en-tête énumère chacune avec la question **de ce module** qui la justifie (R13)
- [X] T020 `src/service/perimeter.rs` — remonter du message, du dossier ou de la revue **jusqu'à l'édition**, puis vérifier le périmètre, sans jamais rendre quoi que ce soit avant que le contrôle ne soit passé
- [X] T021 `src/domain/ownership.rs` — **la seule définition de « qui, dans une organisation, peut agir sur ce dossier »**. Hypothèse tenue : adhésion active. Isolée pour qu'un arbitrage n'en change qu'une (voir l'encadré ci-dessus)
- [X] T022 [P] Test sur base réelle : `tests/perimetre.rs` — un dossier hors périmètre se refuse **exactement** comme un dossier inexistant, sur les trois niveaux d'ascendance

### Les deux écritures hors schéma, isolées

- [X] T023 `src/repo/themes.rs` — **écriture hors schéma n° 1** : poser et purger les thématiques, triplet `('programme','proposals',id)` **écrit littéralement**, jamais reçu de la requête. Vérification d'appartenance à la taxonomie attendue (R11, écarts n° 3 et n° 94)
- [X] T024 `src/repo/people.rs` — **écriture hors schéma n° 2** : retrouver une personne par son adresse, la créer sinon, **sans déduire prénom ni nom de l'adresse**, jamais de compte ni de rôle (R12, écart n° 26 du contrat)

### Le montage, et les deux préfixes partagés

- [X] T025 `src/lib.rs` — exposer `routes()`, `people_routes()` et `organization_routes()`, plus un commentaire disant pourquoi les deux derniers existent
- [X] T026 **Modifier du code livré** : `backend/crates/modules/org/src/lib.rs` — extraire les routes de `/organizations` dans `organization_routes()`, **sans qu'aucune route change de chemin**
- [X] T027 `backend/crates/api/src/lib.rs` — monter `programme` d'après le registre des modules, et **composer `/organizations` une seule fois** comme `/people` l'est depuis B1 (R18)
- [X] T028 Vérifier que `backend/crates/api/tests/routes_org.rs` **reste vert** : c'est lui qui prouve qu'aucune route de B2 n'est devenue muette
- [X] T029 [P] `src/routes/openapi.rs` — l'agrégat OpenAPI du module, engendré depuis les gestionnaires et le catalogue d'erreurs

### Le harnais de test

- [X] T030 `tests/commun/mod.rs` — une fabrique qui enchaîne édition avec fuseau, appel ouvert et sa grille par défaut, organisation vérifiée, personne membre active, et rend de quoi déposer. **Le semis ne fournit aucun dossier** : sans elle, chaque test recommencerait quarante lignes de préparation (R22)

**Point de contrôle** : `cargo test -p programme` au vert, `make check-back` au vert, aucune arête entre crates de module. Les huit histoires peuvent alors commencer.

---

## Phase 3 — US1 : une organisation dépose son dossier (P1)

**Objectif** : du premier enregistrement à la confirmation de dépôt, avec le même numéro de bout en bout.

**Test indépendant** : créer un brouillon sur un appel ouvert, l'enregistrer trois fois, le compléter, le déposer ; recommencer sur un appel clos et sur une organisation au plafond.

### Les dépôts

- [X] T031 [P] [US1] `src/repo/proposals.rs` — créer un dossier **toujours en brouillon**, avec l'adresse d'URL dérivée, le repli et le **réessai suffixé** sur collision d'unicité (R5, écarts n° 95 et n° 96)
- [X] T032 [P] [US1] `src/repo/proposals.rs` — mettre à jour un dossier **sans toucher à son état** : corriger n'est pas déposer
- [X] T033 [P] [US1] `src/repo/organizations.rs` — poser et retirer les co-organisations, **jamais la ligne du porteur**, que le déclencheur tient
- [X] T034 [P] [US1] `src/repo/speakers.rs` — poser, modifier, réordonner et retirer les intervenants, avec leurs deux instantanés
- [X] T035 [P] [US1] `src/repo/proposals.rs` — lire le contexte du formulaire : l'appel ouvert de l'édition, et le décompte du plafond de l'organisation, ce brouillon exclu

### Les règles du dépôt

- [X] T036 [US1] `src/service/draft_write.rs` — la composition d'un enregistrement : textes en français, assainissement, longueurs, publics visés **un par entrée**, thématiques par leurs codes seuls
- [X] T037 [US1] `src/service/draft_write.rs` — conversion du créneau : heure murale → instant, **dans le fuseau de l'édition**, calculée en base (R6)
- [X] T038 [US1] `src/service/draft_write.rs` — bornes de l'appel : durée, plage horaire quotidienne **fin comprise**, formats admis
- [X] T039 [US1] `src/service/draft_write.rs` — co-organisations : rôle jamais porteur, pas de doublon, **refus d'ajouter le porteur** (que le `ON CONFLICT` du déclencheur ferait basculer en silence)
- [X] T040 [US1] `src/service/draft_write.rs` — intervenants : rapprochement par adresse, création sinon, **identité verrouillée** quand la personne possède un compte, instantanés modifiables dans tous les cas (écart n° 31)
- [X] T041 [US1] `src/service/draft_write.rs` — contact du dossier : le déposant par défaut, **règle explicite** (écart n° 30)
- [X] T042 [US1] `src/service/submit.rs` — le dépôt : classement des trois refus **avant** l'écriture, réponse nommée portant l'échéance ou le plafond, puis tentative (R9)
- [X] T043 [US1] `src/service/submit.rs` — bornes d'intervenants de l'appel, **qu'aucun déclencheur ne vérifie** (écart n° 27)
- [X] T044 [US1] `src/service/submit.rs` — la réponse de dépôt porte le nombre de revues attendues et la date d'annonce, **lus sur l'appel**
- [X] T045 [US1] `src/service/draft_write.rs` — émettre `programme.coorganization.requested` **par organisation ajoutée**, et rien d'autre

### Les routes

- [X] T046 [P] [US1] `src/routes/submission.rs` — `GET /proposals/form-context`
- [X] T047 [P] [US1] `src/routes/submission.rs` — `GET /proposals/draft`
- [X] T048 [US1] `src/routes/submission.rs` — `POST /proposals` et `PUT /proposals/{id}`
- [X] T049 [US1] `src/routes/submission.rs` — `POST /proposals/{id}/submit`
- [X] T050 [P] [US1] `src/routes/people.rs` — `GET /people/lookup`, déposée dans le préfixe partagé, **et qui ne rend jamais l'annuaire**
- [X] T051 [US1] Déclarer les chemins littéraux de `/proposals` **avant** `/proposals/{id}` dans `lib.rs`, sans quoi ils seraient capturés

### Les tests

- [X] T052 [P] [US1] `tests/depot.rs` — le parcours nominal : brouillon, trois enregistrements, dépôt. **Le numéro ne change jamais**
- [X] T053 [P] [US1] `tests/depot.rs` — **titre vide au premier enregistrement**, et deux dossiers homonymes dans une même édition : les deux doivent aboutir (écart n° 95)
- [X] T054 [P] [US1] `tests/depot.rs` — un dossier créé avec un état demandé autre que brouillon naît **en brouillon** (écart n° 96)
- [X] T055 [P] [US1] `tests/depot.rs` — les trois refus de recevabilité, chacun **portant sa valeur** : échéance, plafond, organisation non vérifiée
- [X] T056 [P] [US1] `tests/depot.rs` — bornes d'intervenants, de durée et de plage horaire, chacune sur son champ
- [X] T057 [P] [US1] `tests/depot.rs` — un texte de présentation portant un script, un attribut d'événement et un lien `javascript:` : **ce qui est stocké est propre**
- [X] T058 [P] [US1] `tests/depot.rs` — un texte au-delà de sa borne : refus nommant le champ et la limite
- [X] T059 [P] [US1] `tests/depot.rs` — le créneau : saisi à 14:30 dans le fuseau de l'édition, **relu à 14:30** depuis un autre fuseau
- [X] T060 [P] [US1] `tests/depot.rs` — un code de thématique hors taxonomie est refusé en le nommant, et **un triplet d'entité envoyé par le client n'est jamais honoré** (écart n° 3)
- [X] T061 [P] [US1] `tests/depot.rs` — un intervenant inconnu crée la personne ; **ni prénom ni nom ne sont déduits de l'adresse** ; une personne avec compte a son identité verrouillée
- [X] T062 [P] [US1] `tests/depot.rs` — l'ajout d'une co-organisation émet **un** événement par organisation, et le porteur ne peut pas y être ajouté

---

## Phase 4 — US2 : la machine à états, lue et jamais réécrite (P1)

**Objectif** : chacun ne voit que ce qui lui est ouvert, et l'écriture tente au lieu de rejouer.

**Test indépendant** : demander les transitions d'un même dossier comme déposant, comme noteur et comme décideur — trois réponses différentes ; puis tenter une transition non offerte.

- [X] T063 [P] [US2] `src/repo/transitions.rs` — lire la table des règles telle quelle, pour la route globale du contrat
- [X] T064 [US2] `src/repo/transitions.rs` — **une seule requête** croisant les règles applicables à l'état courant, la permission sur la portée de **l'édition du dossier**, et la qualité de porteur (R7)
- [X] T065 [US2] `src/service/transition.rs` — tenter la transition, **sans rejouer le graphe** ; traduire `restrict_violation` en réponse nommée, avec le message français du déclencheur **repris mot pour mot**
- [X] T066 [US2] `src/service/transition.rs` — traduire `not_null_violation` en « motif exigé », code **distinct** du précédent, sûr parce que la transaction n'écrit que deux colonnes nullables (R8)
- [X] T067 [US2] `src/service/transition.rs` — **n'émettre aucun événement** : le déclencheur le fait. Un commentaire le dit à l'endroit où la tentation existe
- [X] T068 [US2] `src/repo/transitions.rs` — lire le journal d'un dossier, **c'est lui qui porte chaque motif** (écart n° 97)
- [X] T069 [P] [US2] `src/routes/detail.rs` — `GET /proposals/transitions` (règles globales) et `GET /proposals/{id}/transitions` (**le journal**)
- [X] T070 [P] [US2] `src/routes/detail.rs` — `GET /proposals/{id}/available-transitions` (écart n° 101, R19)
- [X] T071 [US2] `src/service/transition.rs` — l'action groupée : autorisation **dossier par dossier**, une sélection pouvant traverser deux éditions, et un écart nommé pour chacun de ceux qui n'ont pas suivi
- [X] T072 [P] [US2] `src/routes/admin_list.rs` — `POST /proposals/status`
- [X] T073 [P] [US2] `tests/transitions.rs` — le même dossier vu par trois personnes de droits différents : trois réponses conformes à la table
- [X] T074 [P] [US2] `tests/transitions.rs` — **une transition acceptée écrit UNE ligne dans l'outbox, pas deux** (avertissement n° 1), et le journal gagne une ligne avec son auteur et son motif
- [X] T075 [P] [US2] `tests/transitions.rs` — une transition non déclarée et un motif manquant sortent sous **deux codes distincts**, tous deux en 200
- [X] T076 [P] [US2] `tests/transitions.rs` — un retrait motivé écrase la colonne de décision **et** le journal garde les deux motifs (écart n° 97)

**🏁 Jalon — l'appel de la COP31 reçoit des dossiers.** T001–T076 : une organisation dépose, corrige et retire ; la machine à états tient. C'est le cœur du prompt.

---

## Phase 5 — US3 : le comité pilote sa liste (P1)

**Objectif** : tout l'écran en une réponse, borné par le périmètre, avec des actions groupées qui disent ce qui n'a pas suivi.

**Test indépendant** : charger la liste comme administratrice globale, puis détachée, puis sans aucun droit ; forger l'identifiant d'une autre édition ; lancer les deux actions groupées sur une sélection hétérogène.

- [X] T077 [P] [US3] `src/repo/dashboard.rs` — lire la vue de pilotage **telle quelle**, sans recomposer ses jointures, avec ses **deux** colonnes de titre
- [X] T078 [P] [US3] `src/repo/reads.rs` — les dossiers qu'une personne n'a jamais ouverts, par la fonction du modèle
- [X] T079 [US3] `src/service/list.rs` — la composition : lignes lues **une fois**, sept facettes comptées dessus (R16), non-lus, fuseau, ville, échéance effective, revues attendues
- [X] T080 [US3] `src/service/list.rs` — le périmètre : global, éditions listées, **périmètre vide → refus explicite**, jamais liste vide
- [X] T081 [P] [US3] `src/repo/cross.rs` — la composition du comité de l'appel, avec la charge de chacun
- [X] T082 [P] [US3] `src/repo/assignments.rs` — confier un dossier, retirer une affectation, lire les affectations d'un dossier
- [X] T083 [US3] `src/service/list.rs` — l'affectation groupée sous `event.call.manage` : écarte le déjà confié, le déporté, l'introuvable, chacun nommé
- [X] T084 [US3] `src/service/list.rs` — émettre `programme.review.assigned` **par dossier**, jamais un pour le lot
- [X] T085 [P] [US3] `src/routes/admin_list.rs` — `GET /proposals/list`
- [X] T086 [P] [US3] `src/routes/admin_list.rs` — `GET /proposals/dashboard` et `GET /proposals/committee`
- [X] T087 [P] [US3] `src/routes/admin_list.rs` — `POST /proposals/assignments`
- [X] T088 [P] [US3] `src/routes/detail.rs` — `GET /proposals` filtrée par organisation, et `GET /proposals/{id}`
- [X] T089 [P] [US3] `tests/liste.rs` — les trois cas du périmètre, **et six identifiants forgés** menant au même refus qu'un inexistant
- [X] T090 [P] [US3] `tests/liste.rs` — les décomptes des facettes correspondent **exactement**, filtre par filtre, aux lignes rendues
- [X] T091 [P] [US3] `tests/liste.rs` — une action groupée sur une sélection hétérogène : appliqués + écartés = taille de la sélection, chaque écart portant sa raison
- [X] T092 [P] [US3] `tests/liste.rs` — un dossier effacé n'y figure pas ; les deux colonnes de titre portent bien deux types différents
- [X] T093 [P] [US3] `tests/liste.rs` — l'affectation groupée de douze dossiers émet **douze** événements

---

## Phase 6 — US4 : la fiche d'évaluation, et le voile (P1)

**Objectif** : onze tables en une réponse, et rien qui ne devait pas sortir.

**Test indépendant** : ouvrir la même fiche comme affecté n'ayant pas noté, comme ayant noté, comme administrateur qui décide sans noter — et comparer les charges utiles.

- [X] T094 [P] [US4] `src/repo/reviews.rs` — la revue d'une personne, les revues des pairs, l'avancement nominatif du comité
- [X] T095 [P] [US4] `src/repo/scores.rs` — les notes par critère et leurs commentaires, en `INSERT … ON CONFLICT DO UPDATE`
- [X] T096 [P] [US4] `src/repo/comments.rs` — le fil, **filtré par visibilité à la source**, jamais après coup
- [X] T097 [P] [US4] `src/repo/documents.rs` — les pièces avec leur objet stocké et l'adresse **composée en base**, nulle quand l'objet n'est pas servi
- [X] T098 [US4] `src/service/desk.rs` — la composition des onze lectures, **une transaction, une connexion**, par la porte d'écriture — elle pose l'accusé de lecture (R3)
- [X] T099 [US4] `src/service/desk.rs` — le voile : quand il est baissé, **la requête des revues des pairs n'est pas exécutée** ; le décompte l'est (R4)
- [X] T100 [US4] `src/service/desk.rs` — l'accusé de lecture posé à l'ouverture, la réponse disant l'état **d'avant** la visite
- [X] T101 [US4] `src/service/desk.rs` — l'historique de participation de l'organisation, **réduit aux colonnes de la fiche** (écart n° 54)
- [X] T102 [US4] `src/service/desk.rs` — le champ `available_transitions`, alimenté par la requête de T064
- [X] T103 [US4] `src/service/review.rs` — enregistrer ou déposer une revue, **appeler la consolidation dans la même transaction**, rendre les agrégats **relus** (avertissement n° 4, R10)
- [X] T104 [US4] `src/service/review.rs` — noter exige **une affectation non déportée** ; lire un dossier non confié reste permis (R21)
- [X] T105 [US4] `src/service/review.rs` — le déport, **motif obligatoire**
- [X] T106 [US4] `src/service/comments.rs` — écrire un message avec sa visibilité, et **forcer une demande de correction en visibilité partagée** (écart n° 99)
- [X] T107 [US4] `src/service/comments.rs` — émettre `programme.comment.shared` sur un message partagé, et **rien** sur un message de comité ou privé
- [X] T108 [P] [US4] `src/routes/admin_desk.rs` — `GET /proposals/{id}/review-desk`
- [X] T109 [P] [US4] `src/routes/admin_desk.rs` — `PUT /proposals/{id}/reviews`, `POST /proposals/{id}/recusal`, `POST /proposals/{id}/decision`
- [X] T110 [P] [US4] `tests/evaluation.rs` — **le voile, par inspection de la charge utile** : aucune note, aucune recommandation, aucun nom de pair. Puis le voile levé, et le cas de qui décide sans noter
- [X] T111 [P] [US4] `tests/evaluation.rs` — une note au-dessus du maximum **de son critère** : refus nommant le critère et sa borne
- [X] T112 [P] [US4] `tests/evaluation.rs` — après un dépôt de revue, les agrégats rendus **égalent** ceux relus en base ; un zéro sur un critère éliminatoire marque le dossier
- [X] T113 [P] [US4] `tests/evaluation.rs` — une revue en brouillon ne compte dans aucun agrégat et n'est vue d'aucun pair ; noter sans affectation est refusé
- [X] T114 [P] [US4] `tests/evaluation.rs` — les trois visibilités, chacune sur son lecteur ; une demande de correction écrite « comité » **ressort partagée**
- [X] T115 [P] [US4] `tests/evaluation.rs` — un déport sans motif est refusé ; une pièce en quarantaine est rendue **sans adresse**

**🏁 Jalon — le comité travaille.** T001–T115 : les quatre histoires P1 sont livrées. On dépose, on pilote, on évalue, on décide.

---

## Phase 7 — US5 : l'organisation suit son dossier (P2)

**Objectif** : une composition qui lui est propre, et où rien du comité n'entre.

**Test indépendant** : charger l'espace d'une organisation ayant deux dossiers dont un noté, et **balayer la réponse entière**.

- [X] T116 [US5] `src/service/workspace.rs` — la composition : dossiers, éditions, journaux, demandes **ouvertes**, séances programmées. **Ni note, ni rang, ni nom de membre du comité, ni inscrit nommé** (écart n° 8)
- [X] T117 [US5] `src/service/workspace.rs` — le fil du déposant, filtré sur la visibilité partagée **à la source**
- [X] T118 [US5] `src/service/workspace.rs` — l'accès borné par l'**adhésion active**, jamais par le périmètre d'administration
- [X] T119 [US5] `src/service/comments.rs` — la réponse du déposant : **toujours** partagée, **jamais** une demande de correction
- [X] T120 [US5] `src/service/comments.rs` — la résolution posée et **retirée** par le déposant, portée par une permission et non par un formulaire ; le comité garde la main de son côté (écart n° 35)
- [X] T121 [P] [US5] `src/routes/workspace.rs` — `GET /organizations/{id}/workspace` et `GET /organizations/{id}/editions`, déposées dans le préfixe partagé
- [X] T122 [P] [US5] `src/routes/workspace.rs` — `GET /proposals/{id}/file`
- [X] T123 [P] [US5] `src/routes/workspace.rs` — `POST /proposals/{id}/comments`, `POST` et `DELETE /proposal-comments/{id}/resolution`
- [X] T124 [P] [US5] `tests/espace_organisation.rs` — **balayage de la charge utile entière** : aucune note, aucun rang, aucun nom de membre du comité, aucun inscrit
- [X] T125 [P] [US5] `tests/espace_organisation.rs` — trois messages de visibilités différentes, **un seul** rendu au déposant
- [X] T126 [P] [US5] `tests/espace_organisation.rs` — résolution posée puis retirée, et le compteur de demandes ouvertes qui suit les deux gestes
- [X] T127 [P] [US5] `tests/espace_organisation.rs` — une personne étrangère à l'organisation est refusée

---

## Phase 8 — US6 : corriger et renvoyer (P2)

**Objectif** : rouvrir un dossier tel qu'il a été saisi, le corriger, le renvoyer même l'appel clos, **sans toucher à la séance**.

**Test indépendant** : rouvrir un dossier déposé deux mois plus tôt, comparer champ à champ, renvoyer sur un appel clos, vérifier la séance.

- [X] T128 [US6] `src/service/draft_read.rs` — la recomposition : textes ramenés au français, créneau en **heure murale du fuseau de l'édition**, chaque intervenant portant s'il a un compte (écart n° 39)
- [X] T129 [US6] `src/service/draft_read.rs` — **une seule implémentation**, partagée par les deux écrans : deux recompositions divergeraient au premier champ ajouté
- [X] T130 [US6] `src/service/draft_write.rs` — un dossier reste modifiable tant que **l'édition n'est pas terminée** ; rejeté, retiré ou annulé, il ne l'est plus (`PROPOSAL_NOT_EDITABLE`)
- [X] T131 [US6] `src/service/resubmit.rs` — le renvoi : **pas de contrôle de fenêtre**, plafond vérifié (écart n° 38)
- [X] T132 [US6] `src/service/draft_write.rs` — **ne propager AUCUN champ** vers une séance programmée, et le dire dans un commentaire à l'endroit où la tentation existe
- [X] T133 [P] [US6] `src/routes/submission.rs` — `GET /proposals/{id}/draft` et `POST /proposals/{id}/resubmit`
- [X] T134 [P] [US6] `tests/correction.rs` — la recomposition, champ à champ, y compris l'heure murale depuis un autre fuseau
- [X] T135 [P] [US6] `tests/correction.rs` — un renvoi sur appel **clos** aboutit ; le même dossier par la route de **dépôt** est refusé
- [X] T136 [P] [US6] `tests/correction.rs` — le plafond refuse aussi un renvoi excédentaire
- [X] T137 [P] [US6] `tests/correction.rs` — corriger un dossier en évaluation **ne change pas son état**
- [X] T138 [P] [US6] `tests/correction.rs` — corriger un dossier retenu laisse sa séance **strictement inchangée** : créneau, salle, inscrits, rappels

---

## Phase 9 — US7 : les pièces du dossier (P2)

**Objectif** : rattacher, lire, détacher — sans jamais détruire l'objet stocké.

**Test indépendant** : rattacher un objet déjà stocké, le lire depuis la fiche, le détacher, vérifier que l'objet demeure.

- [X] T139 [P] [US7] `src/repo/documents.rs` — rattacher et détacher, avec titre, type, ordre et caractère public
- [X] T140 [US7] `src/service/documents.rs` — rattacher un objet **déjà stocké**, jamais téléverser ; refuser un objet inconnu (`PROPOSAL_UNKNOWN_REFERENCE`)
- [X] T141 [US7] `src/service/documents.rs` — le détachement **ne détruit pas l'objet** : le module ne détruit pas ce qu'il n'a pas créé
- [X] T142 [P] [US7] `src/routes/detail.rs` — `GET`, `POST /proposals/{id}/documents` et `DELETE /proposals/{id}/documents/{document_id}`
- [X] T143 [P] [US7] `tests/pieces.rs` — rattachement, lecture, détachement, **et l'objet toujours là après**
- [X] T144 [P] [US7] `tests/pieces.rs` — une pièce interne ne sort sur aucune lecture publique ; une personne sans accès est refusée

---

## Phase 10 — US8 : l'historique, et les dossiers repris de la v1 (P3)

**Objectif** : une frise qui ne ment pas, y compris sans journal.

**Test indépendant** : vider le journal d'un dossier décidé, lancer la déduction, relancer.

- [X] T145 [P] [US8] `src/repo/proposals.rs` — l'historique champ par champ, par la fonction du modèle, **champs recalculés écartés**
- [X] T146 [US8] `src/service/backfill.rs` — semer les transitions **déductibles** — création, dépôt, décision — sur les seuls dossiers au journal **vide**, condition dans la même requête que l'insertion (R20, écart n° 37)
- [X] T147 [US8] `src/service/backfill.rs` — écrire dans le journal **sans passer par la mise à jour de l'état**, donc **sans réveiller le déclencheur** : une reprise ne doit pas émettre huit mille événements de décision
- [X] T148 [US8] `src/service/proposals.rs` — l'effacement **logique**, avec son auteur et son motif, **et la purge des thématiques** (écart n° 94)
- [X] T149 [P] [US8] `src/routes/admin_ops.rs` — `POST /admin/proposals/transitions-backfill`, **portée globale exigée**
- [X] T150 [P] [US8] `src/routes/detail.rs` — `GET /proposals/{id}/history` et `GET /proposals/{id}/themes`
- [X] T151 [P] [US8] `tests/reprise.rs` — trois lignes semées dans l'ordre, puis **zéro** à la seconde exécution, et **aucun événement émis**
- [X] T152 [P] [US8] `tests/reprise.rs` — un dossier effacé ne laisse **aucun lien de thématique** derrière lui

---

## Phase 11 — Finition et transverses

- [X] T153 [P] Un test qui frappe **les 37 routes** sur la vraie application — d'autant plus nécessaire que **deux préfixes sont partagés** : `tests/routes_programme.rs`
- [X] T154 [P] Vérifier qu'**aucun gestionnaire de travail différé** n'est enregistré par ce module, dans `tests/routes_programme.rs` — c'est une décision, pas un oubli (R20)
- [X] T155 [P] Compléter les annotations OpenAPI des 37 routes et vérifier que les six codes d'erreur y sont **engendrés** depuis le catalogue
- [X] T156 [P] Relire les tailles de fichier : **aucun au-dessus de 1000 lignes**. C'est le module où la limite se rapproche le plus — découper `service/desk.rs` si nécessaire
- [X] T157 [P] `cargo clippy --workspace --all-targets --all-features -- -D warnings` sans un avertissement, et `cargo fmt --all --check`
- [X] T158 `make check-back` au vert, puis **`make check` en entier** — il détruit le volume et recharge le schéma de zéro : à lancer en dernier
- [ ] T159 [P] Éprouver à la main les parcours du [quickstart](quickstart.md), en particulier le voile et le balayage de l'espace organisation, qu'aucun test d'écran ne prouve
- [X] T160 Mettre à jour la progression : journal du jour, `docs/progression/ecrans/b4-propositions.md`, décisions, ligne de suivi de `docs/PROGRESSION.md`

---

## Dépendances

```text
Phase 1 (T001–T006)
      ↓
Phase 2 (T007–T030)  ← BLOQUE TOUT LE RESTE
      ↓
      ├─ Phase 3  US1 dépôt          (T031–T062)
      │        ↓
      ├─ Phase 4  US2 machine à états (T063–T076)   ← a besoin d'un dossier à faire transiter
      │        ↓
      ├─ Phase 5  US3 liste           (T077–T093)
      │        ↓
      ├─ Phase 6  US4 fiche           (T094–T115)   ← a besoin de T064 pour available_transitions
      │        ↓
      ├─ Phase 7  US5 espace org      (T116–T127)   ← a besoin de T096 (fil filtré)
      ├─ Phase 8  US6 correction      (T128–T138)   ← a besoin de T036–T041 (l'écriture)
      ├─ Phase 9  US7 pièces          (T139–T144)   ← a besoin de T097
      └─ Phase 10 US8 historique      (T145–T152)   ← indépendante des autres histoires
                 ↓
Phase 11 Finition (T153–T160)
```

**Trois dépendances que le plan n'avait pas nommées, et qui se voient au découpage** :

1. **US2 ne s'éprouve pas sans US1.** Faire transiter un dossier suppose un dossier. L'inverse n'est pas vrai — un dépôt est une transition, mais elle passe par le déclencheur, que T042 tente sans connaître T064.
2. **US4 dépend d'une tâche d'US2** : `available_transitions` est alimenté par la requête de T064. C'est la seule arête entre deux histoires P1, et elle est étroite.
3. **US5 dépend d'une tâche d'US4** : le fil filtré par visibilité (T096) sert les deux côtés. L'écrire deux fois serait écrire deux filtres, et le second finirait par diverger — c'est exactement le défaut que le filtrage à la source doit empêcher.

**US8 est complètement indépendante** : elle peut se faire à tout moment après la phase 2, y compris en parallèle des histoires P1.

---

## Parallélisation

**Phase 2** — dix tâches de domaine (T010–T017) et les tests unitaires (T018) sont toutes `[P]` : dix fichiers distincts, aucune requête, rien de partagé. C'est la plus grosse fenêtre de parallélisme du découpage.

**Phase 3** — les cinq dépôts (T031–T035) en parallèle, puis les règles en séquence sur `draft_write.rs` (fichier commun), puis les onze tests (T052–T062) en parallèle.

**Phase 6** — les quatre dépôts (T094–T097) en parallèle, puis les services en séquence sur deux fichiers, puis les six tests (T110–T115) en parallèle.

**Toute phase de test est parallélisable en entier** : un fichier par histoire, aucune écriture partagée, chaque test montant sa propre base jetable.

**Ce qui ne se parallélise jamais** : T026, T027 et T028 se suivent — la refactorisation du préfixe, sa composition, et la preuve que rien n'est devenu muet.

---

## Stratégie de livraison

| Jalon | Tâches | Ce qui marche à ce moment-là |
|---|---|---|
| **Fondations** | T001–T030 | rien de visible, mais aucune histoire ne peut commencer avant |
| **🏁 Le dépôt** | T031–T076 | **une organisation dépose, corrige et retire ; la machine à états tient.** C'est le cœur du prompt |
| **🏁 Le comité** | T077–T115 | la liste, la fiche, le voile, la décision. **Les quatre histoires P1 sont livrées** |
| **Le reste** | T116–T152 | espace organisation, correction, pièces, reprise v1 |
| **Livrable** | T153–T160 | les 37 routes frappées, `make check` en entier, progression à jour |

**MVP** : le deuxième jalon. À T076, l'appel à propositions de la COP31 **reçoit des dossiers** — c'est ce que le prompt demande, et le comité peut encore travailler sur la v1 le temps que le troisième jalon arrive.

---

## Décompte

**160 tâches en 11 phases, dont 46 tâches de test et 97 parallélisables.** 37 routes, 6 codes d'erreur, 3 événements émis par le service, 0 travail différé, 1 crate créé, 1 dépendance nouvelle, 1 refactorisation touchant du code livré.

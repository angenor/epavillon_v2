---

description: "Task list — Organisations (B2)"
---

# Tasks: Organisations (B2)

**Input**: Design documents from `/specs/002-organisations/`

**Prerequisites**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: **OBLIGATOIRES.** Le principe X de la constitution impose des tests d'intégration sur base réelle et jetable, sans aucun mock de base. Les tâches de test ne sont pas optionnelles ici.

**Organization**: par histoire utilisateur, pour que chaque tranche soit implémentable, éprouvable et livrable seule.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichier différent, aucune dépendance en cours
- **[Story]** : `US1` … `US8`, correspondant aux histoires de `spec.md`

## Conventions de chemin

Tout vit dans `backend/`, workspace Cargo symétrique de `frontend/` — emplacements imposés par le principe II. **Aucun fichier de ce jalon ne vit hors de `backend/`**, à trois exceptions documentaires : `.env.example`, `docs/progression/` et deux en-têtes du front à corriger en finition.

---

## ⚠️ Deux choses à lire avant de commencer

### 1. La phase 2 touche du code LIVRÉ ET ÉPROUVÉ, et elle est bloquante

Le service de jetons à usage unique **remonte dans le noyau** (research.md § R8), et l'inscription de `identity` est **corrigée** (§ R9). Ce n'est pas un embellissement : trois des cinq finalités de jeton du modèle n'appartiennent pas à `identity` — l'invitation est le geste de CE module, la confirmation d'un intervenant sera celui de B4 — et aucun crate de module n'a le droit de dépendre d'un autre. Sans ce déplacement, US4 n'a que de mauvaises issues : recopier « consommer un jeton atomiquement », ou contredire le modèle.

**Le déplacement se fait à comportement constant.** Les tests de B1 sur la vérification d'adresse, la réinitialisation et le rejeu **ne sont pas réécrits** : ils sont la preuve, et T011 exige qu'ils restent verts.

### 2. L'ordre des deux écritures d'une fusion est l'INVERSE de celui inscrit en A11

`docs/progression/api.md` et l'en-tête de `frontend/app/types/admin-organizations.ts` disent « les choix de champ sont un `UPDATE` de la CIBLE, à faire **AVANT** `org.merge_organizations()` ». **C'est faux pour le nom légal**, qui est le champ le plus souvent arbitré : `ux_organizations_name_country` ne porte que sur les fiches **vivantes**, et tant que la fiche absorbée l'est encore, la survivante ne peut pas reprendre son nom.

Les arbitrages viennent donc **APRÈS** l'appel, **dans la même transaction** — la moitié qui comptait est conservée intacte (T106). Les deux documents fautifs sont corrigés en finition (T126).

---

## Phase 1 — Mise en place

**Objectif** : le crate existe, il compile, il ne dépend de personne, et la configuration refuse de démarrer si un réglage est mal écrit.

- [X] T001 Créer le crate `backend/crates/modules/org` (`Cargo.toml`, `src/lib.rs`) et l'ajouter aux `members` et aux `[workspace.dependencies]` de `backend/Cargo.toml`
- [X] T002 [P] Créer l'arborescence interne vide du crate — `src/domain/`, `src/repo/`, `src/service/`, `src/jobs/`, `src/routes/`, `tests/commun/` — sur la forme qu'`identity` a inaugurée
- [X] T003 [P] Ajouter à `.env.example` les trois réglages d'exploitation : `ORG_DUPLICATE_SCORE_THRESHOLD=60`, `ORG_DUPLICATE_SCAN_BATCH=200`, `ORG_SCORECARD_REFRESH_WINDOW=5m`
- [X] T004 [P] Étendre `backend/crates/kernel/src/config.rs` d'une section `OrgConfig` portant ces trois clés, **validée au démarrage** : un seuil hors de 0–175, une tranche nulle ou une durée illisible arrête le service, jamais une requête
- [X] T005 [P] Écrire le semis de démonstration de la paire de jumelles OSED dans `backend/crates/modules/org/tests/commun/seed.rs` — **hors de `docs/database/`**, ce n'est pas une donnée du modèle
- [X] T006 Vérifier que `platform.modules` porte déjà l'entrée `org` et que `backend/crates/api` ne monte ses routes que si le module y est actif
- [X] T007 Vérifier `cargo build`, `cargo fmt --check` et `cargo clippy -- -D warnings` sur le crate vide, base démarrée

---

## Phase 2 — Fondations (bloquantes)

**Objectif** : le service de jetons dans le noyau, l'inscription corrigée, les contrats d'événements, et le module monté dans les deux binaires. **Aucune histoire ne peut commencer avant.**

### Le service de jetons remonte dans le noyau (R8)

- [X] T008 Créer `backend/crates/kernel/src/tokens.rs` : `TokenPurpose`, `TokenRejection`, `IssuedToken`, l'expiration dérivée de la finalité par `TokenTtls` — **déjà présente dans `kernel::config` depuis B1**
- [X] T009 Y déplacer `create`, `invalidate_pending`, `consume`, `check` et `purge` depuis `backend/crates/modules/identity/src/repo/tokens.rs`, **à comportement constant** — aucune règle réécrite en chemin
- [X] T010 Réécrire `identity` pour appeler `kernel::tokens` : supprimer `repo/tokens.rs`, déplacer ce qui doit l'être de `domain/token.rs`, adapter `service/registration.rs`, `service/password_reset.rs` et `jobs/purge.rs` — **la purge récurrente reste une tâche d'`identity`**, c'est une opération d'exploitation
- [X] T011 Vérifier que les quatre tests de B1 restent verts **sans avoir été réécrits** : `identity/tests/jeton_verification.rs`, `reinitialisation.rs`, `rejeu_du_jeton.rs`, `mot_de_passe_refuse.rs`
- [X] T012 Corriger `backend/crates/modules/identity/src/service/registration.rs` (écart n° 81, R9) : une personne **connue mais sans compte** obtient un compte et un lien de vérification au lieu du rappel « vous avez déjà un compte ». **La réponse reste invariable et le hachage reste payé dans les deux cas**
- [X] T013 Écrire `backend/crates/modules/identity/tests/invitee_peut_creer_son_compte.rs` : une personne créée sans compte s'inscrit, obtient son lien, et se connecte

### Les contrats et l'ossature du module

- [X] T014 [P] Créer `backend/crates/contracts/src/org.rs` : les constantes des six types d'événements et leurs charges utiles, **avec le commentaire disant que `org.organization.merged` est émis par la base** (contracts/events.md)
- [X] T015 [P] Déclarer les identifiants typés par agrégat dans `backend/crates/modules/org/src/domain/ids.rs`
- [X] T016 [P] Déclarer les trois spécifications de permission — consulter, gérer, fusionner — dans `backend/crates/modules/org/src/domain/permissions.rs`, à côté de celles d'`identity`
- [X] T017 [P] Créer l'état partagé du module (pool, configuration) dans `backend/crates/modules/org/src/state.rs`
- [X] T018 Déclarer ce que le module expose — ses routes, ses travaux différés, aucun consommateur — dans `backend/crates/modules/org/src/lib.rs`
- [X] T019 Monter les routes dans `backend/crates/api` et enregistrer les travaux dans `backend/crates/worker`

### Le harnais des tests

- [X] T020 [P] Écrire le semis d'organisations **à distribution réaliste** dans `backend/crates/modules/org/tests/commun/mod.rs` (R3) : noms composés depuis un corpus francophone, de sorte que beaucoup de fiches partagent leurs premiers mots — 5 000 noms tirés au hasard rendraient la mesure excellente et fausse
- [X] T021 Écrire la fabrique de comptes et de périmètres d'administration — global, détaché sur une édition, aucun droit — dans `backend/crates/modules/org/tests/commun/mod.rs`

### Vérification des fondations

- [X] T022 Vérifier `cargo tree -p org` : **aucune arête** vers un autre crate de module
- [X] T023 Vérifier que `make check-back` passe en entier, tests de B1 compris

**✅ Point de contrôle** : le module existe, il est monté, et le socle des jetons sert les deux modules sans qu'aucun ne dépende de l'autre.

---

## Phase 3 — US1 : Trouver son organisation quoi qu'on tape (P1) 🎯 MVP

**Objectif** : les cinq façons de désigner une organisation ramènent la même fiche, et **les deux lectures rendent des résultats différents sur la même requête** — c'est l'écart n° 23, et c'est le cœur du module.

**Éprouvable seule** : sur l'IFDD, que `900_seed.sql` sème **avec ses cinq dénominations**, exercer sigle, début du nom complet, deux lettres, traduction et ancien nom ; puis comparer les deux lectures depuis un compte dont l'adresse porte le domaine d'une organisation.

- [X] T024 [P] [US1] Déclarer le résultat de recherche, ses motifs, la limite bornée (défaut 10, maximum 50) et la marge de sur-lecture dans `backend/crates/modules/org/src/domain/search.rs`
- [X] T025 [US1] Implémenter **les deux lectures au-dessus de la fonction du modèle**, déclarées côte à côte, dans `backend/crates/modules/org/src/repo/search.rs` : la filtrée (`WHERE 'name_similarity' = ANY(match_reasons)`) et la brute. **Documenter leur différence là, comme le prompt l'exige.** La fonction du modèle n'est pas modifiée
- [X] T026 [US1] Implémenter dans `backend/crates/modules/org/src/service/search.rs` : la **sur-lecture** (limite + 5), le filtre, la troncature, et la garde des deux caractères — sous deux caractères, **liste vide et non erreur** (R1, FR-013)
- [X] T027 [US1] Implémenter la lecture par identifiant et la liste bornée (défaut 50, maximum 200, fiches vivantes, triées par nom légal) dans `backend/crates/modules/org/src/repo/organizations.rs`
- [X] T028 [US1] Implémenter `GET /organizations/similar`, `GET /organizations` et `GET /organizations/{id}` dans `backend/crates/modules/org/src/routes/public.rs`, **session requise et rien de plus** (FR-014, écart n° 74)
- [X] T029 [US1] Implémenter `GET /admin/organizations/similar` — **non filtrée** — dans `backend/crates/modules/org/src/routes/admin.rs`, gardée par la permission de consultation et un périmètre non vide
- [X] T030 [US1] Écrire `backend/crates/modules/org/tests/recherche_multi_signaux.rs` : les cinq façons de désigner l'IFDD ramènent la même fiche, **une seule fois**, avec la dénomination qui a déclenché la correspondance
- [X] T031 [US1] Écrire `backend/crates/modules/org/tests/deux_lectures_de_recherche.rs` : sur **la même requête**, la lecture d'utilisateur et celle de revue rendent des résultats différents (SC-003, SC-004)
- [X] T032 [US1] Écrire `backend/crates/modules/org/tests/recherche_bornes.rs` : un caractère → liste vide ; `limit=100000` → borné à 50 ; une fiche absorbée ne remonte jamais et son ancien nom mène à la vivante
- [X] T033 [US1] Écrire `backend/crates/modules/org/tests/recherche_150ms.rs` : 5 000 fiches semées par T020, cent recherches de formes variées, **95ᵉ centile sous 150 ms**, et **le plan d'exécution rendu dans le message d'échec** (SC-002)
- [X] T034 [US1] **SANS OBJET — la mesure passe (95ᵉ centile à 55 ms, cible 150).** Si T033 ne passait pas : appliquer l'ordre des remèdes de research.md § R2 — réviser l'appel, augmenter la statistique sur la dénomination normalisée, **et ne proposer une modification du SQL qu'en dernier, avec la mesure comme justification écrite**
- [X] T035 [US1] Annoter les quatre routes pour l'OpenAPI engendré dans `backend/crates/modules/org/src/routes/public.rs` et `routes/admin.rs`, en portant **dans l'annotation** la différence entre les deux lectures

**✅ Point de contrôle** : chercher « institut », « IFDD », « IEPF » ou « in » ramène la même fiche — et le formulaire de rattachement ne propose plus de créer une fiche qui existe déjà. **C'est le défaut n° 1 de la v1, prévenu.**

---

## Phase 4 — US2 : Être rattaché sans intervention, ou attendre un référent (P1)

**Objectif** : le domaine décide, pas la volonté de la personne. Et une adhésion refusée peut se redemander.

**Éprouvable seule** : deux comptes, l'un sur `@ifdd.francophonie.org` (vérifié **et** en rattachement automatique dans le semis), l'autre sur `@francophonie.org` (vérifié, **non** marqué). Les deux issues doivent différer sans qu'on ait rien semé.

- [X] T036 [P] [US2] Déclarer les issues de rattachement, la direction d'une adhésion et les deux files dans `backend/crates/modules/org/src/domain/membership.rs`
- [X] T037 [US2] Implémenter « ce que révèle mon adresse » dans `backend/crates/modules/org/src/repo/domains.rs` : messageries grand public **lues en base** et jamais recopiées, fiche vérifiée d'abord puis celle en rattachement automatique
- [X] T038 [US2] Implémenter `GET /organizations/by-email-domain` dans `routes/public.rs` — le domaine vient de **la session**, le paramètre du client est **ignoré** (FR-017, écart n° 75)
- [X] T039 [US2] Implémenter la demande de rattachement en **un unique ordre** `INSERT … ON CONFLICT (organization_id, person_id) DO UPDATE … WHERE status = 'revoked'` dans `backend/crates/modules/org/src/repo/memberships.rs` (R7, écart n° 72)
- [X] T040 [US2] Implémenter `backend/crates/modules/org/src/service/join.rs` : résolution de la fiche absorbée par la fonction du modèle, calcul de l'issue depuis le domaine, émission de `org.membership.requested`. **Ne jamais calculer `is_primary`** — la base l'attribue (FR-025)
- [X] T041 [US2] Implémenter `POST /organizations/{id}/members` et `GET /people/{id}/memberships` dans `backend/crates/modules/org/src/routes/memberships.rs` — la seconde ouverte à soi-même, ou à la permission de consultation des utilisateurs
- [X] T042 [US2] Écrire `backend/crates/modules/org/tests/rattachement_automatique.rs` : domaine vérifié et marqué → **active** ; domaine vérifié non marqué → **en attente** ; deux fiches sur un domaine → la vérifiée l'emporte
- [X] T043 [US2] Écrire `backend/crates/modules/org/tests/adhesion_revoquee_puis_redemandee.rs` : refuser, redemander, refuser encore — **jamais plus d'une ligne** par (organisation, personne) (SC-008)
- [X] T044 [US2] Écrire `backend/crates/modules/org/tests/messagerie_grand_public.rs` : une adresse Gmail ne propose rien, et deux organisations ne se rapprochent pas pour autant
- [X] T045 [US2] Vérifier qu'une demande visant une fiche **absorbée** ouvre l'adhésion sur la fiche vivante (FR-024)

**✅ Point de contrôle** : une personne rejoint son organisation — d'office si son domaine le prouve, en attendant un référent sinon.

---

## Phase 5 — US3 : Créer une fiche sans en fabriquer une deuxième (P1)

**Objectif** : rien n'est bloqué, sauf le doublon exact que la base refuse — et ce refus nomme la fiche existante.

**Éprouvable seule** : créer une fiche en ayant vu des fiches proches, puis créer une fiche portant un nom déjà pris dans le même pays.

- [X] T046 [P] [US3] Déclarer les deux issues de création et la composition de l'adresse d'URL — avec **repli quand la normalisation efface tout le nom** — dans `backend/crates/modules/org/src/domain/organization.rs`
- [X] T047 [US3] Implémenter l'insertion dans `repo/organizations.rs` : adresse composée par `platform.slugify`, collision suffixée et **rejouée une fois** puis erreur interne
- [X] T048 [US3] Implémenter `backend/crates/modules/org/src/service/create.rs` : statut `candidate` **jamais `active`**, conservation des fiches proches montrées, adhésion **référent active** pour le créateur, émission de `org.organization.created`
- [X] T049 [US3] Traduire `ux_organizations_name_country` en discriminant `name_taken` **portant la fiche en cause** sous forme de résultat de recherche, dans `backend/crates/modules/org/src/service/create.rs`
- [X] T050 [US3] Implémenter `POST /organizations` dans `routes/public.rs`, rendant **200** pour les deux issues
- [X] T051 [US3] Écrire `backend/crates/modules/org/tests/creation_et_doublon_exact.rs` : `name_taken` en 200 avec la fiche, et une simple ressemblance qui **ne bloque rien**
- [X] T052 [US3] Écrire `backend/crates/modules/org/tests/creation_concurrente.rs` : cent créations simultanées du même nom dans le même pays → **une seule fiche** (SC-005)
- [X] T053 [US3] Écrire `backend/crates/modules/org/tests/creation_adresse_url.rs` : un nom que la normalisation efface produit tout de même une adresse valide ; deux noms voisins ne se heurtent pas
- [X] T054 [US3] Vérifier que le créateur est **référent**, que la fiche devient son rattachement principal s'il n'en avait pas, et que le nom légal et le sigle sont devenus cherchables **sans écriture du service**

**✅ Point de contrôle** : **l'écran de rattachement (A2) est entièrement servi.** On cherche, on rejoint, on crée — et le doublon se prévient au lieu de se réparer.

---

## Phase 6 — US4 : Deux files d'attente qui ne se confondent jamais (P2)

**Objectif** : un référent tranche ce qu'il a reçu ; une personne accepte ce qu'on lui a envoyé. Et un refus révoque au lieu d'effacer.

**Éprouvable seule** : inviter une adresse inconnue, tenter de l'approuver comme référent (refus), suivre le lien (acceptation), puis s'inscrire avec cette même adresse.

- [X] T055 [P] [US4] Compléter `domain/membership.rs` : la charge utile d'une invitation, la décision d'un référent, les trois issues d'invitation
- [X] T056 [US4] Implémenter dans `repo/memberships.rs` : création de la personne **sans compte**, ouverture de l'adhésion portant `invited_by` et `invited_at`, et les deux lectures de file — celle du référent et celle de la personne
- [X] T057 [US4] Implémenter l'invitation dans `backend/crates/modules/org/src/service/membership.rs` : personne créée **sans nom déduit de l'adresse**, jeton de finalité invitation par `kernel::tokens`, courriel mis en file — **le jeton en clair ne vit que dans la charge utile du travail**
- [X] T058 [US4] Traduire une seconde invitation en discriminant `already_invited` **avant** que la contrainte d'unicité ne remonte, dans `backend/crates/modules/org/src/service/membership.rs` (FR-039)
- [X] T059 [US4] Implémenter la décision d'un référent dans `backend/crates/modules/org/src/service/membership.rs` : **refus explicite si l'adhésion est une invitation** (`ORG_MEMBERSHIP_IS_INVITATION`), approbation qui pose auteur et date, refus qui **révoque sans supprimer**
- [X] T060 [US4] Implémenter l'acceptation d'une invitation par jeton dans `backend/crates/modules/org/src/service/membership.rs` — **sans session exigée** (R10), correspondance vérifiée si une session existe, et **adresse marquée vérifiée** puisque le lien vient de la prouver
- [X] T061 [US4] Implémenter la révocation d'une adhésion dans `backend/crates/modules/org/src/service/membership.rs`, avec le **refus du retrait du dernier référent actif** (`ORG_LAST_MANAGER`), contournable par la permission de gestion des organisations (FR-041)
- [X] T062 [US4] Implémenter les quatre routes d'adhésion dans `routes/memberships.rs` : invitation, décision, acceptation, révocation
- [X] T063 [US4] Implémenter les trois courriels dans `backend/crates/modules/org/src/jobs/emails.rs` par le contrat d'envoi du noyau — **identifiant du message réservé AVANT l'envoi**, le doublon réel étant concurrent (piège de B1)
- [X] T064 [US4] Émettre `org.membership.requested` (portant la direction), `org.membership.approved` et `org.membership.revoked` **dans la transaction** de chaque changement d'état
- [X] T065 [US4] Écrire `backend/crates/modules/org/tests/adhesions_deux_files.rs` : aucune invitation approuvable par l'organisation, aucune demande acceptable par un jeton — sur toutes les combinaisons (SC-007)
- [X] T066 [US4] Écrire `backend/crates/modules/org/tests/invitation_acceptee_par_jeton.rs` : sans session ; rejeu du lien → « déjà utilisé » ; session d'une autre personne → refus
- [X] T067 [US4] Écrire `backend/crates/modules/org/tests/dernier_referent.rs` : le retrait est refusé, puis accepté après désignation d'un remplaçant, puis forcé par un administrateur
- [X] T068 [US4] **FAIT au navigateur le 20/08.** Invitation émise par une référente → travail en file avec son jeton → worker → relais du site → **courriel lu dans Mailpit**, sujet portant le nom de l'organisation, « Bonjour Invité·e » (le nom n'est pas déduit de l'adresse), lien et durée de sept jours conformes. Le **premier essai a échoué** en `MAIL_RELAY_UNREACHABLE` (front pas encore prêt) et **la reprise a réussi** — la chaîne d'essais fonctionne. Acceptation **sans session** : adhésion active, adresse marquée vérifiée, rejeu refusé « déjà utilisé ». Éprouver la chaîne du courriel à la main jusqu'à **Mailpit** : invitation reçue, lien suivi, adhésion active — worker arrêté puis relancé, **un seul courriel**

**✅ Point de contrôle** : **l'espace organisation (A5) a sa moitié adhésions.** Une organisation se constitue sans passer par l'IFDD.

---

## Phase 7 — US5 : Le back-office ne voit que ce qu'on lui a confié (P2)

**Objectif** : la règle métier n° 8 tenue sur une entité qui n'appartient à aucune édition.

**Éprouvable seule** : trois comptes — global, détaché sur une édition, sans droit — et une URL forgée.

- [X] T069 [P] [US5] Déclarer les formes de la liste et de la fiche — facettes, drapeau de restriction, compteurs de doublons — dans `backend/crates/modules/org/src/domain/admin.rs`
- [X] T070 [US5] Implémenter la liste **en une requête** dans `backend/crates/modules/org/src/repo/admin_list.rs` : projection analytique, **statut, sceau, score de confiance et pointeur de fusion relus sur la table vivante** (FR-048), filtre de périmètre par activité déposée ou tenue, facettes comptées sur le même jeu (R14)
- [X] T071 [US5] Implémenter la garde dans `backend/crates/modules/org/src/service/admin_list.rs` : permission de consultation **sur une portée quelconque ET périmètre non vide**. Les trois cas restent distincts — global, éditions listées, **aucun droit → refus explicite** (FR-043, écart n° 73)
- [X] T072 [US5] Implémenter les huit lectures de la fiche **dans une seule transaction de lecture** dans `backend/crates/modules/org/src/repo/admin_detail.rs` (R15) : identité, fiche de performance, dénominations, domaines et leurs partages, membres, activités, historique, fusions, paires ouvertes
- [X] T073 [US5] Implémenter `backend/crates/modules/org/src/service/admin_detail.rs` : une fiche **absorbée** s'ouvre normalement avec son renvoi ; une fiche **hors périmètre** rend un refus **indiscernable d'une fiche inexistante**
- [X] T074 [US5] Implémenter `GET /admin/organizations` et `GET /admin/organizations/{id}` dans `routes/admin.rs`
- [X] T075 [US5] Écrire `backend/crates/modules/org/tests/back_office_liste_et_fiche.rs` : les trois périmètres, le drapeau de restriction, et les facettes comptées sur le jeu affiché
- [X] T076 [US5] Écrire `backend/crates/modules/org/tests/perimetre_organisation_url_forgee.rs` — **obligation n° 2 du principe X** (SC-009)
- [X] T077 [US5] Écrire `backend/crates/modules/org/tests/perimetre_vide_refuse.rs` : un refus, **jamais une liste vide**
- [X] T078 [US5] Écrire `backend/crates/modules/org/tests/permission_ordinaire_ne_suffit_pas.rs` : détenir la permission de consultation **sans** périmètre n'ouvre pas le back-office (écart n° 73)
- [X] T079 [US5] Vérifier qu'une fiche absorbée est **consultable** et porte son renvoi, et que l'historique rend le libellé d'auteur dénormalisé

**✅ Point de contrôle** : la liste et la fiche du back-office (A11) sont servies, bornées, et une URL forgée ne mène nulle part.

---

## Phase 8 — US6 : Poser le sceau, vérifier un domaine, confirmer une dénomination (P2)

**Objectif** : ce qui fait passer une fiche de « en attente de rapprochement » à fiche de référence — et ce qui alimente le rattachement automatique de US2.

**Éprouvable seule** : poser le sceau sur une fiche `candidate`, puis vérifier un domaine déjà vérifié ailleurs.

- [X] T080 [P] [US6] Déclarer les charges utiles des trois écritures et leur issue commune dans `domain/admin.rs`
- [X] T081 [US6] Implémenter la pose et le retrait du sceau dans `repo/organizations.rs` : poser **admet du même geste** une fiche `candidate` ; retirer **ne change pas** le statut
- [X] T082 [US6] Implémenter la vérification manuelle d'un domaine et le réglage du rattachement automatique dans `repo/domains.rs` — méthode `manual` seule, les deux autres restant hors jalon
- [X] T083 [US6] Traduire `ux_organization_domains_verified` en discriminant `domain_taken` **nommant la fiche qui détient le domaine**, dans `backend/crates/modules/org/src/service/admin_write.rs` — sans ce nom, le refus est incompréhensible
- [X] T084 [US6] Traduire `ck_domain_autojoin_requires_verification` en `ORG_DOMAIN_VERIFICATION_REQUIRED`, champ `auto_join`, dans `backend/crates/modules/org/src/service/admin_write.rs`
- [X] T085 [US6] Implémenter la confirmation d'une dénomination dans `backend/crates/modules/org/src/repo/names.rs`, et **refuser le retrait d'une dénomination posée par la base** (`ORG_NAME_IS_DERIVED`)
- [X] T086 [US6] Implémenter `backend/crates/modules/org/src/service/admin_write.rs` : **la fiche entière recomposée** après chaque écriture, et mise en file du recalcul de score et du rafraîchissement de la projection
- [X] T087 [US6] Implémenter les trois routes `PUT` du back-office dans `routes/admin.rs`, gardées par la permission de **gestion**
- [X] T088 [US6] Écrire `backend/crates/modules/org/tests/sceau_et_statut.rs` : le sceau admet une fiche `candidate`, son retrait ne la déclasse pas
- [X] T089 [US6] Écrire `backend/crates/modules/org/tests/domaine_deja_verifie_ailleurs.rs` — **obligation n° 3 du principe X** : l'invariant de la base traduit, et le refus qui **nomme** la fiche

**✅ Point de contrôle** : l'IFDD reconnaît une organisation, ouvre son domaine au rattachement automatique, et la fiche entière revient à l'écran après chaque geste.

---

## Phase 9 — US7 : La file des doublons se remplit toute seule (P2)

**Objectif** : personne ne va chercher les doublons à la main, et le score de confiance suit ce qu'on vient de faire.

**Éprouvable seule** : semer deux fiches proches, déclencher le balayage, écarter la paire, rejouer le balayage.

- [X] T090 [P] [US7] Implémenter le balayage **par tranches avec curseur** dans `backend/crates/modules/org/src/jobs/duplicates.rs` : chaque exécution pose la suivante, la dernière planifie le lendemain, la clé d'unicité porte le jour et le curseur (R11)
- [X] T091 [US7] Implémenter l'enregistrement d'une paire dans `backend/crates/modules/org/src/repo/duplicates.rs` : ordre par `LEAST`/`GREATEST`, `ON CONFLICT … DO UPDATE … WHERE reviewed_at IS NULL` — **une paire arbitrée n'est jamais ressuscitée, une paire en attente est mise à jour** (FR-059)
- [X] T092 [US7] Implémenter la file et ses deux sections — en attente, déjà tranchées, triées par similarité décroissante — dans `backend/crates/modules/org/src/repo/duplicates.rs`
- [X] T093 [US7] Implémenter les décisions dans `backend/crates/modules/org/src/service/duplicates.rs` : `distinct` retire pour de bon, `deferred` remet à plus tard, et une paire reportée se remet en circulation
- [X] T094 [US7] Implémenter `GET /admin/organizations/duplicates` et `PUT /admin/organizations/duplicates/{pairId}` dans `routes/admin.rs`, exigeant la permission de fusion **en portée globale** — un administrateur détaché n'y accède **pas du tout**
- [X] T095 [P] [US7] Implémenter le recalcul du score dans `backend/crates/modules/org/src/jobs/trust_score.rs` : clé d'unicité **par organisation**, écriture **seulement si la valeur change**, **sans acteur** (R12)
- [X] T096 [P] [US7] Implémenter le rafraîchissement de la projection dans `backend/crates/modules/org/src/jobs/scorecard.rs` : **en concurrence, hors transaction**, coalescé sur une fenêtre (R13)
- [X] T097 [US7] Réarmer la chaîne du balayage au démarrage de `backend/crates/worker`, au cas où sa dernière occurrence serait morte avant d'avoir posé la suivante
- [X] T098 [US7] Écrire `backend/crates/modules/org/tests/balayage_ne_ressuscite_pas.rs` : dix passages, **aucune paire en double, aucune paire écartée ramenée** (SC-013)
- [X] T099 [US7] Écrire `backend/crates/modules/org/tests/score_de_confiance_coalesce.rs` : cent approbations d'adhésion → **un seul recalcul**, et un domaine vérifié se voit au premier rechargement (SC-014)

**✅ Point de contrôle** : la file du back-office se remplit sans intervention, et le score de confiance cesse d'être une colonne que personne n'écrit.

---

## Phase 10 — US8 : Fusionner sans rien perdre, et sans rien casser (P2)

**Objectif** : le quatrième verrou du modèle, et la seule réponse aux doublons déjà créés. **C'est l'opération la plus dangereuse du module : rien ne l'annule d'un clic.**

**Éprouvable seule** : fusionner la paire OSED et vérifier, chiffre par chiffre, que le décompte annoncé avant est celui rendu après.

- [X] T100 [P] [US8] Déclarer dans `backend/crates/modules/org/src/domain/merge.rs` : les dix champs comparés, les six avertissements, les trois sorts de transfert, et **le champ non arbitrable**
- [X] T101 [US8] Implémenter `backend/crates/modules/org/src/repo/merge_counts.rs` — **le seul fichier du module qui compose son SQL** : une requête `UNION ALL`, une branche par ligne du registre `org.organization_references`, identifiants cités par `quote_ident` (R4). Y écrire pourquoi, en tête
- [X] T102 [US8] Implémenter dans `backend/crates/modules/org/src/repo/merge.rs` : lecture des deux fiches réduites à ce qui permet de trancher, dénominations et domaines apportés avec leur présence côté cible, et **relecture du journal des fusions** dans la transaction
- [X] T103 [US8] Implémenter l'aperçu **pour un sens donné** dans `backend/crates/modules/org/src/service/merge.rs`, **recalculé à l'inversion** — le décompte n'est pas symétrique
- [X] T104 [US8] Implémenter les six avertissements dans `backend/crates/modules/org/src/service/merge.rs`, au premier rang « la fiche absorbée porte le sceau et la survivante ne l'a pas » — **non bloquants** : l'écran ne décide pas à la place de l'équipe
- [X] T105 [US8] Revérifier le **nom de confirmation** par `platform.normalize_label` — casse et accents ignorés, sigle accepté au même titre que le nom légal. Masquer un bouton n'a jamais empêché une requête
- [X] T106 [US8] Implémenter **l'ordre des écritures** dans la même transaction : contrôles → **appel de `org.merge_organizations()`** → **PUIS** arbitrages de champ sur la fiche survivante → relecture du décompte. **Inverser cet ordre fait échouer toute fusion arbitrant le nom légal** (R5, écart n° 70)
- [X] T107 [US8] Refuser un arbitrage portant sur l'adresse d'URL par `ORG_MERGE_FIELD_NOT_ARBITRABLE`, **champ nommé**, dans `backend/crates/modules/org/src/service/merge.rs` (R6, écart n° 71)
- [X] T108 [US8] **N'émettre AUCUN événement et ne marquer AUCUNE paire** après l'appel — la fonction de base fait les deux. Écrire le commentaire **à l'endroit où l'on serait tenté d'ajouter la ligne**, dans `backend/crates/modules/org/src/service/merge.rs` (écart n° 76)
- [X] T109 [US8] **Relever sur la base les SQLSTATE réels** des trois exceptions de `docs/database/040_organizations.sql`, puis écrire leur traduction dans `backend/crates/kernel/src/pg_error.rs` — B1 a payé une fois d'avoir recopié un code depuis un document au lieu de le mesurer
- [X] T110 [US8] Exiger la permission de fusion **en portée globale** (`ORG_MERGE_GLOBAL_SCOPE_REQUIRED`) dans `backend/crates/modules/org/src/service/merge.rs` : il n'existe pas de fusion limitée à une édition
- [X] T111 [US8] Implémenter `GET /admin/organizations/{id}/merge-preview` et `POST /admin/organizations/merge` dans `routes/admin.rs`
- [X] T112 [US8] Écrire `backend/crates/modules/org/tests/fusion_complete.rs` : la fiche absorbée survit et pointe vers la vivante, la paire passe « fusionnée », **l'ancien nom trouve toujours la bonne fiche** (SC-011)
- [X] T113 [US8] Écrire `backend/crates/modules/org/tests/decompte_de_fusion_exact.rs` : l'aperçu et le journal comparés **ligne de registre par ligne de registre**, écart de zéro (SC-010)
- [X] T114 [US8] Écrire `backend/crates/modules/org/tests/fusion_arbitrage_apres_lappel.rs` : une fusion **arbitrant le nom légal** aboutit — elle échouerait sur une violation d'unicité si l'ordre était inversé
- [X] T115 [US8] Écrire `backend/crates/modules/org/tests/fusion_arbitrage_annule_tout.rs` : un arbitrage qui échoue ne laisse **ni fiche absorbée, ni rattachement déplacé, ni ligne au journal** (SC-012)
- [X] T116 [US8] Écrire `backend/crates/modules/org/tests/outbox_une_seule_fusion.rs` — **obligation n° 4 du principe X**. Il **compte** les événements, il ne vérifie pas leur présence
- [X] T117 [US8] Écrire `backend/crates/modules/org/tests/fusion_cible_deja_fusionnee.rs` : le message du trigger ressort **mot pour mot**, « Cibler la fiche finale »

**✅ Point de contrôle** : **le défaut n° 1 de la v1 est pris par les quatre bouts.** Ce qui a été créé en double se répare sans rien perdre, et la reprise des données de la v1 aura son outil.

---

## Phase 11 — Finition et points transverses

- [X] T118 [P] Annoter les 21 chemins auprès de leur gestionnaire et assembler la documentation dans `backend/crates/modules/org/src/routes/openapi.rs` — OpenAPI **engendré**, jamais écrit à la main
- [X] T119 [P] Enregistrer les onze codes d'erreur du module dans le catalogue de `backend/crates/kernel/src/error.rs`, pour qu'ils apparaissent seuls dans la documentation
- [X] T120 Vérifier `GET /api/docs` : **21 chemins** et **11 codes** de ce module — un code oublié n'existe pas
- [X] T121 [P] Régénérer `.sqlx/` par `cargo sqlx prepare --workspace -- --all-targets --all-features` — sans `--all-targets`, les requêtes des tests manquent et la construction hors ligne échoue
- [X] T122 Vérifier qu'aucun fichier de `backend/` ne dépasse **1000 lignes**
- [X] T123 Vérifier `cargo tree -p org` : **zéro arête** entre deux crates de module
- [X] T124 `cargo fmt --all` puis `make check` **en entier depuis la racine** — base détruite et rechargée de zéro. **Passe : 249 tests, clippy sans avertissement, `nuxt typecheck` et `npm run build` verts.** Le typecheck a d'abord dû être réparé : `vue-router` était épinglé sur la branche 4 quand Nuxt 4.5 dépend de la 5, et `vue-tsc` échouait à lire sa configuration avant d'avoir vérifié le moindre type. **Rappel : le stockage objet est reparti de zéro — relancer `make garage-init`.**
- [X] T125 **FAIT — API, worker et navigateur.** Au navigateur (20/08) : Mailpit et le courriel d'invitation, l'accueil, la garde de connexion, l'espace organisation avec sa file d'adhésions, la liste du back-office **avec ses facettes comptées** et sa pastille de doublons, le refus « accès refusé — droit nécessaire : consultation des organisations » (jamais une liste vide), et la file des doublons avec la paire OSED et ses quatre motifs. **Deux constats consignés pour B7** : le lien d'invitation mène à une page inexistante, et le bouton « Examiner la fusion » ne navigue pas. Le site tourne toujours sur ses données simulées — il n'éprouve donc pas l'API, qui l'est par curl. **L'API et le worker sont éprouvés pour de vrai** (inscription, vérification, connexion, recherche sur l'ancien sigle, rattachement d'office, création, doublon exact nommé, refus de fusion sans portée globale ; worker : réarmement, balayage par tranches, scores, projection). **Le navigateur reste à faire.** Éprouver les huit histoires **à la main**, `quickstart.md` en main, depuis un vrai navigateur et Mailpit
- [X] T126 [P] Corriger les deux documents qui portent l'**ancien ordre** des écritures de la fusion : `docs/progression/api.md` et l'en-tête de `frontend/app/types/admin-organizations.ts`
- [X] T127 [P] Consigner pour **B7** : retirer les deux filtres de `match_reasons` devenus inertes (`frontend/app/pages/organization/join.vue`, `frontend/app/components/proposal/StepOrganizations.vue`), et retirer l'adresse d'URL des champs arbitrables de l'écran de fusion
- [X] T128 [P] Mettre à jour la progression — journal du jour, `docs/progression/ecrans/b2-organisations.md`, décisions, lignes de suivi dans `docs/PROGRESSION.md` et `docs/progression/api.md`
- [X] T129 Noter dans `docs/progression/pieges.md` les trois pièges du module : `merge_organizations()` émet **déjà** son événement et marque **déjà** la paire ; l'unicité des adhésions **ignore** le statut, donc une lecture suivie d'une écriture perd la course ; et l'unicité du nom ne portant que sur les fiches **vivantes**, l'ordre des deux écritures d'une fusion n'est pas interchangeable

---

## Dépendances

```
Phase 1 — Mise en place
      ▼
Phase 2 — Fondations  ⚠️ bloquante, ET elle touche du code livré (jetons, inscription)
      ▼
   ┌──────────────────────────────────────────────┐
   │  US1 recherche (P1)                          │  ← MVP
   │      ▼                                       │
   │  US2 rattachement (P1)                       │
   │      ▼                                       │
   │  US3 création (P1)                           │
   └──────────────────────────────────────────────┘
      ▼
   ┌──────────────────────────────────────────────┐
   │  US4 adhésions              [indépendante]   │
   │  US5 back-office ──► US6 écritures de fiche  │   (P2)
   │  US7 détection continue     [indépendante]   │
   └──────────────────────────────────────────────┘
      ▼
   US8 fusion (P2) — a besoin de US5 (la fiche) et de US7 (la file)
      ▼
Phase 11 — Finition
```

**Ce qui dépend vraiment de quoi** :

- **US2 après US1** : rejoindre suppose d'avoir trouvé. Et la lecture « ce que révèle mon adresse » réutilise la lecture des domaines que US1 pose.
- **US3 après US2** : la création ouvre une adhésion de référent — c'est la mécanique de US2, avec un rôle différent.
- **US4 après la phase 2 seulement**, mais **impossible sans elle** : l'invitation passe par le service de jetons remonté dans le noyau (T008–T011).
- **US6 après US5** : chaque écriture rend **la fiche entière recomposée**, qu'il faut donc savoir composer.
- **US8 après US5 et US7** : l'aperçu réutilise la fiche réduite de US5, et la file de US7 est ce qui amène une paire à fusionner. Une fusion « à la main » reste possible sans la file, mais l'écran part de là.
- **US7 est indépendante** de US4, US5 et US6 : elle ne parle qu'au worker.

---

## Ce qui peut tourner en parallèle

**Phase 2** — huit fichiers sans dépendance mutuelle, une fois les jetons déplacés :

```
T014 contrats · T015 identifiants · T016 permissions · T017 état
T020 semis réaliste — **T021 suit, même fichier**
```

**Phase 3** — le domaine avant le dépôt : `T024` seul, puis `T025` et `T027` de front.

**Phases 6 à 9, une fois le socle P1 fini** — trois chantiers menés de front, sur des fichiers disjoints :

```
US4         adhésions        service/membership.rs, jobs/emails.rs, routes/memberships.rs
US5 + US6   back-office      repo/admin_list.rs, admin_detail.rs, service/admin_*.rs
US7         travaux de fond  jobs/duplicates.rs, trust_score.rs, scorecard.rs
```

**Phase 10** — `T100`, `T101` et `T102` sur trois fichiers distincts, avant que `T103` ne les assemble.

**Phase 11** — `T118 · T119 · T121 · T126 · T127 · T128` sans ordre imposé.

---

## Stratégie de livraison

**Le plus petit incrément qui vaille** : phases 1 à 3 — **T001 à T035**. Chercher « institut », « IFDD », « IEPF » ou deux lettres ramène la même fiche, et les deux lectures rendent enfin des résultats différents. **C'est le défaut n° 1 de la v1, prévenu**, et c'est démontrable en une minute sur la fiche que le semis fournit déjà.

**Le parcours de rattachement complet** : jusqu'à la phase 5 — **T001 à T054**. L'écran A2 est entièrement servi : on cherche, on rejoint — d'office ou en attente —, on crée sans fabriquer un doublon.

**Le module utile au back-office** : jusqu'à la phase 9 — **T001 à T099**. La liste, la fiche, le sceau, les domaines, et la file qui se remplit toute seule. À ce point, l'équipe peut tenir le référentiel au quotidien.

**Le module complet** : jusqu'à la phase 11 — **T001 à T129**. La fusion, et donc la réparation de ce que la v1 a laissé.

**Ce qu'on ne livre pas, et c'est écrit** : la vérification d'un domaine par enregistrement DNS ou par courriel (hors jalon, le contrat du front l'annonce déjà), l'affichage du nom traduit d'une organisation (question n° 2 en attente auprès du commanditaire), la route composée de l'espace organisation (elle appartient à B4), et une table d'alias d'adresses d'URL (besoin non prouvé).

---

## Récapitulatif

| Phase | Histoire | Priorité | Tâches | Dont tests |
|---|---|---|---|---|
| 1 | Mise en place | — | T001–T007 | — |
| 2 | Fondations — jetons dans le noyau, inscription corrigée | — | T008–T023 | 2 |
| 3 | US1 — recherche | **P1** | T024–T035 | 4 |
| 4 | US2 — rattachement | **P1** | T036–T045 | 3 |
| 5 | US3 — création | **P1** | T046–T054 | 3 |
| 6 | US4 — adhésions, les deux files | P2 | T055–T068 | 3 |
| 7 | US5 — back-office, lecture bornée | P2 | T069–T079 | 4 |
| 8 | US6 — écritures de la fiche | P2 | T080–T089 | 2 |
| 9 | US7 — détection continue et score | P2 | T090–T099 | 2 |
| 10 | US8 — fusion | P2 | T100–T117 | 6 |
| 11 | Finition | — | T118–T129 | — |
| | | | **129 tâches** | **29 tests** |

Les quatre obligations minimales du principe X sont couvertes par **T030/T075** (chemin nominal), **T076** (URL forgée), **T089 et T117** (invariants de la base traduits, dont un message repris mot pour mot) et **T116** (écriture dans l'outbox — **et il compte**).

Les critères de réussite qu'aucun autre test ne tiendrait : **T033** (SC-002, les 150 ms), **T031** (SC-003 et SC-004, les deux lectures), **T043** (SC-008), **T052** (SC-005), **T098** (SC-013), **T099** (SC-014), **T113** (SC-010), **T115** (SC-012).

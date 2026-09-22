---
description: "Tâches de l'étape 0c — thématiques, « Ma journée » et profil"
---

# Tasks: Guide Négo — thématiques, « Ma journée » et profil (étape 0c)

**Input** : les documents de `/specs/010-guide-nego-accueil-profil/`

**Prérequis** : [plan.md](plan.md) · [spec.md](spec.md) · [research.md](research.md) ·
[data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

**Tests** : **oui, et énumérés par les contrats** — huit tests d'API sur base réelle
([api-thematiques.md § 4](contracts/api-thematiques.md)), six tests de file sans navigateur
([hors-connexion.md § 2](contracts/hors-connexion.md)), deux contrôles distincts pour les textes
([api-textes.md § 4](contracts/api-textes.md)).

**Branche** : `010-guide-nego-accueil-profil` · **Un commit par phase.**

## Format : `[ID] [P?] [Récit] Description`

- **[P]** : parallélisable — fichiers différents, aucune dépendance sur une tâche non finie.
- **[US1]…[US4]** : le récit servi. Les phases 1 à 4 et 9 n'en portent pas : elles servent tout le monde.
- Chaque tâche nomme son fichier.

## Les portes, et celle qu'on ne franchit pas

Pendant le cycle : `npm run typecheck`, `npm run test:guide-nego`, `npm run check:guide-nego`,
`cargo test -p negotiation`, `cargo test -p kernel`, `make openapi`, `make check-api-contract`.

**En fin de cycle seulement, API arrêtée** : `make check-safe`.

> **`make check` et `make check-db` ne se lancent jamais** : ils détruisent la base locale, qui n'a pas
> de sauvegarde. La base **se migre**.

---

## Phase 1 : Setup — faire de la place, et rien d'autre

**Objectif** : `frontend/app/composables/useApi.ts` est à 973 lignes pour un garde-fou à 1000. Le bloc
`auth` sort **avant** toute autre écriture.

**Commit à part, le premier de l'étape. Le diff doit se lire comme un déplacement** : aucune méthode
nouvelle, aucune signature retouchée, aucun comportement changé.

- [ ] T001 Créer `frontend/app/composables/api/auth.ts` : `createAuthApi(deps)` portant les neuf méthodes déplacées telles quelles — `login`, `logout`, `session`, `register`, `verifyEmail`, `resendVerification`, `requestPasswordReset`, `checkPasswordResetToken`, `resetPassword` —, avec les types de `~/types/auth` qu'elles emploient
- [ ] T002 Retirer le bloc `auth` et son commentaire de `frontend/app/composables/useApi.ts`, y monter `const auth = createAuthApi(deps)` **avant le littéral de retour** (l'ordre compte : `createGuideNegoApi({ auth, … })` en a besoin), et élaguer les imports de types devenus inutiles
- [ ] T003 Vérifier que **aucun des 14 appelants de `api.auth.*` n'est modifié** — `stores/auth.ts`, `composables/guide-nego/useGnSession.ts`, `pages/auth/{verify-email,reset-password,forgot-password}.vue`, `pages/guide-nego/{nouveau-mot-de-passe,verification-adresse}.vue` —, puis `npm run typecheck`, `npm run build` et `make check-api-contract`

**Checkpoint** : `useApi.ts` sous 910 lignes, site inchangé, rien d'autre touché. **Commit.**

---

## Phase 2 : Fondation — le modèle d'abord

**Objectif** : le vocabulaire et la table existent en base avant qu'une ligne de métier ne soit écrite.

**⚠️ La base se migre, elle ne se recharge pas.** Le volume n'est jamais détruit.

- [ ] T004 Déclarer le vocabulaire `negotiation_theme` dans `docs/database/020_reference.sql`, dans l'`INSERT INTO reference.taxonomies` — `is_multi_select = true`, `is_hierarchical = false`, `is_system = true` —, précédé du commentaire qui dit pourquoi il n'est pas `activity_theme`
- [ ] T005 Semer les dix termes dans l'`INSERT INTO reference.taxonomy_terms` de `docs/database/020_reference.sql`, codes et libellés `fr`/`en` de [research.md § R13](research.md), `sort_order` de 10 en 10, avant le `ON CONFLICT`
- [ ] T006 Créer `negotiation.theme_subscriptions` dans `docs/database/100_negotiations.sql` sur le patron de `network_memberships` : colonnes, `xmod_fk_theme_subscriptions_person`, `ck_theme_subscriptions_period`, `ux_theme_subscriptions_active`, `ix_theme_subscriptions_theme`, `tg_theme_subscriptions_audit`, `tg_theme_subscriptions_check_theme`, et les `COMMENT ON` en français
- [ ] T007 Écrire `specs/010-guide-nego-accueil-profil/migration.sql`, rejouable — `CREATE TABLE IF NOT EXISTS`, gardes `DO $$ … EXCEPTION WHEN duplicate_object` sur les triggers, `INSERT … ON CONFLICT DO NOTHING` sur le vocabulaire et les termes
- [ ] T008 Appliquer `specs/010-guide-nego-accueil-profil/migration.sql` **deux fois** sur la base locale, puis comparer les schémas avec `pg_dump --schema-only` à ce que `docs/database/020_reference.sql` et `100_negotiations.sql` décrivent
- [ ] T009 Consigner le changement de modèle dans `docs/progression/modele.md` — seule exception à la règle qui veut que Guide Négo n'écrive pas dans la progression de l'ePavillon (ADR-017)

**Checkpoint** : `SELECT * FROM reference.taxonomy_terms WHERE taxonomy_code = 'negotiation_theme'` rend dix lignes, et `GET /api/reference/taxonomies/negotiation_theme/terms` les sert **sans une ligne de Rust**. **Commit.**

---

## Phase 3 : Fondation — les thématiques côté API

**Objectif** : les deux routes, leur empreinte, leur `If-Match`, et les huit preuves.

- [ ] T010 [P] Ajouter trois codes stables au catalogue de `backend/crates/kernel/src/error.rs` : liste vide, thématique inconnue, état périmé — le catalogue **s'étend, il ne se double pas**
- [ ] T011 [P] Déclarer les formes `MyThemes` et `ThemesPayload` dans `frontend/app/types/negotiation.ts` et les réexporter depuis `frontend/app/types/index.ts` — **des codes, jamais de libellés** ([api-thematiques.md § 2](contracts/api-thematiques.md))
- [ ] T012 Écrire `backend/crates/modules/negotiation/src/domain/themes.rs` : les formes sérialisées, et le calcul d'empreinte **sur les codes triés** (`kernel::crypto::token_hash`, 16 octets, entre guillemets)
- [ ] T013 Écrire `backend/crates/modules/negotiation/src/repo/themes.rs` : lire les suivis vivants d'une personne, et le remplacement en bloc — fermer par `left_at`, ouvrir les nouveaux, ne pas toucher les inchangés
- [ ] T014 Écrire `backend/crates/modules/negotiation/src/service/themes.rs` : transaction par `Db::write(&ctx)`, verrouillage des suivis de la personne, **comparaison de l'`If-Match` dans la transaction**, traduction de l'erreur du trigger en message qui **nomme le code refusé**
- [ ] T015 Écrire `backend/crates/modules/negotiation/src/routes/themes.rs` : `GET` et `PUT /negotiation/me/themes`, `ETag`, `If-None-Match` → 304, `If-Match` absent accepté, `If-Match` périmé → 412
- [ ] T016 Inscrire les deux chemins dans `backend/crates/modules/negotiation/src/routes/openapi.rs` et monter `routes::themes::configurer` dans `backend/crates/modules/negotiation/src/lib.rs` — chemins **plats**, jamais un `web::scope`
- [ ] T017 [P] Test « chemin nominal » dans `backend/crates/modules/negotiation/tests/thematiques_nominal.rs` : un `PUT` de deux codes ouvre deux suivis, le `GET` les rend avec son empreinte
- [ ] T018 [P] Test « état périmé » dans `backend/crates/modules/negotiation/tests/thematiques_etat_perime.rs` : lire l'empreinte, changer l'état par une autre voie, rejouer avec l'ancienne — **412, et la base est inchangée**
- [ ] T019 [P] Test « rejeu identique » dans `backend/crates/modules/negotiation/tests/thematiques_rejeu.rs` : deux `PUT` du même corps donnent le même état et **aucune ligne d'audit de plus**
- [ ] T020 [P] Test « sans If-Match » dans `backend/crates/modules/negotiation/tests/thematiques_sans_if_match.rs` : accepté, pour ne pas casser l'écran en ligne
- [ ] T021 [P] Test « vocabulaire gardé par la base » dans `backend/crates/modules/negotiation/tests/thematiques_vocabulaire.rs` : un `INSERT` direct d'un terme d'`activity_theme` est refusé par le trigger, et l'API rend un message qui nomme le code
- [ ] T022 [P] Test « on ferme, on ne supprime pas » dans `backend/crates/modules/negotiation/tests/thematiques_fermeture.rs` : retirer une thématique pose `left_at`, la ligne reste, l'index unique n'empêche pas de la reprendre plus tard
- [ ] T023 [P] Test « toute écriture laisse son auteur » dans `backend/crates/modules/negotiation/tests/thematiques_audit.rs` : `platform.audit_log` porte un `actor_id` à chaque ligne
- [ ] T024 [P] Test « refus sans session » dans `backend/crates/api/tests/routes_negotiation_themes.rs` : `401` sur les deux routes, URL forgée comprise, et le `304` éprouvé en HTTP de bout en bout
- [ ] T025 Lancer `cargo test -p negotiation`, puis `make openapi` et `make check-api-contract` depuis la racine, et vérifier `frontend/app/types/api.ts` — les deux formes annoncées sont définies, zéro route en attente

**Checkpoint** : les huit preuves du contrat passent. **Commit.**

---

## Phase 4 : Fondation — la plomberie hors connexion

**Objectif** : la file d'écritures différées, qui n'existait pas, et le vocabulaire gardé avec le drapeau.

- [ ] T026 Ajouter le magasin `ecritures` à la base IndexedDB `guide-nego` dans `frontend/app/utils/guide-nego/garde.ts`, par montée de version, **sans toucher au magasin `lectures`** ni à ce qu'il porte
- [ ] T027 Écrire `frontend/app/utils/guide-nego/file.ts` : poser une intention (clé, corps, empreinte, identifiant de personne, heure), une seule entrée par clé — **l'empreinte gardée reste celle de la première** —, lire, retirer, vider ; aucune fonction ne lève
- [ ] T028 Écrire l'envoi de la file dans `frontend/app/utils/guide-nego/file.ts` : succès → retrait ; `412` → retrait, relecture et message ; `400`/`401`/`403` → retrait et message ; panne réseau ou `5xx` → l'entrée reste
- [ ] T029 Ajouter l'écoute de l'événement `online` dans `frontend/app/composables/guide-nego/useGnConnexion.ts`, qui n'écoutait que `offline`
- [ ] T030 Déclencher le départ de la file **à l'ouverture de l'application** dans `frontend/app/layouts/guide-nego.vue`, et au retour au premier plan — deux déclencheurs simultanés n'envoient qu'une fois
- [ ] T031 Vider la file à la déconnexion, dans `frontend/app/composables/guide-nego/useGnSession.ts` : ce qu'une personne a choisi ne part pas sous le compte de la suivante
- [ ] T032 Lire le vocabulaire `negotiation_theme` **à la première ouverture en ligne, avec le drapeau**, et le garder comme lui — dix termes, lecture publique sans session, dans `frontend/app/layouts/guide-nego.vue` et `frontend/app/composables/guide-nego/useGnThematiques.ts`
- [ ] T033 [P] Test « une seule entrée par clé » dans `frontend/test/guide-nego/file-intention.test.ts` : deux intentions successives, une entrée, et **l'empreinte de la première**
- [ ] T034 [P] Test « 412 » dans `frontend/test/guide-nego/file-412.test.ts` : l'entrée part, la relecture est déclenchée, le message est produit
- [ ] T035 [P] Test « 5xx garde, refus définitif retire » dans `frontend/test/guide-nego/file-echecs.test.ts`
- [ ] T036 [P] Test « déconnexion et compte » dans `frontend/test/guide-nego/file-compte.test.ts` : la file se vide, et une intention prise par quelqu'un d'autre ne part jamais
- [ ] T037 [P] Test « trois déclencheurs, un seul envoi » dans `frontend/test/guide-nego/file-declencheurs.test.ts` : ouverture, `online`, retour au premier plan — chacun déclenche, deux à la fois n'envoient qu'une fois

**Checkpoint** : `npm run test:guide-nego` au vert, six preuves de file. **Commit.**

---

## Phase 5 : US1 — Mes thématiques (Priority: P1) 🎯 MVP

**Objectif** : une négociatrice choisit ses thématiques à sa première entrée, les modifie ensuite, et les
retrouve sur un autre appareil.

**Test indépendant** : cocher deux thématiques sur un téléphone, se connecter sur un second navigateur
avec le même compte, les y retrouver sans geste ; puis en changer une depuis le profil.

- [ ] T038 [P] [US1] Créer `frontend/app/components/guide-nego/GnAvatar.vue` — 40 px, rayon 24, initiales 15/700, image du compte quand elle existe — et son `frontend/i18n/locales/{fr,en}/components/gn-avatar.json`
- [ ] T039 [P] [US1] Ajouter `GnAvatar` à la planche dans `frontend/app/components/guide-nego/planche/PlancheComposantsSurfaces.vue`, dans les deux thèmes, et le retirer de la liste « Ce qui n'est pas montré ici »
- [ ] T040 [US1] Ajouter `mesThematiques()` et `suivreDesThematiques(codes, empreinte?)` à `frontend/app/composables/api/guide-nego.ts` — **jamais dans `useApi.ts`**
- [ ] T041 [US1] Écrire `frontend/app/composables/guide-nego/useGnThematiques.ts` sur le motif de `useGnAcces` : `useGnLecture` pour le vocabulaire et pour les suivis, `assurer()`, `pret`, `luA`, et l'écriture qui passe par la file
- [ ] T042 [US1] Écrire `frontend/app/pages/guide-nego/thematiques.vue` : `GnCase` par ligne de 56 px, récapitulatif du pied, bouton inactif à zéro avec son aide, indicateur d'étapes à la première entrée seulement
- [ ] T043 [P] [US1] Écrire `frontend/i18n/locales/fr/pages/guide-nego.thematiques.json` et son jumeau `en` — libellés d'écran seulement, **aucun libellé de thématique**
- [ ] T044 [US1] Proposer l'écran **une seule fois** à la première entrée d'une personne connectée qui ne suit rien, par une clé locale de `frontend/app/utils/guide-nego/stockage.ts`, sans jamais enfermer personne
- [ ] T045 [US1] Afficher le message du `412` — « Vos thématiques ont changé sur un autre appareil » — et relire, dans `frontend/app/pages/guide-nego/thematiques.vue`
- [ ] T046 [US1] Porter les quatre états de `frontend/app/pages/guide-nego/thematiques.vue` : chargement, vide (aucune thématique proposée), erreur, accès refusé
- [ ] T047 [P] [US1] Test de l'écran dans `frontend/test/guide-nego/thematiques.test.ts` : zéro coché n'est pas validable, le récapitulatif compte juste, et la garde de première entrée ne se déclenche qu'une fois

**Checkpoint** : le critère de sortie de l'étape est tenu. **Commit.**

---

## Phase 6 : US2 — « Ma journée » (Priority: P1)

**Objectif** : l'accueil quotidien s'ouvre avec ses cinq blocs, chacun dans son état vide, en ligne comme
hors connexion, sans jamais ressembler à une panne.

**Test indépendant** : ouvrir l'onglet Accueil sur un compte neuf, en ligne puis en mode avion.

- [ ] T048 [US2] Ajouter la prop d'avatar à `frontend/app/components/guide-nego/GnEntete.vue`, exclusive du bouton retour, et la relayer depuis `frontend/app/components/guide-nego/GnEcran.vue` — aucune page n'instancie `GnEntete` directement
- [ ] T049 [US2] Composer les cinq blocs de `frontend/app/pages/guide-nego/index.vue` dans l'ordre fixe — prochaine session de négociation, changements du jour, aujourd'hui vos trois agendas, documents récents, accès au lexique
- [ ] T050 [US2] Donner à chacun des quatre premiers blocs de `frontend/app/pages/guide-nego/index.vue` **son** `GnEtatVide` : ce qui manque, quand cela reviendra, une sortie quand il y en a une — jamais une erreur, jamais un chargement qui dure
- [ ] T051 [US2] Poser dans `frontend/app/pages/guide-nego/index.vue` le titre « Ma journée » et le sous-titre — date du jour et fuseau de l'appareil, nommé ([research.md § R11](research.md))
- [ ] T052 [US2] Porter l'état sans compte de `frontend/app/pages/guide-nego/index.vue` : l'écran s'ouvre et invite à créer un compte ou à se connecter, sans avatar
- [ ] T053 [US2] Vérifier le comportement hors connexion de `frontend/app/pages/guide-nego/index.vue` : bandeau « Hors connexion — lu à … », compteur « connus à … », et passage à « Synchronisé à … » au retour du réseau **sans rechargement**
- [ ] T054 [P] [US2] Écrire `frontend/i18n/locales/fr/pages/guide-nego.accueil.json` complété et son jumeau `en` — les cinq blocs, leurs états vides, l'accès au lexique
- [ ] T055 [P] [US2] Test dans `frontend/test/guide-nego/ma-journee.test.ts` : les cinq blocs paraissent dans l'ordre, chacun avec son état vide, et **aucun message d'erreur** quand rien n'est disponible
- [ ] T056 [US2] Vérifier qu'aucun libellé de `frontend/i18n/locales/{fr,en}/pages/guide-nego.accueil.json` n'emploie « Programme » seul, et que chaque bloc nomme son agenda en toutes lettres

**Checkpoint** : le cadre des étapes 1, 3a, 4 et 5 est posé. **Commit.**

---

## Phase 7 : US3 — Profil et réglages (Priority: P2)

**Objectif** : tout ce qui concerne la personne au même endroit, sans défaire ce que l'étape 0b y a posé.

**Test indépendant** : ouvrir le profil par ses deux chemins, changer ses thématiques, lire la place
occupée, la libérer, se déconnecter.

- [ ] T057 [P] [US3] Créer `frontend/app/components/guide-nego/GnJauge.vue` — 6 px, accent sur gris pâle, **toujours avec le nombre écrit** (« 7 Mo utilisés · 2,1 Go libres ») — et son `frontend/i18n/locales/{fr,en}/components/gn-jauge.json`
- [ ] T058 [P] [US3] Ajouter `GnJauge` à la planche dans `frontend/app/components/guide-nego/planche/PlancheComposantsSurfaces.vue`, dans les deux thèmes
- [ ] T059 [US3] Écrire `frontend/app/composables/guide-nego/useGnPlace.ts` : l'estimation du navigateur, le repli qui **dit** qu'on ne peut pas mesurer, et la libération — données lues et documents, **jamais la coquille**
- [ ] T060 [US3] Écrire `frontend/app/pages/guide-nego/ressources/telechargements.vue` : l'état vide (aucun document, et à quelle étape ils arrivent), la jauge, et la libération derrière une `GnConfirmation` qui dit ce qui reste lisible sans réseau
- [ ] T061 [US3] Ajouter le groupe « Mon suivi » à `frontend/app/pages/guide-nego/ressources/reglages.vue` : « Mes thématiques » avec sa valeur, « Mes téléchargements », puis « Mon accès » livré en 0b — dans cet ordre
- [ ] T062 [US3] Faire afficher à la ligne « Mes thématiques » de `frontend/app/pages/guide-nego/ressources/reglages.vue` les **noms** croisés avec le vocabulaire gardé, et **le nombre** en repli quand le vocabulaire n'a jamais été lu
- [ ] T063 [US3] Ajouter le groupe « Application » à `reglages.vue` : l'entrée « À propos » et la ligne « Dernière synchronisation » — l'heure **sans fuseau** (écart 32), et ce n'est pas une action
- [ ] T064 [US3] Déplacer le nom et le pays en tête de `reglages.vue`, avec l'avatar, sans toucher au groupe « Compte » ni à la déconnexion de 0b
- [ ] T065 [P] [US3] Écrire `frontend/i18n/locales/fr/pages/guide-nego.telechargements.json`, compléter `guide-nego.reglages.json`, et leurs jumeaux `en`
- [ ] T066 [US3] Découper `reglages.vue` en composants de groupe **si elle dépasse 300 lignes** — le garde-fou est à 1000, mais un écran de réglages qui grossit à chaque étape se découpe avant d'y arriver

**Checkpoint** : le profil porte tout ce que 0c lui doit. **Commit.**

---

## Phase 8 : US4 — À propos et les textes qui engagent (Priority: P3)

**Objectif** : une source unique pour la politique de confidentialité et les conditions d'utilisation,
servie au site comme à l'application, et la version qui fait preuve.

**Test indépendant** : ouvrir chaque texte en ligne puis sans réseau ; vérifier que le site et
l'application reçoivent la même version ; modifier un texte sans lever sa version et constater le refus.

**⚠️ Cette phase touche `programme` et `kernel` : la faire seule.**

- [ ] T067 [US4] Créer `backend/crates/kernel/src/legal/` : les quatre fichiers Markdown — politique et conditions, `fr` et `en` —, chacun portant sa version et sa date d'entrée en vigueur en tête
- [ ] T068 [US4] Écrire le module `backend/crates/kernel/src/legal/mod.rs` : embarquement par `include_str!`, lecture de l'en-tête, accès par clé et par langue avec **repli sur le français**, et l'empreinte du corps
- [ ] T069 [P] [US4] Test « l'empreinte est figée par version » dans `backend/crates/kernel/tests/legal_empreinte.rs` : modifier un texte sans lever sa version fait diverger l'empreinte et **échouer le test**, avec le nom du fichier
- [ ] T070 [P] [US4] Test « la grammaire close couvre les fichiers réels » dans `backend/crates/kernel/tests/legal_grammaire.rs` : rendre les quatre fichiers et **échouer sur toute construction inconnue** — tableau, note de bas de page, image, bloc de code —, en nommant la ligne
- [ ] T071 [P] [US4] Test « une version par texte, pas par langue » dans `backend/crates/kernel/tests/legal_versions.rs` : les deux langues d'un texte déclarent la même version
- [ ] T072 [US4] Écrire `backend/crates/api/src/routes/legal.rs` : `GET /legal/{cle}` publique, corps **sérialisé une fois au montage**, `ETag`, `404` sur clé inconnue, et la langue servie annoncée
- [ ] T073 [US4] Annoter la route pour OpenAPI et l'inscrire au registre de `backend/crates/api/src/openapi.rs` — **sans bloc `security`**, c'est une route publique ; déclarer la forme `LegalText` dans `frontend/app/types/platform.ts`
- [ ] T074 [US4] Retirer `privacy_policy_version` de `backend/crates/kernel/src/config.rs` — le champ brut, son défaut, sa validation et `ProgrammeConfig.privacy_policy_version` — et la ligne `PRIVACY_POLICY_VERSION` de `.env.example`
- [ ] T075 [US4] Faire lire la version depuis `kernel::legal` dans `backend/crates/modules/programme/src/service/registration.rs:122` — **une ligne**, la signature d'`exiger_le_consentement` ne bouge pas
- [ ] T076 [US4] Réécrire le commentaire de doctrine de `backend/crates/modules/programme/src/repo/consents.rs:26-31` : la version ne vient plus de la configuration
- [ ] T077 [US4] Vérifier qu'aucune ligne existante d'`identity.consents` n'est réécrite — les preuves sous `2026-01` restent telles quelles
- [ ] T078 [P] [US4] Créer `frontend/app/components/guide-nego/GnTexteLong.vue` : rendu de la grammaire close, tout le reste échappé ; et son test `frontend/test/guide-nego/texte-long.test.ts`
- [ ] T079 [P] [US4] Ajouter `GnTexteLong` à la planche dans `frontend/app/components/guide-nego/planche/PlancheComposantsSurfaces.vue`, dans les deux thèmes
- [ ] T080 [US4] Écrire `frontend/app/composables/guide-nego/useGnTextes.ts` : lecture par `useGnLecture`, garde par texte, heure de lecture
- [ ] T081 [US4] Écrire `frontend/app/pages/guide-nego/ressources/a-propos.vue` : édition, étiquette de source, paragraphe de confidentialité, groupe des textes — **et aucun interrupteur d'accord** (écart 40)
- [ ] T082 [US4] Écrire `frontend/app/pages/guide-nego/ressources/textes/[cle].vue` : le texte entier, sa version, son heure de lecture hors connexion
- [ ] T083 [US4] Écrire les licences dans `frontend/i18n/locales/{fr,en}/pages/guide-nego.a-propos.json` — police et bibliothèques embarquées, **aucun accord demandé**
- [ ] T084 [P] [US4] Écrire `frontend/i18n/locales/fr/pages/guide-nego.a-propos.json` et `guide-nego.textes.json`, plus leurs jumeaux `en`
- [ ] T085 [US4] Ajouter `mesTextes` / `texte(cle)` à `frontend/app/composables/api/guide-nego.ts`, puis `make openapi` et `make check-api-contract`

**Checkpoint** : une seule source de textes, et deux contrôles qui mordent. **Commit.**

---

## Phase 9 : Recette et finitions

**Objectif** : dérouler [quickstart.md](quickstart.md), et ne rien laisser de ce qui se voit.

### Depuis un poste

- [ ] T086 Dérouler les §§ 1 à 3 de `specs/010-guide-nego-accueil-profil/quickstart.md` : le vocabulaire servi, le suivi de bout en bout, ce que la base refuse et ce que l'API répond
- [ ] T087 Dérouler le § 4 de `specs/010-guide-nego-accueil-profil/quickstart.md` : les huit points du hors-connexion, dont **les deux appareils** et le **téléphone partagé**
- [ ] T088 Dérouler les §§ 5 à 7 de `specs/010-guide-nego-accueil-profil/quickstart.md` : « Ma journée » vide sans erreur, le profil, « À propos » et ses textes — dont les deux contrôles qu'on fait mordre exprès
- [ ] T089 Mesurer le § 8 de `specs/010-guide-nego-accueil-profil/quickstart.md` : 320, 360 et 390 px, thème clair et sombre, sur les écrans nouveaux et le profil — aucun débordement, aucune cible sous 44 px, anneau de focus partout
- [ ] T090 [P] Relire les traductions `en` sous `frontend/i18n/locales/en/` et comparer les clés `fr`/`en` par `npm run check:guide-nego`
- [ ] T091 [P] Vérifier que `frontend/app/pages/guide-nego/composants.vue` montre `GnAvatar`, `GnJauge` et `GnTexteLong` dans les deux thèmes, et que la section « Ce qui n'est pas montré ici » est à jour
- [ ] T092 Vérifier la non-régression du site — `frontend/app/pages/index.vue`, `/negociations`, le back-office et `frontend/app/pages/auth/*.vue` : accueil, `/negociations`, back-office, les quatre écrans d'authentification — **`auth` a bougé de fichier en phase 1**
- [ ] T093 Lancer `node frontend/scripts/guide-nego-verifier-garde.mjs` contre la version construite : toutes les adresses gardées servies en 200, écrans nouveaux compris
- [ ] T094 Lancer `make check-safe` depuis la racine, **API arrêtée** — deux tests d'`identity` sont sensibles à une activité concurrente sur la base
- [ ] T095 Mettre à jour `docs/AppNego/progress.md` et, si `docs/database/` a bougé depuis la phase 2, `docs/progression/modele.md` : la ligne d'état et le journal, avec ce qui a été trouvé en construisant

### Sur appareil réel — ne se fait pas depuis un poste

- [ ] T096 Installer sur un Android puis un iPhone depuis l'adresse de `docs/DEPLOIEMENT.md` § 14, entrer, choisir ses thématiques en **3G bridée**, puis en mode avion
- [ ] T097 Vérifier que la file repart **après une nuit de veille**, réseau rétabli sans ouvrir l'application : c'est le départ à l'ouverture qui doit l'attraper, aucun `online` n'ayant été émis
- [ ] T098 Dérouler **T112 de l'étape 0b** (`specs/009-guide-nego-compte-admission/tasks.md`), toujours due : installation, retour du courriel sur iPhone via Safari, `client_kind` après une nuit, § 2 en 3G lente

**Checkpoint** : l'étape est livrée. **Commit.**

---

## Dependencies & Execution Order

### Entre les phases

- **Phase 1 (place)** — aucune dépendance. **Elle passe en premier** : 27 lignes de marge, tout ajout franchirait le garde-fou.
- **Phase 2 (modèle)** — après la 1 par convention de commit, mais indépendante techniquement. **Bloque les phases 3 et 5.**
- **Phase 3 (API)** — dépend de la 2. **Bloque la 5.**
- **Phase 4 (plomberie)** — dépend de la 3 pour l'empreinte et le `412` ; le vocabulaire gardé (T032) ne dépend que de la 2. **Bloque la 5.**
- **Phase 5 (US1)** — dépend des phases 2, 3 et 4.
- **Phase 6 (US2)** — dépend de T038 (l'avatar, en phase 5) et de rien d'autre : « Ma journée » ne lit aucune donnée.
- **Phase 7 (US3)** — dépend de la 5 pour la ligne « Mes thématiques » et de la 6 pour l'avatar.
- **Phase 8 (US4)** — **indépendante de tout le reste**. Elle pourrait se faire à tout moment ; elle est placée tard parce qu'elle touche `programme` et `kernel`.
- **Phase 9 (recette)** — dépend de tout.

### Ce qui peut avancer en parallèle

- Phase 3 : les huit tests T017 à T024 — fichiers distincts, aucune dépendance entre eux.
- Phase 4 : les cinq tests T033 à T037.
- Phase 8 : les trois contrôles T069 à T071, et les composants T078/T079.
- Les fichiers i18n T043, T054, T065, T084 — chacun dans son écran.
- **Les phases 5 à 8 se répartissent entre deux personnes** : une sur les écrans (5, 6, 7), une sur les textes (8), qui ne partagent aucun fichier.

### Ce qui ne se parallélise pas

- T004 et T005 touchent le même fichier SQL.
- T012 à T016 s'enchaînent : formes, puis dépôt, puis service, puis routes, puis montage.
- T026 à T031 s'enchaînent : le magasin, puis la file, puis ses déclencheurs.

---

## Implementation Strategy

### Ce qui fait un MVP

**Phases 1 à 5.** À ce point, une négociatrice choisit ses thématiques, les retrouve ailleurs, et le
choix pris sans réseau part au retour. C'est le critère de sortie de l'étape, et il se démontre seul.

### Livraison par tranches

1. Phases 1 et 2 → la place et le modèle. Rien ne se voit encore.
2. Phases 3 et 4 → les fondations. Rien ne se voit toujours, et c'est normal.
3. Phase 5 → **le critère de sortie est tenu**. Démontrable.
4. Phase 6 → l'accueil prend sa forme, et les quatre étapes suivantes savent où poser leurs lignes.
5. Phase 7 → le profil est complet.
6. Phase 8 → les textes, et une dette du site en moins.
7. Phase 9 → la recette.

### Notes

- **Un commit par phase**, sur la branche `010-guide-nego-accueil-profil`.
- La phase 1 ne fait qu'une chose, et son diff doit se lire comme un déplacement.
- `make check` et `make check-db` ne se lancent jamais ; `make check-safe` en fin de cycle, API arrêtée.
- Aucun fichier de plus de mille lignes — et `reglages.vue` se découpe avant d'y arriver.
- Tout composant nouveau rejoint la page interne des composants, dans les deux thèmes.

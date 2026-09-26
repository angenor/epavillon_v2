# Tasks: Guide Négo — Pavillon de la Francophonie (étape 5)

**Input**: [spec.md](spec.md) · [plan.md](plan.md) · [research.md](research.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

**Dossier** : `/Users/mac/Documents/projets/IFDD/epavillon_v2-dev2`, branche `017-guide-nego-pavillon`, base `epavillon_dev2`, API 8096, site 3005.
**Règles** : CLAUDE.md, constitution, `/Users/mac/Documents/projets/IFDD/epavillon_v2/.orchestration/protocole.md` — seuls ses propres PID ; aucune boucle sur `pgrep -f` ; `cargo test` sur les seuls crates touchés ; jamais `make check`, `check-db`, `down -v`. **Trois conditions du 26/09** : ajouts seulement (aucun champ retiré ni renommé), site inchangé (`test:site`, `check-api-contract` verts), rien de privé rendu public.
**Une phase = un commit**, dernière ligne `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`. `prg/` = `backend/crates/modules/programme/`, `fe/` = `frontend/`.

## Phase 1 — L'API du programme, par ajouts

- [x] T001 `docs/database/075_programme_sessions.sql` : `programme.v_public_schedule` gagne **en fin de liste** `waitlist_enabled`, `registration_required`, `registration_opens_at`, `registration_closes_at`, `waitlisted_count`, `listing_changed_at`, `language_codes` (`LEFT JOIN` du dossier de proposition, seule colonne qui en vient) ; un commentaire renvoie à `080_live.sql`, où la vue se **redéfinit en entier** (`CREATE OR REPLACE VIEW`, colonnes existantes identiques et dans le même ordre) avec en plus, en fin, `replay_url` et `replay_duration_seconds` selon research R1 (une seule rediffusion par séance par `LEFT JOIN LATERAL … LIMIT 1`, source large, durée arrondie) ; `live.streams` et `media.assets` au registre `prg/src/repo/cross/mod.rs`
- [x] T002 `specs/017-guide-nego-pavillon/migration.sql` rejouable (la définition finale de la vue, celle de `080`) ; sauvegarde ; migration deux fois sur `epavillon_dev2` ; comparaison à une base jetable ; ligne dans `docs/progression/modele.md`
- [x] T003 `prg/src/repo/public_schedule.rs` et domaine : champs ajoutés ; `ETag`/`304` sur `GET /schedule` et `GET /registrations/mine` (`kernel::empreinte`, `Cache-Control: private, no-cache` pour la seconde) ; `make sqlx-prepare`
- [x] T004 Détail public (`prg/src/repo/session_parts.rs` et la route du détail) : lecture publique dédiée **composée champ par champ** (`jsonb_build_object`, jamais `to_jsonb` de `identity.people` ni d'`org`) : `speakers[].display_name` et `organizations[].name` (← `legal_name`), `acronym`, `country_code`, `country` **ajoutés** ; le sort de `attended`/`confirmed_at` suit la réponse de l'orchestrateur ; `make sqlx-prepare`
- [x] T005 Types TS du site étendus de champs **facultatifs** (`fe/app/types/views.ts`, `fe/app/types/programme/session.ts`) ; OpenAPI et `make openapi` si les descriptions changent ; `make check-api-contract`
- [x] T006 [P] Tests `prg/tests/pavillon_ajouts.rs` : toutes les clés rendues avant sont encore rendues, identiques (hors ce que l'orchestrateur aura tranché sur `attended`) ; les nouvelles présentes ; **aucune coordonnée ni pièce du dossier** servie ; **autant de lignes qu'avant** avec deux rediffusions sur une séance ; rediffusion et durée ; langue nulle sans dossier ; liste d'attente ; `304` ; puis `npm run typecheck`, `npm run test:site`, `make check-api-contract` verts

## Phase 2 — Plomberie client

- [x] T007 `fe/app/composables/api/pavillon.ts` (édition, lieux, détail par slug, formulaire, s'inscrire, annuler, mes inscriptions) + une ligne de montage dans `useApi.ts`
- [x] T008 [P] `fe/app/utils/guide-nego/pavillon.ts` + tests (bande des jours de toute l'édition, jour, veille, jours suivants dans le fuseau, états de l'activité, marque d'inscription ou de rediffusion, prochaine, ligne de « Ma journée », formulaire « d'un geste » ou non, **aucun filtre de thématique**)
- [x] T009 `useGnPavillon` (gardes `pavillon:<slug>`, `pavillon-activite:<édition>:<slug>`) et `useGnInscriptionsPavillon` (garde `mes-inscriptions-pavillon` ; intention `inscription-pavillon:<session_id>` ; dernière gagne ; inscription puis annulation pas encore parties → rien ; annulation : identifiant = ligne de la séance non annulée, lu au départ ; introuvable ou `404` → succès ; `RegistrationLocked` → refus dit ; issues `full`/`closed`/`not_open_yet` dites ; `RegistrationNotAccepted` → « Sans inscription ») ; `useGnPays` garde `iso2` ; jeux d'exemple

## Phase 3 — US1 : lire

- [x] T010 [US1] `GnBlocLieu`, `GnLigneActivite` ; `GnSectionPavillon.vue` réécrit (bloc de lieu, bande des jours de l'édition, Aujourd'hui, Hier — rediffusions, Les jours suivants, états) ; planche
- [x] T011 [US1] `pages/guide-nego/francophonie/pavillon/[slug].vue` (sans « Ajouter à mon agenda ») ; rediffusion ouvrable
- [x] T012 [US1] `GnEtiquettePavillon` reçoit `vers` = la fiche de l'activité liée, dans `GnLigneReunion.vue` et la fiche d'une réunion
- [x] T013 [US1] Vérifier au navigateur : quickstart § 1, § 3 (étiquette)

## Phase 4 — US2 : s'inscrire

- [x] T014 [US2] `GnFormulaireInscription` (types de champ du modèle seulement, pays prérempli depuis le profil de 0c — uuid converti en ISO2 —, consentement d'une donnée sensible) ; bouton sur la fiche (M'inscrire, Inscrite, Rejoindre la liste d'attente, Liste d'attente — position N, Complet, closes, pas encore ouvertes, se désinscrire) ; sans compte → connexion
- [x] T015 [US2] Vérifier au navigateur : quickstart § 2

## Phase 5 — US3 : Ma journée

- [x] T016 [US3] `GnJourneeLignePavillon.vue` réécrit ; vérifier au navigateur

## Phase 6 — Recette

- [ ] T017 Quickstart § 0 à 4 et 6 ; la rediffusion de recette se pose **en SQL** dans `epavillon_dev2` (rien n'écrit `live.streams`) ; § 4 : `npm run typecheck`, `test:site`, `make check-api-contract`, et **la page `/programme` du site au navigateur (bloquant)**
- [ ] T018 [P] Écarts dans `docs/AppNego/05-design.md` (pas d'agenda sur une activité ; bloc de lieu de la fiche ; bande des jours ; ce que la recette relève) ; Points ouverts de `progress.md` : **rien n'écrit `live.streams`** — sans outil de saisie, la rediffusion ne paraîtra pas en production
- [ ] T019 [P] § 15 de `docs/DEPLOIEMENT.md` : migration 017 ; essais sur appareil réel
- [ ] T020 i18n `en` contre `fr`
- [ ] T021 **Le scénario qui clôt le MVP** (quickstart § 5) — seulement après fusion des étapes 2 et 4 dans `main` et de `main` dans cette branche ; sinon, noté pour la clôture
- [ ] T022 Une ligne au journal de `docs/AppNego/progress.md`

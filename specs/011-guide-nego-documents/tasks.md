---
description: "Tâches de l'étape 1 — la bibliothèque de documents et le lecteur"
---

# Tasks: Guide Négo — la bibliothèque de documents et le lecteur (étape 1)

**Input** : les documents de `/specs/011-guide-nego-documents/`

**Prérequis** : [plan.md](plan.md) · [spec.md](spec.md) · [research.md](research.md) · [essai-extraction.md](essai-extraction.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

**Tests** : **oui, énumérés par les contrats**.
- Sur base réelle : [api-documents.md](contracts/api-documents.md) et [api-admin-documents.md](contracts/api-admin-documents.md), dont SC-007.
- Sans navigateur : [hors-connexion.md](contracts/hors-connexion.md) et [forme-lisible.md](contracts/forme-lisible.md).
- L'extraction : sur `fixtures/petit.pdf`.

**Branche** : `011-guide-nego-documents`, créée par T001. **Un commit par phase.**

## Format : `[ID] [P?] [Récit] Description`

- **[P]** : parallélisable — fichiers différents, aucune dépendance sur une tâche non finie.
- **[US1]…[US6]** : le récit servi. Les phases de fondation (1 à 6 et 8) et la recette n'en portent pas ; une tâche de fondation dit dans sa description le récit qu'elle prépare.
- Chaque tâche nomme son fichier.

## Les portes, et celle qu'on ne franchit pas

- **Pendant le cycle** : `npm run typecheck`, `npm run test:guide-nego`, `npm run check:guide-nego`, `cargo test -p negotiation`, `cargo test -p media`, `cargo test -p kernel`, `make openapi`, `make check-api-contract`.
- **En fin de cycle seulement, API arrêtée** : `make check-safe`.

> **`make check` et `make check-db` ne se lancent jamais** : ils détruisent la base locale, qui n'a pas de sauvegarde. La base **se migre**.

---

## Phase 1 : Setup — l'essai, avant tout

**Objectif** : confirmer ou renverser R1 et R2 sur le vrai guide, `.essais/guide-cdp30.pdf` (90 pages, 2,9 Mo). **Rien du modèle, de l'API ni des écrans ne s'écrit avant la conclusion, relue avec le commanditaire.**

- [X] T001 Créer la branche `011-guide-nego-documents` depuis `main`
- [X] T002 Ajouter la cible `make pdfium` au `Makefile` : elle télécharge le binaire PDFium du poste (macOS arm64, Linux x64) depuis `bblanchon/pdfium-binaries`, version épinglée, dans `.outils/pdfium/`, ignoré par Git. Ajouter `PDFIUM_LIB_PATH` à `.env.example`, avec une ligne qui dit d'où il vient
- [X] T003 Ajouter `pdfium-render` (MIT ou Apache-2.0, version épinglée) en dépendance **de développement** de `backend/crates/modules/negotiation/Cargo.toml` : elle ne sert qu'à l'exemple, jusqu'à la décision
- [X] T004 Écrire `backend/crates/modules/negotiation/examples/essai_extraction.rs` :
  - il lit le PDF donné en argument ;
  - il sort, par page, les segments avec leur police (nom, graisse, italique), leur cadre et l'étiquette de la page, ainsi que les signets ;
  - il écrit `.essais/sortie/brut.json` et une image JPEG par page, à 1080 px et en qualité 75, dans `.essais/sortie/pages/`
- [X] T005 Compléter l'exemple d'une première version des règles de [R7](research.md) :
  - colonnes par abscisse ;
  - en-têtes et pieds répétés ;
  - notes en petit corps ;
  - zones `origine` ;
  - césures, **dont la réinsertion du tiret attesté** (« Convention-cadre », `essai-extraction.md`) ;
  - italiques en `terme` ;
  - titres.

  Il écrit `.essais/sortie/forme-lisible.json` et `.essais/sortie/rapport.md`
- [X] T006 Lancer l'essai sur `.essais/guide-cdp30.pdf` et remplir la grille d'[essai-extraction.md](essai-extraction.md) sur les pages témoins nommées : les huit critères et les six mesures, avec des extraits courts
- [X] T007 *(sans objet : aucun critère n'échoue)* **Seulement si un critère échoue avec PDFium** : rejouer le même contrôle avec pdfplumber et pypdfium2, par un script jetable sous `.essais/`, et consigner la comparaison dans `essai-extraction.md`
- [X] T008 Écrire la conclusion d'`essai-extraction.md` :
  - l'issue A, B ou C ;
  - ce que garde le téléphone ([R2](research.md)) ;
  - les règles de R7 ajustées.

  Reporter les changements dans [research.md](research.md) (R1, R2, R7), et dans [plan.md](plan.md) si l'issue est B ou C
- [X] T009 **Relire la conclusion avec le commanditaire** et attendre son accord. Sur l'issue B, faire valider le coût du service d'ADR-004 avant la phase 2. Consigner la dépendance retenue dans `docs/progression/decisions/` *(fait dans [ADR-021](../../docs/AppNego/adr/021-pdfium-dans-le-worker.md) : les décisions de Guide Négo vivent dans `docs/AppNego/adr/`, constitution § « Le suivi de Guide Négo »)*

**Checkpoint** : l'issue est écrite et acceptée. **Commit** (l'exemple, `make pdfium`, la grille remplie ; jamais le PDF).

---

## Phase 2 : Fondation — les fichiers privés deviennent privés

**Objectif** : [R4](research.md) et [R5](research.md). Deux **déplacements purs** d'abord, puis le bucket privé et la garde.

**Le site ne doit rien voir changer** : le dépôt des images marche comme avant.

- [X] T010 Déplacer `backend/crates/modules/media/src/storage/{mod,s3,sigv4,filesystem}.rs` vers `backend/crates/kernel/src/storage/`, **sans rien changer au code**. Les dépendances passent de `media/Cargo.toml` à `kernel/Cargo.toml`. `media` importe `kernel::storage`. `cargo test -p media` et `cargo test -p kernel` doivent rester au vert, et le diff se lire comme un déplacement. **Commit à part**
- [X] T011 Sortir la partie « dépôt » de `frontend/app/components/media/ImageField.vue` — contrôle préalable, envoi par `api.media.upload`, progression, erreurs — dans `frontend/app/composables/useDepotMedia.ts`, **sans rien changer au comportement**. `ImageField` l'emploie. `npm run typecheck` au vert, puis un dépôt d'image réel au navigateur. **Commit à part**
- [X] T012 Ajouter le réglage `media.private_bucket` = `"epavillon-prive"` à la semence des réglages de `docs/database/050_media.sql`, puis la fonction `media.object_location(p_asset_id uuid)`, STABLE, qui rend le bucket, la clé, l'état, la taille et le type d'un objet non supprimé, avec son `COMMENT ON` ([data-model.md § 9](data-model.md))
- [X] T013 Faire choisir le bucket selon la visibilité dans `backend/crates/modules/media/src/repo/assets.rs` et `service/upload.rs` : `private` va dans `media.private_bucket`, le reste dans le bucket par défaut. Ajouter un test dans `backend/crates/modules/media/tests/` : un objet privé atterrit dans le bucket privé
- [X] T014 Créer le bucket privé **sans** `bucket website --allow` dans la cible `garage-init` du `Makefile` et dans `ops/init-garage-prod.sh`, avec une ligne qui dit pourquoi il n'est pas ouvert au web
- [X] T015 Ouvrir la garde `("negotiation","documents")` dans `backend/crates/modules/media/src/domain/guards.rs` : permission `negotiation.document.publish`, type `application/pdf` seul, visibilité imposée `private`. Inverser le test de `guards.rs:160-161`, et ajouter les refus d'un fichier non PDF et d'une personne sans la permission
- [X] T016 [P] Rendre `altText` facultatif dans `UploadPayload` (`frontend/app/types/media.ts`) quand le type n'est pas une image, et le vérifier côté serveur au premier dépôt d'un PDF. Si le serveur l'exige pour tout type, le corriger dans `media/src/service/upload.rs`, avec son test
- [X] T017 [P] Créer `frontend/app/components/media/FileField.vue` sur `useDepotMedia` : choix du fichier, type accepté en prop, progression, erreurs, émissions `update:assetId`. Pas de recadrage ni de texte alternatif. L'ajouter à `frontend/app/pages/style-guide.vue`
- [X] T018 Écrire la requête qui compte les objets `private` déjà déposés dans le bucket public (dette de R4), dans `specs/011-guide-nego-documents/quickstart.md` § Préalables, et la jouer en local

**Checkpoint** : un PDF privé n'est pas lisible par l'adresse du proxy média ; les images du site s'affichent comme avant. **Commit.**

---

## Phase 3 : Fondation — le modèle

**Objectif** : tout [data-model.md](data-model.md) existe en base avant une ligne de métier.

**⚠️ La base se migre, elle ne se recharge pas.** Le volume n'est jamais détruit.

- [X] T019 Ajouter les termes `summary` (« Résumé » / « Summary ») et `bulletin` (« Bulletin ») à la taxonomie `document_type`, et relire le libellé fr de `negotiation_guide` (« Guide »), dans `docs/database/020_reference.sql`
- [X] T020 Modifier `negotiation.documents` dans `docs/database/100_negotiations.sql` ([data-model.md § 1](data-model.md)) :
  - `ck_documents_source_xor` se dédouble en `ck_documents_source_at_most_one` et `ck_documents_published_has_source` ;
  - `event_id` avec `xmod_fk_documents_event` et son index partiel ;
  - `issued_on`, `unpublished_at` ;
  - `external_publisher` renommé `publisher`, le vecteur de recherche suivant ;
  - `is_rag_eligible DEFAULT false` ;
  - `ux_documents_supersedes` ;
  - le déclencheur `tg_documents_no_supersede_cycle` et sa fonction.

  Mettre à jour les `COMMENT ON`
- [X] T021 Ajouter dans `docs/database/100_negotiations.sql` l'ENUM `negotiation.rendition_status` et la table `negotiation.document_renditions`, avec ses contraintes, son déclencheur `updated_at` et ses commentaires ([data-model.md § 4-5](data-model.md))
- [X] T022 Ajouter dans `docs/database/100_negotiations.sql` la table `negotiation.document_pages`, avec sa PK, son vecteur généré, l'index GIN `ix_document_pages_search` et ses commentaires ([§ 6](data-model.md))
- [X] T023 Ajouter dans `docs/database/100_negotiations.sql` la table `negotiation.correction_notes`, avec ses `xmod_fk_*`, `ck_correction_notes_withdrawal`, `ix_correction_notes_live`, le déclencheur `tg_correction_notes_page_exists`, l'audit et les commentaires ([§ 7](data-model.md))
- [X] T024 Semer dans `docs/database/100_negotiations.sql` les permissions `negotiation.correction.post` et `negotiation.correction.withdraw`, le rôle `expert` (`allowed_scopes {global}`) et ses liens, en `ON CONFLICT DO NOTHING`. Le commentaire du rôle nomme les étapes 2, 7 et 8 ([§ 8](data-model.md))
- [X] T025 Écrire `specs/011-guide-nego-documents/migration.sql`, rejouable, dans `BEGIN … COMMIT`, sur le patron de `specs/010-guide-nego-accueil-profil/migration.sql` : `IF NOT EXISTS`, `DROP … IF EXISTS` puis `CREATE`, `DO $$ … duplicate_object`, renommage gardé par un test d'existence, `ON CONFLICT DO NOTHING`
- [X] T026 Jouer la migration **deux fois** sur la base locale. Comparer ensuite `pg_dump --schema-only` au modèle chargé sur une base jetable, après tri. Vérifier que `platform.cross_module_fk_report` reste vide
- [X] T027 Consigner le changement dans `docs/progression/modele.md`. Corriger [02-domaine.md](../../docs/AppNego/02-domaine.md) l.29, qui cite encore `negotiation_track` pour les thématiques
- [X] T028 `make sqlx-prepare`, puis `cargo test -p negotiation`, `-p media` et `-p identity` au vert : les gabarits de test se rechargent depuis le SQL

**Checkpoint** : le modèle est en base, migré sans destruction. **Commit.**

---

## Phase 4 : Fondation — l'extraction

**Objectif** : [R6](research.md) et [R7](research.md), et la forme de [forme-lisible.md](contracts/forme-lisible.md). Elle sert l'aperçu (US1) et le lecteur (US3, US4).

- [X] T029 Passer `pdfium-render` en dépendance normale de `backend/crates/modules/negotiation/Cargo.toml`, derrière le chargement de `PDFIUM_LIB_PATH`. Ajouter la bibliothèque à l'image du worker (`backend/Dockerfile` ou son équivalent dans `ops/`)
- [X] T030 [P] Écrire les types de la forme lisible — `ReadingPage`, `Block`, `Span`, `OutlineEntry` — dans `backend/crates/modules/negotiation/src/domain/extraction/forme.rs`, sérialisés comme le contrat
- [X] T031 [P] Écrire les règles pures de R7, ajustées par l'essai, **un fichier par règle**, sous `backend/crates/modules/negotiation/src/domain/extraction/` : `colonnes.rs`, `entetes.rs`, `notes.rs`, `origine.rs`, `cesures.rs`, `termes.rs`, `titres.rs`, `sommaire.rs`, `pages.rs`. Chacune a ses tests unitaires sur des segments construits à la main *(fait : `colonnes.rs` est devenu `ordre.rs` — l'essai a montré que l'ordre du flux fait foi, seules les formes flottantes se replacent ; s'y ajoutent `lignes.rs`, `decoupage.rs` et `blocs.rs`)*
- [X] T032 Écrire l'assemblage dans `backend/crates/modules/negotiation/src/domain/extraction/mod.rs` : segments PDFium → règles → pages, sommaire, verdict et indicateurs (`quality`), poids de la copie (`reading_bytes`)
- [X] T033 Écrire `backend/crates/modules/negotiation/src/repo/document_pages.rs` : remplacer toutes les pages d'un document dans une transaction, lire la forme lisible entière, et `backend/crates/modules/negotiation/src/repo/renditions.rs` pour l'état
- [X] T034 Écrire le travail `negotiation.document.extract` dans `backend/crates/modules/negotiation/src/jobs/extract.rs` :
  - il lit `media.object_location()` ;
  - tant que l'objet n'est pas prêt, il se replanifie avec un délai croissant ;
  - `quarantined` ou `failed` le font conclure « échec », avec un motif lisible ;
  - sinon il lit l'objet par `kernel::storage`, extrait le texte, rend les images de page et les dépose dans le bucket privé sous `documents/<id>/pages/<n>.jpg`, puis écrit les pages et l'état.

  Il écrit par `Db::write`, avec pour acteur la personne qui a attaché le fichier
- [X] T035 Enregistrer le travail dans `negotiation::job_handlers` (`backend/crates/modules/negotiation/src/jobs/mod.rs`) et dans `backend/crates/worker/src/main.rs`
- [X] T036 *(fabriqué par `examples/fabriquer_petit_pdf.rs` ; le travail entier est éprouvé par `tests/extraction_travail.rs`)* Commiter un petit PDF de test, `backend/crates/modules/negotiation/tests/fixtures/petit.pdf` (quatre pages : deux colonnes, un pied répété, une note, un italique anglais, un tableau, un mot coupé), et écrire `backend/crates/modules/negotiation/tests/extraction.rs` : la forme produite respecte les invariants de [forme-lisible.md](contracts/forme-lisible.md), et chaque règle y est vérifiée
- [X] T037 Mesurer l'extraction du vrai guide par le travail, sur un poste de développement. Elle doit durer moins d'une minute pour 90 pages ; consigner la mesure dans `essai-extraction.md`

**Checkpoint** : un PDF attaché devient des pages en base, et des images dans le bucket privé. **Commit.**

---

## Phase 5 : Fondation — l'API

**Objectif** : les contrats servis, avant les écrans. Toutes les routes sont dans le crate `negotiation`, plates, comme celles de 0b.

- [X] T038 Ajouter les treize codes d'erreur ([api-documents.md](contracts/api-documents.md), [api-admin-documents.md](contracts/api-admin-documents.md)) au catalogue de `backend/crates/kernel/src/error.rs`, et les traductions des contraintes (`ck_documents_source_at_most_one`, `ck_documents_published_has_source`, `ux_documents_supersedes`, le cycle, la taxonomie, la page) dans `backend/crates/kernel/src/pg_error.rs`
- [X] T039 Déclarer les permissions `DOCUMENT_PUBLISH`, `CORRECTION_POST` et `CORRECTION_WITHDRAW`, avec leurs `PermissionSpec`, dans `backend/crates/modules/negotiation/src/domain/permissions.rs`
- [X] T040 [P] Écrire `backend/crates/modules/negotiation/src/repo/document_themes.rs` sur le patron de `programme/src/repo/themes.rs:100-150` : couple `('negotiation','documents')` en littéral, taxonomie `negotiation_theme` et `is_active` vérifiés à l'écriture, code refusé nommé (`NEGOTIATION_DOCUMENT_UNKNOWN_THEME`), lecture qui écarte les termes désactivés, effacement des liens dans la transaction appelante (R16)
- [X] T041 [P] Écrire `backend/crates/modules/negotiation/src/repo/documents.rs` :
  - brouillon, modification, source, publication, dépublication, nouvelle version, suppression d'un brouillon (avec ses liens de thématiques) ;
  - la liste publique **selon l'accès** ;
  - le bout publié de la chaîne de remplacement, par requête récursive (R18).
- [X] T042 [P] Écrire `backend/crates/modules/negotiation/src/repo/corrections.rs` (poser, retirer — idempotent —, lister les vivantes de tous les documents publiés, lister toutes celles d'un document) et `backend/crates/modules/negotiation/src/repo/bookmarks.rs` (lister, poser, retirer, tous idempotents)
- [X] T043 Écrire `backend/crates/modules/negotiation/src/service/documents.rs` (lectures publiques, recherche dans le texte sans passage pour un réservé inaccessible, compteur) et `service/admin_documents.rs` :
  - fichier figé une fois publié ;
  - publication conditionnée à l'extraction ou au mode « tel quel » ;
  - mise en file de l'extraction dans la transaction de `PUT …/file`.
- [X] T044 Écrire `backend/crates/modules/negotiation/src/service/corrections.rs` : poser et retirer, en portée globale, chaque écriture par `Db::write`
- [X] T045 Écrire les routes publiques dans `backend/crates/modules/negotiation/src/routes/documents.rs` : liste et recherche, forme lisible, image de page servie par `kernel::storage`, notes, compteur, favoris ([api-documents.md](contracts/api-documents.md)). Chaque lecture listée porte son `ETag` et répond `304` par `routes::inchange()`, avec `Cache-Control` selon l'accès
- [X] T046 Écrire les routes d'administration dans `backend/crates/modules/negotiation/src/routes/admin_documents.rs` ([api-admin-documents.md](contracts/api-admin-documents.md)), dont la variante d'image de page et le PDF d'origine. Les lectures ouvertes à `publish` **ou** `correction.post` se testent dans le gestionnaire
- [X] T047 Monter les deux fichiers de routes dans `backend/crates/modules/negotiation/src/lib.rs` (`routes()` et `admin_routes()`), puis `make openapi` et `make check-api-contract`
- [X] T048 [P] Tests sur base réelle, `backend/crates/modules/negotiation/tests/documents_public.rs` : chemin nominal, `304`, remplacement (brouillon ignoré, chaîne de trois), dépublié absent et `404`, favoris idempotents, compteur sans compte
- [X] T049 [P] Tests sur base réelle, `backend/crates/modules/negotiation/tests/documents_reserves.rs` (**SC-007**) : sans compte, avec un compte sans accès et par identifiant forgé, la liste masque résumé et thématiques, `reading`, `image` et `downloads` rendent `403`, la recherche ne rend ni page ni extrait
- [X] T050 [P] Tests sur base réelle, `backend/crates/modules/negotiation/tests/documents_admin.rs` :
  - brouillon, dépôt, extraction sur `fixtures/petit.pdf`, aperçu, publication, nouvelle version, dépublication ;
  - chaque invariant traduit ;
  - suppression d'un brouillon et de ses liens `entity_terms` ;
  - audit.
- [X] T051 [P] Tests sur base réelle, `backend/crates/modules/negotiation/tests/corrections.rs` : l'expert lit l'aperçu, pose et retire une note, **mais ne publie ni ne modifie** ; l'administrateur publie **mais ne pose pas** de note ; page inconnue refusée ; retrait gardé en historique
- [X] T052 Étendre `backend/crates/modules/negotiation/tests/perimetre_url_forgee.rs` et `perimetre_vide_refuse.rs` aux nouvelles routes d'administration : chacune déclare sa garde, et un administrateur d'événement ne passe pas
- [X] T053 [P] Tests HTTP de bout en bout dans `backend/crates/api/tests/routes_negotiation_documents.rs` : `ETag`/`304`, `Cache-Control`, image servie, `403` d'un réservé
- [X] T054 [P] Écrire les types `frontend/app/types/negotiation-documents.ts` et `frontend/app/types/admin-negotiation-documents.ts`, nommés comme les contrats, et les jeux d'exemple `frontend/app/mocks/negotiation-documents.ts`, qui reprennent les cinq documents de la maquette
- [X] T055 Écrire `frontend/app/composables/api/guide-nego-documents.ts` et `frontend/app/composables/api/admin-negotiation-documents.ts`, puis les brancher dans `frontend/app/composables/useApi.ts`, qui ne gagne que le branchement. `make check-api-contract` au vert

**Checkpoint** : les contrats sont servis et testés, SC-007 prouvé. **Commit.**

---

## Phase 6 : Récit 1 — Publier le guide en une journée (P1) 🎯 MVP

**Objectif** : l'IFDD publie le guide depuis le back-office, en le feuilletant page par page.

**Test indépendant** : publier le guide et un lien externe en chronométrant, feuilleter l'aperçu, basculer « ouvrir tel quel », puis constater que l'ancien guide renvoie au nouveau ([quickstart.md § 1](quickstart.md)).

- [X] T056 [P] [US1] Créer les traductions `frontend/i18n/locales/fr/pages/admin.negociations.documents.list.json`, `.form.json` et `.preview.json`, et leurs `en/` — nommées comme celles des codes (`admin.negociations.*`)
- [X] T057 [US1] Écrire `frontend/app/pages/admin/negociations/documents/index.vue` avec les composants `ui/` : état (brouillon, publié, dépublié), type, version, remplacement, extraction ; quatre états ; `UiForbiddenState` sans la permission
- [X] T058 [US1] Écrire le formulaire partagé `frontend/app/components/admin/negotiation/DocumentForm.vue` :
  - titre et résumé fr/en ;
  - type et thématiques, lus au vocabulaire, jamais écrits dans le code ;
  - COP, parmi les éditions servies par la lecture publique des éditions ;
  - version, date, éditeur, langue ;
  - source : `FileField` en PDF **ou** lien, l'autre champ désactivé quand l'un est rempli ;
  - « remplace… », parmi les documents publiés ;
  - réservé ou public ;
  - marqueur de l'assistant décoché par défaut, avec « ne prend effet qu'avec l'assistant ».
- [X] T059 [US1] Écrire `frontend/app/pages/admin/negociations/documents/nouveau.vue`, qui crée le brouillon, et `[id]/index.vue` (un `[id].vue` ferait de l'aperçu une route enfant) : modification, dépôt puis `PUT …/file`, état de l'extraction relu jusqu'à `ready` ou `failed`, relance, publier, dépublier, « Publier une nouvelle version », supprimer un brouillon. Messages d'erreur de l'API affichés tels quels
- [X] T060 [US1] Écrire `frontend/app/pages/admin/negociations/documents/[id]/apercu.vue` :
  - page par page, l'image à gauche et la forme lisible à droite ;
  - le verdict et les indicateurs en tête ;
  - le sommaire repéré ;
  - « ouvrir tel quel » ;
  - navigation au clavier (← →) ;
  - colonnes empilées sur tablette.
- [X] T061 [US1] Ajouter l'entrée « Documents » au menu de Guide Négo dans `frontend/app/layouts/admin.vue`, visible à qui détient `negotiation.document.publish` **ou** `negotiation.correction.post`
- [X] T062 [US1] Dérouler [quickstart.md § 1](quickstart.md) au navigateur sur le vrai guide, chronométrer (SC-001 et SC-001 bis), et consigner les mesures

**Checkpoint** : le guide est publiable par le back-office. **Commit.**

---

## Phase 7 : Fondation — la plomberie hors connexion

**Objectif** : [hors-connexion.md](contracts/hors-connexion.md) et [R11](research.md)-[R12](research.md). Elle conditionne les récits 2, 3 et 5 côté client.

- [X] T063 Apprendre à `lireEtiquete` d'envoyer `If-None-Match` et de rendre « inchangé » sur `304`, sans changer ses appelants de 0c ; ajouter son test dans `frontend/tests/guide-nego/` — les appels étiquetés sortent de `http.ts` dans `composables/api/etiquete.ts`, testable sans Nuxt
- [X] T064 Monter la base `guide-nego` à la version 3 dans `frontend/app/utils/guide-nego/garde.ts` : magasins `copies` et `a-telecharger`, repli en mémoire, aucune fonction qui lève
- [X] T065 Écrire la logique pure des copies dans `frontend/app/utils/guide-nego/copies.ts` : les deux caches `gn-documents-publics` et `gn-documents-reserves`, les clés, la réconciliation avec la liste (dépublié, devenu réservé sans accès), les effacements de [hors-connexion.md](contracts/hors-connexion.md) § Les effacements
- [X] T066 Écrire `frontend/app/composables/guide-nego/useGnCopies.ts` :
  - télécharger en flux avec progression sur `Content-Length`, **sans rien écrire avant le dernier octet** ;
  - annuler par `AbortController` ;
  - écrire la copie dans le cache du bon côté, puis dans `copies` ;
  - envoyer le compteur une fois ;
  - retirer un document, tout retirer.
- [X] T067 Écrire la file de téléchargements demandés sans réseau, dans `frontend/app/utils/guide-nego/a-telecharger.ts` et `useGnCopies`, branchée sur les déclencheurs de `frontend/app/layouts/guide-nego.vue` (ouverture, `online`, `visibilitychange`), distincte de la file d'écritures de 0c
- [X] T068 Brancher l'effacement des réservés dans `frontend/app/composables/guide-nego/useGnSession.ts`, **avant** la vidange de la file et la fermeture de session ; et dans `useGnAcces.ts`, à la lecture d'un accès perdu. **Rien sur une API injoignable** — les deux enchaînements vivent dans `utils/guide-nego/effacements.ts`, et la session finie ailleurs efface aussi
- [X] T069 Écrire les réglages de l'appareil dans `frontend/app/utils/guide-nego/appareil-lecture.ts` : taille du texte, progression par document et version, documents ouverts (pour « Nouveau »), derniers ouverts ; `localStorage` protégé par `try/catch`
- [X] T070 Mettre à jour les textes de déconnexion dans `frontend/i18n/locales/{fr,en}/pages/guide-nego.reglages.json` : ce qui reste (les publics) et ce qui s'efface (les réservés)
- [X] T071 [P] Tests sans navigateur dans `frontend/tests/guide-nego/`, sur de faux `caches` et un faux IndexedDB — `copies.test.ts`, `deconnexion-reserves.test.ts`, `a-telecharger.test.ts`, `nouveau.test.ts` — couvrant les neuf cas de [hors-connexion.md](contracts/hors-connexion.md) § Tests
- [X] T072 [P] Ajouter à `frontend/tests/guide-nego/sw-garde.test.ts` le cas nommé : le ménage du service worker laisse `gn-documents-*`

- [X] T072 bis Deux exigences du commanditaire (23/09) : `navigator.storage.persist()` demandé au premier téléchargement, dans le geste de la personne, et son issue gardée (`gn.stockage-persistant`), redemandée tant qu'elle est refusée ; **à chaque ouverture**, les copies se vérifient contre les caches — une copie que le navigateur a vidée, même en partie, redevient « non téléchargée », et une entrée orpheline s'efface (`verifierLesCopies`, appelé par `layouts/guide-nego.vue`)

**Checkpoint** : un document se garde, s'efface et survit à un déploiement, prouvé sans navigateur. **Commit.**

---

## Phase 8 : Récit 2 — Trouver un document et lire sa fiche, même sans compte (P1)

**Objectif** : la bibliothèque, ses filtres et la fiche, sans compte et sans réseau.

**Test indépendant** : [quickstart.md § 2](quickstart.md).

- [X] T073 [P] [US2] Créer les traductions `frontend/i18n/locales/fr/pages/guide-nego.documents.json` et `guide-nego.document.json`, et leurs `en/`. **Aucun libellé de type, de thématique ni de COP**
- [X] T074 [P] [US2] Écrire la logique pure de la bibliothèque dans `frontend/app/utils/guide-nego/documents.ts` : filtres combinés, compteurs par valeur, « Afficher n documents », recherche sur titre, résumé et éditeur **sans accents ni casse**, « Nouveau » (moins de sept jours **et** jamais ouvert), marques d'une ligne, avec `frontend/tests/guide-nego/documents.test.ts`
- [X] T075 [P] [US2] Créer `frontend/app/components/guide-nego/GnLigneDocument.vue` (`composants.md:41-44`) et `GnFeuilleFiltre.vue` (`composants.md:145-147`), sur `GnMarqueEtat`, `GnPilule` et `GnFeuilleBasse`, avec leurs traductions de composant
- [X] T076 [P] [US2] Créer `frontend/app/components/guide-nego/GnBandeauRemplace.vue` (`composants.md:185-187`), tout le bandeau étant la cible
- [X] T077 [US2] Écrire `frontend/app/composables/guide-nego/useGnDocuments.ts` : la liste par `useGnLecture` (clé `documents`) avec son empreinte, relue par `relireLaBibliotheque` (`If-None-Match`, un `304` garde la liste) ; `useGnCopies().rapprocher()` à chaque lecture réussie ; la recherche dans le texte en ligne
- [X] T078 [US2] Écrire `frontend/app/pages/guide-nego/ressources/documents/index.vue` : titre et compte, recherche, trois filtres, liste, « n nouveaux », vide après filtre avec « Retirer les filtres », hors connexion avec « n lisible(s) maintenant » et les deux libellés de ligne, quatre états
- [X] T079 [US2] Écrire `frontend/app/pages/guide-nego/ressources/documents/[id]/index.vue` (et non `[id].vue` : `[id]/lire.vue` vient à côté) :
  - les six états de la fiche : non téléchargé, remplacé, en cours, téléchargé, lien, réservé ;
  - thématiques ou « Aucune » ;
  - détails avec la COP ;
  - « Favori » (compte exigé) ;
  - « Partager » par le partage du téléphone, repli : copier l'adresse ;
  - « Télécharger pour lire sans réseau » **sans compte** ;
  - le verrou de 0b (`GnVerrou`) sur un réservé.
- [X] T080 [US2] Relier « Documents de négociation » depuis `frontend/app/pages/guide-nego/ressources/index.vue`
- [X] T081 [US2] Corriger l'écran « 02 Ouverture » (écart 41) dans `frontend/i18n/locales/{fr,en}/pages/guide-nego.ouverture.json` : `avec-compte.favoris` → « Favoris, quiz », et le groupe « sans compte » annonce « Documents de négociation, à lire sans réseau »

**Checkpoint** : sans compte, on trouve, on filtre, on lit une fiche, y compris hors connexion. **Commit.**

---

## Phase 9 : Récit 3 — Télécharger et lire en salle, sans réseau (P1)

**Objectif** : le critère de sortie. Le lecteur, sa barre, la reprise, et l'état « pas sur le téléphone ».

**Test indépendant** : [quickstart.md § 3](quickstart.md), étapes 1 à 4 et 8.

- [X] T082 [P] [US3] Créer les traductions `frontend/i18n/locales/fr/pages/guide-nego.lecteur.json` et `en/`
- [X] T083 [P] [US3] Écrire le rendu de la forme lisible dans `frontend/app/utils/guide-nego/forme-lisible.ts` : blocs et segments en grammaire close, `kind` inconnu ignoré, `term` repéré, avec `frontend/tests/guide-nego/forme-lisible.test.ts`
- [X] T084 [P] [US3] Créer `frontend/app/components/guide-nego/GnBarreLecture.vue` (`composants.md:157-159` : repliée 32 px plus une progression de 6 px, dépliée 68 px ; un toucher au centre bascule, défiler replie ; **sans « Marquer »**, écart 42) et `GnProgression.vue` (barre de téléchargement de 6 px)
- [X] T085 [US3] Écrire `frontend/app/composables/guide-nego/useGnLecteur.ts` : la forme lisible lue **dans le cache d'abord**, par `useGnCopies().lireLaCopie()` qui vérifie la copie entière avant de la rendre, le réseau ensuite ; une image vidée depuis (`imageDeLaCopie` nulle) se relit au réseau, et sans réseau la page s'affiche avec son texte et une ligne qui le dit — **jamais une page blanche** ; le repérage de la page en cours par observateur d'intersection ; la progression écrite au plus toutes les deux secondes ; la reprise pour la même version
- [X] T086 [US3] Écrire `frontend/app/pages/guide-nego/ressources/documents/[id]/lire.vue`, sans barre d'onglets :
  - l'en-tête d'une ligne ;
  - le texte recomposé, avec ses repères de page ;
  - le pied « Page n sur N · section » ;
  - « Reprise à la page n — lue … » et « Début » ;
  - le mode « tel quel » en images de page empilées ;
  - « Voir la page d'origine » sur un bloc `origine`, avec « Page d'origine disponible avec le réseau » si l'image n'est pas gardée.
- [X] T087 [US3] Écrire l'état « Ce document n'est pas sur votre téléphone » dans `lire.vue` : pages, taille, « Télécharger au retour du réseau », et un document lisible maintenant s'il y en a un
- [X] T088 [US3] *(fait en phase 8 avec la fiche ; la barre passe par `GnProgression`)* Brancher la progression du téléchargement (« Téléchargement — n % », « x Mo sur y Mo », « Annuler ») et « Retirer du téléphone — n Mo » dans la fiche `[id].vue`
- [X] T089 [US3] *(23/09, navigateur sans interface ; l'étape 8 avec le guide, seul fichier de la base locale — sa copie retirée hors connexion)* Dérouler [quickstart.md § 3](quickstart.md) étapes 1 à 4 et 8 **sur la version construite**, en mode avion, application fermée puis rouverte, sans compte (SC-002, SC-003)

**Checkpoint** : le guide se lit en salle sans réseau. **Critère de sortie atteint sur poste. Commit.**

---

## Phase 10 : Récit 4 — Lire confortablement (P2)

**Objectif** : sommaire, recherche dans le document, taille, thème, terme anglais.

**Test indépendant** : [quickstart.md § 3](quickstart.md), étapes 5 à 7.

- [X] T090 [P] [US4] Créer `frontend/app/components/guide-nego/GnLigneSommaire.vue` (`composants.md:72-74`) et `GnOccurrence.vue` (« Occurrence 3 sur 5 », précédente, suivante, fermer)
- [X] T091 [P] [US4] Écrire la recherche dans le document dans `frontend/app/utils/guide-nego/lecteur.ts` : normalisation des accents et de la casse, passages avec page, section et extrait, décompte « n passages dans N pages », passage de la page en cours. Test de rapidité : cent pages en moins d'une seconde (SC-009), dans `frontend/tests/guide-nego/recherche.test.ts`
- [X] T092 [US4] Ajouter à `lire.vue` le sommaire (chapitres repliables, section en cours, saut de page) et la recherche avec ses occurrences, le surlignage plein de l'occurrence courante et le clair des autres
- [X] T093 [US4] Ajouter à `lire.vue` la feuille « Réglages » : taille Normale 17, Grande 20, Très grande 24, interligne 1,5, titres fixes, par `GnSegmente` ; le thème **par `useGnTheme` de 0a**, pas un second réglage ; « S'applique à tous les documents. Aussi dans votre profil. » La page en cours est conservée au changement de taille
- [X] T094 [US4] Ajouter à `lire.vue` la feuille du terme touché (`GnFeuilleBasse`), titrée du terme, **sans traduction ni définition**, avec la phrase qui dit qu'elles viennent avec le lexique, et « Revenir au texte »
- [X] T095 [US4] Vérifier le lecteur à 320, 360 et 390 px, dans les deux thèmes et les trois tailles, sans défilement horizontal (SC-010)

**Checkpoint** : le lecteur est complet. **Commit.**

---

## Phase 11 : Récit 5 — « Mes documents », la place, et ce qui s'efface (P2)

**Objectif** : les favoris du compte, les copies de l'appareil, tout retirer, et les réservés effacés.

**Test indépendant** : [quickstart.md § 5](quickstart.md).

- [X] T096 [P] [US5] Créer les traductions `frontend/i18n/locales/fr/pages/guide-nego.mes-documents.json` et `en/`
- [X] T097 [US5] *(fait en phase 8 : « Favori » de la fiche en a besoin ; la file de 0c sert la famille de clés `favori-` par `inscrireFamille`)* Écrire `frontend/app/composables/guide-nego/useGnFavoris.ts` : lecture par `useGnLecture` (clé `mes-favoris`) ; pose et retrait par la file de 0c (clé `favori-<id>`, corps = état voulu, sans `If-Match`) ; effacement de la garde des favoris à la déconnexion
- [X] T098 [US5] Transformer `frontend/app/pages/guide-nego/ressources/telechargements.vue` en « Mes documents » *(renommée `mes-documents.vue`, pour que la page et sa traduction portent le même nom ; l'adresse suit)* :
  - l'en-tête « n téléchargé(s) · n favori(s) » ;
  - « Sur le téléphone » avec la place et les dates ;
  - `GnJauge` : place utilisée et libre, jamais un faux zéro ;
  - « Favoris », avec leur rappel, et l'invitation à se connecter sans compte ;
  - « Tout retirer du téléphone » par `GnConfirmation` (aplat rouge assombri, écart 33), qui nomme les documents et la place ;
  - la place relue après l'effacement ;
  - si `useGnCopies().persistance` vaut `refusee` : une ligne qui dit que le téléphone peut effacer les copies quand il manque de place, et qu'il faudra les retélécharger (exigence du 23/09).
- [X] T099 [US5] Mettre à jour la ligne « Mes téléchargements » du profil (`frontend/app/pages/guide-nego/ressources/reglages.vue`) : nombre de documents gardés et leur place, qui mène à « Mes documents »
- [X] T100 [US5] Remplir le bloc « Documents récents » de « Ma journée » (`frontend/app/pages/guide-nego/index.vue`, `utils/guide-nego/journee.ts`) avec les derniers ouverts sur l'appareil et leur dernière page lue ; son état vide reste pour un appareil neuf
- [X] T101 [US5] Corriger le paragraphe de confidentialité de « À propos » dans `frontend/i18n/locales/{fr,en}/pages/guide-nego.a-propos.json` : les téléchargements restent sur le téléphone ; thématiques, favoris et accords suivent le compte
- [ ] T102 [US5] Dérouler [quickstart.md § 5](quickstart.md) **sur la version construite** : favori sans réseau, second appareil, tout retirer et la place qui baisse (SC-005), déconnexion (SC-006), accès retiré, API coupée qui n'efface rien, copie dépubliée *(23/09 : déroulé sauf les étapes qui demandent un compte avec l'accès négociateur — la copie réservée, la déconnexion qui l'efface, l'accès retiré ; aucun compte de recette n'a cet accès, et l'ouvrir attend l'accord du commanditaire)*

**Checkpoint** : la place se maîtrise, et les réservés ne restent pas. **Commit.**

---

## Phase 12 : Récit 6 — La note de correction d'un expert (P3)

**Objectif** : poser et retirer une note depuis l'aperçu, la lire dans le lecteur, jusque sur la copie gardée.

**Test indépendant** : [quickstart.md § 4](quickstart.md).

- [X] T103 [US6] *(le panneau vit dans `components/admin/negotiation/PreviewNotes.vue`, posé sous la forme lisible)* Ajouter la pose et le retrait des notes à `frontend/app/pages/admin/negociations/documents/[id]/apercu.vue` : choisir la page, sélectionner un passage dans la forme lisible (l'extrait cité), écrire le texte fr, et en en facultatif ; la liste des notes vivantes et retirées en marge, avec auteurs et dates ; boutons selon `correction.post` et `correction.withdraw`
- [X] T104 [P] [US6] Créer `frontend/app/components/guide-nego/GnNoteCorrection.vue` (`composants.md:201-203`) : filet de 3 px, triangle rouge (écart 20 élargi), ligne repliée de 48 px « Note de correction — passage dépassé », boîte dépliée signée « Nom, expert IFDD — date »
- [X] T105 [US6] *(le repli sans accents sort de `lecteur.ts` dans `repli.ts`, que les deux partagent sans import circulaire)* Écrire la lecture des notes dans `useGnDocuments.ts` (clé `corrections`, empreinte) et leur ancrage dans `frontend/app/utils/guide-nego/forme-lisible.ts` : l'extrait cité retrouvé dans les segments de la page, sinon la tête de page. Test : ancrage trouvé, et repli en tête
- [X] T106 [US6] Poser les notes dans `lire.vue` par-dessus le texte, **sans le modifier**, sur la copie gardée comme en ligne
- [X] T107 [US6] Dérouler [quickstart.md § 4](quickstart.md) : une note posée paraît sans retéléchargement, reste en mode avion, et disparaît une fois retirée (SC-008) *(24/09, sur la version construite, API sur 8081 liée à `localhost:3001` : le port 3000 était pris par un autre projet)*

**Checkpoint** : un passage dépassé se corrige sans republier. **Commit.**

---

## Phase 13 : Recette et finitions

- [X] T108 [P] *(24/09 : sept y étaient depuis les phases 8 à 10 ; `GnNoteCorrection` ajoutée, repliée, dépliée et en tête de page ; « ligne de document » n'était déjà plus dans la note « absents »)* Ajouter les huit composants nouveaux à la planche, `frontend/app/components/guide-nego/planche/`, dans les deux thèmes, et retirer « ligne de document » de la note « absents » (`frontend/i18n/locales/fr/components/gn-planche-composants.json`)
- [X] T109 [P] Relire toutes les traductions `en/` de l'étape *(24/09 : 31 paires concordent clé pour clé ; 77 corrections d'anglais dans 15 fichiers, surtout le calque « read » pour « lu » au sens de chargé, et « offline » partout pour « sans réseau »)*
- [X] T110 *(24/09 : aucun libellé dans le code ni dans les écrans ; seules les planches en portent, comme spécimens — thématiques de 0a, types de la phase 8 —, exception déjà consignée)* Vérifier par `grep` qu'aucun libellé de type, de thématique ni de COP n'apparaît dans `frontend/i18n` ni `frontend/app` (SC-011), puis lancer `npm run check:guide-nego` : bornage, aucun composant du site, aucun fichier de plus de mille lignes
- [X] T111 *(24/09 : aucun débordement sur 30 mesures — cinq écrans, trois largeurs, deux thèmes —, ni en « Très grande » à 320 px ; vide, erreur et accès refusé vus)* Vérifier les quatre états et les tailles 320, 360 et 390 px de la bibliothèque, de la fiche, de « Mes documents » et du lecteur, dans les deux thèmes (SC-010)
- [X] T112 *(24/09 : dépôt d'une vignette par `ImageField` en 201 dans le bucket public, servie par le relais, puis « Annuler » sans rien rattacher ; les médias de l'accueil et de la fiche d'édition chargés ; contrat : toutes les formes définies)* Non-régression du site : dépôt d'une image par `ImageField`, médias publics affichés, `make check-api-contract` à zéro route en attente
- [X] T113 *(24/09 : § 1 déroulé le 23/09 et inchangé ; § 2 sécurité — 403 sans compte et sans accès, recherche sans page ni extrait du réservé, relais média 404 sur le PDF réservé — ; § 3 hors réseau ; § 4 le jour même ; § 5 en partie, voir T102 ; § 6 par T111 et T112. Écarts consignés dans progress.md)* Dérouler [quickstart.md](quickstart.md) en entier sur la version construite et l'API réelle, et consigner les écarts
- [X] T114 `make check-safe`, **API arrêtée** — jamais `make check` *(24/09 : au vert, 1 306 tests Rust, aucun échec)*
- [X] T115 Mettre à jour [docs/AppNego/progress.md](../../docs/AppNego/progress.md) (ligne de l'étape et journal) et, si le déploiement change (bucket privé, PDFium dans le worker, migration), le § 15 de [docs/DEPLOIEMENT.md](../../docs/DEPLOIEMENT.md)
- [ ] T116 **Sur appareil réel, hors poste** : installer sur un Android et un iPhone, télécharger le guide en 3G bridée, le lire en mode avion après une nuit de veille. C'est la preuve du critère de sortie, avec T112 de 0b

---

## Dépendances et ordre

- **Phase 1 (essai) bloque tout.** Sur l'issue B, la phase 4 change de forme ([plan.md § Si l'essai rend l'issue B ou C](plan.md)).
- **Phases 2 → 3 → 4 → 5** : séquentielles. Le stockage partagé précède l'extraction ; le modèle précède tout le reste.
- **Phase 6 (US1)** dépend de 5. Elle fournit les vraies données des phases suivantes.
- **Phase 7 (hors connexion)** dépend de 5. Elle peut se mener **en parallèle de la phase 6** : fichiers disjoints, back-office d'un côté, Guide Négo de l'autre.
- **Phase 8 (US2)** dépend de 7.
- **Phase 9 (US3)** dépend de 7 et 8 : la fiche porte le téléchargement.
- **Phase 10 (US4)** dépend de 9.
- **Phase 11 (US5)** dépend de 7 et 8. Elle peut précéder la phase 10.
- **Phase 12 (US6)** dépend de 6 (aperçu) et de 9 (lecteur).
- **Phase 13** en dernier.

### En parallèle, dans une même phase

- **Phase 2** : T016 et T017, une fois T011 fait.
- **Phase 4** : T030 et T031, puis T036 en regard.
- **Phase 5** : T040, T041 et T042 (dépôts distincts), puis T048 à T051 et T053 (fichiers de test distincts), et T054 pendant les routes.
- **Phases 6 et 7** entières : l'une au back-office, l'autre dans Guide Négo.
- **Phase 8** : T073, T074, T075 et T076.
- **Phase 9** : T082, T083 et T084.
- **Phase 10** : T090 et T091.

## Stratégie

- **Le MVP de l'étape est le critère de sortie** : phases 1 à 9. Le guide est publié (US1), trouvé (US2) et lu en salle sans réseau (US3). Il se démontre au commanditaire à la fin de la phase 9.
- **Ensuite, par valeur** : US5 (la place et les réservés effacés), qui rend l'étape sûre sur une tablette partagée, puis US4 (le confort de lecture), puis US6 (les notes).
- **Chaque phase se ferme par son commit** et ses contrôles ciblés ; `make check-safe` une seule fois, en fin de cycle.

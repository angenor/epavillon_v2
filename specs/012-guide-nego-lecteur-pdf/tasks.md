---
description: "Tâches de l'étape 1b — le lecteur montre le PDF d'origine"
---

# Tasks: Guide Négo — le lecteur montre le PDF d'origine (étape 1b)

**Input** : les documents de `/specs/012-guide-nego-lecteur-pdf/`

**Prérequis** : [plan.md](plan.md) · [spec.md](spec.md) · [research.md](research.md) · [essai-lecteur.md](essai-lecteur.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

**Tests** : **oui, énumérés par les contrats**.
- Sur base réelle : [api-lecture.md](contracts/api-lecture.md), dont SC-009.
- Sans navigateur (`node --test`) : [copie-gardee.md](contracts/copie-gardee.md), la normalisation et le repérage (R7), la détection (R1), la garde (R4).
- Sur le vrai guide : [essai-lecteur.md](essai-lecteur.md), puis la [recette](quickstart.md) sur la version construite.

**Branche** : `012-guide-nego-lecteur-pdf`, déjà créée, partie de `011-guide-nego-documents`, `main` fusionnée. **Un commit par phase.**

## Format : `[ID] [P?] [Récit] Description`

- **[P]** : parallélisable — fichiers différents, aucune dépendance sur une tâche non finie.
- **[US1]…[US6]** : le récit servi, numéroté comme dans [spec.md](spec.md). Les phases de fondation (1 à 4) et la recette n'en portent pas.
- Chemins : `neg/` = `backend/crates/modules/negotiation/`, `F/` = `frontend/`.

## Les portes, et celle qu'on ne franchit pas

- **Pendant le cycle** : `npm run typecheck`, `npm run test:guide-nego`, `npm run check:guide-nego`, `cargo test -p negotiation`, `cargo test -p kernel`, `make openapi`, `make check-api-contract`.
- **En fin de cycle seulement, API arrêtée** : `make check-safe`.

> **`make check` et `make check-db` ne se lancent jamais** : ils détruisent la base locale, qui n'a pas de sauvegarde. La base **se migre**.

---

## Phase 1 : Setup — l'essai, avant tout

**Objectif** : confirmer R1, R2 et R7 sur le vrai guide, `.essais/guide-cdp30.pdf` (90 pages, 2,9 Mo). **Rien du modèle, de l'API ni des écrans ne s'écrit avant la conclusion, relue avec le commanditaire.**

- [ ] T001 **Par le commanditaire ou avec lui — ne bloque rien** : installer Xcode et les environnements de simulateur **iOS 16, 17 et 18** sur le poste (absents au 24/09 : `xcrun simctl` introuvable). Il faut une session App Store et plusieurs dizaines de Go : vérifier d'abord la place libre, vider `backend/target/debug/incremental` s'il le faut, **demander avant de redémarrer Docker**. Sans simulateur ni iPhone ancien, la grille le dit et renvoie à T084
- [X] T002 Relever les caractéristiques du guide — `pdffonts`, `pdfimages -list`, `qpdf --check` — dans la section « Le fichier » de `specs/012-guide-nego-lecteur-pdf/essai-lecteur.md` : polices non incorporées, polices CID, JPEG 2000, JBIG2, linéarisation
- [X] T003 Ajouter `pdfjs-dist` en **version exacte** (6.3.x) à `F/package.json`, `npm install`, et consigner la version dans `essai-lecteur.md`
- [X] T004 *(mesuré de bout en bout le 24/09 ; prototype et banc hors de Git dans `.essais/lecteur-1b/`, voir son `LISEZMOI.md`)* Écrire le prototype jetable `F/app/pages/guide-nego/essai-pdf.vue` (`ssr: false`, hors menu) :
  - pdf.js legacy par import dynamique, travailleur par `?url` ;
  - `PDFViewer`, `EventBus`, `PDFLinkService`, `TouchManager` relié à `updateScale` ;
  - échelle `page-width`, plafond de quatre fois la largeur ;
  - source au choix : l'URL du guide copié dans `F/public/` le temps de l'essai (**jamais commité**), ou ses octets tirés de Cache Storage ;
  - `wasmUrl`, `iccUrl`, `standardFontDataUrl` vers une copie temporaire des ressources du paquet ;
  - un bouton qui garde le guide dans un cache de test, pour le mode avion
- [X] T005 Brancher dans le prototype un repérage **provisoire** : normalisation (césure `U+0002` et trait d'union de fin de ligne, ligatures `ﬁ` `ﬂ` `ﬀ` `ﬃ` `ﬄ`, insécables `U+00A0` et `U+202F`, apostrophes et guillemets, fragments) et recherche dans `textContentItemsStr`. Cinquante expressions et dix passages de note, tirés de la forme lisible du guide en base, avec leurs césures, ligatures et deux colonnes ; compter les passages retrouvés
- [X] T006 Construire (`nuxt build`), servir la version construite, et mesurer dans Chrome en émulation mobile (360 × 780, CPU ×4) les critères 1 à 4, 6 et 8 de la grille ; réseau ordinaire **et** « 3G lente » pour le 4 ; trois allers-retours pour le 2
- [X] T007 Mesurer 5a et 5b — **taux** de passages de recherche et de note retrouvés — et le critère 7 hors connexion sous Chrome : travailleur, `wasm` et polices servis depuis un cache, XHR synchrone du travailleur compris
- [X] T008 *(WebKit 26.6 de Playwright ; sans simulateur iOS, le 9 est déduit et renvoyé à T084)* Sous WebKit — Safari du Mac, puis simulateurs iOS 16, 17 et 18 **s'ils sont disponibles (T001)** — mesurer le 7, puis le 9 : **pages affichées ou non**, et pour chaque version **les fonctions qui manquent** (erreur de chargement, erreur de rendu, `Math.sumPrecise`, `ReadableStream[Symbol.asyncIterator]`, module dans un `Worker`, etc.). Remplir le 10 : la liste des fonctions à détecter, sans polyfill possible
- [X] T009 *(24/09 : issue A retenue, [ADR-022](../../docs/AppNego/adr/022-pdfjs-dans-le-client.md) écrit avec les quatre décisions du commanditaire ; spec, plan, recherche R15, contrat du lecteur et quickstart mis à jour)* Conclure `essai-lecteur.md` : issue A, B ou C, taux de repérage, plancher iOS mesuré. Écrire `docs/AppNego/adr/022-pdfjs-dans-le-client.md` — pdf.js legacy et `PDFViewer`, la copie qui porte le PDF — et passer [ADR-021](../../docs/AppNego/adr/021-pdfium-dans-le-worker.md) au statut « sa règle « le PDF ne va jamais sur le téléphone » est remplacée par ADR-022 »
- [X] T010 *(24/09 : relu, issue A et quatre décisions ; le prototype reste dans `.essais/lecteur-1b/`, hors de Git, comme banc de mesure jusqu'à la recette — rien n'en est dans `frontend/`)* **Relire l'essai avec le commanditaire** — issue, taux de repérage, iPhone qui ne passent pas —, puis **supprimer** `essai-pdf.vue`, la copie du guide et les ressources temporaires. Commit de la phase : la grille, l'ADR, `package.json`

**Point d'arrêt** : issue B ou C → appliquer ce que dit [plan.md](plan.md) (« Si l'essai rend l'issue B ou C ») avant la phase 2.

---

## Phase 2 : Fondation — le modèle

**Objectif** : « ouvrir tel quel » devient le choix « Texte agrandi », et une fonction porte la règle ([data-model.md](data-model.md)).

- [X] T011 Dans `docs/database/100_negotiations.sql`, `negotiation.document_renditions` :
  - `serve_as_is boolean NOT NULL DEFAULT false` devient `large_text_choice boolean NULL`, avec son `COMMENT ON` (« NULL = suit le verdict de l'extraction ; vrai ou faux = choix de l'administratrice, arbitré le 24/09 ») ;
  - le commentaire de `reading_bytes` dit « octets du PDF + octets du JSON de lecture — la copie gardée depuis l'étape 1b »
- [X] T012 Dans le même fichier, écrire `negotiation.document_reading_modes(p_document_id uuid)` — `STABLE`, `SECURITY INVOKER`, rend `has_text` et `large_text` selon [data-model.md § 2](data-model.md) —, avec son `COMMENT ON`. Réécrire les `COMMENT ON` de `image_key`, `image_bytes` et `has_origin_block` : « aperçu du back-office — jamais servi au téléphone depuis l'étape 1b »
- [X] T013 Écrire `specs/012-guide-nego-lecteur-pdf/migration.sql`, **rejouable**, dans une transaction, à jouer **après** celle de l'étape 1 : renommage conditionnel de la colonne, `DROP NOT NULL`, `DROP DEFAULT`, `UPDATE … SET large_text_choice = NULL`, `CREATE OR REPLACE FUNCTION`, commentaires
- [X] T014 Jouer la migration **deux fois** sur la base locale, **sans la détruire**. Comparer le schéma avec un chargement de `docs/database/` dans une base **jetable** — jamais `make check`. Vérifier les quatre cas de la fonction par une requête : verdict vrai, verdict faux, choix contraire, sans texte malgré un choix vrai
- [X] T015 Consigner le changement dans `docs/progression/modele.md` (ADR-017). Commit de la phase

---

## Phase 3 : Fondation — les plages et l'API

**Objectif** : la route du fichier, par plages, l'accès vérifié à chaque morceau ; la lecture sans image ; le choix « Texte agrandi » au back-office ([contracts/api-lecture.md](contracts/api-lecture.md)).

### Le stockage

- [ ] T016 Ajouter `get_range(&self, key, debut: u64, fin: u64) -> Result<Vec<u8>>` au trait `ObjectStore` de `backend/crates/kernel/src/storage/mod.rs`, et son implémentation dans `storage/s3.rs` (`GetObject` avec `Range: bytes=debut-fin`, signé par `sigv4.rs`) et dans `storage/filesystem.rs` (lecture positionnée)
- [ ] T017 [P] Tester `get_range` dans `kernel` sur le stockage de fichiers : début, fin, dernier octet, plage d'un octet, plage au-delà de la taille

### La route du fichier

- [ ] T018 [P] Ajouter le code `NEGOTIATION_DOCUMENT_RANGE_INVALID` (416, « La partie demandée du document n'existe pas. ») à `backend/crates/kernel/src/error.rs`
- [ ] T019 [P] Écrire `neg/src/domain/plage.rs` : l'analyse pure de l'en-tête `Range` — `bytes=a-b`, `bytes=a-`, `bytes=-n`, plusieurs plages (la première seule), forme invalide (ignorée, rend `200`), hors du fichier (`416`) — avec ses tests unitaires dans le fichier
- [ ] T020 Dans `neg/src/service/documents.rs`, écrire `lire_le_fichier(id, plage, acteur)` :
  - passe par `lisible()` **à chaque appel** ;
  - lit la taille par `head`, puis `get_range` ou `get` ;
  - rend les octets, la taille, l'`asset_id` pour l'empreinte, et le caractère réservé
- [ ] T021 Dans `neg/src/routes/documents.rs`, ajouter `GET` et `HEAD /negotiation/documents/{id}/file`, avec le contrat complet :
  - `200` ou `206` et `Content-Range`, `416` et `Content-Range: bytes */total` ;
  - `Accept-Ranges: bytes`, `ETag` fort, `Content-Disposition: inline` ;
  - **`Content-Encoding: identity`** ;
  - `Cache-Control` : `private, max-age=3600, no-transform` pour un public, `private, no-store, no-transform` pour un réservé ;
  - `304` sur `If-None-Match`, comparé par `kernel::empreinte`, suffixe de relais retiré ;
  - l'annotation OpenAPI
- [ ] T022 Supprimer la route publique `GET /negotiation/documents/{id}/pages/{index}/image` de `neg/src/routes/documents.rs`, et la lecture d'image publique de `neg/src/service/documents.rs` (`lire_image`). La route **admin** reste
- [ ] T023 Dans `backend/crates/api/src/lib.rs` (contrôle d'origine maison) : accepter `Range` et `If-None-Match` en préflight, et exposer `Content-Range`, `Accept-Ranges`, `Content-Length` et `ETag`

### La lecture et la liste

- [ ] T024 Dans `neg/src/repo/renditions.rs` : `large_text_choice` remplace `serve_as_is`, en lecture comme en écriture (`poser_tel_quel` devient `poser_le_choix_texte_agrandi`, `Option<bool>`). La relance d'extraction (`demander`) garde le choix. Lecture de `negotiation.document_reading_modes()`
- [ ] T025 Dans `neg/src/service/documents.rs` et `neg/src/domain/documents.rs` :
  - `DocumentReading` perd `mode` et `pages[].image`, et gagne `has_text` et `large_text` ;
  - `DocumentLibrary` (et `DocumentTextHits`) perd `mode`, et gagne `has_text` et `large_text` ;
  - l'empreinte de lecture prend `large_text_choice` à la place de `serve_as_is`
- [ ] T026 Écrire **une seule fois**, dans `neg/src/domain/documents.rs`, la sérialisation de `DocumentReading`, employée par l'API (T025) et par le worker. Dans `neg/src/jobs/extract.rs` : `reading_bytes` = `byte_size` du PDF + taille de cette sérialisation. Le motif d'échec nomme un PDF **protégé par mot de passe**, distingué de l'erreur générique dans `neg/src/pdf.rs:59`
- [ ] T027 Dans `neg/src/routes/admin_documents.rs` et `neg/src/service/admin_documents.rs` : `PUT …/{id}/as-is` devient `PUT …/{id}/large-text`, avec `{ choice: bool | null }`, `Db::write(&ctx)` et la garde `Requires<DocumentPublish>` inchangée. L'aperçu rend `extraction.large_text_choice`, `has_text` et `large_text`

### Les tests sur base réelle

- [ ] T028 [P] Écrire `neg/tests/documents_fichier.rs` :
  - `206` et `Content-Range` pour `a-b`, `a-`, `-n` ; `200` sans `Range` ; `416` ;
  - `304`, y compris avec une empreinte suffixée « -br » ;
  - `HEAD` sans corps ;
  - `Content-Encoding: identity` et `no-transform` sur chaque réponse, `Accept-Encoding: br, gzip` envoyé ;
  - `Accept-Ranges`, `no-store` pour un réservé et jamais pour un public
- [ ] T029 [P] Dans `neg/tests/documents_fichier.rs`, le réservé **à chaque morceau** : sans session, session sans accès, adresse forgée d'un brouillon, document dépublié entre deux morceaux — `403` ou `404`, jamais un octet (SC-009)
- [ ] T030 [P] Adapter `neg/tests/documents_public_lecture.rs` et `neg/tests/documents_reserves.rs` :
  - `mode` et `image` absents ;
  - `has_text` et `large_text` dans les quatre cas ;
  - la route d'image publique rend `404`
- [ ] T031 [P] Ajouter à `neg/tests/documents_admin_extraction.rs` — **pas** à `documents_admin.rs`, qui compte 865 lignes — le choix `large-text` : `true`, `false`, `null` ; la relance qui garde le choix ; l'URL forgée d'un administrateur d'événement refusée ; l'audit qui porte l'auteur ; le motif du PDF protégé
- [ ] T032 `make openapi`, puis `make check-api-contract`. Adapter `F/app/types/negotiation-documents.ts` (`ReadingMode` supprimé, `has_text` et `large_text` ajoutés, `ReadingPage.image` supprimé) et `F/app/types/admin-negotiation-documents.ts` (`serve_as_is` → `large_text_choice`, `ServeAsIsInput` → `LargeTextChoiceInput`). `cargo test -p negotiation -p kernel`. Commit de la phase

---

## Phase 4 : Fondation — pdf.js dans la coquille

**Objectif** : pdf.js, son travailleur et ses ressources sont gardés par la coquille, dans la borne de Guide Négo, et se chargent sans réseau ([R4](research.md)). Le repli d'un téléphone trop ancien se décide ici.

- [ ] T033 Écrire le module Nuxt `F/modules/guide-nego-pdfjs.ts`, déclaré dans `F/nuxt.config.ts`. En développement comme en construction, il copie depuis `node_modules/pdfjs-dist/` vers `F/public/guide-nego/pdfjs/` :
  - `wasm/qcms_bg.wasm`, `wasm/openjpeg.wasm` (et `openjpeg_nowasm_fallback.js` si l'essai l'exige) ;
  - `iccs/`, `standard_fonts/` ;
  - `cmaps/` et `jbig2.wasm` **seulement si T002 les a trouvés nécessaires** ;
  - jamais `quickjs`.

  Ajouter `F/public/guide-nego/pdfjs/` au `.gitignore`
- [ ] T034 Écrire la fonction pure `bornerLaFeuille(css)` dans `F/guide-nego/feuille-pdfjs.ts` : chaque sélecteur de `pdfjs-dist/web/pdf_viewer.css` préfixé par `[data-app="guide-nego"]`, `:root` devenu la borne, `@keyframes` et `@font-face` laissés tels quels, une ligne d'en-tête qui dit que la feuille est engendrée.
  - Le module de T033 l'appelle pour écrire `F/app/assets/guide-nego/pdfjs-viewer.css`, ignoré par Git et importé par le composant lecteur.
  - **`F/scripts/check-guide-nego.mjs` l'appelle aussi**, et engendre la feuille avant de la vérifier : le contrôle ne dépend pas de l'ordre des commandes.
  - **La feuille se termine par la règle de dimensionnement** (ADR-022) : `[data-app="guide-nego"] .pdfViewer, [data-app="guide-nego"] .pdfViewer * { box-sizing: content-box; }`. Sans elle, la couche de texte dérive d'une ligne ou plus. **`check:guide-nego` refuse une feuille qui ne l'a pas.**
  - Test `F/tests/guide-nego/feuille-pdfjs.test.ts` : la feuille du contrôle est identique, octet pour octet, à celle de la construction ; aucun sélecteur hors de la borne ; la règle `content-box` présente
- [ ] T035 Dans `F/guide-nego/liste-de-garde.ts`, ajouter à `FICHIERS_PUBLICS` les chemins copiés par T033, **calculés** depuis le dossier et non recopiés ; adapter `F/tests/guide-nego/liste-de-garde.test.ts` et `F/tests/guide-nego/sw-garde.test.ts`
- [ ] T036 Écrire `F/app/utils/guide-nego/pdf/remplacements.ts`, **pur** : `installerLesRemplacements(portee)` pose `Promise.withResolvers` et `ReadableStream.prototype[Symbol.asyncIterator]` **seulement s'ils manquent**, sur l'objet global qu'on lui donne. Et `F/app/utils/guide-nego/pdf/travailleur.ts`, l'amorce du travailleur : il appelle `installerLesRemplacements(self)`, **puis** importe `pdfjs-dist/legacy/build/pdf.worker.min.mjs` (ADR-022 : iOS 16.4 à 17.3)
- [ ] T036 bis Écrire `F/app/utils/guide-nego/pdf/charger.ts` :
  - `fonctionsManquantes(env)`, **pure**, sur la liste de l'essai (R15) : les blocs `static {}` de classe (sonde de syntaxe), le module dans un `Worker`, `structuredClone`, `Path2D` — **jamais une version lue dans l'identifiant du navigateur** (FR-012 bis) ;
  - `installerLesRemplacements(globalThis)` avant tout import de pdf.js ;
  - `chargerPdfjs()` : import dynamique du legacy, le travailleur lancé depuis l'amorce de T036 (`GlobalWorkerOptions.workerPort`), et un résultat `pret` ou `indisponible` en cas d'échec d'import ;
  - `ouvrirLeDocument(source)` : par octets (`data`) pour une copie gardée ; **en ligne par le transport de T036 ter**, avec `disableAutoFetch: true` et des morceaux de 256 Ko — jamais la lecture par URL de pdf.js, qui prend le fichier entier (essai). Options `wasmUrl`, `iccUrl`, `standardFontDataUrl` sous `assetUrl('/guide-nego/pdfjs/')`, préfixe `/v2` compris
- [ ] T036 ter Écrire `F/app/utils/guide-nego/pdf/transport.ts` : un `PDFDataRangeTransport` qui fait `HEAD` pour la taille, puis `GET` avec `Range` et la session (`credentials: 'include'`), et tient **demandé, reçu, en route** ; il expose `progression` (reçu sur demandé) et `enAttenteDuReseau` (une plage au moins en route). Une réponse autre que `206` ou une coupure rend une erreur de réseau, jamais une erreur de pdf.js
- [ ] T037 [P] Écrire `F/tests/guide-nego/charger.test.ts` : chaque fonction manquante simulée rend `indisponible` ; rien n'est lu dans `navigator.userAgent`. `F/tests/guide-nego/remplacements.test.ts` : **l'absence de `Promise.withResolvers` et de l'itérateur de flux, simulée dans un contexte de page et dans un contexte de travailleur**, est comblée ; une fonction présente n'est jamais remplacée ; l'amorce du travailleur installe les remplacements **avant** d'importer pdf.js. `F/tests/guide-nego/transport.test.ts`, sur un `fetch` simulé : demandé, reçu, en route, progression ; un `200` à une plage rend une erreur de réseau
- [ ] T038 `npm run check:guide-nego` **sur un dépôt sans build préalable** : la feuille est engendrée et passe, aucun sélecteur hors de la borne, la règle `content-box` présente. `nuxt build`, puis `npm run verifier-garde:guide-nego` : les fichiers `pdfjs/`, **l'amorce du travailleur** et le module sont servis et précachés. Commit de la phase

---

## Phase 5 : Récit 4 — La copie gardée : le PDF et son texte (P2), livrée d'abord

**Objectif** : la copie de format 2 — PDF et lecture, sans images, entière ou absente ; les formats anciens s'effacent ([contracts/copie-gardee.md](contracts/copie-gardee.md)). Livrée avant US1, parce que la lecture hors connexion en dépend.

**Test indépendant** : télécharger le guide, couper au milieu, reprendre, comparer la place annoncée et mesurée, simuler une fiche sans `format` et la voir s'effacer.

- [ ] T039 [US4] Dans `F/app/utils/guide-nego/copies.ts` :
  - `FORMAT_DE_COPIE = 2`, et `format` dans la fiche `Copie` ;
  - `pages_images` et `mode` supprimés ; `cles` = `[lecture, pdf]` ;
  - `verifierLesCopies` efface toute fiche sans `format` ou d'un autre format, entrées comprises ;
  - `copieIntacte` exige les deux entrées ;
  - la réconciliation : une empreinte de lecture qui change pour la **même version** ne remplace que l'entrée de lecture (règle 6)
- [ ] T040 [US4] Dans `F/app/composables/guide-nego/useGnCopies.ts`, `executer()` :
  - lit `…/reading` puis `…/file` (sans `Range`, `200`) en flux ;
  - calcule la progression sur la somme des `Content-Length` ;
  - n'écrit qu'une fois tout reçu, sous `enSerie`, fiche en dernier ;
  - ne lit plus aucune image.

  `lireLaCopie` rend la lecture et les octets du PDF ; supprimer `imageDeLaCopie`
- [ ] T041 [P] [US4] Dans `F/app/composables/api/guide-nego-documents.ts` : `cheminDuFichier(id)` et `adresseDuFichier(id)` (absolue, pour pdf.js) ; supprimer `imageDePage`
- [ ] T042 [P] [US4] Étendre `F/tests/guide-nego/copies.test.ts` selon [copie-gardee.md § Tests](contracts/copie-gardee.md) : format 2 ; coupure pendant le PDF ; fiche sans `format` et de format 1 effacées ; choix changé qui ne remplace que la lecture ; réservés effacés avec leur PDF. Adapter `deconnexion-reserves.test.ts`, `mes-documents.test.ts` et `documents-exemples.test.ts`
- [ ] T043 [US4] Adapter les exemples hors ligne — `F/app/mocks/negotiation-documents.ts` et les fichiers qu'il importe, à repérer par `grep -rln "reading\|as_is" F/app/mocks` : plus de `mode`, plus d'`image`, `has_text` et `large_text`. Copier `neg/tests/fixtures/petit.pdf` dans `F/public/gn-exemples/documents/` et le servir comme fichier des documents d'exemple, pour que l'application lise des pages sans API. `npm run test:guide-nego`, `npm run typecheck`. Commit de la phase

---

## Phase 6 : Récit 1 — Lire le guide sur ses pages, en salle, sans réseau (P1) 🎯 MVP

**Objectif** : le document s'ouvre sur ses pages, défile, s'agrandit net, se sélectionne, reprend à la dernière page, en ligne comme en mode avion.

**Test indépendant** : [quickstart § 3 et § 4](quickstart.md), points 1 à 4.

- [ ] T044 [US1] Extraire de `F/app/pages/guide-nego/ressources/documents/[id]/lire.vue` le rendu recomposé — `GnPageLue`, surlignages et occurrences, feuille du terme, notes par bloc — dans `F/app/components/guide-nego/GnLecteurTexte.vue`, **sans changement de comportement**. Vérifier au navigateur que « Texte agrandi » se lit comme avant sur un document en mode texte
- [ ] T045 [US1] Écrire `F/app/components/guide-nego/GnLecteurPages.client.vue`, sur `charger.ts` :
  - conteneur `position: absolute`, `PDFViewer`, `EventBus`, `PDFLinkService` (lien externe dans un nouvel onglet) ;
  - échelle `page-width`, plafond de quatre fois la largeur, `maxCanvasPixels` par défaut (l'essai tient sans le baisser) ;
  - `loadingTask.destroy()` au démontage ;
  - la feuille `pdfjs-viewer.css` importée ;
  - fond autour des pages sur les rôles de Guide Négo, page jamais recolorée (FR-011)
- [ ] T046 [US1] Dans `GnLecteurPages`, les gestes du [contrat](contracts/lecteur.md) :
  - pincement par `TouchManager` → `updateScale({ scaleFactor, origin })` ;
  - double toucher (moins de 300 ms, moins de 24 px) qui double l'échelle autour du point, et revient à la largeur au-delà ;
  - toucher simple émis **après** 300 ms sans second toucher ;
  - l'appui long laissé à la sélection.

  Les gestes vivent dans une fonction pure `F/app/utils/guide-nego/pdf/gestes.ts`
- [ ] T047 [P] [US1] Écrire `F/tests/guide-nego/gestes.test.ts` : toucher, double toucher, deux touchers éloignés, toucher suivi d'un pincement, plafond et retour à la largeur
- [ ] T048 [US1] Dans `F/app/composables/guide-nego/useGnLecteur.ts` :
  - la source du PDF — octets de la copie, sinon `adresseDuFichier(id)` en ligne, sinon l'état `absent` de l'étape 1 ;
  - la page en cours alimentée par `pagechanging` ;
  - la progression écrite toutes les deux secondes, commune aux deux modes ;
  - la reprise « Reprise à la page 59 — … » et « Début » ;
  - supprimer `imageDe` et les URL d'objet d'image
- [ ] T049 [US1] Réorganiser `lire.vue` : l'état, le mode (`pages` par défaut), `GnLecteurPages` ou `GnLecteurTexte`, le pied `GnBarreLecture` avec le numéro imprimé tiré de la lecture (repli sur la position), le recalage à la rotation par `page-width` en gardant la page. Supprimer la classe `gn-lecteur--tel-quel` et le mode « tel quel ». Vérifier `wc -l` sous 1000
- [ ] T050 [US1] Le repli d'un téléphone qui n'affiche pas les pages (FR-012 bis), par deux voies qui mènent au même écran :
  - **la détection** : `charger.ts` rend `indisponible` ;
  - **la seconde sécurité, sans mesure** : `F/app/utils/guide-nego/pdf/bascule.ts`, pur, surveille le premier rendu et rend `bascule` sur une erreur de pdf.js ou une première page non dessinée après **8 s de travail** — réglable. **Le délai ne court que tant que le transport n'a aucune plage en route** (`enAttenteDuReseau` faux), et se suspend dès qu'une part : le réseau lent ne fait jamais basculer (ADR-022). Sur une copie gardée, il court depuis l'ouverture.

  Le lecteur ouvre alors `GnLecteurTexte` quand `large_text` est vrai, avec la ligne `GnAnnonce` ; sinon `GnEtatErreur` « Ce téléphone ne peut pas afficher ce document ». Jamais le visionneur du téléphone. L'événement est compté dans `gn.lecture-bascules` (nombre, dernière cause, date) par `appareil-lecture.ts`. Les pages ne se réessaient qu'à l'ouverture suivante du document : la décision vit dans l'instance du lecteur.
  - Test `F/tests/guide-nego/bascule.test.ts`, avec une horloge simulée : erreur, délai dépassé, rendu à temps ; **30 s d'attente du réseau puis rendu en 2 s : pas de bascule** ; une plage qui part suspend le délai, qui reprend où il en était ; le compteur augmente ; rien n'est relu du stockage pour décider
- [ ] T050 bis [US1] L'attente en ligne (FR-009 bis), `F/app/components/guide-nego/GnAttentePages.vue` :
  - la forme lisible se lit **avant** le PDF ;
  - une jauge « reçu sur demandé », tirée de `transport.progression`, en `role="progressbar"`, qui suit les demandes nouvelles ;
  - **au bout de 3 s sans page** : « Lire le texte en attendant » (si `large_text` ; ouvre `GnLecteurTexte` à la page de reprise) et « Télécharger pour lire sans réseau » (le téléchargement de l'étape 1) ;
  - en lisant le texte en attendant, une ligne « La page arrive » avec « Rester sur le texte » ; la page du PDF prend la place du texte quand elle est dessinée, **sauf** si la personne a touché « Rester sur le texte » ou changé de mode elle-même. La décision vit dans l'instance du lecteur, le mode gardé ne change pas.

  La logique est une fonction pure, `F/app/utils/guide-nego/pdf/attente.ts` (3 s, choix de rester, page prête), testée dans `F/tests/guide-nego/attente.test.ts` avec une horloge simulée
- [ ] T051 [US1] La page non encore reçue quand le réseau tombe : un état par page qui dit qu'il faut le réseau, avec « Télécharger au retour du réseau » (file `a-telecharger` de l'étape 1). Les pages déjà affichées restent
- [ ] T052 [P] [US1] Supprimer `F/app/components/guide-nego/GnImageDePage.vue` et ses emplois ; retirer de `GnPageLue.vue` le chargement d'image (l.68-83, 117-126) et le bouton « voir » des blocs `origin` (l.159-187), que T063 remplace par le renvoi
- [ ] T053 [P] [US1] Textes d'interface en `fr` et `en` : `F/i18n/locales/{fr,en}/pages/guide-nego.lecteur.json` (repli, page non reçue, ligne de reprise inchangée) et `F/i18n/locales/{fr,en}/components/gn-attente-pages.json` (jauge, « Lire le texte en attendant », « Télécharger pour lire sans réseau », « La page arrive », « Rester sur le texte ») ; retirer les clés du mode « tel quel »
- [ ] T054 [US1] Vérifier au navigateur, **sur la version construite**, le [quickstart § 3](quickstart.md) et le § 4, points 1 à 4, dont le mode avion après fermeture, **et la « 3G lente »** : texte lisible en moins de 5 s, page qui prend sa place, aucune bascule. **Et FR-029** : un réservé lu en ligne sans téléchargement, puis déconnexion, puis mode avion — aucune de ses pages ne s'affiche. L'onglet Réseau montre `no-store`, et le travailleur de pdf.js est détruit au démontage. Commit de la phase

**Point de contrôle** : le critère de sortie, lecture comprise, tient sur poste.

---

## Phase 7 : Récit 2 — Trouver un passage et y aller (P1)

**Objectif** : la recherche mène au passage **marqué sur sa page** ; le sommaire mène à la page ; un document sans texte le dit.

**Test indépendant** : [quickstart § 4](quickstart.md), points 5 et 6, et § 5, point 5.

- [ ] T055 [P] [US2] Écrire `F/app/utils/guide-nego/pdf/normaliser.ts`, **pure** : `normaliserPourReperer(chaines: string[])` rend le texte normalisé et une **table de correspondance** vers l'élément et le caractère d'origine. Cas traités :
  - césure — `U+0002` de PDFium, trait d'union en fin d'élément suivi d'un `hasEOL`, ou rien ;
  - ligatures `ﬁ` `ﬂ` `ﬀ` `ﬃ` `ﬄ` ;
  - insécables `U+00A0` et `U+202F` ;
  - apostrophes `’` et `'`, guillemets `«` `»` `“` `”` ;
  - blancs réduits, fragments recollés ;
  - **un second temps, en option** : les éléments qui ne sont qu'une puce (`▪` `•` `◦` `■`, puces de Wingdings), un numéro de liste (`5.`, `a)`) ou un appel de note seul sur son élément (`15`) sont écartés (ADR-022).

  Prolonge `replier()` de `F/app/utils/guide-nego/repli.ts` : accents et casse
- [ ] T056 [P] [US2] Écrire `F/tests/guide-nego/normaliser.test.ts`, cas par cas, **des deux côtés** : chaque cas de T055 sur une chaîne de PDFium et sur les fragments de pdf.js, qui doivent se rejoindre ; la table ramène au bon élément et au bon caractère ; le second temps écarte puce, numéro et appel, et garde un nombre qui fait partie d'une phrase
- [ ] T057 [US2] Écrire `F/app/utils/guide-nego/pdf/reperer.ts`, **pur** : `repererPassage(pages, { contexte, expression })`. Il cherche d'abord le contexte, puis l'expression **si elle est unique sur la page** ; chaque recherche se fait **en deux temps** (tel quel, puis sans puces, numéros ni appels), **sur la page, puis la suivante, puis la précédente** (ADR-022). Il rend la page et l'intervalle plein, ou `ambigu` (toutes les occurrences, en clair) ou `introuvable`. **Jamais un autre endroit marqué plein** (FR-015)
- [ ] T058 [P] [US2] Écrire `F/tests/guide-nego/reperer.test.ts` : contexte retrouvé ; expression unique ; expression répétée sans contexte (`ambigu`) ; passage à cheval sur deux éléments ; passage introuvable. **Et chaque cas manqué à l'essai, en fragments tels que pdf.js les rend** : la puce `▪` (p. 16, « par paliers). L'élargissement de »), le numéro `5.` (p. 48, « Africa ” “Responding to »), les appels `15` (p. 56, « mécanismes. ITMO signifie ») et `18` (p. 68), l'exposant « Autres³⁷ » que le second temps seul perdrait (p. 79), et les passages de la page voisine (9 → 10, 13 → 12)
- [ ] T059 [US2] Passer l'index de recherche de l'étape 1 (`F/app/utils/guide-nego/lecteur.ts`, `chercherDansLeDocument`) par la même normalisation, pour que ses extraits se retrouvent sur la couche de texte ; adapter `F/tests/guide-nego/recherche.test.ts`
- [ ] T060 [US2] Dans `GnLecteurPages`, à `textlayerrendered` : pour le passage choisi, `repererPassage` sur les éléments de `getTextContent()` (pdf.js 6 ne publie plus `textContentItemsStr`), marqués par les éléments `.textLayer span` sans enfant, dans le même ordre, puis des rectangles par `Range.getClientRects()` **en pourcentage de la page** ; surlignage plein pour la courante, clair pour les autres de la page ; défilement qui amène le passage à l'écran ; « Occurrence n sur N », « précédente », « suivante » (`GnOccurrence`) ; le message de FR-015 si `introuvable` ou `ambigu`. Fermer la recherche efface les surlignages
- [ ] T061 [US2] Le sommaire en mode pages : une entrée mène à sa page (`currentPageNumber`), en haut de la page. « Sommaire » et « Rechercher » n'apparaissent que si `has_text` (FR-018)
- [ ] T062 [US2] La fiche (`F/app/pages/guide-nego/ressources/documents/[id]/index.vue`) : un document sans texte dit qu'on le lit sur ses pages, sans recherche dans le texte ; supprimer la mention « tel quel ». Textes dans `F/i18n/locales/{fr,en}/pages/guide-nego.documents.json`. Vérifier le quickstart § 4 (5, 6). Commit de la phase

---

## Phase 8 : Récit 3 — « Texte agrandi » (P2)

**Objectif** : le second mode se voit dans la barre, se garde sur le téléphone, s'annonce une fois, renvoie aux tableaux.

**Test indépendant** : [quickstart § 5](quickstart.md).

- [ ] T063 [US3] Dans `GnLecteurTexte` (et `GnPageLue`), le bloc `origin` porte un renvoi « Tableau — page {label} » ou « Figure — page {label} ». Il passe en mode pages sur cette page, et repère l'endroit par le texte du bloc s'il en a (`repererPassage`), sinon en haut de page. Textes dans `F/i18n/locales/{fr,en}/components/gn-page-lue.json`
- [ ] T064 [P] [US3] Écrire `F/app/components/guide-nego/GnChoixMode.vue` : groupe de deux boutons « Pages » et « Texte », 48 px, `aria-pressed`, jetons de Guide Négo. Textes dans `F/i18n/locales/{fr,en}/components/gn-choix-mode.json`
- [ ] T065 [P] [US3] Dessiner le pictogramme `gn-sliders` — trois curseurs, 24 px, trait de 2, au trait de la famille — dans `F/app/assets/guide-nego/pictogrammes.svg` et `docs/AppNego/design/passation/pictogrammes.svg`, et l'ajouter à `F/app/utils/guide-nego/pictogrammes.ts`
- [ ] T066 [US3] Dans `F/app/components/guide-nego/GnBarreLecture.vue` :
  - l'action `mode`, rendue par `GnChoixMode`, **entre « Rechercher » et « Réglages »**, seulement si `large_text` ;
  - « Réglages » prend `sliders` au lieu de `text-size` (écart 44) ;
  - le commentaire d'en-tête dit que l'emplacement de « Marquer » (écart 42) porte le choix du mode (écart 43).

  Vérifier que les quatre emplacements tiennent à 320 px. Textes dans `gn-barre-lecture.json`
- [ ] T067 [US3] Dans `F/app/components/guide-nego/GnReglagesLecture.vue`, dans l'ordre : `GnChoixMode` (si `large_text`), le thème, puis la taille **en mode texte seulement**. `text-size` reste pour la taille dans la feuille. Vérifier l'emploi de `text-size` dans `F/app/pages/guide-nego/lexique.vue:19` : il ne s'agit pas du bouton des réglages
- [ ] T068 [US3] Dans `F/app/utils/guide-nego/appareil-lecture.ts` : `gn.lecture-mode` (`pages` | `texte`, défaut `pages`) et `gn.lecture-mode-annonce`, par `lireCle` et `poserCle` ; un composable `F/app/composables/guide-nego/useGnModeDeLecture.ts`. Changer de mode garde la page (`sauter()` ou `currentPageNumber`). Un document sans `large_text` s'ouvre en pages et le dit une fois, **sans changer le mode gardé** (FR-021)
- [ ] T069 [US3] La ligne d'annonce (FR-020 bis) : `GnAnnonce`, une fois par téléphone, au premier document qui offre « Texte agrandi », fenêtre de moins de 600 px en portrait ; fermée au premier geste ou par sa croix. Textes dans `guide-nego.lecteur.json`
- [ ] T070 [P] [US3] Étendre `F/tests/guide-nego/stockage.test.ts` : mode par défaut, mode gardé, annonce vue une fois, document sans « Texte agrandi » qui ne change pas le mode. Vérifier le quickstart § 5. Commit de la phase

---

## Phase 9 : Récit 5 — La note de correction sur la page (P3)

**Objectif** : la note se signale en marge, à la hauteur du passage, et se déplie sans cacher la page.

**Test indépendant** : [quickstart § 6](quickstart.md).

- [ ] T071 [US5] Écrire `F/app/components/guide-nego/GnMargeNote.vue` :
  - filet rouge de 3 px au bord gauche de la page, triangle, cible de 48 px, `aria-expanded` ;
  - positionné **en pourcentage** de la page ;
  - dépliée : un panneau **non modal**, sans voile, en bas de l'écran, 40 % de la hauteur au plus, qui reprend le contenu de `GnNoteCorrection` — texte, signature « Nom, expert IFDD — date », bouclier ;
  - le lecteur fait défiler pour garder le passage dans la moitié haute.

  Textes dans `F/i18n/locales/{fr,en}/components/gn-marge-note.json`
- [ ] T072 [US5] Dans `GnLecteurPages`, à `textlayerrendered` : pour chaque note de la page (`notesDe(id)`), `repererPassage` sur le passage entier puis ses premiers mots ; `GnMargeNote` à la hauteur trouvée, sinon en tête de page. La note introuvable ne perd **ni** son texte **ni** son passage cité (FR-032)
- [ ] T073 [US5] Vérifier que les notes viennent toujours de la liste relue, pas de la copie : posées ou retirées après le téléchargement, elles paraissent ou disparaissent dans les deux modes sans retéléchargement. Vérifier le quickstart § 6. Commit de la phase

---

## Phase 10 : Récit 6 — Le back-office vérifie ce que sert le texte (P3)

**Objectif** : « ouvrir tel quel » disparaît ; « Proposer « Texte agrandi » » suit le verdict et se change sans republier.

**Test indépendant** : [quickstart § 1](quickstart.md).

- [ ] T074 [US6] Dans `F/app/composables/api/admin-negotiation-documents.ts` : `ouvrirTelQuel` devient `choisirLeTexteAgrandi(id, choice: boolean | null)` sur `PUT …/large-text`
- [ ] T075 [US6] Dans `F/app/components/admin/negotiation/PreviewVerdict.vue` : l'interrupteur « Ouvrir tel quel » devient « Proposer « Texte agrandi » » (`UiSwitch`), qui montre le choix effectif, avec « par défaut, selon le verdict : … » et « Revenir au verdict » quand un choix est posé. Il est désactivé et le dit quand `has_text` est faux
- [ ] T076 [US6] Dans `F/app/pages/admin/negociations/documents/[id]/apercu.vue` : un en-tête qui dit que le lecteur montre la page d'origine et que le texte sert à la recherche, au sommaire et à « Texte agrandi » (FR-036) ; le motif d'échec affiché pour un `failed`, dont le PDF protégé. Textes dans `F/i18n/locales/{fr,en}/pages/admin.negociations.documents.preview.json`. Vérifier le quickstart § 1. Commit de la phase

---

## Phase 11 : Recette et finitions

- [ ] T077 [P] Ajouter à la planche `F/app/pages/guide-nego/composants.vue` : `GnChoixMode` (les deux états), `GnMargeNote` (repliée, dépliée, en tête de page), le pictogramme `sliders`, et une vignette de `GnLecteurPages` sur le petit PDF d'exemple
- [ ] T078 [P] Relire les traductions `en` nouvelles ou changées par un sous-agent : concordance clé pour clé, et aucun calque
- [ ] T079 [P] Vérifier SC-011 et le lexique : aucun libellé de vocabulaire dans le code, « Texte agrandi » et « Réglages » partout, aucun reste de « tel quel » (`grep -rn "as_is\|serve_as_is\|tel quel\|tel-quel" F/app backend/crates`)
- [ ] T080 Dérouler [quickstart.md](quickstart.md), § 1 à § 8, **sur la version construite** : lecteur, barre, feuille, annonce et note dépliée à 320, 360 et 390 px, clair et sombre, en « Très grande » ; non-régression du site — accueil, média, édition
- [ ] T081 Dans [DEPLOIEMENT.md](../../docs/DEPLOIEMENT.md) § 15 : la migration de 1b à la suite de celle de l'étape 1 ; la relance d'extraction des documents publiés (`POST …/extraction`), pour recalculer `reading_bytes` ; `pdfjs-dist` dans l'image du site ; la vérification 4 du § 11, **déjà inscrite le 24/09**, à dérouler le jour du branchement
- [ ] T082 `make check-safe`, **API arrêtée**
- [ ] T083 Mettre à jour [docs/AppNego/progress.md](../../docs/AppNego/progress.md) : la ligne 1b et le journal du jour ; ce qui reste sur appareil réel. Commit de la phase
- [ ] T084 **Sur appareil réel, par le commanditaire** : le guide lu en mode avion après une nuit sur un Android de milieu de gamme et sur un iPhone, pages nettes, sans saccade, une recherche qui mène au passage ; si possible un iPhone 8 ou X. Avec T112 (0b) et T116 (étape 1)

---

## Dépendances et ordre

```text
Phase 1  L'essai ──► relu avec le commanditaire ──► rien d'autre ne commence avant
Phase 2  Le modèle
Phase 3  L'API ─────────────┐
Phase 4  pdf.js coquille ───┤  (3 et 4 indépendantes : back et front)
Phase 5  US4 la copie ──────┘  (dépend de 3 et 4)
Phase 6  US1 lire ─────────── MVP (dépend de 5)
Phase 7  US2 trouver           (dépend de 6)
Phase 8  US3 texte agrandi     (dépend de 6 ; indépendant de 7, sauf T063 qui emploie reperer.ts)
Phase 9  US5 notes en marge    (dépend de 6 et de reperer.ts, phase 7)
Phase 10 US6 back-office       (dépend de 3 seulement)
Phase 11 Recette
```

### En parallèle, dans une même phase

- **Phase 3** : T017, T018, T019 ensemble ; puis T028 à T031 ensemble, une fois la route écrite.
- **Phases 3 et 4** : un sous-agent sur le back (phase 3), un autre sur le front (phase 4). Les périmètres sont exclusifs : `backend/` et `docs/database/` d'un côté, `frontend/` de l'autre. Seul T032, qui touche les types, attend la fin de la phase 3.
- **Phase 7** : T055 et T056 avec T057 et T058.
- **Phase 8** : T064 et T065 ensemble.
- **Phase 10** peut se mener pendant les phases 7 à 9 : ses fichiers sont disjoints.

## Stratégie

1. **L'essai décide de tout** : si pdf.js ne tient pas sur le vrai guide, aucune ligne n'est perdue.
2. **MVP = phases 1 à 6** : le guide se lit sur ses pages, en ligne et en mode avion, avec reprise — le premier volet du critère de sortie.
3. **Puis la phase 7**, le second volet du critère : une recherche qui mène au passage sur sa page.
4. **Puis les phases 8 à 10**, qui rendent ce que l'étape 1 donnait déjà : « Texte agrandi », les notes, le back-office.
5. **La recette** finit sur poste ; le critère se prouve sur appareil réel (T084).

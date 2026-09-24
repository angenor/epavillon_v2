# Implementation Plan: Guide Négo — le lecteur montre le PDF d'origine (étape 1b)

**Branch**: `012-guide-nego-lecteur-pdf`, partie de `011-guide-nego-documents`, `main` fusionnée | **Date**: 2026-09-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/012-guide-nego-lecteur-pdf/spec.md`

**Artefacts** : [research.md](research.md) · [essai-lecteur.md](essai-lecteur.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

---

## Summary

Le lecteur montre les pages du guide telles que l'IFDD les a mises en page : il les fait défiler, les agrandit et les garde nettes, et le texte s'y sélectionne. Il fonctionne en mode avion. Le texte recomposé de l'étape 1 devient un second mode, « Texte agrandi ». Le texte extrait sert à la recherche, au sommaire et au repérage d'un passage sur la page.

**L'approche, en une phrase par pièce.**

- **Le risque d'abord.** Un prototype jetable a lu le vrai guide avant tout code : issue A, le 24/09. [ADR-022](../../docs/AppNego/adr/022-pdfjs-dans-le-client.md) consigne le résultat et quatre décisions du commanditaire ([R2](research.md), [R15](research.md), [essai-lecteur.md](essai-lecteur.md)).
- **En ligne, un transport de plages écrit ici** nourrit pdf.js et sait ce qui est demandé et reçu : l'attente montre sa progression, offre le texte au bout de 3 s, et la bascule ne compte jamais le réseau ([R15](research.md)).
- **pdf.js, build legacy, et son visionneur de pages** (`PDFViewer`). Le visionneur gère déjà la virtualisation, le plafond des canevas d'iOS et le redessin net de la zone visible. On n'en réécrit rien ([R1](research.md)).
- **Une route de l'API sert le PDF, par plages, à chaque morceau après vérification de l'accès.** Tous les PDF de documents sont dans le bucket privé, publics compris. `kernel::storage` apprend à lire une plage ([R3](research.md)).
- **La coquille garde pdf.js.** Le travailleur y entre par le manifeste de Vite, les `wasm` et les polices par un module qui les copie. La feuille du visionneur y entre en copie bornée à `[data-app="guide-nego"]` ([R4](research.md)).
- **La copie gardée passe au format 2 : le PDF et la lecture, sans images.** Une copie d'un format ancien s'efface ([R5](research.md)).
- **Le passage se repère sur la page par la couche de texte de pdf.js**, après une normalisation des deux textes par une fonction pure testée : césures, ligatures, insécables, apostrophes, fragments. L'essai mesure le taux de passages retrouvés. Rien ne s'ajoute au modèle ([R7](research.md)).
- **Le relais compresse** : la route du PDF pose `Content-Encoding: identity` et `no-transform`, et le jour du branchement vérifie un `206` sans compression à travers le relais (§ 11 de DEPLOIEMENT.md, [R3](research.md)).
- **« Texte agrandi » est le lecteur de l'étape 1**, sorti de la page dans son composant. Son choix prend la place de « Marquer » dans la barre ([R8](research.md), [R9](research.md)).
- **Dans le modèle, « ouvrir tel quel » devient un choix qui suit par défaut le verdict de l'extraction.** Une fonction SQL porte la règle ([R11](research.md)).

---

## Technical Context

**Langage / version** : Rust stable (API, worker) · TypeScript strict (Nuxt 4.5, aucun `any`) · Node 24 pour les tests du client

**Dépendances principales** : Actix Web, SQLx vérifié à la compilation, PostgreSQL 17, Nuxt 4, TailwindCSS v4.
- **Une dépendance nouvelle, côté client seulement** : `pdfjs-dist` 6.3.x (Apache-2.0), version exacte, build `legacy`, avec ses composants de visionneur. Elle se consigne dans l'**ADR-022**, qui remplace une règle d'ADR-021.
- Aucune dépendance Rust nouvelle : le `GetObject` avec `Range` passe par le client S3 existant.

**Stockage** :
- **PostgreSQL** : `negotiation.document_renditions.serve_as_is` devient `large_text_choice`, `reading_bytes` se recalcule, et la fonction `negotiation.document_reading_modes()` apparaît ([data-model.md](data-model.md)).
- **Garage** : inchangé. Les images de page restent, pour l'aperçu seul.
- **Sur l'appareil** : copie de format 2 dans les deux caches de l'étape 1 ; `localStorage` gagne `gn.lecture-mode` et `gn.lecture-mode-annonce`.

**Tests** :
- `cargo test` sur base réelle et jetable : la route du fichier et ses plages, le réservé à chaque morceau, le choix « Texte agrandi », `get_range` ;
- `node --test` côté client : repérage, copie de format 2, mode et annonce, garde ;
- l'essai sur le vrai guide, puis la recette sur la version construite, puis les appareils réels.

**Plateforme cible** : PWA installée, sur Android et iPhone. **Plancher déduit à l'essai : iOS 16.4**, `Promise.withResolvers` étant remplacé dans la page et dans le travailleur ; la mesure sur téléphone reste à T084. Un téléphone en dessous lit « Texte agrandi », par détection des fonctions manquantes ([R1](research.md), [R15](research.md)). Référence de 360 px, sans réseau.

**Type de projet** : application web installable, adossée à l'API du monolithe modulaire.

**Objectifs de performance** :
- défilement des 90 pages sans tâche longue de plus de 100 ms ;
- page nette moins d'une seconde après le geste ;
- première page en ligne en moins de 3 s sur un réseau ordinaire, et en moins de 8 s depuis une copie gardée ; en « 3G lente », **le texte se lit en moins de 5 s** et la page du PDF suit (23 à 28 s pour le guide, accepté — SC-003 revu) ;
- passages rendus en moins d'une seconde hors connexion ;
- mémoire stable sur trois allers-retours (SC-001 à SC-004).

**Contraintes** :
- préfixe `/v2`, portée du service worker bornée à `guide-nego/` ;
- coquille alourdie d'environ 1,3 Mo compressé (polices standard toutes gardées), installée en tout ou rien ;
- aucun sélecteur de feuille de style hors de `[data-app="guide-nego"]` ;
- aucun fichier de plus de mille lignes ;
- **jamais `make check`**.

**Échelle et périmètre** :
- **Routes** : 1 nouvelle (`GET` et `HEAD` du fichier), 3 changées, 1 supprimée, 1 renommée au back-office.
- **Base** : 1 colonne changée, 1 fonction.
- **Composants** : 5 nouveaux dans Guide Négo, et 1 pictogramme.
- **Client** : un module Nuxt nouveau ; la page du lecteur réorganisée ; le téléchargement et les copies changés.

---

## Constitution Check

*Porte à franchir avant la recherche, et à repasser après la conception.*

| Principe | Verdict | Comment il est tenu |
|---|---|---|
| **I — Le modèle fait autorité** | ✅ | `100_negotiations.sql` change **avant** le code : la colonne renommée et redéfinie, la fonction, les commentaires. `migration.sql` rejouable, jouée deux fois et comparée, sans détruire la base. `docs/progression/modele.md` le consigne |
| **II — Frontières de modules** | ✅ | `negotiation` lit une plage par `kernel::storage`, où le contrat de stockage vit depuis l'étape 1. Aucune arête vers `media` |
| **III — `xmod_fk_*`** | ✅ *(sans objet)* | Aucune clé nouvelle |
| **IV — Effets de bord par l'outbox** | ✅ *(sans objet)* | Aucun effet inter-modules. Le recalcul de `reading_bytes` passe par la relance d'extraction existante |
| **V — Permission et portée** | ✅ | La route du fichier : `negotiation.space.access` en portée globale pour un réservé, **vérifiée à chaque morceau**. `…/large-text` : `negotiation.document.publish`, avec le filtrage par périmètre. Un test d'URL forgée par route |
| **VI — SQLx vérifié, pas d'ORM** | ✅ | La fonction se lit par une requête statique |
| **VII — Contexte d'écriture** | ✅ | `…/large-text` passe par `Db::write(&ctx)`, et l'audit l'enregistre |
| **VIII — Invariants non réimplémentés** | ✅ | La règle « Texte agrandi » tient dans une seule fonction SQL, que la liste, la lecture et l'aperçu lisent. La publication garde sa règle `ready` |
| **IX — Erreurs à code stable** | ✅ | Un code nouveau, `NEGOTIATION_DOCUMENT_RANGE_INVALID` (416), avec son message en français |
| **X — Tests sur base réelle** | ✅ | Plages, `416`, `304`, réservé à chaque morceau, choix « Texte agrandi » : sur base réelle ([contracts/api-lecture.md](contracts/api-lecture.md)) |
| **XI — Hors connexion d'abord** | ✅ | La copie contient tout ce qu'il faut pour lire. pdf.js, son travailleur, ses `wasm` et ses polices sont dans la coquille, et la garde le prouve. Le mode avion se vérifie dès l'essai. Un document lu en ligne ne se fait pas passer pour gardé |
| **XII — Confiance** | ✅ | La page est celle de l'IFDD, sans recoloration. Le lecteur ne marque jamais un passage ailleurs qu'à sa place. Un texte mal recomposé peut être retiré, et la note se pose à côté de la page, sans la modifier. « Marquer » reste absent |
| **XIII — Un design propre et borné** | ⚠️ → ✅ | `pdf_viewer.css` déclare `:root` et des sélecteurs globaux. Le module en écrit une **copie bornée**, que `check:guide-nego` vérifie comme toute feuille ([R4](research.md)). Quatre composants nouveaux et un pictogramme vont sur la planche. Le back-office garde `UiSwitch` et ses composants |
| **XIV — Une seule porte** | ✅ | Aucun service annexe. Le PDF passe par l'API, jamais par une adresse de stockage |
| **Trois agendas** | ✅ *(sans objet)* | — |
| **Dépendance d'ampleur** | ✅ | `pdfjs-dist` : ADR-022, après l'essai, qui remplace la règle « le PDF ne va jamais sur le téléphone » d'ADR-021 |
| **Garde-fou des 1000 lignes** | ✅ | `lire.vue` (603 lignes) perd son rendu recomposé au profit de `GnLecteurTexte`. `routes/documents.rs` (303) reçoit la route du fichier. `useGnCopies.ts` (444) change sans grossir : les images partent, le PDF arrive. Les tests proches de la limite (`documents_admin.rs`, 865 lignes) ne reçoivent rien : les tests du fichier ont leur propre fichier |
| **Le suivi vit dans `progress.md`** | ✅ | Sauf `docs/progression/modele.md`, puisque `docs/database/` bouge (ADR-017) |

**Aucune violation à justifier.** Le ⚠️ du principe XIII se lève par la conception.

*Repassée après la conception* : la route du fichier a sa garde et ses tests de morceau. La règle « Texte agrandi » vit à un seul endroit. La feuille du visionneur est bornée, et la coquille garde tout ce que pdf.js charge. La porte tient.

---

## Project Structure

### Documentation (this feature)

```text
specs/012-guide-nego-lecteur-pdf/
├── spec.md
├── plan.md                  # ce fichier
├── research.md              # R1 à R14
├── essai-lecteur.md         # la grille de la phase 1, à remplir
├── data-model.md
├── contracts/
│   ├── api-lecture.md       # route du fichier, lecture, « large-text »
│   ├── copie-gardee.md      # format 2
│   └── lecteur.md           # l'écran, ses modes, sa barre, ses gestes
├── quickstart.md
├── migration.sql            # phase 2
├── checklists/requirements.md
└── tasks.md                 # /speckit-tasks
```

### Source Code (repository root)

```text
docs/database/100_negotiations.sql   ~ large_text_choice · + document_reading_modes()
docs/AppNego/adr/022-…md             + pdf.js dans le client (phase 1)
docs/AppNego/adr/021-…md             ~ statut : sa dernière règle remplacée par 022
docs/DEPLOIEMENT.md § 11             ~ vérification 4 : un 206 sans compression à travers le relais (fait le 24/09)
docs/DEPLOIEMENT.md § 15             ~ migration 1b, relance d'extraction

backend/crates/
├── kernel/src/storage/              ~ mod.rs, s3.rs, filesystem : + get_range
├── kernel/src/error.rs              + NEGOTIATION_DOCUMENT_RANGE_INVALID
├── api/src/lib.rs (contrôle d'origine)  ~ Range accepté, Content-Range exposé
└── modules/negotiation/
    ├── src/domain/plage.rs          + analyse de l'en-tête Range (pure, testée)
    ├── src/routes/documents.rs      + GET/HEAD …/file · − …/pages/{i}/image
    ├── src/service/documents.rs     + lire_le_fichier · ~ lecture sans image, has_text/large_text
    ├── src/routes/admin_documents.rs  ~ …/as-is → …/large-text
    ├── src/service/admin_documents.rs ~ choisir_le_texte_agrandi · aperçu
    ├── src/repo/renditions.rs       ~ large_text_choice · document_reading_modes
    ├── src/jobs/extract.rs          ~ reading_bytes = PDF + JSON · motif du mot de passe
    └── tests/                       + documents_fichier.rs · ~ documents_public, documents_admin

frontend/
├── package.json                     + pdfjs-dist (exacte)
├── modules/guide-nego-pdfjs.ts      + copie wasm/iccs/standard_fonts → public/guide-nego/pdfjs/,
│                                      feuille du visionneur bornée
├── guide-nego/feuille-pdfjs.ts     + bornerLaFeuille(), appelée par la construction ET par check:guide-nego ; + la règle content-box sous .pdfViewer
├── guide-nego/liste-de-garde.ts     ~ FICHIERS_PUBLICS + pdfjs/
├── app/utils/guide-nego/
│   ├── pdf/charger.ts               + pdf.js legacy, détection, options (disableAutoFetch)
│   ├── pdf/remplacements.ts         + Promise.withResolvers, itérateur de flux — page et travailleur (pur)
│   ├── pdf/travailleur.ts           + amorce du travailleur : remplacements, puis pdf.worker
│   ├── pdf/transport.ts             + transport de plages : demandé, reçu, en route (PDFDataRangeTransport)
│   ├── pdf/bascule.ts               + seconde sécurité : erreur, ou 8 s de travail sans plage en route (pur)
│   ├── pdf/normaliser.ts            + césures, ligatures, insécables, apostrophes, table de correspondance (pur)
│   ├── pdf/reperer.ts               + repérage en deux temps, page puis ±1 (pur)
│   ├── copies.ts                    ~ FORMAT_DE_COPIE = 2, sans images
│   └── appareil-lecture.ts          ~ mode, annonce
├── app/composables/guide-nego/      ~ useGnLecteur (source PDF, mode) · useGnCopies (PDF en flux)
├── app/composables/api/             ~ guide-nego-documents (fichier, sans image) ·
│                                      admin-negotiation-documents (large-text)
├── app/components/guide-nego/       + GnLecteurPages.client · GnLecteurTexte · GnChoixMode · GnMargeNote · GnAttentePages
│                                    ~ GnBarreLecture (4 emplacements, sliders) · GnReglagesLecture
│                                    ~ GnPageLue (renvoi « Tableau », sans image) − GnImageDePage
├── app/assets/guide-nego/pictogrammes.svg  + gn-sliders
├── app/pages/guide-nego/ressources/documents/[id]/lire.vue   ~ état, modes, feuilles
├── app/pages/guide-nego/ressources/documents/[id]/index.vue  ~ fiche : sans texte, le dire
├── app/components/admin/negotiation/PreviewVerdict.vue       ~ « Proposer « Texte agrandi » »
├── app/types/                       ~ negotiation-documents · admin-negotiation-documents
├── i18n/locales/{fr,en}/            ~ guide-nego.lecteur, gn-barre-lecture, gn-reglages-lecture,
│                                      admin.negotiation-documents ; + gn-choix-mode, gn-marge-note
└── tests/guide-nego/                + normaliser · reperer · charger · remplacements · transport · bascule · feuille-pdfjs · gestes · ~ copies, liste-de-garde, sw-garde, stockage
```

**Structure Decision** : le serveur ne change que dans `negotiation` et `kernel`. Côté client, pdf.js n'entre que par `utils/guide-nego/pdf/charger.ts` : un seul point d'entrée, que la garde suit par le manifeste. Le back-office ne charge pas pdf.js ; son aperçu garde les images de PDFium ([R11](research.md)).

---

## Séquencement

**Sur la branche `012-guide-nego-lecteur-pdf`.** **Un commit par phase**, avec ses contrôles ciblés. `make check-safe` **en fin de cycle seulement**, API arrêtée.

| # | Phase | Ce qu'elle livre | Pourquoi là |
|---|---|---|---|
| 1 | **L'essai** ✅ 24/09, issue A | Le prototype jetable sur le vrai guide, [essai-lecteur.md](essai-lecteur.md) rempli : fluidité, mémoire, netteté, taux de passages retrouvés (recherches et notes), **version d'iOS réellement exigée** — simulateurs iOS 16, 17, 18 — et liste des fonctions à détecter. L'issue A, B ou C, l'**ADR-022**. **Relu avec le commanditaire** | Tout le cycle repose sur la tenue de pdf.js sur un téléphone. Rien ne s'écrit avant |
| 2 | **Le modèle** | `100_negotiations.sql`, `migration.sql` rejouable, `docs/progression/modele.md` | Rien du métier avant le SQL |
| 3 | **Les plages et l'API** | `get_range`, la route du fichier — `Content-Encoding: identity`, `no-transform`, empreinte comparée sans suffixe de relais — et ses tests, la lecture sans image, `has_text`/`large_text`, `…/large-text`, `reading_bytes`, le motif du mot de passe, `make openapi`, `check-api-contract` | Le client a besoin d'un contrat servi |
| 4 | **pdf.js dans la coquille** | `pdfjs-dist`, `modules/guide-nego-pdfjs.ts`, `charger.ts` — polyfill, **détection des fonctions manquantes** et son test qui simule leur absence —, la feuille bornée, la garde et ses tests, `verifier-garde` | La preuve hors connexion précède tout écran |
| 5 | **La copie, format 2** | `copies.ts`, `useGnCopies`, l'effacement des formats anciens, le remplacement de la seule lecture, les tests | US1 et US2 lisent la copie |
| 6 | **US1 — Lire les pages** | `GnLecteurPages`, les gestes, le pied, la reprise, le thème, **l'attente en ligne et ses deux sorties** (FR-009 bis), le repli d'un téléphone qui n'affiche pas les pages (FR-012 bis), `lire.vue` réorganisée, `GnLecteurTexte` extrait **sans changement de comportement** | Le critère de sortie |
| 7 | **US2 — Trouver** | `normaliser.ts` et `reperer.ts`, fonctions pures, et leurs tests cas par cas ; l'index de l'étape 1 passé par la même normalisation ; le passage marqué sur la page, le sommaire vers la page, la fiche d'un document sans texte | L'autre moitié du critère |
| 8 | **US3 — « Texte agrandi »** | `GnChoixMode` dans la barre et la feuille, `sliders`, l'annonce unique, le renvoi « Tableau », le mode gardé | S'appuie sur 6 |
| 9 | **US5 — Les notes en marge** | `GnMargeNote`, le panneau non modal | S'appuie sur 6 et 7 |
| 10 | **US6 — Le back-office** | `PreviewVerdict`, l'en-tête de l'aperçu, le motif d'échec | Indépendant des phases 6 à 9, placé après pour la continuité |
| 11 | **Recette** | Le [quickstart](quickstart.md) déroulé sur la version construite, la planche complétée, `en` relu, les écrans à 320, 360 et 390 px, la non-régression du site, `make check-safe`. DEPLOIEMENT.md § 15 et `progress.md` | — |

US4 (la copie) se livre en phase 5 : c'est de la plomberie, et elle conditionne la lecture hors connexion.

### Si l'essai rend l'issue B ou C

- **B** : la phase 4 lit les `wasm` en ligne seulement. L'essai dit ce que le guide y perd, et le commanditaire le voit avant la phase 2.
- **C** : les réglages de mémoire se baissent dans `charger.ts`. Si rien ne tient sur Android milieu de gamme, le cycle s'arrête et revient au commanditaire, avant tout code.

### Ce qui ne se fait pas depuis un poste

- Le guide lu en mode avion, après une nuit, sur un **Android de milieu de gamme** et un **iPhone réels** : c'est la preuve du critère.
- Le téléphone bridé en 3G réelle.
- Le plancher iOS 16.4 sur un iPhone 8 ou X, et un iPhone sous 16.4 qui bascule.

Les deux se font par le commanditaire, au § 15 de DEPLOIEMENT.md, avec T112 (0b) et T116 (étape 1).

---

## Complexity Tracking

*Aucune violation de la constitution à justifier.* La copie bornée de la feuille du visionneur n'est pas une exception : c'est la forme que le principe XIII impose à une feuille venue d'ailleurs.

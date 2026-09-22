# Implementation Plan: Guide Négo — la bibliothèque de documents et le lecteur (étape 1)

**Branch**: `011-guide-nego-documents` — à créer avant `/speckit-implement` | **Date**: 2026-09-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/011-guide-nego-documents/spec.md`

**Artefacts** : [research.md](research.md) · [essai-extraction.md](essai-extraction.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md)

---

## Summary

L'IFDD publie le *Guide des négociations* le jour où elle le reçoit. Toute personne, avec ou sans compte, le trouve, le télécharge et le lit en salle sans réseau :
- le texte se recompose à la taille voulue ;
- la page reprend où elle s'était arrêtée ;
- les termes anglais se touchent ;
- les passages dépassés portent la note d'un expert.

Les documents réservés ne quittent jamais l'API pour qui n'a pas l'accès, et s'effacent du téléphone à la déconnexion.

**L'approche, en une phrase par pièce.**

- **Le risque d'abord.** L'étape commence par un **essai d'extraction sur le vrai guide** : ordre de lecture, en-têtes et pieds, notes et tableaux, césures, italiques. L'outil se choisit sur ce résultat — PDFium dans le worker Rust est pressenti —, et l'essai dit aussi ce que pèse la copie gardée ([R1](research.md), [R2](research.md), [essai-extraction.md](essai-extraction.md)).
- **La forme lisible vit en base, page par page.** Elle sert la recherche dans le texte, l'ancre des notes, et un seul corps servi sous empreinte ([R3](research.md), [R7](research.md)).
- **Les fichiers privés deviennent privés.** Un bucket non ouvert au web, un contrat de stockage passé dans `kernel`, et l'API seule sert un PDF ou une image de page, après avoir vérifié l'accès. Aujourd'hui, `private` dépose dans le bucket public ([R4](research.md)).
- **Le dépôt réutilise ce qui existe.** La garde média s'ouvre au couple des documents, et le champ de fichier naît du dépôt sorti de `ImageField` ([R5](research.md)).
- **Le téléphone filtre seul.** La liste entière arrive sous empreinte, et les filtres tournent sans réseau ; seule la recherche dans le texte des documents demande le réseau ([R8](research.md), [R9](research.md)).
- **Deux caches, un par accès.** Effacer les réservés à la déconnexion est une suppression, pas un tri. « Tout retirer » ne touche jamais aux lectures ([R11](research.md)).
- **L'expert est un rôle**, de portée globale, avec une permission par geste. Les étapes 2, 7 et 8 y ajouteront les leurs ([R15](research.md)).
- **Les thématiques d'un document passent par `reference.entity_terms`**, sur le patron de `programme` ([R16](research.md)).

---

## Technical Context

**Langage / version** : Rust stable (API, worker) · TypeScript strict (Nuxt 4, aucun `any`)

**Dépendances principales** : Actix Web, SQLx vérifié à la compilation, PostgreSQL 17, Nuxt 4, TailwindCSS v4.
- **Une dépendance nouvelle**, soumise à l'essai : `pdfium-render` (MIT ou Apache-2.0), avec la bibliothèque native PDFium (BSD-3 ou Apache-2.0), **dans le worker seulement**. La décision s'inscrit dans `docs/progression/decisions/` (constitution, contraintes).
- `image` (déjà présent, JPEG) encode les pages.
- Aucune dépendance côté client.

**Stockage** :
- **PostgreSQL** :
  - `negotiation` : documents modifiés, `document_renditions`, `document_pages`, `correction_notes`, rôle `expert` ;
  - `reference` : deux types de document, et les thématiques par `entity_terms` ;
  - `media` : réglage du bucket privé, `object_location()`.
- **Garage** : un second bucket, `epavillon-prive`.
- **Sur l'appareil** :
  - deux caches (`gn-documents-publics`, `gn-documents-reserves`) ;
  - IndexedDB `guide-nego` v3, magasins `copies` et `a-telecharger` ;
  - `localStorage` pour la taille du texte, la progression, les documents ouverts et les derniers ouverts.

**Tests** :
- `cargo test` sur base réelle et jetable (`kernel::testing::TestDb`) ; l'extraction sur un petit PDF de test commité ;
- `node --test` côté client, sur de faux `caches` et IndexedDB ;
- écrans à 320, 360 et 390 px dans les deux thèmes et les trois tailles de texte.

**Plateforme cible** : PWA installable, rendu côté navigateur, 360 px de référence, utilisable sans réseau. Le back-office est sur écran large et tablette.

**Type de projet** : application web installable, adossée à l'API du monolithe modulaire.

**Objectifs de performance** :
- la bibliothèque s'affiche depuis la garde sans attendre le réseau ;
- le `304` coûte zéro octet ;
- la recherche dans un document de cent pages rend ses passages en moins d'une seconde, sans réseau (SC-009) ;
- l'extraction de cent pages, images comprises, dure moins d'une minute ;
- l'aperçu se feuillette en moins de dix minutes (SC-001 bis).

**Contraintes** :
- préfixe `/v2`, portée du service worker bornée à `guide-nego/` ;
- aucun composant ni jeton du site dans l'application ;
- aucun fichier de plus de mille lignes ;
- **jamais `make check`**.

**Échelle et périmètre** :
- **Côté application** : cinq écrans ou vues — bibliothèque, fiche, lecteur (avec sommaire, recherche, réglages, feuille de terme), « Mes documents », l'état « pas sur le téléphone ».
- **Côté back-office** : quatre écrans.
- **Routes** : 9 publiques et 17 d'administration.
- **Base** : 3 tables et 1 ENUM, 7 colonnes ou contraintes changées, 1 rôle et 2 permissions.
- **Composants** : 8 nouveaux côté application, 1 champ de fichier au back-office.
- **Volume** : quelques dizaines de documents par COP.

---

## Constitution Check

*Porte à franchir avant la recherche, et à repasser après la conception.*

| Principe | Verdict | Comment il est tenu |
|---|---|---|
| **I — Le modèle fait autorité** | ✅ | Tout ce que liste [data-model.md](data-model.md) s'écrit dans `docs/database/` **avant** le code. La base locale se migre par un script rejouable, joué deux fois et comparé. `docs/progression/modele.md` le consigne |
| **II — Frontières de modules** | ⚠️ → ✅ | `negotiation` doit lire un objet stocké, que seul `media` sait lire. **Le contrat de stockage passe dans `kernel`**, comme `kernel::mail`, au lieu d'une arête `negotiation → media` ([R4](research.md)). La lecture de l'emplacement passe par une fonction SQL du schéma propriétaire, comme `media.object_url()`. `tests/frontieres.rs` le prouve |
| **III — `xmod_fk_*`** | ✅ | `xmod_fk_documents_event`, `xmod_fk_document_renditions_asset`, `xmod_fk_correction_notes_author`, `xmod_fk_correction_notes_withdrawer`. `platform.cross_module_fk_report` reste vide |
| **IV — Effets de bord par l'outbox** | ✅ *(sans objet)* | Aucun effet inter-modules. L'extraction est un **travail** (`platform.jobs`) mis en file dans la transaction qui attache le fichier. Aucun événement n'est émis sans consommateur ([api-admin-documents.md](contracts/api-admin-documents.md)) |
| **V — Permission et portée** | ✅ | `negotiation.document.publish` (existe), `negotiation.correction.post` et `.withdraw` (nouvelles), toutes en **portée globale**. Les lectures ouvertes à l'une ou l'autre se testent dans le gestionnaire. L'accès réservé est `negotiation.space.access`. Chaque route a son test d'URL forgée et d'administrateur d'événement refusé |
| **VI — SQLx vérifié, pas d'ORM** | ✅ | Requêtes statiques. La chaîne des remplacements se lit par une requête récursive écrite, non composée |
| **VII — Contexte d'écriture** | ✅ | `Db::write(&ctx)` sur chaque écriture du back-office, des favoris et du travail d'extraction : l'acteur du travail est celui qui a attaché le fichier. Un test vérifie l'auteur dans l'audit |
| **VIII — Invariants non réimplémentés** | ✅ | Tenus par la base, et traduits en français : fichier ou lien, source d'un publié, un seul successeur, pas de boucle, taxonomie du type, page d'une note. Le service ne vérifie que ce que la base ne voit pas : l'extraction prête, le fichier figé |
| **IX — Erreurs à code stable** | ✅ | Treize codes nouveaux au catalogue de `kernel`, messages en français. Un réservé refusé dit ce qu'il faut pour l'ouvrir, sans rien divulguer |
| **X — Tests sur base réelle** | ✅ | Pour chaque route : chemin nominal, URL forgée, invariant traduit. L'extraction se teste sur un petit PDF commité. L'accès réservé est essayé de trois façons (SC-007) |
| **XI — Hors connexion d'abord** | ✅ | Bibliothèque, fiche, « Mes documents » et lecteur se lisent sans réseau, avec leur heure. Les écritures sans réseau (favori) passent par la file de 0c, une seule fois. Un téléchargement demandé sans réseau part au retour du réseau |
| **XII — Confiance** | ✅ | La note se pose **par-dessus** le texte, sans jamais le modifier. La feuille du terme n'invente aucune traduction. « Marquer » n'est pas livré, parce qu'il serait sans effet (écart 42). Le marqueur de l'assistant est décoché par défaut, et rien n'est indexé |
| **XIII — Un design propre et borné** | ✅ | Huit composants nouveaux dans le dossier de Guide Négo, sur la planche, bâtis sur `design/passation/`. Le back-office emploie les composants `ui/` du site ; son champ de fichier naît du dépôt **sorti** de `ImageField`, pas d'une copie ([R5](research.md)) |
| **XIV — Une seule porte** | ✅ | Pas de service annexe si l'essai donne l'issue A. **Issue B** : le service Python monte selon ADR-004 — interne, sans route publique, écrivant dans `tool` seulement — et son coût est écrit en [R1](research.md) |
| **Trois agendas** | ✅ *(sans objet)* | Aucun agenda à cette étape. « Programme » seul n'apparaît nulle part |
| **Garde-fou des 1000 lignes** | ✅ | `useApi.ts` (896 lignes) ne gagne que son branchement ; les méthodes vont dans deux fichiers nouveaux ([R20](research.md)). L'extraction se découpe par règle |
| **Le suivi vit dans `progress.md`** | ✅ | Sauf `docs/progression/modele.md`, puisque `docs/database/` bouge (ADR-017) |

**Aucune violation à justifier.** Le ⚠️ du principe II se lève par la conception, pas par une exception.

*Repassée après la conception* : les contrats ne créent aucune arête entre modules, chaque route a sa garde écrite, et chaque invariant a son code. La porte tient.

---

## Project Structure

### Documentation (this feature)

```text
specs/011-guide-nego-documents/
├── spec.md
├── plan.md                   # ce fichier
├── research.md               # R1 à R20
├── essai-extraction.md       # la grille de la phase 1, à remplir
├── data-model.md
├── contracts/
│   ├── api-documents.md
│   ├── api-admin-documents.md
│   ├── forme-lisible.md
│   └── hors-connexion.md
├── quickstart.md
├── migration.sql             # phase 3
├── checklists/requirements.md
└── tasks.md                  # /speckit-tasks
```

### Source Code (repository root)

```text
docs/database/
├── 020_reference.sql         + deux types de document
├── 050_media.sql             + réglage media.private_bucket, media.object_location()
└── 100_negotiations.sql      ~ documents · + rendition_status, document_renditions,
                                document_pages, correction_notes, rôle expert

ops/ · Makefile               + bucket epavillon-prive (sans website), cible `make pdfium`
.env.example                  + PDFIUM_LIB_PATH

backend/crates/
├── kernel/src/
│   ├── storage/              # DÉPLACÉ de media : trait, s3, sigv4, filesystem
│   └── error.rs              + treize codes
├── modules/media/src/
│   ├── domain/guards.rs      + ("negotiation","documents") → document.publish, PDF, private
│   └── repo/assets.rs        ~ bucket choisi par la visibilité
├── modules/negotiation/
│   ├── examples/essai_extraction.rs     # phase 1, jetable
│   ├── src/domain/
│   │   ├── documents.rs · corrections.rs · permissions.rs (+ trois)
│   │   └── extraction/       # colonnes, entetes, notes, cesures, termes, sommaire, pages
│   ├── src/repo/             + documents.rs · document_pages.rs · document_themes.rs · corrections.rs · bookmarks.rs
│   ├── src/service/          + documents.rs · admin_documents.rs · corrections.rs
│   ├── src/routes/           + documents.rs · admin_documents.rs
│   ├── src/jobs/extract.rs   # negotiation.document.extract
│   └── tests/                + documents_*.rs · corrections_*.rs · fixtures/petit.pdf
├── api/tests/                + routes_negotiation_documents.rs
└── worker/src/main.rs        ~ enregistre le travail d'extraction

frontend/app/
├── composables/
│   ├── useDepotMedia.ts                 # SORTI de ImageField
│   ├── api/guide-nego-documents.ts      # NOUVEAU
│   ├── api/admin-negotiation-documents.ts
│   ├── api/http.ts                      ~ lireEtiquete envoie If-None-Match
│   └── guide-nego/                      + useGnDocuments · useGnCopies · useGnLecteur · useGnFavoris
├── utils/guide-nego/                    + documents (filtres, Nouveau, chaîne) · copies · lecteur
│                                          (recherche sans accents, repérage) · forme-lisible
├── components/
│   ├── media/FileField.vue              # NOUVEAU, sur useDepotMedia
│   └── guide-nego/                      + GnLigneDocument · GnBandeauRemplace · GnBarreLecture ·
│                                          GnLigneSommaire · GnNoteCorrection · GnOccurrence ·
│                                          GnProgression · GnFeuilleFiltre (+ planche)
├── pages/guide-nego/ressources/
│   ├── documents/index.vue · [id].vue · [id]/lire.vue
│   └── telechargements.vue              ~ devient « Mes documents »
├── pages/admin/negociations/documents/  index · nouveau · [id] · [id]/apercu
├── types/                               + negotiation-documents.ts · admin-negotiation-documents.ts
├── mocks/negotiation-documents.ts       # les cinq documents de la maquette
└── i18n/locales/{fr,en}/pages/          + guide-nego.documents.*, guide-nego.lecteur.*,
                                           admin.negotiation-documents.* ; ~ ouverture, a-propos, reglages
frontend/tests/guide-nego/               + copies, deconnexion-reserves, nouveau, recherche, forme-lisible, sw-garde
```

**Structure Decision** : tout le métier va dans le crate `negotiation`, qui possède les documents. `media` ne gagne qu'une garde et le choix du bucket, `kernel` le contrat de stockage qu'il abrite désormais, comme la messagerie. Côté client, les documents vivent sous `ressources/`, l'onglet où la maquette les range. Le lecteur est une page à lui, sans barre d'onglets (`composants.md:16`).

---

## Séquencement

**Sur la branche `011-guide-nego-documents`**, créée avant `/speckit-implement`. **Un commit par phase**, avec ses contrôles ciblés. `make check-safe` **en fin de cycle seulement**, API arrêtée.

| # | Phase | Ce qu'elle livre | Pourquoi là |
|---|---|---|---|
| 1 | **L'essai** | `make pdfium`, `examples/essai_extraction.rs`, la grille remplie, l'issue A, B ou C, les règles de R7 ajustées. **Relue avec le commanditaire** | Le risque de l'étape ; découvert le 2 novembre, il ne laisserait aucun temps. Rien ne s'écrit avant |
| 2 | **Le stockage privé** | `kernel::storage` déplacé, **sans changement de comportement** ; le bucket privé et son réglage ; `media.object_location()` ; la garde des documents. Côté client, `useDepotMedia` sorti de `ImageField`, puis `FileField` | Deux déplacements purs d'abord, comme la phase 1 de 0c. Le diff se lit comme un déplacement, et le site ne doit rien voir changer |
| 3 | **Le modèle** | `docs/database/`, `migration.sql` rejouable, `docs/progression/modele.md`, la décision de dépendance dans `docs/progression/decisions/` | Rien du métier avant le SQL |
| 4 | **L'extraction** | `domain/extraction/` (R7), le travail `negotiation.document.extract`, les images de page en bucket privé, les tests sur `fixtures/petit.pdf` | Le back-office en a besoin pour l'aperçu |
| 5 | **L'API** | Routes publiques et d'administration, treize codes, empreintes, OpenAPI, `make check-api-contract`, tests sur base réelle, dont SC-007 | Le client a besoin d'un contrat servi |
| 6 | **US1 — Publier** | Les quatre écrans du back-office, dont l'aperçu page par page et « ouvrir tel quel » ; le menu | Le critère de sortie, et les vraies données de toutes les phases suivantes |
| 7 | **La plomberie hors connexion** | IndexedDB v3 (`copies`, `a-telecharger`), les deux caches, le téléchargement complet ou rien, `If-None-Match`, les effacements (déconnexion, accès perdu, relecture), les textes de déconnexion, tests sans navigateur | Conditionne les récits 3 et 5 |
| 8 | **US2 — Trouver** | Bibliothèque, filtres sur le téléphone, recherche dans le texte, fiche et ses six états, verrou, partage, « Nouveau », écart 41 | S'appuie sur 5 et 7 |
| 9 | **US3 et US4 — Lire** | Le lecteur : texte recomposé et mode « tel quel », barre, reprise, sommaire, recherche et occurrences, taille, thème, feuille du terme, « pas sur le téléphone » | Le critère de sortie |
| 10 | **US5 et US6 — Garder et corriger** | « Mes documents », tout retirer, favoris et file, « Documents récents » de « Ma journée », « À propos » ; les notes dans le lecteur | S'appuie sur 7 et 9 |
| 11 | **Recette** | Le quickstart déroulé, `en` relu, planche complétée, non-régression du site, `make check-safe` | — |

### Si l'essai rend l'issue B ou C

- **B** : la phase 4 devient « le service interne d'ADR-004 et l'import Rust ». Le commanditaire valide le coût écrit en [R1](research.md) avant la phase 2.
- **C** : la phase 4 reste, pour les documents simples, et le guide se publie « tel quel ». Les phases 6 à 10 ne changent pas : elles portent déjà les deux modes.

### Ce qui ne se fait pas depuis un poste

- Le guide téléchargé en 3G bridée.
- La reprise après une nuit de veille.
- **Le guide lu en mode avion sur un Android et un iPhone réels.** C'est la vraie preuve du critère de sortie, avec T112 de 0b, toujours due.

## Complexity Tracking

*Aucune violation de la constitution à justifier.* Le déplacement du stockage vers `kernel` n'est pas une exception : c'est la forme que le principe II prescrit à une infrastructure partagée.

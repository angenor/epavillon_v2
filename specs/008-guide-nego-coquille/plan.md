# Implementation Plan: Guide Négo — coquille et système de design (0a)

**Branch**: `008-guide-nego-coquille` | **Date**: 2026-09-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/008-guide-nego-coquille/spec.md`

---

## Summary

Poser dans le Nuxt existant un sous-arbre `guide-nego/` rendu côté navigateur, installable, ouvrable hors connexion, fermé par un drapeau, avec son système de design borné. **Aucun crate, aucune route d'API, une ligne de semis.** L'approche tient en huit points, détaillés dans [research.md](research.md) :

1. **Une seule adresse par écran, et le français** — pas de variante `/en/`, qui ferait deux portées et deux applications ; l'interface est en français quel que soit le téléphone, et Guide Négo ne lit ni n'écrit le cookie de langue du site (R3). À vérifier en première tâche.
2. **Manifeste et service worker en chemins relatifs** — le préfixe `/v2/` est un argument de construction ; des adresses relatives le rendent sans objet, dans le manifeste statique comme dans la liste du service worker (R4).
3. **Un service worker engendré à la construction, borné à `guide-nego/`** — un module local calcule la liste exacte des fichiers de Guide Négo et l'inscrit dans `sw.js`. Il ne peut contrôler aucune page du site. `@vite-pwa/nuxt` est écarté, et ADR-019 le dit (R5).
4. **Précache complet, navigation cache d'abord** : le site est redéployé pendant la COP ; sous réseau lent, l'application s'ouvre sur la version gardée, et la nouvelle se garde en arrière-plan sans s'exécuter. Le drapeau, lui, reste lu par l'API — et n'attend pas non plus : la garde vaut aussitôt, le verdict du réseau s'applique à l'arrivée (R5, R7).
5. **Une garde des lectures, livrée une fois** : IndexedDB, avec l'heure de chaque lecture. Son seul client à cette étape est le drapeau ; l'étape 1 y posera les documents (R6).
6. **Le registre des modules fermés du site est étendu, pas doublé** : deux champs facultatifs. La règle « seule une réponse qui dit éteint ferme » vit dans une fonction pure, testée (R7).
7. **Le thème ne touche ni `<html>`, ni le cookie du site** : autre clé, autre élément, autre valeur (R8).
8. **La passation est reprise avec quatre corrections** : le bloc `:root` rentre sous `[data-app="guide-nego"]`, une collision de nom, les écarts tranchés, et huit valeurs sombres mesurées par un script (R9, R10).

## Technical Context

**Language/Version** : TypeScript strict (Nuxt 4.5, Vue 3.5), CSS natif. JavaScript simple pour le service worker. SQL pour une ligne de semis. Node 24 en local.

**Primary Dependencies** : celles du front, inchangées — `nuxt`, `@nuxtjs/i18n` v10, `pinia`. **Une dépendance de développement ajoutée : `sharp`**, pour engendrer les icônes, à la main. Ni module PWA, ni Workbox, ni bibliothèque IndexedDB, ni cadre de test.

**Storage** : PostgreSQL — une ligne ajoutée à `900_seed.sql` § 2. Sur le téléphone : IndexedDB (`guide-nego` / `lectures`), `localStorage` (`gn.theme`, `gn.ouverture-vue`), Cache Storage (`gn-<buildId>`). Voir [data-model.md](data-model.md).

**Testing** : `node --test` sur la logique pure (`frontend/tests/guide-nego/`) ; trois scripts de contrôle branchés sur `check-front` ; le reste à la main, sur téléphone réel, selon [quickstart.md](quickstart.md). Aucun test d'intégration de base : aucune écriture, aucune route.

**Target Platform** : Chrome sur Android et Safari sur iPhone, versions courantes ; 360 px de large, vérifié à 390.

**Project Type** : application web installable, sous-arbre d'un front Nuxt existant.

**Performance Goals** : ouverture hors connexion en moins de deux secondes (SC-002) ; aucune requête vers une autre origine une fois l'application gardée (SC-008).

**Constraints** : aucun rendu serveur ni chemin absolu sous `guide-nego/` (ADR-002) · tout sélecteur sous `[data-app="guide-nego"]`, feuilles **et** styles de composants, aucune téléportation hors de `#gn-portail`, aucun jeton ni composant du site, aucune classe Tailwind (principe XIII) · aucune chaîne ni couleur en dur · cibles de 48 px · aucun fichier au-dessus de mille lignes · le site inchangé (SC-009) · **jamais `make check` ni `down -v`** : la ligne de semis s'insère à la main.

**Scale/Scope** : 11 pages, 1 mise en page, ~24 composants `Gn*`, 7 sections de planche, 5 composables, 5 modules sous `utils/guide-nego/`, 4 feuilles de style, 1 module de construction et son modèle de service worker, 1 manifeste, 5 icônes, ~14 fichiers de traduction par langue, 3 scripts, 1 ligne de semis, 1 ADR. Touchés hors de `guide-nego/` : `nuxt.config.ts`, `utils/feature-modules.ts`, `middleware/feature-flag.global.ts`, `mocks/feature-flags.ts`, la page de chargement des routes sans rendu serveur, `Makefile`, `package.json`.

## Constitution Check

*GATE : à passer avant la phase 0, à repasser après la phase 1.*

| # | Principe | Avant conception | Après conception |
|---|---|---|---|
| I | Le modèle fait autorité | ✅ `010_platform.sql` § drapeaux et `900_seed.sql` § 2 lus | ✅ Une ligne de semis, écrite **avant** le code, consignée dans `docs/progression/modele.md`. Insérée à la main sur la base montée ; aucun rechargement |
| II, III, IV | Frontières, `xmod_fk_*`, outbox | ✅ Sans objet : aucun crate, aucune clé, aucun effet de bord | ✅ Inchangé |
| V | Permission et portée | ✅ Lecture publique ; aucun droit en jeu à cette étape | ✅ Le verdict du drapeau vient de `platform.is_feature_enabled()`, jamais recalculé côté client. Conséquence écrite : allumé = activé **et** 100 % |
| VI, VII, VIII, X | SQLx, contexte d'écriture, invariants, tests sur base | ✅ Sans objet : aucun Rust | ✅ Inchangé |
| IX | Erreurs à code stable | ✅ Aucune erreur nouvelle | ✅ Une panne d'API ne s'affiche pas comme une erreur : elle vaut « hors connexion » (FR-020 bis) |
| XI | Hors connexion d'abord | ✅ Cœur de l'étape | ✅ Tous les écrans s'ouvrent sans réseau, et sous réseau lent sur la version gardée ; toute donnée gardée porte `lu_a` ; aucune écriture à cette étape, donc aucune file. Vérifié sur construction et sur téléphone |
| XII | Confiance | ✅ Aucune donnée importée, aucun contenu | ✅ « Synchronisé à » dit l'heure de la dernière lecture, pas l'instant |
| XIII | Un design propre et borné | ⚠️ `theme.css` de la passation déclare sous `:root` | ✅ Corrigé à la reprise (R9). Les styles des composants s'écrivent eux aussi sous la borne, et rien ne se téléporte dans `<body>` (`#gn-portail`). Les trois vérifications du principe deviennent un script bloquant de `check-front`, qui lit les feuilles et les blocs `<style>` (R9, R12). La page fermée est au design de Guide Négo |
| XIV | Une seule porte | ✅ Aucun service annexe | ✅ Inchangé |
| — | Trois agendas | ✅ | ✅ Titres entiers dans les états vides ; « Programme » seul refusé par le script |
| — | Suivi de Guide Négo | ✅ | ✅ `progress.md` ; `docs/progression/modele.md` pour la seule ligne de semis |
| — | Dépendance d'ampleur | ✅ | ✅ Aucune. Le choix de ne pas prendre de module PWA est consigné : ADR-019 |

**Aucune violation.** « Complexity Tracking » est vide.

## Project Structure

### Documentation (this feature)

```text
specs/008-guide-nego-coquille/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── ecrans.md
│   ├── hors-connexion.md
│   └── systeme-de-design.md
└── tasks.md                 # /speckit-tasks
```

### Source Code (repository root)

```text
docs/database/900_seed.sql                     # + guide_nego.enabled
docs/AppNego/adr/019-…                         # service worker engendré par un module local, cache d'abord
docs/AppNego/05-design.md                      # écarts 30 et 31 (faits) ; + tableau des valeurs sombres retenues

frontend/
├── nuxt.config.ts                             # routeRules, components prefix Gn, imports.dirs
├── modules/guide-nego-garde.ts                # build:manifest → guide-nego/sw.js (version + liste)
├── guide-nego/sw.modele.js                    # hors de modules/, que Nuxt balaie
├── package.json                               # + sharp (dev), scripts de contrôle
├── app/
│   ├── spa-loading-template.html              # fond selon gn.theme, avant tout
│   ├── layouts/guide-nego.vue                 # data-app, data-theme, feuilles, manifeste, enregistrement du service worker
│   ├── middleware/feature-flag.global.ts      # lit closedPath et lastKnown
│   ├── utils/feature-modules.ts               # + entrée guide-nego, deux champs facultatifs
│   ├── utils/guide-nego/                      # drapeaux.ts · connexion.ts · theme.ts · onglets.ts — purs ; garde.ts — IndexedDB
│   ├── composables/guide-nego/                # useGnLecture · useGnDrapeaux · useGnConnexion · useGnTheme · useGnInstallation
│   ├── pages/guide-nego/
│   │   ├── index.vue · negociations.vue · francophonie.vue · echanges.vue · lexique.vue
│   │   ├── ressources/index.vue · ressources/reglages.vue
│   │   └── ouverture.vue · installer.vue · fermee.vue · composants.vue
│   ├── components/guide-nego/                 # Gn*.vue — voir contracts/systeme-de-design.md
│   │   └── planche/                           # une section par fichier
│   ├── assets/guide-nego/                     # theme.css · mesures.css · police.css · base.css · pictogrammes.svg · police/
│   └── mocks/feature-flags.ts                 # + guide_nego.enabled, allumé
├── i18n/locales/{fr,en}/pages/guide-nego.*.json · components/gn-*.json
├── public/guide-nego/                         # manifest.webmanifest · icones/ — sw.js est engendré
├── scripts/                                   # check-guide-nego.mjs · guide-nego-contrastes.mjs · guide-nego-icones.mjs
└── tests/guide-nego/*.test.ts

Makefile                                       # check-front gagne trois contrôles
```

**Structure Decision** : sous-arbre du front existant, sept dossiers tous nommés `guide-nego` (R1). Aucun fichier sous `backend/`.

## Ordre de construction

1. **Vérifier R3** : une page nue sous `guide-nego/`, `ssr: false`, route non localisée ; avec `epavillon_locale=en`, l'affichage est en `fr` et le cookie reste inchangé.
2. **Le modèle** : ligne de semis, `modele.md`, insertion à la main, données d'exemple.
3. **Les fondations** : feuilles, police, sprite, `GnPicto`, mise en page, thème sans éclair ; scripts de bornage et de contrastes branchés d'emblée.
4. **Le drapeau** : logique pure et ses tests, garde des lectures, registre étendu, page fermée.
5. **La coquille** : en-tête, barre d'onglets, états vides, onze pages, connexion et bandeau.
6. **Le hors-connexion** : manifeste, icônes, module de garde et modèle du service worker, installation ; vérifié sur construction, avec et sans `/v2/`, en mode avion **et** sous débit bridé avec une nouvelle construction.
7. **Les composants communs** et la page des composants, section par section.
8. **La recette** : [quickstart.md](quickstart.md) sur téléphone réel ; `make check-safe` ; ADR-019 ; `progress.md`.

Les étapes 3, 4 et 7 se prêtent à des sous-agents en parallèle, périmètres de fichiers disjoints ; 5 et 6 sont séquentielles.

## Complexity Tracking

Aucune violation à justifier.

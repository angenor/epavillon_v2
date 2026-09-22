# Implementation Plan: Guide Négo — thématiques, « Ma journée » et profil (étape 0c)

**Branch**: `010-guide-nego-accueil-profil` | **Date**: 2026-09-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/010-guide-nego-accueil-profil/spec.md`

**Artefacts** : [research.md](research.md) · [data-model.md](data-model.md) ·
[contracts/](contracts/) · [quickstart.md](quickstart.md)

---

## Summary

Une personne entrée dans Guide Négo choisit les filières de négociation qu'elle suit, les retrouve sur
tout appareil où elle se connecte, et arrive sur « Ma journée » — l'accueil quotidien, dont les cinq
blocs sont **vides et le disent**, parce que les modules qui les rempliront viennent aux étapes 1, 3a, 4
et 5. Le profil rassemble ce qui la concerne, et « À propos » sert les textes qui l'engagent.

**L'approche, en une phrase par pièce.**

- **Le vocabulaire** `negotiation_theme` se sème dans `reference.taxonomy_terms` et **se lit par la route
  publique des termes, qui existe déjà** : aucune route nouvelle pour l'afficher.
- **Le suivi** vit dans une table propre du schéma `negotiation`, sur le patron de
  `network_memberships` — surtout pas dans `identity.negotiator_profiles.specializations`, qui n'a de
  ligne pour personne et décrit une compétence attestée, pas une préférence ([R1](research.md)).
- **L'écriture** est un **remplacement en bloc**, donc idempotente, ce qui rend sûre la file d'écritures
  différées que cette étape construit — elle n'existait pas, et la constitution l'exige ([R3](research.md), [R5](research.md)).
  Et parce que l'idempotence protège du rejeu mais **pas de l'ancienneté**, une intention porte l'empreinte
  de l'état sur lequel elle a été prise : un choix parti en retard reçoit `412` et s'abandonne, au lieu
  d'effacer un choix plus récent fait ailleurs ([R16](research.md)).
- **Les textes** sont des fichiers embarqués dans `kernel`, servis par une route publique de `api` au
  site comme à l'application ; la version servie remplace le réglage `PRIVACY_POLICY_VERSION`
  ([R6](research.md)).
- **« Ma journée »** ne lit rien : c'est un cadre, et un cadre mal posé se paye cinq fois.

---

## Technical Context

**Langage / version** : Rust stable (API, worker) · TypeScript strict (Nuxt 4, aucun `any`)

**Dépendances principales** : Actix Web, SQLx à vérification à la compilation, PostgreSQL 17, Nuxt 4,
TailwindCSS v4. **Aucune dépendance nouvelle** — le rendu Markdown est écrit à grammaire close
([R7](research.md)) plutôt qu'emprunté.

**Stockage** : PostgreSQL — schémas `reference` (vocabulaire) et `negotiation` (suivi). Sur l'appareil :
IndexedDB `guide-nego`, magasins `lectures` (existant) et `ecritures` (**nouveau**) ; `localStorage`
pour le thème et les clés de parcours.

**Tests** : `cargo test` sur **base réelle et jetable** (`kernel::testing::TestDb`), `node --test` côté
client, contrôles d'écran à 320 · 360 · 390 px dans les deux thèmes.

**Plateforme cible** : application installable (PWA), rendu côté navigateur, **360 px de référence**,
lisible en plein soleil, utilisable sans réseau.

**Type de projet** : application web installable adossée à l'API du monolithe modulaire — le même dépôt,
le même compte, la même base que l'ePavillon.

**Objectifs de performance** : l'écran s'affiche depuis la garde **sans attendre le réseau** ; lecture
réseau abandonnée au bout de 5 s (mécanisme de 0a) ; « Ma journée » ouverte en moins de deux secondes
après la première ouverture.

**Contraintes** : hors connexion d'abord ; toute donnée lue porte l'heure de sa lecture ; aucun fichier
de plus de 1000 lignes — `useApi.ts` est à **973**, d'où la sortie préalable du bloc `auth`
([R10](research.md)) ; aucun composant ni jeton du site ; `make check` interdit.

**Échelle / périmètre** : quatre écrans nouveaux, un écran complété, trois composants nouveaux, deux
routes d'écriture, une route publique de textes, une table, un vocabulaire de dix termes. Quelques
milliers de négociatrices et négociateurs, une COP à la fois.

---

## Constitution Check

*Porte à franchir avant la recherche, et à repasser après la conception.*

| Principe | Verdict | Comment il est tenu |
|---|---|---|
| **I — Le modèle fait autorité** | ✅ | Le vocabulaire et la table s'écrivent dans `docs/database/` **d'abord** ; la base se migre par un script rejouable ; le changement se consigne dans `docs/progression/modele.md` |
| **II — Frontières de modules** | ✅ | `negotiation` ne dépend que de `kernel` et `contracts` — `tests/frontieres.rs` le prouve déjà. Les textes vivent dans `kernel` et **non dans un module**, précisément pour que `programme` puisse lire la version sans arête nouvelle ([R6](research.md)) ; la route publique se pose dans `api`, comme celle du référentiel |
| **III — Frontières vérifiables (`xmod_fk_*`)** | ✅ | `xmod_fk_theme_subscriptions_person` vers `identity.people`. La clé vers `reference.taxonomy_terms` n'en porte pas : `reference` est noyau partagé, et la vue de contrôle l'exempte |
| **IV — Effets de bord par l'outbox** | ✅ *(sans objet)* | Un changement de thématiques n'a **aucun effet inter-modules** : 3b lira la table. Émettre un événement que personne ne consomme serait du bruit, pas de la rigueur |
| **V — Autorisation par permission et portée** | ✅ | Les deux routes agissent sur **soi** et lisent la session, comme `GET /negotiation/me/access`. Aucune route d'administration à cette étape ([R14](research.md)), donc aucun périmètre à filtrer |
| **VI — SQLx vérifié, pas d'ORM** | ✅ | Requêtes statiques vérifiées à la compilation ; aucune composition dynamique |
| **VII — Contexte d'écriture** | ✅ | `Db::write(&ctx)` pose `app.actor_id` et `app.request_id` avant la première écriture — sans quoi l'audit de la table serait anonyme. Vérifié par un test, comme `toute_ecriture_laisse_son_auteur` |
| **VIII — Les invariants de la base ne se réimplémentent pas** | ✅ | Le vocabulaire est gardé par `negotiation.tg_check_term_taxonomy`, qui **existe déjà** ; le code traduit l'erreur PostgreSQL en message français qui nomme le code refusé |
| **IX — Erreurs à code stable** | ✅ | Deux codes nouveaux au catalogue de `kernel` — liste vide, thématique inconnue. Le catalogue **s'étend, il ne se double pas** |
| **X — Tests d'intégration sur base réelle** | ✅ | Chemin nominal, refus, invariant de base traduit, idempotence du `PUT`, audit. Aucun mock de base |
| **XI — Hors connexion d'abord** | ✅ | Les quatre écrans se lisent sans réseau avec leur heure de lecture ; **la file d'écritures différées se construit ici** — elle manquait |
| **XII — Confiance** | ✅ | **Aucun interrupteur sans effet** : les trois accords de la maquette ne sont pas livrés (écart 40). Aucune donnée importée à cette étape |
| **XIII — Un design propre et borné** | ✅ | Trois composants nouveaux dans le dossier de Guide Négo, bâtis sur `design/passation/`, ajoutés à la page interne. Aucun composant ni jeton du site — les **méthodes d'API** partagées, elles, se réutilisent |
| **XIV — Une seule porte** | ✅ *(sans objet)* | Aucun service annexe |
| **Trois agendas, jamais confondus** | ✅ | « Ma journée » est le seul écran où ils se côtoient, et **chaque ligne portera son origine**. Le mot « Programme » seul ne paraît ni à l'écran ni dans un nom exposé |
| **Garde-fou des 1000 lignes** | ⚠️ → ✅ | `useApi.ts` est à 973 : le bloc `auth` sort **avant** toute autre écriture ([R10](research.md)). `reglages.vue` se découpe si elle dépasse 300 lignes |
| **Le suivi vit dans `progress.md`** | ✅ | Sauf `docs/progression/modele.md`, puisque `docs/database/` bouge (ADR-017) |

**Aucune violation à justifier.** La section *Complexity Tracking* reste vide.

---

## Project Structure

### Documentation (this feature)

```text
specs/010-guide-nego-accueil-profil/
├── plan.md              # Ce fichier
├── research.md          # Les quinze décisions, et ce qui a été écarté
├── data-model.md        # Le vocabulaire, la table, ce qui sort du modèle
├── contracts/
│   ├── api-thematiques.md
│   ├── api-textes.md
│   └── hors-connexion.md
├── quickstart.md        # La recette
├── migration.sql        # Rejouable — la base se migre, elle ne se recharge pas
├── checklists/
│   └── requirements.md
└── tasks.md             # Engendré par /speckit-tasks
```

### Source Code (repository root)

```text
docs/database/
├── 020_reference.sql              # + vocabulaire negotiation_theme et ses dix termes
└── 100_negotiations.sql           # + negotiation.theme_subscriptions

backend/crates/
├── kernel/src/
│   ├── legal/                     # NOUVEAU — textes embarqués (include_str!), version, empreinte
│   ├── config.rs                  # − PRIVACY_POLICY_VERSION, − ProgrammeConfig.privacy_policy_version
│   └── error.rs                   # + deux codes stables
├── api/src/routes/
│   └── legal.rs                   # NOUVEAU — GET /legal/{cle}, publique, à côté de reference.rs
├── modules/negotiation/src/
│   ├── domain/themes.rs           # NOUVEAU — formes rendues
│   ├── repo/themes.rs             # NOUVEAU — lire, remplacer en bloc
│   ├── service/themes.rs          # NOUVEAU — transaction, contexte d'écriture, traduction d'erreur
│   ├── routes/themes.rs           # NOUVEAU — GET et PUT /negotiation/me/themes, ETag
│   ├── routes/openapi.rs          # + les deux chemins
│   └── lib.rs                     # + le montage
├── modules/programme/src/
│   ├── service/registration.rs    # la version vient de kernel — une ligne
│   └── repo/consents.rs           # commentaire de doctrine réécrit
└── modules/negotiation/tests/     # thematiques_*.rs, idempotence, audit, vocabulaire

frontend/app/
├── pages/guide-nego/
│   ├── index.vue                  # « Ma journée » — cinq blocs, avatar, garde de première entrée
│   ├── thematiques.vue            # NOUVEAU — premier choix et modification, même écran
│   └── ressources/
│       ├── reglages.vue           # + Mon suivi, + Application
│       ├── telechargements.vue    # NOUVEAU — état vide et place occupée
│       ├── a-propos.vue           # NOUVEAU
│       └── textes/[cle].vue       # NOUVEAU — un texte long, gardé hors connexion
├── components/guide-nego/
│   ├── GnAvatar.vue               # NOUVEAU
│   ├── GnJauge.vue                # NOUVEAU
│   ├── GnTexteLong.vue            # NOUVEAU — rendu Markdown à grammaire close
│   ├── GnEcran.vue · GnEntete.vue # + l'avatar, relayé
│   └── planche/                   # + les trois composants
├── composables/
│   ├── api/auth.ts                # NOUVEAU — sortie du bloc auth de useApi.ts
│   ├── api/guide-nego.ts          # + thématiques, + textes
│   └── guide-nego/
│       ├── useGnThematiques.ts    # NOUVEAU
│       ├── useGnTextes.ts         # NOUVEAU
│       ├── useGnPlace.ts          # NOUVEAU — mesure et libération
│       └── useGnConnexion.ts      # + écoute de « online », qui manquait
├── utils/guide-nego/
│   ├── garde.ts                   # + magasin « ecritures »
│   └── file.ts                    # NOUVEAU — la file d'écritures différées
├── types/negotiation.ts           # + les formes nommées par l'API
└── i18n/locales/{fr,en}/
    ├── pages/guide-nego.thematiques.json · .a-propos.json · .telechargements.json · .textes.json
    └── components/gn-avatar.json · gn-jauge.json
```

**Structure Decision** — le dépôt impose ses emplacements et cette étape n'en invente aucun. Trois choix
méritent d'être dits : les **textes dans `kernel`** et leur route dans `api`, parce qu'aucun module ne
peut les porter sans créer l'arête que le principe II interdit ; le **suivi dans le schéma
`negotiation`**, parce que c'est une donnée de Guide Négo et non du socle d'identité ; et la **file
d'écritures dans `utils/guide-nego/`**, à côté de la garde qu'elle prolonge, bornée à ce qu'elle sert.

---

## Séquencement

**Sur la branche `010-guide-nego-accueil-profil`, comme à l'étape 0b. Un commit par phase**, avec ses
contrôles ciblés. `make check-safe` **en fin de cycle seulement**, API arrêtée.

| # | Phase | Ce qu'elle livre | Pourquoi là |
|---|---|---|---|
| 1 | **La place, côté client** | Sortie du bloc `auth` vers `composables/api/auth.ts` — **et rien d'autre** | 27 lignes de marge sous le garde-fou : tout ajout la franchirait. Voir la note ci-dessous |
| 2 | **Le modèle** | Vocabulaire, dix termes, table, triggers, index, migration rejouable, ligne dans `docs/progression/modele.md` | Rien du métier ne s'écrit avant le SQL |
| 3 | **Les thématiques, côté API** | `GET` et `PUT /negotiation/me/themes`, l'empreinte sur les codes triés, `If-Match` et le `412`, trois codes d'erreur, OpenAPI, tests sur base réelle | Le client a besoin d'un contrat servi |
| 4 | **La file d'écritures** | Magasin `ecritures`, intention portant empreinte et compte, départ à l'ouverture · `online` · retour au premier plan, vidange à la déconnexion, tests sans navigateur | Elle conditionne le récit 1 hors connexion |
| 5 | **US1 — Mes thématiques** | `GnAvatar`, l'écran de choix, `useGnThematiques`, la garde de première entrée, le message du `412` | Le critère de sortie de l'étape |
| 6 | **US2 — Ma journée** | Les cinq blocs et leurs états vides, l'avatar dans l'en-tête | Le cadre des quatre étapes suivantes |
| 7 | **US3 — Profil** | Mon suivi, `GnJauge`, les téléchargements, la place occupée et sa libération | S'appuie sur 5 |
| 8 | **US4 — Textes** | `kernel/legal`, `GET /legal/{cle}`, les **deux** contrôles — empreinte figée, et grammaire close éprouvée sur les fichiers réels —, la bascule de `PRIVACY_POLICY_VERSION`, « À propos », `GnTexteLong` | Touche `programme` : la faire seule, et tard |
| 9 | **Recette** | Le quickstart déroulé, traductions `en` relues, planche complétée, non-régression du site, `make check-safe` | — |

### La phase 1 est un commit à part, et ne fait qu'une chose

Elle **déplace** le bloc `auth` et ne change rien d'autre : pas de méthode nouvelle, pas de signature
retouchée, **aucun des 14 appelants modifié**. Rien d'autre du site ne bouge dans toute l'étape — c'est
le seul fichier partagé auquel 0c touche, avec la ligne de `programme` en phase 8.

Sa porte : `npm run typecheck`, `npm run build` et `make check-api-contract` au vert, et la revue du
diff doit se lire comme un déplacement.

### Ce qui ne se fait pas depuis un poste

L'installation sur Android et iPhone réels, le choix pris en 3G bridée, la file qui repart **après une
nuit de veille** — c'est le cas que l'événement `online` ne couvre pas et que le départ à l'ouverture
existe pour attraper —, et **T112 de l'étape 0b**, toujours due.

## Complexity Tracking

*Aucune violation de la constitution à justifier.*

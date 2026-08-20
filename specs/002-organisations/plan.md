# Implementation Plan: Organisations (B2)

**Branch**: `002-organisations` | **Date**: 2026-08-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-organisations/spec.md`

---

## Summary

Poser le second module métier — `backend/crates/modules/org` — sur le socle livré en B1, et rendre tenable par l'API la règle métier n° 1 du projet : **une organisation, plusieurs dénominations**. C'est le défaut n° 1 de la version 1, et le modèle a posé quatre verrous contre lui ; ce module en est la surface.

L'approche tient en six points, tous déduits du modèle ou d'une mesure, jamais choisis librement :

1. **Une seule fonction de recherche, deux lectures.** La fonction du modèle n'est pas touchée. La lecture destinée à une personne l'enveloppe d'un filtre d'une ligne — n'admettre que les fiches portant une ressemblance de dénomination — et **sur-lit** pour que la limite reste tenue ; celle destinée à la revue des doublons ne filtre rien. Le domaine continue d'alimenter le score des deux (R1).
2. **La cible de 150 ms se mesure avant de se traiter.** Le semis de 5 000 organisations imite la distribution réelle des noms, sans quoi la mesure serait excellente et fausse ; l'ordre des remèdes est fixé d'avance et ne commence pas par modifier le SQL (R2, R3).
3. **La fusion s'écrit dans un ordre qui n'est pas celui qu'on croyait.** Les arbitrages de champ viennent **après** l'appel de fusion et dans la même transaction : tant que la fiche absorbée est vivante, l'unicité du nom interdit à la survivante de reprendre le sien (R5). L'adresse d'URL, elle, n'est pas arbitrable (R6).
4. **Le décompte de transfert se lit dans le registre, en une requête composée.** C'est le seul SQL dynamique du module, borné à un fichier, et c'est ce qui fait que le chiffre annoncé et le chiffre réel viennent du même raisonnement (R4).
5. **Une pièce remonte dans le noyau, parce que trois modules en auront besoin** : le service de jetons à usage unique, dont l'invitation est une finalité (R8). Et un défaut de B1 est corrigé au passage, sans quoi l'invitation resterait une demi-fonctionnalité : une personne créée par invitation ne pouvait pas se créer de compte (R9).
6. **Ce qui est de fond reste de fond.** Détection des doublons, score de confiance et rafraîchissement de la projection sont trois travaux différés, coalescés par clé d'unicité, jamais des triggers ajoutés au modèle (R11, R12, R13).

---

## Technical Context

**Language/Version**: Rust stable, édition 2021, chaîne épinglée par `backend/rust-toolchain.toml` — inchangée depuis B1.

**Primary Dependencies**: **aucune dépendance nouvelle.** Le module se contente de ce que le workspace déclare déjà : Actix Web 4, SQLx 0.9 (PostgreSQL, macros vérifiées à la compilation), `utoipa` pour l'OpenAPI engendré, `serde`, `uuid`, `time`. Le seul point d'attention est le **SQL composé dynamiquement** du décompte de fusion (R4), qui échappe par nature à la vérification à la compilation et vit dans un fichier unique.

**Storage**: PostgreSQL 17 + pgvector — **schéma existant, non modifié**. Les extensions dont ce module dépend (`pg_trgm`, `unaccent`) sont déjà chargées par `000_bootstrap.sql`. Valkey toujours inutilisé (R19). Garage hors périmètre.

**Testing**: `cargo test` sur base **réelle et jetable**, harnais `kernel::testing` de B1. Aucun mock de base. Deux tests portent des critères que rien d'autre ne tiendrait : la mesure de la recherche sur 5 000 fiches, et le décompte de fusion comparé avant / après.

**Target Platform**: serveur Linux ; développement macOS et Linux via `ops/docker-compose.dev.yml`.

**Project Type**: service web (API) adossé à un front Nuxt existant. Les deux binaires de B1 accueillent le module : `api` monte ses routes, `worker` ses six travaux différés.

**Performance Goals**: **une cible chiffrée, et elle est dure** — la recherche répond en moins de **150 ms** au 95ᵉ centile sur **5 000 organisations**, limite par défaut, mesuré côté serveur (SC-002). Rien d'autre n'est contraint : la fiche et la liste du back-office s'ouvrent à la main, le décompte de fusion est un écran d'arbitrage, et les trois travaux de fond n'ont pas de délai de service.

**Constraints**: `DATABASE_URL` renseignée et base démarrée pour compiler (SQLx) · aucun fichier de `backend/` au-dessus de 1000 lignes · aucun avertissement Clippy · aucune arête entre deux crates de module · les chemins et noms de champs sont ceux que `frontend/app/composables/useApi.ts` et `composables/api/admin-organizations.ts` consomment déjà · **aucune modification de `docs/database/`** sans justification écrite, ce que ce plan ne propose nulle part.

**Scale/Scope**: 21 routes HTTP, 1 crate créé, 6 travaux différés — trois de fond, trois courriels —, 1 pièce déplacée dans le noyau, 1 correction dans `identity`. Volumétrie de référence : **5 000 organisations**, quelques dizaines de milliers de dénominations, quelques milliers d'adhésions. La détection des doublons est le seul traitement dont le coût croît en produit — il est donc découpé en tranches (R11).

---

## Constitution Check

*GATE : à passer avant la phase 0, à repasser après la phase 1.*

| # | Principe | Évaluation avant conception | Évaluation après conception |
|---|----------|------------------------------|------------------------------|
| I | Le modèle de données fait autorité | ✅ `040_organizations.sql` relu intégralement en B2-specify, plus `130_analytics.sql` § 5, `030_identity.sql` § 6 et les huit fichiers qui alimentent le registre des références. **Aucune modification proposée** | ✅ La conception a **plié devant le modèle** à trois reprises : l'ordre des écritures de la fusion est dicté par la portée de l'unicité du nom (R5), l'adresse d'URL cesse d'être arbitrable parce que son unicité n'a pas de condition de statut (R6), et la reprise d'une adhésion révoquée existe parce que l'unicité porte sur le couple sans regarder l'état (R7). Trois fois, c'est le SQL qui a décidé |
| II | Frontières de modules | ✅ Un crate, dépendant du noyau et des contrats, de rien d'autre | ✅ Tenu, et **c'est ce qui a décidé de R8** : trois des cinq finalités de jeton n'appartiennent pas à `identity`, et aucun module ne peut en dépendre — le service remonte donc dans le noyau plutôt que d'être recopié. `cargo tree -p org` doit ne montrer aucune arête vers un autre module |
| III | Frontières vérifiables en base (`xmod_fk_*`) | ✅ Aucune clé étrangère créée — aucun DDL dans ce module | ✅ `make check-db` reste au vert sans rien changer |
| IV | Effets de bord par l'outbox transactionnel | ✅ `platform.emit_event()` appelée dans la transaction du changement d'état, jamais d'insertion à la main | ✅ Tenu, avec **une soustraction explicite** : la fusion **n'émet rien**, parce que la fonction de base émet déjà son événement (écart n° 76). C'est le piège d'`identity` répété à l'identique, et il est inscrit dans `contracts/events.md` à l'endroit où l'on serait tenté d'ajouter la ligne. Les trois travaux de fond passent par `platform.jobs`, ce que la constitution prévoit pour un travail qui n'annonce pas un changement d'état |
| V | Autorisation par permission et par portée | ✅ Trois permissions du modèle, testées par le garde du noyau | ✅ **Rien à écrire (R16)** : B1 a construit le garde pour ce module. Une correction de fond apportée par la conception — la permission de consultation est détenue par le rôle d'utilisateur ordinaire, donc la liste exige **permission ET périmètre non vide** (écart n° 73), ce qui est exactement la garde posée en B1 sur la liste des utilisateurs. La qualité de **référent** n'est pas une permission mais une adhésion, lue en base : la tester par un nom de rôle serait l'entorse que le principe interdit |
| VI | SQLx vérifié à la compilation, pas d'ORM | ✅ Macros `query!`/`query_as!` | ⚠️ **Une exception, assumée et bornée** : le décompte de transfert compose son SQL depuis le registre des références (R4). Justifiée en « Complexity Tracking » ci-dessous. Tout le reste est vérifié à la compilation, et `.sqlx/` est régénéré avec `--all-targets` |
| VII | Contexte d'écriture systématique | ✅ Unique porte d'écriture du noyau | ✅ Tenu, y compris pour la fusion : la fonction de base lit l'acteur dans le contexte de transaction, elle **doit** donc être appelée par cette porte, faute de quoi le journal des fusions n'aurait pas d'auteur. **Une trace anonyme légitime est nommée** — le recalcul de score par le worker (R12), qui n'a pas d'auteur parce que personne ne l'a demandé |
| VIII | Les invariants de la base ne sont pas réimplémentés | ✅ Dix contraintes et trois triggers identifiés, aucun recopié | ✅ Les refus de la base sont **traduits** : unicité du nom, unicité du domaine vérifié, unicité de l'adhésion, chaîne de fusion (message du trigger repris **mot pour mot**), rattachement automatique sans vérification. Table dans `contracts/errors.md` |
| IX | Erreurs d'API : code stable, message français | ✅ Type d'erreur unique du noyau | ✅ Onze codes ajoutés, tous engendrés dans l'OpenAPI depuis le catalogue du noyau. La règle de statut de B1 est reprise sans changement (R17) |
| X | Tests d'intégration sur base réelle | ✅ Harnais de B1 | ✅ Les quatre obligations minimales ont chacune leur test nommé (R18), plus deux tests qui tiennent des critères que rien d'autre ne tiendrait |

**Verdict** : une entorse, justifiée ci-dessous ; aucune autre.

Trois points ont été pesés et tranchés dans `research.md` plutôt que laissés à l'implémentation, parce qu'une décision tacite y aurait produit une faute : **l'ordre des écritures de la fusion** (R5), qui échouerait à l'exécution sur le champ le plus souvent arbitré ; **la place du service de jetons** (R8), qui aurait produit une seconde implémentation de la consommation atomique d'un jeton ; et **le semis de la mesure** (R3), qui aurait rendu SC-002 vert sans rien prouver.

---

## Project Structure

### Documentation (this feature)

```text
specs/002-organisations/
├── spec.md              # Ce qu'il faut faire (/speckit-specify)
├── plan.md              # Ce fichier
├── research.md          # Phase 0 — 19 décisions techniques et leurs alternatives
├── data-model.md        # Phase 1 — le modèle existant, et ce que le code en fait
├── contracts/
│   ├── routes.md        #   les routes, leur autorisation, leur politique de statut
│   ├── errors.md        #   les codes stables ajoutés et la traduction PostgreSQL
│   └── events.md        #   les événements émis, ceux qu'on n'émet pas, les travaux différés
├── quickstart.md        # Phase 1 — comment lancer, éprouver et vérifier
├── checklists/
│   └── requirements.md  # Contrôle qualité de la spécification
└── tasks.md             # Phase 2 — produit par /speckit-tasks, PAS par ce fichier
```

### Source Code (repository root)

```text
backend/
├── Cargo.toml                        # + un membre : crates/modules/org
└── crates/
    ├── kernel/
    │   ├── tokens.rs                 # NOUVEAU — jetons à usage unique, remontés d'identity (R8)
    │   └── …                         # le reste inchangé : auth, db, error, events, jobs, mail…
    ├── contracts/
    │   ├── org.rs                    # NOUVEAU — charges utiles des événements org.*
    │   └── identity.rs               # inchangé
    ├── modules/
    │   ├── identity/                 # DEUX modifications, et pas une de plus
    │   │   ├── repo/tokens.rs        #   supprimé — délègue à kernel::tokens
    │   │   └── service/registration.rs #   corrigé — personne connue SANS compte (R9)
    │   └── org/                      # NOUVEAU
    │       ├── domain/               #   types purs : score, motifs, sorts de transfert, issues
    │       │   ├── search.rs          #     résultat de recherche, motifs, seuil de sur-lecture
    │       │   ├── membership.rs      #     les deux files, les issues de rattachement
    │       │   ├── merge.rs           #     champs arbitrables, avertissements, décompte
    │       │   └── ids.rs
    │       ├── repo/                 #   requêtes SQLx, un fichier par agrégat
    │       │   ├── search.rs · organizations.rs · names.rs · domains.rs
    │       │   ├── memberships.rs · duplicates.rs · merge.rs
    │       │   └── merge_counts.rs    #     LE SEUL fichier composant du SQL (R4)
    │       ├── service/              #   les règles
    │       │   ├── search.rs · join.rs · create.rs · membership.rs
    │       │   ├── admin_list.rs · admin_detail.rs · admin_write.rs
    │       │   └── merge.rs
    │       ├── jobs/                 #   les travaux de fond
    │       │   ├── duplicates.rs · trust_score.rs · scorecard.rs
    │       │   └── emails.rs          #     les trois courriels d'adhésion
    │       ├── routes/               #   handlers, DTO, annotations OpenAPI
    │       │   ├── public.rs · memberships.rs · admin.rs · openapi.rs
    │       ├── lib.rs                #   ce que le module expose : routes, travaux, consommateurs
    │       └── tests/                #   tests d'intégration sur base réelle
    ├── api/                          # + monte les routes d'org
    └── worker/                       # + enregistre les travaux d'org
```

**Structure Decision** : la forme est celle qu'`identity` a inaugurée en B1 — domaine, dépôt, service, travaux, routes —, recopiée sans être réinventée, pour la raison qui l'avait fait choisir : une unité par **agrégat**, ce qui garde chaque fichier loin de la limite de mille lignes et permet de n'ouvrir que ce qui concerne la tâche. Les quatre modules suivants la recopieront à leur tour.

**Trois endroits sont touchés hors du crate `org`, et il faut les avoir en tête** :

- **`backend/crates/kernel/tokens.rs`** — le service de jetons remonte ici (R8). La configuration des durées par finalité y est **déjà**, depuis B1.
- **`backend/crates/modules/identity/`** — deux modifications, et pas une de plus : le dépôt de jetons délègue au noyau, et l'inscription cesse de laisser sans compte une personne créée par invitation (R9). Les tests de B1 sur la vérification d'adresse, la réinitialisation et le rejeu de jeton **ne sont pas réécrits** : ils sont la preuve que le déplacement s'est fait à comportement constant.
- **`.env.example`** — trois réglages d'exploitation nouveaux : seuil d'entrée dans la file des doublons, taille de tranche du balayage, fenêtre de coalescence du rafraîchissement de la projection.

Rien ne change côté `frontend/`. `NUXT_PUBLIC_API_BASE` reste vide jusqu'à B7, et les deux filtres que les écrans appliquent aujourd'hui sur les résultats de recherche deviendront inertes sans devenir faux.

---

## Complexity Tracking

| Violation | Pourquoi elle est nécessaire | Alternative plus simple, et pourquoi elle est écartée |
|-----------|------------------------------|-------------------------------------------------------|
| **Principe VI — un fichier compose son SQL dynamiquement** (`repo/merge_counts.rs`, R4) | Le décompte de transfert doit parcourir `org.organization_references`, qui est un **registre** : dix-huit lignes aujourd'hui, davantage quand les modules hors jalon vivront. C'est aussi ce que fait `merge_organizations()` elle-même, à partir de la même source — c'est la seule façon que le chiffre annoncé avant et le chiffre rendu après viennent du même raisonnement, ce qu'exige SC-010 | *Énumérer les tables en Rust* : c'est exactement ce que le registre existe pour éviter ; le décompte deviendrait faux au premier module qui s'y déclare, **et personne ne le verrait**. *Ajouter une fonction SQL au modèle* : plus élégant, mais c'est une modification du SQL que rien n'impose — le besoin est entièrement satisfait depuis l'application. À reproposer si un second appelant apparaît. Le risque d'injection est nul (identifiants venant du DDL) et néanmoins couvert par `quote_ident` |

Aucune autre violation. Le déplacement du service de jetons dans le noyau (R8) **n'en est pas une** : le noyau connaît déjà le schéma `identity` — c'est là que vit le garde d'autorisation depuis B1 —, et c'est précisément la décision qui **évite** l'entorse au principe II.

---

## Ce que ce plan ne tranche pas, volontairement

- **La route composée de l'espace organisation** (`GET /organizations/{id}/workspace`). Elle mêle organisation, dossiers, séances et rappels : elle appartient à B4, qui composera sa part d'organisation par une lecture franchissant la frontière, symétrique de celle que ce module fait vers `programme` (R14). B2 ne la livre pas et n'en préjuge pas.
- **L'attribution du rôle d'utilisateur ordinaire à l'inscription.** Rien ne l'attribue aujourd'hui (écart n° 74), ce qui décide de la garde de la recherche (FR-014). Le jour où l'inscription l'attribuera, l'exigence pourra être resserrée sans changer le contrat. Inscrit dans `points-bloques.md` pour B7.
- **La table d'alias d'adresses d'URL**, qui serait la vraie réponse à R6. Aucun écran public n'utilise encore l'adresse d'une organisation ; le besoin n'est pas prouvé.
- **La reprise des données de la version 1** (`910_migration_v1.sql`), qui remplira la file des doublons bien plus vite que le balayage. Elle a son propre jalon, et le module est écrit pour qu'elle n'ait rien à réinventer : c'est le même balayage, sur un référentiel plus grand.
- **La vérification d'un domaine par enregistrement DNS ou par courriel.** Le modèle porte les trois méthodes ; seule la manuelle est livrée, et c'est ce que le contrat du front annonce déjà.

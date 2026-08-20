# Implementation Plan: Événements (B3)

**Branch**: `003-evenements` | **Date**: 2026-08-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/003-evenements/spec.md`

---

## Summary

Poser le troisième module métier — `backend/crates/modules/event` — sur le socle de B1 et les frontières éprouvées en B2, et rendre possible ce que tout le jalon attend : **ouvrir l'appel à propositions de la COP31**. Sans édition, pas d'appel ; sans appel, pas de dossier ; sans grille, pas d'évaluation.

L'approche tient en sept points, tous déduits du modèle ou du contrat du front, jamais choisis librement :

1. **Le sigle est la seule règle que le service ajoute au modèle**, et elle est bornée : exigée quand le pavillon est tenu, vérifiée sur l'état **résultant** de l'écriture, avec une valeur par défaut proposée. La base pourrait l'exprimer par une vérification conditionnelle — **on s'en abstient délibérément**, parce que l'arbitrage retenu veut qu'une édition sans pavillon reste enregistrable sans sigle, migration de la v1 comprise (R1).
2. **Le périmètre d'administration n'a rien à écrire.** B1 a livré le garde, l'extracteur et les trois cas distincts. Ce module n'ajoute qu'une chose : **remonter à l'édition** d'un fil, d'un lieu, d'une salle, d'un canal ou d'un appel **avant** de vérifier le périmètre (R2).
3. **Les jours civils d'une édition se calculent en base, dans le fuseau de l'édition.** Les calculer en Rust demanderait une base de fuseaux qui n'est pas celle de PostgreSQL — c'est exactement le défaut qui a fait tomber le formulaire du front sur `Europe/Geneva` (R5).
4. **Le plan de génération du calendrier et son exécution partagent la même fonction pure**, et l'exécution le recalcule dans sa transaction : jamais elle ne fait confiance au plan que le client lui renvoie (R4).
5. **Tout ce qui détache une séance le compte AVANT de détacher.** Les clés étrangères sont `ON DELETE SET NULL` : après coup, le lien n'existe plus et le chiffre serait faux (R8).
6. **Un critère porteur de notes ne se supprime pas.** La clé étrangère est `ON DELETE CASCADE` : la base **détruirait les notes en silence**. Le service compte et refuse — c'est le seul endroit du module où l'invariant à tenir est celui que la base ne tient pas (R9, écart n° 91).
7. **Publier la programmation traverse deux schémas : la frontière passe par l'outbox.** Ce module contrôle, estampille l'édition et **annonce** ; le module Programmation rend ses séances publiques. Et les deux routes de publication vivent sous le préfixe du planificateur, composé **une seule fois dans `api`** — le piège de B1 où deux `web::scope` du même préfixe faisaient taire trois routes (R10, R11).

---

## Technical Context

**Language/Version**: Rust stable, édition 2021, chaîne épinglée par `backend/rust-toolchain.toml` — inchangée depuis B1.

**Primary Dependencies**: **aucune dépendance nouvelle.** Actix Web 4, SQLx 0.9 (PostgreSQL, macros vérifiées à la compilation), `utoipa`, `serde`, `uuid`, `time` — tout est déjà déclaré par le workspace. **Aucun SQL composé dynamiquement dans ce module** : contrairement à B2, toutes les requêtes passent la vérification à la compilation.

**Storage**: PostgreSQL 17 + pgvector — **schéma existant, non modifié**. Valkey toujours inutilisé (R20). Garage hors périmètre : ce module **lit** les images rattachées, il n'en pose aucune (R17).

**Testing**: `cargo test` sur base **réelle et jetable**, harnais `kernel::testing` de B1. Aucun mock de base. Le semis fournit **quatre séries et un canal de diffusion général** mais **aucune édition** : chaque test crée la sienne (R19).

**Target Platform**: serveur Linux ; développement macOS et Linux via `ops/docker-compose.dev.yml`.

**Project Type**: service web (API) adossé à un front Nuxt existant. Les deux binaires de B1 accueillent le module : `api` monte ses routes, `worker` son travail différé.

**Performance Goals**: **aucune cible chiffrée, et c'est un fait, pas un oubli.** Rien ici n'est une recherche interactive : la liste des éditions se compte en dizaines de lignes, le détail est un écran d'administration ouvert à la main, la page publique est une lecture d'une vue prête à l'emploi. Le seul point qui mérite d'être surveillé est le **détail en une réponse** (douze lectures), traité par une transaction en lecture seule sur **une seule connexion** plutôt que par un éventail concurrent — c'est la leçon de B2, où une transaction perdante retenait deux connexions du pool (R3).

**Constraints**: `DATABASE_URL` renseignée et base démarrée pour compiler (SQLx) · aucun fichier de `backend/` au-dessus de 1000 lignes · aucun avertissement Clippy · aucune arête entre deux crates de module · les chemins et noms de champs sont **exactement** ceux que `composables/api/admin-events.ts`, le bloc `events` de `composables/useApi.ts` et `composables/api/planner.ts` consomment déjà · **aucune modification de `docs/database/`**, ce que ce plan ne propose nulle part.

**Scale/Scope**: **37 routes HTTP**, 1 crate créé, **1 travail différé** récurrent, **6 événements de domaine**, **3 codes d'erreur** ajoutés au catalogue du noyau, **2 champs additifs** au contrat du front (obligations pour B7). Volumétrie de référence : quelques dizaines d'éditions, une douzaine de journées par édition, une poignée de fils, de lieux, de salles et de canaux, un appel et six critères par édition, une dizaine de membres de comité.

---

## Constitution Check

*GATE : à passer avant la phase 0, à repasser après la phase 1.*

| # | Principe | Évaluation avant conception | Évaluation après conception |
|---|----------|------------------------------|------------------------------|
| I | Le modèle de données fait autorité | ✅ `060_events.sql` relu intégralement en B3-specify, plus `030_identity.sql` § 6, `075_programme_sessions.sql` § 6 et 7, `070_programme_proposals.sql` § 2 et 6, `900_seed.sql` § 4. **Aucune modification proposée** | ✅ La conception a **plié devant le modèle** quatre fois : les jours civils se calculent avec le fuseau **en base** parce que la base de fuseaux de PostgreSQL fait foi (R5) ; le canal par défaut se retire **avant** de se poser parce que l'index unique n'est pas différable (R6) ; le décompte de détachement se prend **avant** la suppression parce que la clé est `SET NULL` (R8) ; et la publication passe par l'outbox parce que la vue publique filtre sur la date de **chaque séance** et non sur celle de l'édition (R10) |
| II | Frontières de modules | ✅ Un crate, dépendant du noyau et des contrats, de rien d'autre | ✅ Tenu. **Deux endroits auraient pu le rompre et ne le rompent pas** : la publication, qui écrirait dans `programme` — traitée par annonce (R10) ; et les deux routes du planificateur, dont le préfixe sera partagé avec B5 — traitées par une composition **dans `api`**, comme `/people` en B1 (R11). `cargo tree -p event` doit ne montrer aucune arête vers un autre module |
| III | Frontières vérifiables en base (`xmod_fk_*`) | ✅ Aucune clé étrangère créée — aucun DDL dans ce module | ✅ `make check-db` reste au vert sans rien changer |
| IV | Effets de bord par l'outbox transactionnel | ✅ `platform.emit_event()` dans la transaction du changement d'état, jamais d'insertion à la main | ✅ Tenu, et **c'est ici que le principe porte tout son poids** : la publication de la programmation est exactement le cas qu'il décrit — un effet dans un autre module, annoncé et non appelé (R10). **Une vérification a levé un doute utile** : aucun déclencheur de ce module n'émet d'événement, contrairement à `identity` et `org` — le service émet donc tout lui-même, et la double émission est impossible (écart n° 87). Six événements, six raisons, et une liste explicite de ce qui **n'émet rien** |
| V | Autorisation par permission et par portée | ✅ Deux permissions du modèle, testées séparément par le garde du noyau | ✅ **Presque rien à écrire (R2)** : `Perimeter`, `AdminScope` et `require_perimeter` existent depuis B1. Trois décisions de conception : la **création** exige la portée globale (FR-011) ; les objets enfants **remontent à leur édition avant** le contrôle, sans jamais divulguer ce qu'ils ont lu (R2) ; et la **publication** est gardée par la permission de planifier, celle que le modèle attribue au rôle chargé de publier le programme — le garde vivant dans le noyau, cela ne crée aucune arête (R12) |
| VI | SQLx vérifié à la compilation, pas d'ORM | ✅ Macros `query!`/`query_as!` | ✅ **Aucune exception.** Contrairement à B2, ce module ne compose aucun SQL : la seule requête « variable » est la mise à jour partielle d'une édition, traitée par un `UPDATE` complet à partir de la charge utile, qui est totale (R13) |
| VII | Contexte d'écriture systématique | ✅ Unique porte d'écriture du noyau | ✅ Tenu. **Deux seules tables du module sont auditées** — l'édition et l'appel : c'est là que le contexte compte, et c'est là que l'unique porte est indispensable. Les journées, fils, lieux, salles et canaux ne le sont pas ; on ne le compense pas, on le **consigne** (écart n° 92) |
| VIII | Les invariants de la base ne sont pas réimplémentés | ✅ Quatorze contraintes nommées identifiées, aucune recopiée | ⚠️ **Deux règles vivent dans le service, et une seule est une entorse.** Le **sigle** n'en est pas une : le modèle ne le porte pas et ne doit pas le porter (R1). Le **critère porteur de notes** en est une, justifiée en « Complexity Tracking » : la clé étrangère est `ON DELETE CASCADE`, la base détruirait les notes sans rien dire, et le contrat du front exige qu'on refuse |
| IX | Erreurs d'API : code stable, message français | ✅ Type d'erreur unique du noyau | ✅ **Trois codes ajoutés seulement** — la majorité des refus de ce module sont **exprimés par le contrat du front** et sortent donc en 200, règle de B1 reprise sans changement. Tous engendrés dans l'OpenAPI depuis le catalogue du noyau |
| X | Tests d'intégration sur base réelle | ✅ Harnais de B1 | ✅ Les quatre obligations minimales ont chacune leur test nommé (R18), plus **un test qui frappe les trente-sept routes sur la vraie application** — la leçon de B2, où trois routes sur vingt et une étaient muettes |

**Verdict** : une entorse, justifiée ci-dessous ; aucune autre.

Quatre points ont été tranchés dans `research.md` plutôt que laissés à l'implémentation, parce qu'une décision tacite y aurait produit une faute **à l'exécution** : le calcul des jours civils **en base** (R5), qui produirait un calendrier décalé d'un jour sur la moitié des fuseaux ; l'ordre du canal par défaut (R6), qui violerait l'index unique ; le décompte **avant** détachement (R8), qui rendrait zéro ; et la suppression d'un critère (R9), qui **détruirait des notes**.

---

## Project Structure

### Documentation (this feature)

```text
specs/003-evenements/
├── spec.md              # Ce qu'il faut faire (/speckit-specify)
├── plan.md              # Ce fichier
├── research.md          # Phase 0 — 20 décisions techniques et leurs alternatives
├── data-model.md        # Phase 1 — le modèle existant, et ce que le code en fait
├── contracts/
│   ├── routes.md        #   les 37 routes, leur autorisation, leur politique de statut
│   ├── errors.md        #   les 3 codes stables ajoutés et la traduction PostgreSQL
│   └── events.md        #   les 6 événements émis, ceux qu'on n'émet pas, le travail différé
├── quickstart.md        # Phase 1 — comment lancer, éprouver et vérifier
├── checklists/
│   └── requirements.md  # Contrôle qualité de la spécification
└── tasks.md             # Phase 2 — produit par /speckit-tasks, PAS par ce fichier
```

### Source Code (repository root)

```text
backend/
├── Cargo.toml                        # + un membre : crates/modules/event
└── crates/
    ├── kernel/
    │   └── error.rs                  # + 3 variantes au catalogue (EVENT_*)
    ├── contracts/
    │   └── event.rs                  # NOUVEAU — charges utiles des événements event.*
    ├── modules/
    │   ├── identity/                 # INCHANGÉ
    │   ├── org/                      # INCHANGÉ
    │   └── event/                    # NOUVEAU
    │       ├── domain/               #   types purs : aucune requête, tout est testable seul
    │       │   ├── acronym.rs         #     la règle du sigle et la valeur proposée (R1)
    │       │   ├── calendar.rs        #     le plan de génération, fonction pure (R4)
    │       │   ├── call.rs            #     grille, diff de critères, effet sur les notes (R9)
    │       │   ├── permissions.rs     #     les trois permissions consommées
    │       │   └── ids.rs
    │       ├── repo/                 #   requêtes SQLx, un fichier par agrégat
    │       │   ├── editions.rs · days.rs · tracks.rs · venues.rs · channels.rs
    │       │   ├── calls.rs · criteria.rs · committee.rs
    │       │   ├── public.rs          #     les deux vues jointes, en une requête (R16)
    │       │   └── cross.rs           #     LES SEULES lectures hors schéma, réunies ici (R14)
    │       ├── service/              #   les règles
    │       │   ├── edition_read.rs · edition_write.rs
    │       │   ├── detail.rs          #     la composition des six onglets (R3)
    │       │   ├── days.rs · tracks.rs · venues.rs · channels.rs
    │       │   ├── call.rs · committee.rs
    │       │   └── publication.rs     #     contrôle, estampille, annonce (R10)
    │       ├── jobs/
    │       │   └── autoclose.rs       #     clôture d'un appel échu, récurrent (R15)
    │       ├── routes/               #   handlers, DTO, annotations OpenAPI
    │       │   ├── public.rs · admin_events.rs · admin_tabs.rs
    │       │   ├── admin_call.rs · planner.rs · openapi.rs
    │       ├── lib.rs                #   ce que le module expose : routes, travail, planner_routes
    │       └── tests/                #   tests d'intégration sur base réelle
    ├── api/                          # + monte les routes d'event
    │                                 # + compose le scope /admin/planner UNE SEULE FOIS (R11)
    └── worker/                       # + enregistre la clôture automatique des appels
```

**Structure Decision** : la forme est celle qu'`identity` a inaugurée en B1 et qu'`org` a confirmée en B2 — domaine, dépôt, service, travaux, routes —, recopiée sans être réinventée. Une unité par **agrégat**, ce qui garde chaque fichier loin de la limite de mille lignes et permet de n'ouvrir que ce qui concerne la tâche.

**Deux inflexions propres à ce module**, et elles ont chacune leur raison :

- **`repo/cross.rs` réunit toutes les lectures hors schéma** — décomptes de séances et de dossiers, contrôle avant publication, images rattachées, thématiques d'un fil, noms de personnes. Les disperser dans les huit dépôts rendrait la frontière invisible ; réunies, elles se relisent en un fichier, et c'est là qu'un ajout se discute (R14).
- **`routes/planner.rs` est exposé séparément de `routes()`** — `lib.rs` publie `planner_routes(cfg)` en plus de `routes(cfg)`, pour qu'`api` compose le préfixe partagé une seule fois. C'est le patron de `people_routes` posé en B1, appliqué avant que le défaut ne se reproduise (R11).

**Trois endroits sont touchés hors du crate `event`** :

- **`backend/crates/kernel/error.rs`** — trois variantes au catalogue. C'est l'emplacement établi depuis B1 : le catalogue est central pour que l'OpenAPI l'engendre en entier.
- **`backend/crates/contracts/event.rs`** — les charges utiles des six événements de domaine, dont celle que le module Programmation consommera en B5.
- **`backend/crates/api/src/lib.rs`** — montage du module, et **composition du préfixe `/admin/planner`**.

Rien ne change côté `frontend/`. `NUXT_PUBLIC_API_BASE` reste vide jusqu'à B7. **Deux ajouts additifs au contrat** sont livrés côté API et ignorés par le front jusqu'au raccordement : le sigle proposé dans la réponse d'enregistrement d'une édition, et le refus de suppression d'un critère porteur de notes.

---

## Complexity Tracking

| Violation | Pourquoi elle est nécessaire | Alternative plus simple, et pourquoi elle est écartée |
|-----------|------------------------------|-------------------------------------------------------|
| **Principe VIII — le service refuse la suppression d'un critère porteur de notes** (`domain/call.rs`, R9) | `xmod_fk_review_scores_criterion` est `ON DELETE CASCADE` : supprimer un critère **détruit les notes qui s'y rapportent**, sans erreur et sans trace. Or `event.review_criteria` porte l'argumentaire d'une décision de sélection — c'est précisément ce que la v1 n'avait pas et qui rendait un refus inexplicable. Le contrat du front prévoit d'ailleurs `score_count` sur chaque critère : l'écran compte déjà sur cette information pour prévenir | *Laisser faire la base* : c'est perdre l'argumentaire d'une décision opposable, en silence. *Changer la clé en `RESTRICT`* : ce serait modifier le modèle, ce que le prompt interdit sans justification — et la cascade est **juste** quand c'est l'appel entier qui disparaît, ce qui est son cas d'usage d'origine. *Neutraliser silencieusement la suppression* : l'équipe croirait le critère retiré. Le service compte donc et refuse, en nommant le critère et le nombre de notes. **À reproposer comme correction du modèle si un second appelant apparaît** |

Aucune autre violation. **La règle du sigle n'en est pas une** : elle n'existe nulle part dans le modèle, elle ne redouble donc aucun invariant. Le modèle *pourrait* la porter — `CHECK (NOT has_pavilion OR acronym IS NOT NULL)` — et on s'en abstient **exprès**, parce que l'arbitrage retenu veut qu'une édition sans pavillon reste enregistrable sans sigle et que la reprise des données de la v1 en dépend (R1).

---

## Ce que ce plan ne tranche pas, volontairement

- **La granularité du calendrier d'une série de webinaires** (écart n° 2 d'A10). Sur l'édition d'un an du jeu de données, la génération proposerait **302 journées vides**. Aucune borne dure n'est codée : le plan annonce le nombre **avant** d'écrire, et rien ne s'écrit sans geste explicite. Le choix entre « ne générer que pour les séries de genre COP » et « une série de webinaires s'en passe » appartient au commanditaire.
- **Les permissions du rôle de programmation** (écart n° 88). Ce rôle ne détient aucune permission de ce module : un chargé de programmation ne peut ni créer un fil, ni déclarer une salle. Le corriger serait modifier le semis, donc le modèle.
- **Le rattachement des images d'une édition.** Il appartient au module Média (B6). Ce module **lit** les trois déclinaisons résolues et **accepte sans les poser** les identifiants d'objet que le formulaire envoie (R17). Inscrit comme obligation de B6.
- **Le rappel d'échéance aux organisations.** Les règles de rappel et les modèles de message vivent dans le module Engagement (B6) ; ce jalon ne livre que la **clôture automatique** d'un appel échu (R15).
- **Les inscriptions, les séances et leur planification.** Elles sont à B5. Ce module leur pose le décor — journées, salles, canaux, fils — et le contrôle qui les rendra publiques.
- **La suppression d'une édition, la dé-publication d'une programmation et l'écriture d'une série.** Aucun écran ne les offre ; les ajouter « pour être complet » créerait trois surfaces sans usage, dont l'une cascade sur six tables.

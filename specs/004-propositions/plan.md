# Implementation Plan: Propositions (B4)

**Branch**: `004-propositions` | **Date**: 2026-08-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-propositions/spec.md`

---

## Summary

Poser le quatrième module métier — `backend/crates/modules/programme` — et rendre possible ce que B3 a rendu ouvrable : **recevoir les dossiers de la COP31, les évaluer, et décider**. C'est le module le plus dense du jalon : cinq écrans du front en dépendent, et onze tables composent sa fiche d'évaluation.

L'approche tient en sept points, tous déduits du modèle ou du contrat du front, jamais choisis librement :

1. **Le service n'émet aucun événement de changement d'état, parce que le déclencheur les émet déjà.** C'est l'inverse de B3, et le retour du piège de B1 et B2. Émettre à son tour produirait deux courriels par transition, visible seulement en production (R2, écart n° 93).
2. **L'adresse d'URL d'un dossier est dérivée par le service**, avec repli quand le titre est vide et suffixe sur collision. La colonne est obligatoire et unique par édition, et le contrat du formulaire ne la porte pas : sans cela, **le tout premier enregistrement d'un brouillon échoue** (R5, écart n° 95).
3. **La consolidation des notes est appelée par le service**, dans la transaction du dépôt d'une revue. Aucun déclencheur ne l'appelle, et sans appel explicite le classement du comité est faux **sans qu'aucune erreur ne le signale** (R10, écart n° 98).
4. **Le voile de l'aveugle n'est pas un filtre : ce qui est masqué n'est pas lu.** Lire puis vider laisse la donnée à portée d'un champ oublié ; ne pas lire supprime la classe entière de défauts (R4).
5. **La recevabilité est classée avant l'écriture**, parce que le contrat attend deux réponses **portant des valeurs** — l'échéance, le plafond — que le déclencheur n'expose que dans une phrase française. Seule entorse au principe VIII, justifiée plus bas (R9).
6. **Deux préfixes de route sont déjà pris**, et l'un doit être refactorisé : `/organizations` devient composé dans `api` comme `/people` l'est depuis B1. Deux `web::scope` du même préfixe ne se complètent pas, et le défaut a déjà coûté trois routes muettes en B2 (R18).
7. **Le chemin que le prompt proposait pour les transitions offertes est déjà celui du journal.** Le journal le garde ; les transitions offertes prennent `available-transitions`, et le champ de la fiche (R19, écart n° 101).

---

## Technical Context

**Language/Version**: Rust stable, édition 2021, chaîne épinglée par `backend/rust-toolchain.toml` — inchangée depuis B1.

**Primary Dependencies**: **une dépendance nouvelle, et une seule** — `ammonia`, pour l'assainissement du HTML de la présentation détaillée (R14, écart n° 32). Elle n'est ni un framework, ni un runtime, ni une couche d'accès : une fonction pure sur une chaîne, fondée sur un analyseur conforme. Décision consignée dans `docs/progression/decisions/2026-08-20.md`, comme la constitution l'exige. Tout le reste est déjà déclaré par le workspace. **Aucun SQL composé dynamiquement** : comme en B3, toutes les requêtes passent la vérification à la compilation.

**Storage**: PostgreSQL 17 + pgvector — **schéma existant, non modifié**. Valkey toujours inutilisé (R23). Garage hors périmètre : ce module **rattache** un objet déjà stocké, il n'en dépose aucun.

**Testing**: `cargo test` sur base **réelle et jetable**, harnais `kernel::testing` de B1. Aucun mock. Le semis ne fournit **aucun dossier** : chaque test enchaîne édition, appel, grille, organisation et personne — le premier parcours de bout en bout du jalon (R22).

**Target Platform**: serveur Linux ; développement macOS et Linux via `ops/docker-compose.dev.yml`.

**Project Type**: service web (API) adossé à un front Nuxt existant. `api` monte les routes ; **`worker` n'est pas modifié** — ce module ne déclare aucun travail différé (R20), premier du jalon dans ce cas, et c'est vérifié plutôt que constaté.

**Performance Goals**: **aucune cible chiffrée, et c'est un fait, pas un oubli.** Rien ici n'est une recherche interactive. Deux points méritent d'être surveillés : la **fiche d'évaluation** (onze lectures), traitée par une transaction sur **une seule connexion** — la leçon de B2, où une transaction perdante retenait deux connexions du pool ; et la **déduction v1**, seule opération à volumétrie réelle, qui tourne une fois sur quelques milliers de dossiers.

**Constraints**: `DATABASE_URL` renseignée et base démarrée pour compiler (SQLx) · aucun fichier de `backend/` au-dessus de 1000 lignes — **c'est le module où la limite se rapproche le plus** · aucun avertissement Clippy · aucune arête entre deux crates de module · les chemins et noms de champs sont **exactement** ceux que `composables/api/proposals.ts`, `proposal-review.ts` et `organization-workspace.ts` consomment déjà · **aucune modification de `docs/database/`**, ce que ce plan ne propose nulle part.

**Scale/Scope**: **37 routes HTTP**, 1 crate créé, **0 travail différé**, **3 événements de domaine émis par le service** (huit autres le sont par la base), **6 codes d'erreur** ajoutés au catalogue du noyau, **1 refactorisation de préfixe** touchant du code livré, **3 champs additifs** au contrat du front. Volumétrie de référence : quelques dizaines à quelques centaines de dossiers par édition, trois revues par dossier, six critères par appel.

---

## Constitution Check

*GATE : à passer avant la phase 0, à repasser après la phase 1.*

| # | Principe | Évaluation avant conception | Évaluation après conception |
|---|----------|------------------------------|------------------------------|
| I | Le modèle de données fait autorité | ✅ `070_programme_proposals.sql` relu intégralement en B4-specify, plus `060` § 5 à 7, `020` § 4, `030` § 6, `050` § 8, `010`, `910` § 6.3. **Aucune modification proposée** | ✅ La conception a **plié devant le modèle** cinq fois : le service n'émet rien parce que le déclencheur émet (R2) ; l'adresse d'URL est dérivée parce que la colonne est obligatoire et que le contrat ne la porte pas (R5) ; la consolidation est appelée parce qu'aucun déclencheur ne l'appelle (R10) ; l'heure murale se convertit **en base** parce que la base de fuseaux de PostgreSQL fait foi (R6) ; et la création pose « brouillon » parce que le garde ne couvre pas l'insertion (R5, écart n° 96) |
| II | Frontières de modules | ✅ Un crate, dépendant du noyau et des contrats, de rien d'autre. **Un crate pour tout le schéma `programme`**, dont B5 remplira l'autre moitié (R1) | ✅ Tenu. **Deux écritures hors schéma**, toutes deux bornées et isolées : les thématiques, qui n'ont **aucune autre porte** (R11, précédent B3) ; la création d'une personne inconnue, qui suit un **précédent livré** — le module Organisations le fait déjà pour une invitation (R12). Justifiées en « Complexity Tracking ». `cargo tree -p programme` doit ne montrer aucune arête |
| III | Frontières vérifiables en base (`xmod_fk_*`) | ✅ Aucune clé étrangère créée — aucun DDL | ✅ `make check-db` reste au vert sans rien changer |
| IV | Effets de bord par l'outbox transactionnel | ✅ `platform.emit_event()` dans la transaction, jamais d'insertion à la main | ✅ Tenu, **et c'est ici que la vérification a payé** : le déclencheur émet déjà les huit changements d'état (écart n° 93). Le service n'en émet aucun, et **trois** que la base n'annonce pas, chacun avec son consommateur identifié en B6. Un événement **par dossier** dans une action groupée, jamais un pour le lot. Un test compte **une** ligne d'outbox par transition, pas deux |
| V | Autorisation par permission et par portée | ✅ Cinq permissions du modèle, testées séparément par le garde du noyau | ✅ Trois décisions : toute route paramétrée **remonte à l'édition avant** de vérifier, sur trois niveaux — commentaire, dossier, édition (R13) ; l'action groupée évalue **dossier par dossier**, une sélection pouvant traverser deux éditions ; et l'espace organisation est borné par l'**adhésion active**, pas par le périmètre — une organisation n'administre rien |
| VI | SQLx vérifié à la compilation, pas d'ORM | ✅ Macros `query!`/`query_as!` | ✅ **Aucune exception.** Les quatre traversées de type sont celles de B1 à B3, héritées et non réinventées : énumération en `text`, `i18n_text` en `jsonb`, `numeric` en `float8`, domaines par double transtypage (R24, data-model § 6) |
| VII | Contexte d'écriture systématique | ✅ Unique porte d'écriture du noyau | ✅ Tenu, avec **une conséquence assumée** : la fiche d'évaluation **écrit** — elle pose l'accusé de lecture — et passe donc par la porte d'écriture, là où B3 avait pu se contenter d'une transaction en lecture seule (R3). Une lecture qui écrit est assumée par le modèle ; on ne la déguise pas en deux appels dont l'un serait hors contexte |
| VIII | Les invariants de la base ne sont pas réimplémentés | ✅ Quinze contraintes nommées identifiées, aucune recopiée | ⚠️ **Une entorse, justifiée en « Complexity Tracking »** : le classement des trois refus de recevabilité avant l'écriture (R9), parce que le contrat attend des réponses **portant des valeurs**. **Douze autres règles vivent dans le service et n'en sont pas** : la base ne les porte pas du tout (bornes d'intervenants, longueurs, assainissement, purge, consolidation, voile, filtrage des visibilités…) — data-model § 3 les liste avec leur écart |
| IX | Erreurs d'API : code stable, message français | ✅ Type d'erreur unique du noyau | ✅ **Six codes ajoutés**, et pas plus : sept refus métier sont déjà des membres d'union du contrat et sortent en **200**. Les deux codes d'erreur PostgreSQL du garde d'état sont distingués **par le moment, jamais par le texte** du message (R8) |
| X | Tests d'intégration sur base réelle | ✅ Harnais de B1 | ✅ Les quatre obligations ont chacune leur test nommé, plus **un test qui frappe les 37 routes** — d'autant plus nécessaire que deux préfixes sont partagés (R18) —, et **deux tests par balayage de charge utile** : le voile et l'espace organisation. Un test d'écran ne prouve rien sur ce qui sort de l'API |

**Verdict** : une entorse, justifiée ci-dessous ; deux écritures hors schéma, bornées et justifiées ; aucune autre.

Cinq points ont été tranchés dans `research.md` plutôt que laissés à l'implémentation, parce qu'une décision tacite y aurait produit une faute **à l'exécution** : la double émission d'événements (R2), qui enverrait tout en double ; la dérivation de l'adresse d'URL (R5), sans laquelle le premier enregistrement échoue ; l'appel à la consolidation (R10), sans lequel le classement est faux en silence ; la distinction des deux codes d'erreur du garde (R8), qui se ferait sinon au texte ; et la composition du préfixe `/organizations` (R18), sans laquelle deux routes seraient muettes.

---

## Project Structure

### Documentation (this feature)

```text
specs/004-propositions/
├── spec.md              # Ce qu'il faut faire (/speckit-specify)
├── plan.md              # Ce fichier
├── research.md          # Phase 0 — 24 décisions techniques et leurs alternatives
├── data-model.md        # Phase 1 — le modèle existant, et ce que le code en fait
├── contracts/
│   ├── routes.md        #   les 37 routes, leur autorisation, leur politique de statut
│   ├── errors.md        #   les 6 codes stables ajoutés et la traduction PostgreSQL
│   └── events.md        #   ce que la base émet, ce que le service émet, ce qui n'émet rien
├── quickstart.md        # Phase 1 — comment lancer, éprouver et vérifier
├── checklists/
│   └── requirements.md  # Contrôle qualité de la spécification
└── tasks.md             # Phase 2 — produit par /speckit-tasks, PAS par ce fichier
```

### Source Code (repository root)

```text
backend/
├── Cargo.toml                        # + un membre : crates/modules/programme
│                                     # + une dépendance : ammonia (R14)
└── crates/
    ├── kernel/
    │   └── error.rs                  # + 6 variantes au catalogue (PROPOSAL_*)
    ├── contracts/
    │   └── programme.rs              # NOUVEAU — charges utiles des 3 événements du service
    ├── modules/
    │   ├── identity/                 # INCHANGÉ
    │   ├── org/
    │   │   └── lib.rs                # MODIFIÉ — expose organization_routes() séparément (R18)
    │   ├── event/                    # INCHANGÉ
    │   └── programme/                # NOUVEAU — le schéma entier ; B5 y ajoutera les séances
    │       ├── domain/               #   types purs : aucune requête, tout testable seul
    │       │   ├── slug.rs            #     dérivation, repli, suffixe (R5)
    │       │   ├── limits.rs          #     les huit longueurs maximales (R15, écart n° 28)
    │       │   ├── sanitize.rs        #     la liste blanche HTML (R14, écart n° 32)
    │       │   ├── transitions.rs     #     ce que le service fait des règles LUES, jamais le graphe
    │       │   ├── eligibility.rs     #     le classement des trois refus (R9)
    │       │   ├── blind.rs           #     la règle du voile, trois conditions (R4)
    │       │   ├── draft.rs           #     la recomposition, structure et conversions (R6)
    │       │   ├── facets.rs          #     les sept facettes, comptées sur les lignes (R16)
    │       │   ├── permissions.rs     #     les cinq permissions consommées
    │       │   └── ids.rs
    │       ├── repo/                 #   requêtes SQLx, un fichier par agrégat
    │       │   ├── proposals.rs · organizations.rs · speakers.rs · documents.rs
    │       │   ├── transitions.rs · comments.rs · reads.rs
    │       │   ├── assignments.rs · reviews.rs · scores.rs
    │       │   ├── dashboard.rs       #     la vue de pilotage, telle quelle
    │       │   ├── themes.rs          #     ÉCRITURE hors schéma n° 1 — les thématiques (R11)
    │       │   ├── people.rs          #     ÉCRITURE hors schéma n° 2 — l'intervenant inconnu (R12)
    │       │   └── cross.rs           #     LES SEULES lectures hors schéma, réunies ici (R13)
    │       ├── service/              #   les règles
    │       │   ├── draft_write.rs · submit.rs · resubmit.rs · draft_read.rs
    │       │   ├── transition.rs      #     tenter, traduire, ne jamais rejouer le graphe (R7, R8)
    │       │   ├── list.rs            #     la liste et ses facettes en une lecture (R16)
    │       │   ├── desk.rs            #     la composition des onze tables, et le voile (R3, R4)
    │       │   ├── review.rs          #     notation, consolidation appelée ici (R10)
    │       │   ├── comments.rs        #     les trois visibilités, filtrées à la source
    │       │   ├── workspace.rs       #     la composition propre au soumissionnaire (écart n° 8)
    │       │   ├── documents.rs
    │       │   └── backfill.rs        #     la déduction des transitions v1 (R20, écart n° 37)
    │       ├── routes/               #   handlers, DTO, annotations OpenAPI
    │       │   ├── submission.rs · workspace.rs · admin_list.rs · admin_desk.rs
    │       │   ├── detail.rs · people.rs · admin_ops.rs · openapi.rs
    │       ├── lib.rs                #   routes(), people_routes(), organization_routes()
    │       └── tests/                #   tests d'intégration sur base réelle
    ├── api/
    │   └── lib.rs                    # MODIFIÉ — monte programme ; compose /organizations
    │                                 #   une seule fois, comme /people (R18)
    └── worker/                       # INCHANGÉ — aucun travail différé (R20)
```

**Structure Decision** : la forme est celle qu'`identity` a inaugurée en B1, qu'`org` a confirmée en B2 et qu'`event` a reprise en B3 — domaine, dépôt, service, routes —, recopiée sans être réinventée. Une unité par **agrégat**, ce qui garde chaque fichier loin de la limite de mille lignes.

**Trois inflexions propres à ce module**, chacune avec sa raison :

- **`domain/` est nettement plus fourni que dans les trois modules précédents.** Dix fichiers, parce que ce module porte dix règles que la base ne porte pas (data-model § 3). Les mettre dans le service les rendrait intestables sans base ; dans le domaine, chacune se prouve seule.
- **Deux fichiers d'écriture hors schéma, séparés de `cross.rs`.** B3 réunissait ses lectures hors schéma dans un fichier ; ici il y a aussi deux **écritures**, et les mélanger avec les lectures effacerait la distinction qui compte — lire hors de son schéma est ordinaire, y écrire est une dérogation. Trois fichiers, trois régimes.
- **`lib.rs` expose trois configurateurs** — les routes propres, celles de `/people`, celles de `/organizations` —, pour que `api` compose les deux préfixes partagés une seule fois (R18).

**Quatre endroits sont touchés hors du crate `programme`** :

- **`backend/crates/kernel/src/error.rs`** — six variantes au catalogue. Emplacement établi depuis B1 : le catalogue est central pour que l'OpenAPI l'engendre en entier.
- **`backend/crates/contracts/programme.rs`** — les charges utiles des trois événements du service, que B6 consommera.
- **`backend/crates/modules/org/src/lib.rs`** — **du code livré est modifié** : les routes de `/organizations` sont exposées par une fonction distincte, sans qu'aucune route ne change de chemin.
- **`backend/crates/api/src/lib.rs`** — montage du module, et **composition du préfixe `/organizations`**.

Rien ne change côté `frontend/`. `NUXT_PUBLIC_API_BASE` reste vide jusqu'à B7. **Trois ajouts additifs au contrat** sont livrés côté API et ignorés par le front jusqu'au raccordement : le champ `available_transitions` de la fiche, la route des transitions offertes, et les deux routes de pièces jointes.

---

## Complexity Tracking

| Violation | Pourquoi elle est nécessaire | Alternative plus simple, et pourquoi elle est écartée |
|-----------|------------------------------|-------------------------------------------------------|
| **Principe VIII — le service classe les trois refus de recevabilité avant de tenter le dépôt** (`domain/eligibility.rs`, R9) | Le contrat du front n'attend pas une erreur mais **deux réponses portant des valeurs** : l'échéance pour un appel clos, le plafond pour un quota atteint. Le déclencheur ne les rend que dans une phrase française interpolée. Et le même code d'erreur PostgreSQL — `restrict_violation` — sert à la fois aux transitions interdites et aux trois refus de recevabilité : sans classement préalable, on ne saurait même pas laquelle des quatre causes s'applique. Le classement est **borné** — trois conditions lues, dans la même transaction que l'écriture qu'elles précèdent — et le déclencheur n'est ni désactivé ni contourné : une course retombe sur lui | *Laisser passer et traduire l'exception* : l'écran afficherait « le dépôt a été refusé » sans dire quand l'appel a fermé, ce dont l'organisation a précisément besoin. *Reconnaître la cause au texte du message* : trois messages français, dont deux interpolent des valeurs, qui changent à la première reformulation du SQL — la dépendance la plus fragile qu'on puisse écrire. *Ajouter un code d'erreur par cause dans le SQL* : ce serait modifier le modèle, ce que le prompt interdit sans justification |
| **Principe II — le module écrit dans `reference.entity_terms`** (`repo/themes.rs`, R11) | La table est polymorphe et **sans clé étrangère** vers les propositions : aucun autre module ne peut poser les thématiques d'un dossier, et aucune contrainte référentielle ne les purge. C'est la dérogation que B3 s'est déjà accordée pour les fils de programmation, bornée de la même façon | *Un contrat d'événement vers un module « référentiel »* : ce module n'existe pas — `reference` est un schéma du noyau partagé, sans crate. *Laisser le front écrire* : c'est l'écart n° 3 dans son intégralité, un client pouvant alors rattacher des termes à n'importe quelle entité de n'importe quel schéma |
| **Principe II — le module crée une personne inconnue dans `identity.people`** (`repo/people.rs`, R12) | La clé de l'intervenant est obligatoire et le contrat exige une **réponse synchrone** portant la personne : le formulaire l'affiche, la rattache et détecte le doublon au clavier suivant. Le précédent est **livré** — le module Organisations crée déjà la personne visée par une invitation dont l'adresse est inconnue. L'écriture est bornée : adresse, prénom, nom saisis ; jamais de compte, jamais de rôle | *Un contrat d'événement consommé par le module Identité* : création différée, réponse sans identifiant, doublon indétectable au moment où le déposant est encore devant son écran. *Refuser un intervenant inconnu* : c'est refuser la moitié des dossiers — un expert invité n'a pas de compte sur la plateforme |

Aucune autre violation. **Les douze autres règles portées par le service n'en sont pas** : la base ne les porte pas du tout, il n'y a donc rien à redoubler. Elles sont listées avec leur écart en `data-model.md` § 3.

---

## Ce que ce plan ne tranche pas, volontairement

- **La question n° 8 des points bloqués**, posée le 16/08 et jamais tranchée : le déposant voit-il sa note et son rang ? L'option A est tenue depuis A5 — non. Trancher autrement n'ouvrirait que deux champs de la composition de l'espace organisation.
- **La question de l'écart n° 35** : une résolution posée par le déposant vaut-elle clôture pour le comité, ou déclaration ? La déclaration est tenue, le comité gardant la faculté de retirer. Y répondre ne changera qu'une permission.
- **Le contact du dossier** (écart n° 30). Le déposant par défaut, règle explicite ; le demander à l'étape des organisations est un geste d'écran, pas d'API.
- **La confirmation d'un intervenant par lui-même.** Le modèle porte la colonne et la finalité de jeton ; aucun écran ne l'offre, le prompt ne la demande pas. La poser « pour être complet » créerait une surface sans usage, et un courriel que personne n'a décidé d'envoyer.
- **Le français obligatoire** (écart n° 29). C'est un arbitrage du commanditaire sur ce que la plateforme demande à une organisation anglophone, pas une ligne de code.
- **Le téléversement des pièces et des photos.** Il appartient à B6 ; ce module rattache un objet déjà stocké.
- **Les séances, leurs décomptes et leurs rappels.** Ils appartiennent à B5 et B6. Ce module les **nomme** dans la composition de l'espace organisation, avec le contrat que le front déclare déjà.
- **La pagination et le tri serveur de la liste** (R17). Le contrat du front trie et filtre côté écran, et le prompt B7 les basculera. Les livrer maintenant produirait une surface que personne n'appelle.
- **Le corps de la reprise de la v1.** La déduction des transitions est livrée ici parce qu'elle n'a pas d'autre foyer (écart n° 100) ; le reste de la migration des activités reste à écrire, ailleurs, le jour où la bascule se prépare.

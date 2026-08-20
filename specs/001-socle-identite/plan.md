# Implementation Plan: Socle technique et Identité (B1)

**Branch**: `001-socle-identite` | **Date**: 2026-08-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-socle-identite/spec.md`

---

## Summary

Créer l'arborescence `backend/` que les cinq modules suivants réutiliseront — noyau technique, contrats d'événements, binaire HTTP, binaire des travaux différés — et y poser le premier module métier : `identity`.

L'approche tient en cinq points, tous déduits du modèle plutôt que choisis librement :

1. **Le jeton d'accès est signé, pas stocké.** `identity.sessions` ne porte qu'une empreinte de jeton de **rafraîchissement** : il n'existe aucune colonne où loger un jeton d'accès opaque. L'accès est donc un jeton signé court, qui ne transporte **aucune permission** — seulement un renvoi vers la session. Les permissions se relisent en base à chaque requête, ce qui rend la révocation immédiate (FR-033).
2. **Le mot de passe se vérifie toujours, même quand l'adresse est inconnue**, contre une empreinte factice calculée au démarrage. C'est la seule façon de tenir FR-020 et SC-001.
3. **Le jeton en clair ne franchit jamais l'outbox.** Il vit dans la charge utile d'un travail différé, effacée dès l'envoi. L'événement de domaine, lui, ne porte aucun secret.
   **L'API n'émet aucun courriel elle-même** : son serveur n'en a pas le droit. Elle compose le message et le remet au serveur du site, qui seul dispose du SMTP. Le noyau expose un *contrat* d'envoi, pas un client — le jour où l'autorisation vient, on change d'implémentation et aucun module ne bouge.
4. **Les erreurs de la base sont traduites, jamais anticipées** : une table de correspondance `(SQLSTATE, nom de contrainte) → (code stable, message français, champ)`, plus le passage direct des messages français déjà écrits par les triggers du modèle.
5. **Les tests tournent sur une base réelle et jetable**, produite par recopie d'une base modèle chargée une fois depuis `docs/database/`.

---

## Technical Context

**Language/Version**: Rust stable, édition 2021, `rust-toolchain.toml` épinglé sur **1.97.1** — SQLx 0.9 exige au moins 1.94, et l'épinglage sert surtout à ce que `cargo fmt --check` et `cargo clippy -- -D warnings` rendent le même verdict d'un poste à l'autre

**Primary Dependencies**: Actix Web 4 · **SQLx 0.9** (PostgreSQL, macros vérifiées à la compilation ; 0.8 était annoncée, mais `sqlx-cli` du poste est en 0.9 et `cargo sqlx prepare` refuse une version de crate différente) · `argon2` (Argon2id) · `ed25519-dalek` (signature du jeton d'accès — le JWT `EdDSA` est composé dans le module, pour que la clé se lise en 32 octets sur une ligne de `.env` plutôt qu'en bloc PEM ; écart n° 21) · `reqwest` (remise du courriel au serveur du site) · `utoipa` + `utoipa-actix-web` (OpenAPI généré) · `figment` (configuration) · `tracing` + `opentelemetry-otlp` · `serde`, `uuid`, `time`, `rand`, `sha2`. La caractéristique `ipnetwork` de SQLx est activée pour la seule colonne `inet` du modèle — sans elle, l'adresse d'origine d'une session traverserait en texte et c'est PostgreSQL qui refuserait une valeur mal formée, donc un 500 là où il faut un 422

Côté site, pour la seule route de relais : `nodemailer`. **Aucune bibliothèque SMTP côté Rust** — le serveur de l'API n'a pas le droit d'émettre.

**Storage**: PostgreSQL 17 + pgvector — **schéma existant, non modifié**, chargé depuis `docs/database/` (19 fichiers). Valkey **non utilisé par ce module** (voir research.md § R12). Garage hors périmètre (module `media`, B6).

**Testing**: `cargo test` sur base **réelle et jetable** — base modèle chargée une fois depuis `docs/database/`, recopiée par test (`CREATE DATABASE … TEMPLATE …`). Aucun double en mémoire, aucun mock de base (principe X).

**Target Platform**: serveur Linux ; développement macOS et Linux via `ops/docker-compose.dev.yml`

**Project Type**: service web (API) adossé à un front Nuxt existant. Deux binaires : `api` (HTTP) et `worker` (relais d'outbox + file de travaux).

**Performance Goals**: aucune cible chiffrée imposée à ce module. Deux contraintes de forme, elles, sont dures : l'écart de temps entre « adresse inconnue » et « mot de passe faux » reste sous 10 % (SC-001), et le coût Argon2id est calibré autour de **50 à 100 ms** par vérification — au-delà, la connexion devient perceptiblement lente ; en deçà, le hachage cesse de protéger.

**Constraints**: `DATABASE_URL` renseignée et base démarrée pour compiler (SQLx) · aucun fichier de `backend/` au-dessus de 1000 lignes · aucun avertissement Clippy · préfixe d'API `/api`, prescrit par `.env.example` pour le raccordement du site (la clé y reste **vide** jusqu'à B7) · les noms de champs et les chemins sont ceux que `frontend/app/composables/useApi.ts` consomme déjà · **l'API et le site sont sur deux serveurs distincts, et seul celui du site peut émettre du courriel** (contrainte d'hébergement énoncée le 20/08) : l'API doit pouvoir joindre le site en HTTP

**Scale/Scope**: ~30 routes HTTP, 5 crates créés, 1 module métier. Volumétrie attendue de la COP31 : quelques milliers de personnes, quelques centaines de connexions par jour. Aucun besoin de mise à l'échelle horizontale dans ce jalon — mais le relais d'outbox réserve déjà ses lignes sans se bloquer, donc plusieurs instances ne se marchent pas dessus.

---

## Constitution Check

*GATE: à passer avant la phase 0, à repasser après la phase 1.*

| # | Principe | Évaluation avant conception | Évaluation après conception |
|---|----------|------------------------------|------------------------------|
| I | Le modèle de données fait autorité | ✅ `030_identity.sql` et `010_platform.sql` relus intégralement en B1-specify. **Aucune modification proposée.** Chaque nom cité vient du fichier | ✅ La conception a **plié devant le modèle** au moins une fois : l'absence de colonne pour un jeton d'accès a décidé de sa forme (R1), et non l'inverse |
| II | Frontières de modules | ✅ Les cinq emplacements de la constitution sont créés tels quels | ✅ `identity` ne dépend que de `kernel` et `contracts`. `api` et `worker` dépendent d'`identity` ; l'inverse n'existe pas. Vérifiable par `cargo tree`. **Le garde d'autorisation est placé dans `kernel` (R16)** précisément pour que B2 n'ait pas à dépendre d'`identity` — l'arête interdite est évitée à la conception, pas à la revue |
| III | Frontières vérifiables en base (`xmod_fk_*`) | ✅ Aucune clé étrangère créée — aucun DDL dans ce module | ✅ `make check-db` reste au vert sans rien changer |
| IV | Effets de bord par l'outbox transactionnel | ✅ `platform.emit_event()` appelée, jamais d'INSERT à la main. Garde d'idempotence par `platform.inbox_events` | ✅ Une **précision** : le courriel n'est pas un effet inter-module dans ce jalon (`engagement` arrive en B6), il passe donc par `platform.jobs`, ce que la constitution prévoit explicitement pour un travail différé. L'événement de domaine est émis **en plus**, et ne porte aucun secret (R8). La remise au serveur du site (R13) ne change rien à cela : elle est le dernier maillon d'un travail de la file, avec sa reprise d'essai et sa file morte |
| V | Autorisation par permission et par portée | ✅ `identity.has_permission()` et `identity.administered_events()` sont les seuls points de décision | ✅ **Tenu.** Un point de décision unique, paramétré par permission et par portée. L'extracteur `Requires<P>` couvre la portée **globale** ; une portée **ciblée**, qui dépend du chemin, se vérifie dans le gestionnaire par `require_permission(…, Scope::Event(id))` — arbitrage assumé, non contrainte technique (`match_info()` serait accessible à un extracteur, au prix de décider la forme d'une erreur loin du gestionnaire qui la rend). Aucun test de nom de rôle nulle part |
| VI | SQLx vérifié à la compilation, pas d'ORM | ✅ Macros `query!`/`query_as!` uniquement | ✅ `.sqlx/` versionné pour que l'éditeur et une future intégration continue construisent sans base (R4) |
| VII | Contexte d'écriture systématique | ✅ `SET LOCAL app.actor_id` / `app.request_id` en tête de transaction | ✅ **Une seule porte, qui les pose elle-même** (R14). Précision d'implémentation : c'est une discipline, pas une clôture — le pool reste accessible, l'écoute `LISTEN/NOTIFY` et le harnais de test en exigent un vrai. **Aucune écriture du jalon ne passe à côté**, y compris celles du worker sur des tables non auditées et celles du compteur d'échecs sur le chemin de la connexion. Ces dernières ont failli y échapper — quatre allers-retours au lieu d'un, et l'écart se mesure dans le temps de réponse (SC-001). La réponse n'a pas été de renoncer à la garantie mais de **lancer l'écriture avant d'attendre le hachage** : deux millisecondes se replient sans trace derrière une dizaine (écart n° 27) |
| VIII | Les invariants de la base ne sont pas réimplémentés | ✅ Onze contraintes du modèle identifiées, aucune recopiée en Rust | ✅ Table de correspondance dans `contracts/errors.md`. Les messages français déjà écrits par les triggers du modèle sont **repris tels quels**, pas réécrits |
| IX | Erreurs d'API : code stable, message français | ✅ Type d'erreur unique dans le noyau | ✅ Catalogue de codes stables dans `contracts/errors.md`, chacun documenté dans l'OpenAPI généré |
| X | Tests d'intégration sur base réelle | ✅ Base jetable chargée depuis `docs/database/` | ✅ Les quatre obligations minimales de la constitution ont chacune leur test nommé dans `quickstart.md` |

**Verdict** : aucune violation. La section « Complexity Tracking » reste vide.

Trois points ont été pesés et tranchés dans `research.md` plutôt que d'être laissés à l'implémentation, parce qu'une décision tacite y aurait produit une entorse : la forme du jeton d'accès (R1), le chemin du jeton en clair vers le courriel (R8), et **la place du garde d'autorisation (R16)** — celui-ci décidé au découpage des tâches, et qui conditionne la conformité de B2 à B6 autant que celle de B1.

---

## Project Structure

### Documentation (this feature)

```text
specs/001-socle-identite/
├── spec.md              # Ce qu'il faut faire (/speckit-specify)
├── plan.md              # Ce fichier
├── research.md          # Phase 0 — les 16 décisions techniques et leurs alternatives
├── data-model.md        # Phase 1 — le modèle existant, et ce que le code en fait
├── contracts/
│   ├── routes.md        #   les routes, leur autorisation, leur politique de statut HTTP
│   ├── errors.md        #   le catalogue de codes stables et la traduction PostgreSQL
│   └── events.md        #   les événements de domaine émis et les travaux différés
├── quickstart.md        # Phase 1 — comment lancer, éprouver et vérifier
├── checklists/
│   └── requirements.md  # Contrôle qualité de la spécification
└── tasks.md             # Phase 2 — produit par /speckit-tasks, PAS par ce fichier
```

### Source Code (repository root)

```text
backend/                          # workspace Cargo — symétrique de frontend/
├── Cargo.toml                    # membres du workspace, dépendances communes
├── rust-toolchain.toml
├── .sqlx/                        # requêtes préparées, versionnées (R4)
└── crates/
    ├── kernel/                   # AUCUNE connaissance métier
    │   ├── config.rs             #   configuration typée, chargée de l'environnement
    │   ├── context.rs            #   contexte de requête : requête, acteur, locale
    │   ├── db.rs                 #   pool, et l'UNIQUE façon d'ouvrir une transaction en écriture
    │   ├── error.rs              #   type d'erreur unique : code stable + message français + champ
    │   ├── pg_error.rs           #   traduction (SQLSTATE, contrainte) → erreur d'API
    │   ├── i18n.rs               #   négociation de la locale, résolution des textes du modèle
    │   ├── crypto.rs             #   Argon2id, jetons aléatoires, empreintes
    │   ├── events.rs             #   émission, et registre des consommateurs
    │   ├── jobs.rs               #   réservation, échec, registre des travaux
    │   ├── mail.rs               #   CONTRAT d'envoi + remise au serveur du site (R13)
    │   ├── telemetry.rs          #   traces, journalisation, identifiant de requête
    │   ├── auth.rs               #   GARDE d'autorisation — dans le noyau (R16)
    │   └── testing.rs            #   harnais de base jetable, derrière `testing` (R15)
    ├── contracts/                # ce que les modules s'échangent, et RIEN d'autre
    │   └── identity.rs           #   charges utiles des événements `identity.*`
    ├── modules/
    │   └── identity/
    │       ├── domain/           #   types métier purs, testables sans base
    │       ├── repo/             #   requêtes SQLx, un fichier par agrégat
    │       ├── service/          #   les règles : connexion, jetons, sessions, RBAC, RGPD
    │       ├── jobs/             #   les travaux différés du module
    │       ├── routes/           #   handlers, DTO, annotations OpenAPI
    │       ├── lib.rs            #   ce que le module expose : ses routes, ses travaux, ses consommateurs
    │       └── tests/            #   tests d'intégration sur base réelle
    ├── api/                      # binaire HTTP — cargo run -p api
    └── worker/                   # relais d'outbox + file de travaux — cargo run -p worker
```

**Structure Decision** : l'arborescence est **imposée par le principe II de la constitution** et ne relève pas d'un choix. Le seul arbitrage restant est le découpage **interne** d'un crate de module, et il suit la règle du projet — une unité par **agrégat** — pour la même raison qu'au front : garder chaque fichier loin de la limite de 1000 lignes et permettre de n'ouvrir que ce qui concerne la tâche. Les cinq modules suivants recopieront cette forme sans la réinventer.

Trois endroits du dépôt sont touchés hors `backend/` :

- **`.env.example`** — les réglages d'exploitation des écarts n° 18 et 19 y sont déclarés (FR-014, FR-017), avec les clés de signature, l'adresse publique de composition des liens et les deux clés du relais de courriel. Les clés SMTP existantes **changent de lecteur** : elles étaient annoncées pour l'API, elles sont lues par le site.
- **`frontend/server/api/internal/mail.post.ts`** — la route privée qui reçoit un message déjà composé et l'envoie par SMTP. Quelques dizaines de lignes, aucune interface, **générique** : elle sert tous les courriels de la plateforme, pas seulement ceux de l'identité. C'est le seul morceau de B1 qui vit hors de `backend/`, et il y est parce que la contrainte d'hébergement l'y met. **Il est fait pour disparaître** le jour où le serveur de l'API obtient le droit d'émettre.
- **`Makefile`** — `check-back` existe déjà et devient actif dès que `backend/` apparaît. Aucune modification n'est prévue ; si elle s'avérait nécessaire, elle se consigne dans les décisions du jour.

---

## Complexity Tracking

Aucune violation de la constitution à justifier.

---

## Ce que ce plan ne tranche pas, volontairement

- **La limitation de débit sur les points d'entrée publics.** Le verrouillage de compte protège un compte donné, pas l'énumération distribuée sur des milliers d'adresses. Aucune exigence de la spécification ne la demande, Valkey reste donc inutilisé dans ce module. Le point est nommé dans `research.md` § R12 pour qu'il ne se redécouvre pas.
- **La rotation des clés de signature.** Une seule clé au démarrage dans ce jalon ; le format de jeton retenu (R1) accepte un identifiant de clé, ce qui laisse la rotation possible plus tard sans changer le contrat.
- **Le second facteur**, arbitré hors périmètre le 20/08.
- **La date de reprise de l'envoi de courriel par l'API elle-même.** Elle dépend d'une autorisation d'hébergement, pas du code. Le contrat d'envoi du noyau est écrit pour que ce jour-là ne coûte qu'une implémentation et une clé de configuration.

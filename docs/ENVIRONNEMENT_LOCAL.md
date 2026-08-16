# Environnement local

Tout se développe en local avant tout déploiement sur VPS.

**Les fichiers existent désormais dans le dépôt** — `ops/docker-compose.dev.yml`, `ops/garage.toml`, `Makefile`, `.env.example`. Ce sont **eux** qui font foi ; les extraits reproduits ci-dessous expliquent les intentions et signalent les pièges, mais une divergence se tranche toujours en faveur du fichier.

Mise en route, machine neuve :

```bash
cp .env.example .env      # ajuster les ports si un autre projet les occupe
make up                   # démarre les cinq services et attend le chargement du schéma
make garage-init          # layout, bucket, clé S3 — recopier les deux clés dans .env
make check-db-safe        # contrôle que la base est conforme
```

---

## Les services

| Service | Rôle | Accès |
|---------|------|-------|
| PostgreSQL 17 + pgvector | Base de données ; le schéma se charge automatiquement au premier démarrage | `localhost:5432` |
| Valkey | Cache et sessions — utile seulement quand la mesure le justifiera | `localhost:6379` |
| Jaeger | Collecteur et visualisation des traces OpenTelemetry ; remplacé par Grafana/Tempo au déploiement | `http://localhost:16686` |
| Mailpit | Capture les courriels sans rien envoyer — indispensable pour tester les rappels sans polluer de vraies boîtes | `http://localhost:8025` |
| Garage | Stockage objet compatible S3 ; demande une initialisation manuelle avant toute écriture | `localhost:3900` (S3) · `localhost:3903` (admin) |

## `ops/docker-compose.dev.yml`

Extrait — le fichier réel publie chaque port sous la forme `${POSTGRES_PORT:-5432}:5432`, afin qu'un `.env` puisse le décaler sans modifier le compose. Les valeurs par défaut sont exactement celles indiquées dans le tableau ci-dessus.

```yaml
services:
  postgres:
    image: pgvector/pgvector:pg17
    container_name: epavillon-postgres
    environment:
      POSTGRES_PASSWORD: dev
      POSTGRES_DB: epavillon
    ports: ["${POSTGRES_PORT:-5432}:5432"]
    volumes:
      - pgdata:/var/lib/postgresql/data
      # Les 18 fichiers SQL sont exécutés dans l'ordre alphabétique au premier
      # démarrage — la numérotation 000 → 910 fait le travail.
      - ../docs/database:/docker-entrypoint-initdb.d:ro
    # pg_stat_statements est créé par 000_bootstrap.sql, mais l'extension reste
    # inexploitable tant que sa bibliothèque n'est pas préchargée au démarrage.
    command: ["postgres", "-c", "shared_preload_libraries=pg_stat_statements"]
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres -d epavillon"]
      interval: 5s
      timeout: 3s
      retries: 20

  valkey:
    image: valkey/valkey:8-alpine
    ports: ["6379:6379"]
    command: valkey-server --save 60 1

  jaeger:
    image: jaegertracing/all-in-one:1.60
    environment:
      COLLECTOR_OTLP_ENABLED: "true"
    ports:
      - "16686:16686"   # interface web
      - "4317:4317"     # OTLP gRPC
      - "4318:4318"     # OTLP HTTP

  mailpit:
    image: axllent/mailpit:latest
    ports:
      - "8025:8025"     # interface web
      - "1025:1025"     # SMTP

  garage:
    image: dxflrs/garage:v1.0.1
    container_name: epavillon-garage
    ports:
      - "3900:3900"     # API S3
      - "3901:3901"     # RPC — indispensable à `garage status` et au layout
      - "3902:3902"     # accès web aux objets publics
      - "3903:3903"     # API d'administration
    volumes:
      - garage-meta:/var/lib/garage/meta
      - garage-data:/var/lib/garage/data
      - ./garage.toml:/etc/garage.toml:ro

volumes:
  pgdata:
  garage-meta:
  garage-data:
```

**Point important** : le schéma n'est chargé qu'au **premier** démarrage du conteneur, quand le volume est vide. Après une modification d'un fichier SQL, il faut détruire le volume :

```bash
docker compose -f ops/docker-compose.dev.yml down -v
docker compose -f ops/docker-compose.dev.yml up -d
```

C'est aussi la façon la plus fiable de vérifier qu'une base neuve accepte le schéma complet. La ligne `command:` du service précharge `pg_stat_statements` : sans elle, l'extension créée par `000_bootstrap.sql` existe mais ne collecte rien.

### Le healthcheck passe au vert AVANT la fin du chargement

Piège mesuré au premier démarrage, et qui coûte cher parce qu'il ne ressemble pas à une erreur : `pg_isready` répond « accepting connections » au bout de deux secondes alors que les 18 fichiers SQL sont encore en cours d'exécution. Le script d'entrée de l'image démarre en effet un serveur temporaire, accessible par la socket locale, le temps de jouer les scripts d'initialisation. Un script qui enchaîne sur le healthcheck interroge donc une base à moitié construite et échoue sur des objets « inexistants » qui existeront une seconde plus tard.

La cible `wait-db` du `Makefile` attend deux conditions : conteneur sain **et** `legacy.id_map` présent — cette table est créée par `910_migration_v1.sql`, le dernier fichier de la série. Un `sleep` fixe ne vaut rien ici : le chargement prend de 15 à 40 secondes selon la machine.

Corollaire visible dans les journaux : le healthcheck qui frappe pendant l'arrêt du serveur temporaire y laisse un `FATAL: the database system is shutting down`. Il est **normal** ; `assert-init-logs` l'écarte explicitement, avec `starting up`.

### Si un port est déjà pris

Fréquent quand plusieurs projets tournent sur la même machine : le conteneur refuse alors de démarrer, ou pire, l'outil se connecte à la base d'un autre projet. Chaque port publié est paramétrable par le `.env` — `POSTGRES_PORT`, `S3_PORT`, `GARAGE_RPC_PORT`… la liste complète est en fin de `.env.example`.

Deux précautions :

- répercuter le décalage dans les URL du même fichier (`DATABASE_URL`, `S3_ENDPOINT`, `OTEL_EXPORTER_OTLP_ENDPOINT`) — rien ne les recalcule ;
- le `Makefile` passe `--env-file .env` à `docker compose` quand ce fichier existe. Sans lui, Compose ne lirait que le `.env` du dossier `ops/`, et les valeurs par défaut s'appliqueraient en silence.

## `ops/garage.toml`

Développement local, nœud unique, sans réplication. En production : au moins trois nœuds et `replication_factor = 3`.

```toml
metadata_dir = "/var/lib/garage/meta"
data_dir     = "/var/lib/garage/data"
db_engine    = "sqlite"

replication_factor = 1

rpc_bind_addr   = "[::]:3901"
rpc_public_addr = "127.0.0.1:3901"
# Remplacer par une valeur propre à la machine : openssl rand -hex 32
rpc_secret      = "0000000000000000000000000000000000000000000000000000000000000001"

[s3_api]
s3_region     = "garage"
api_bind_addr = "[::]:3900"
root_domain   = ".s3.garage.localhost"

[s3_web]
bind_addr   = "[::]:3902"
root_domain = ".web.garage.localhost"
index       = "index.html"

[admin]
api_bind_addr = "[::]:3903"
# Sans jeton, l'API d'administration refuse tous les appels.
# Remplacer par une valeur propre à la machine : openssl rand -base64 32
admin_token = "dev-admin-token-a-remplacer"
```

### Initialisation de Garage

**Un nœud Garage neuf refuse toute écriture tant que son layout n'est pas assigné.** Le conteneur démarre, l'API S3 répond, et chaque dépôt d'objet échoue — symptôme facile à prendre pour un problème d'identifiants. Ces quatre étapes ne se font qu'une fois, après le premier `up -d`, et sont à refaire après chaque `down -v` puisqu'il efface aussi les volumes `garage-meta` et `garage-data`.

**En pratique, une seule commande suffit : `make garage-init`.** Elle enchaîne les quatre étapes, relève l'identifiant du nœud et calcule seule le numéro de la nouvelle version du layout, puis affiche la clé d'accès et son secret — les deux valeurs à recopier dans `.env`. Le détail ci-dessous reste utile pour comprendre ce qu'elle fait et pour diagnostiquer.

Tout passe par `docker exec` : le binaire `garage` vit dans le conteneur, et ces commandes utilisent le port RPC 3901 — d'où sa publication dans le compose.

```bash
# 1. Relever l'identifiant du nœud : la longue chaîne hexadécimale en tête de ligne
docker exec epavillon-garage /garage status

# 2. Lui donner une place dans le layout — une zone, une capacité
docker exec epavillon-garage /garage layout assign -z dc1 -c 1G <node_id>
docker exec epavillon-garage /garage layout apply --version 1

# 3. Créer le bucket applicatif
docker exec epavillon-garage /garage bucket create epavillon

# 4. Créer une clé d'accès et l'autoriser en lecture / écriture sur le bucket
docker exec epavillon-garage /garage key create epavillon-dev
docker exec epavillon-garage /garage bucket allow --read --write epavillon --key epavillon-dev
```

Trois précisions qui évitent une demi-heure de tâtonnement :

- `<node_id>` est l'identifiant renvoyé par `garage status` ; ses premiers caractères suffisent tant qu'ils sont sans ambiguïté. **Il change à chaque `down -v`** : ne pas le figer dans un script.
- `--version 1` est le numéro de la **nouvelle** version du layout : 1 à la première application, puis 2, 3… à chaque modification. Se tromper de numéro donne `Error: Internal error: Invalid new layout version` — d'où le calcul automatique de `make garage-init`, qui lit « Current cluster layout version » et ajoute 1.
- `key create` affiche une **Key ID** et une **Secret key** — ce sont `S3_ACCESS_KEY_ID` et `S3_SECRET_ACCESS_KEY` du `.env`. Le secret ne s'affiche qu'une fois ; `garage key info --show-secret epavillon-dev` le retrouve.
- `garage bucket create` et `key create` échouent si l'objet existe déjà ; la cible du `Makefile` tolère cet échec (`|| true`) pour rester rejouable.

Contrôle final — le bucket doit apparaître avec sa permission de lecture/écriture (`make garage-info`) :

```bash
docker exec epavillon-garage /garage bucket info epavillon
```

Un contrôle plus concluant encore est un dépôt réel : tant que le layout n'est pas appliqué, l'API S3 répond mais toute écriture échoue, et `bucket info` seul ne le dit pas.

---

## Variables d'environnement

Un seul fichier `.env` à la racine du dépôt, dont `.env.example` est la copie versionnée et commentée — le second est le seul des deux que git suit (voir `.gitignore` plus bas).

**`DATABASE_URL` doit être renseignée et la base démarrée avant tout `cargo build`** : SQLx vérifie chaque requête *à la compilation* en interrogeant réellement le serveur PostgreSQL. Sans base joignable, le backend ne compile pas — ce n'est pas une erreur d'exécution mais une erreur de `cargo`, et `check-back` échoue au premier fichier.

```dotenv
# --- Base de données ---------------------------------------------------------
# Lue par SQLx À LA COMPILATION : la base doit tourner avant `cargo build`.
DATABASE_URL=postgres://postgres:dev@localhost:5432/epavillon

# --- API ---------------------------------------------------------------------
API_BIND_ADDR=127.0.0.1:8080
RUST_LOG=info,epavillon=debug

# --- Front (Nuxt) ------------------------------------------------------------
# Le préfixe NUXT_PUBLIC_ expose la variable au navigateur : rien d'autre ici.
NUXT_PUBLIC_API_BASE=http://localhost:8080/api

# --- Cache -------------------------------------------------------------------
VALKEY_URL=redis://localhost:6379

# --- Stockage objet (Garage, compatible S3) ----------------------------------
# Les deux clés proviennent de `garage key create` — voir « Initialisation de Garage ».
S3_ENDPOINT=http://localhost:3900
S3_REGION=garage
S3_BUCKET=epavillon
S3_ACCESS_KEY_ID=
S3_SECRET_ACCESS_KEY=
S3_FORCE_PATH_STYLE=true

# --- Courriel (Mailpit : capture tout, n'envoie rien) ------------------------
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_FROM=ne-pas-repondre@epavillon.local

# --- Traces (Jaeger, OTLP sur HTTP) ------------------------------------------
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
OTEL_SERVICE_NAME=epavillon-api
```

Le fichier réel se termine par un dernier bloc, `POSTGRES_PORT`, `S3_PORT`, `GARAGE_RPC_PORT`… — les ports publiés par le compose, à ne toucher qu'en cas de collision avec un autre projet (voir « Si un port est déjà pris »).

Le front n'a besoin que de `NUXT_PUBLIC_API_BASE` ; tout le reste est du côté serveur et ne doit jamais se retrouver dans le bundle. Mailpit n'exige ni authentification ni TLS en local : hôte et port suffisent.

---

## Vérifications avant commit

> *« Vu la contrainte de temps et vu que je suis seul, est-ce vraiment utile de faire de la CI maintenant ? »*

Non pour une chaîne d'intégration continue au sens habituel — pas de GitHub Actions, pas de matrice de tests, pas de déploiement automatisé. Seul et pressé, c'est du temps pris sur la livraison.

Oui pour **trois vérifications locales**. Elles coûtent une demi-heure à mettre en place et rattrapent exactement les erreurs qui font perdre une journée. Le jour où quelqu'un d'autre rejoint le projet, ce `Makefile` devient un fichier de CI en dix lignes.

Le fichier `Makefile` à la racine les porte. Ses cibles :

| Cible | Ce qu'elle fait |
|---|---|
| `make check` | les trois vérifications — **détruit la base**, voir plus bas |
| `make check-db` | recharge le schéma sur une base vierge, puis les assertions |
| `make check-db-safe` | les mêmes assertions, sans rien détruire |
| `make check-front` · `check-back` | typecheck + build Nuxt · fmt + clippy + tests Cargo |
| `make up` · `down` · `logs-db` | services locaux |
| `make garage-init` · `garage-info` | stockage objet |

Le cœur tient en quatre assertions, exécutées dans cet ordre :

1. **Les journaux d'initialisation ne contiennent ni `ERROR:` ni `FATAL:`** — une erreur pendant le chargement laisse une base incomplète sans empêcher le conteneur de tourner.
2. **Les 15 schémas attendus sont présents**, `legacy` compris : c'est le contrôle qu'aucun fichier n'a été sauté.
3. **`platform.cross_module_fk_report` ne contient aucune ligne non conforme** — les lignes fautives sont imprimées avant l'échec.
4. **`analytics.refresh_all(true)` ne laisse aucune ligne en échec dans `analytics.refresh_log`** — l'erreur de chaque vue est imprimée avant l'échec.

`check-front` et `check-back` sont inertes tant que `frontend/` et `backend/` n'existent pas : elles l'annoncent et rendent la main sans échouer, pour que `make check` reste utilisable dès aujourd'hui.

### Ce que ces assertions changent

Les deux compteurs ne sont plus **affichés**, ils sont **testés**. Auparavant, une clé étrangère inter-modules non conforme franchissait le portail en silence : la cible sortait avec le code 0, le `1` s'imprimait au milieu de la sortie, et personne ne le lisait. Un contrôle visuel humain n'est pas un contrôle — le `|| exit 1` fait toute la différence.

Une précision utile pour `check-db-safe` : sur une base qui n'est pas neuve, `analytics.refresh_log` conserve les échecs passés, et une ligne en échec garde donc la cible rouge tant qu'elle n'a pas été corrigée. C'est voulu — un rafraîchissement qui a échoué une fois mérite d'être regardé.

### `check-db` détruit la base — à savoir avant de lancer `make check`

> **`check-db` commence par `down -v`. Chaque `make check` supprime le volume `pgdata` et, avec lui, TOUTES les données saisies à la main depuis le dernier chargement du schéma.** Sans confirmation, sans sauvegarde.

C'est le prix de la seule vérification qui compte vraiment : prouver que le schéma complet se charge sur une base vierge. Mais [CLAUDE.md](../CLAUDE.md) fait de `make check` une condition de commit — il faut donc savoir qu'un commit fréquent signifie un jeu d'essai détruit tout aussi souvent.

D'où la cible `check-db-safe` : mêmes assertions, base en place. Le réflexe à prendre :

| Moment | Commande | Effet sur les données |
|---|---|---|
| Pendant le développement | `make check-db-safe` | aucune perte |
| Avant un commit important | `make check` | base recréée de zéro |

Un jeu d'essai qu'on tient à garder se rejoue depuis un script SQL versionné, jamais depuis la mémoire du volume Docker.

Ce que `down -v` emporte au passage, et qui surprend : **les volumes `garage-meta` et `garage-data`**. Le layout du nœud disparaît avec eux, et le stockage objet redevient muet — écritures refusées, identifiants d'accès invalides. `check-db` le rappelle en fin de course ; il faut relancer `make garage-init` et recopier la nouvelle clé dans `.env`.

---

## `.gitignore`

La règle qui compte : **`docs/database/` est du code source, pas une sauvegarde.** Une exclusion `*.sql` trop large mettrait tout le modèle de données hors du dépôt — c'est arrivé sur l'ancien projet. Seules les sauvegardes sont exclues : `*.dump`, `backup*`, `dumps/`.

Second point, moins évident : **une règle d'exclusion ne s'applique pas à un fichier déjà suivi**. Quatre `.DS_Store` étaient versionnés sous `docs/logos-IFDD-OIF/` ; ajouter `.DS_Store` au `.gitignore` ne les aurait pas fait disparaître. Ils ont été retirés de l'index par `git rm --cached` — le fichier reste sur le disque, git cesse simplement de le suivre.

```gitignore
node_modules/
target/
frontend/.nuxt/
frontend/.output/
dist/

.env
.env.*
!.env.example

# Sauvegardes de base de données — mais PAS docs/database/,
# qui est le modèle de données versionné.
*.dump
backup*
dumps/

.DS_Store
*.log
.idea/
.vscode/
coverage/
ops/data/
```

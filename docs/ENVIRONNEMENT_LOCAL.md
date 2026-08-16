# Environnement local

Tout se développe en local avant tout déploiement sur VPS. Ce document décrit ce qu'il faut mettre en place — les fichiers eux-mêmes seront créés au moment d'initialiser le projet.

---

## Les services

| Service | Rôle | Accès |
|---------|------|-------|
| PostgreSQL 17 + pgvector | Base de données ; le schéma se charge automatiquement au premier démarrage | `localhost:5432` |
| Valkey | Cache et sessions — utile seulement quand la mesure le justifiera | `localhost:6379` |
| Jaeger | Collecteur et visualisation des traces OpenTelemetry ; remplacé par Grafana/Tempo au déploiement | `http://localhost:16686` |
| Mailpit | Capture les courriels sans rien envoyer — indispensable pour tester les rappels sans polluer de vraies boîtes | `http://localhost:8025` |
| Garage | Stockage objet compatible S3 | `localhost:3900` |

## `ops/docker-compose.dev.yml`

```yaml
services:
  postgres:
    image: pgvector/pgvector:pg17
    container_name: epavillon-postgres
    environment:
      POSTGRES_PASSWORD: dev
      POSTGRES_DB: epavillon
    ports: ["5432:5432"]
    volumes:
      - pgdata:/var/lib/postgresql/data
      # Les 18 fichiers SQL sont exécutés dans l'ordre alphabétique au premier
      # démarrage — la numérotation 000 → 910 fait le travail.
      - ../docs/database:/docker-entrypoint-initdb.d:ro
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
    ports:
      - "3900:3900"     # API S3
      - "3902:3902"     # accès web aux objets publics
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

C'est aussi la façon la plus fiable de vérifier qu'une base neuve accepte le schéma complet.

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
```

---

## Vérifications avant commit

> *« Vu la contrainte de temps et vu que je suis seul, est-ce vraiment utile de faire de la CI maintenant ? »*

Non pour une chaîne d'intégration continue au sens habituel — pas de GitHub Actions, pas de matrice de tests, pas de déploiement automatisé. Seul et pressé, c'est du temps pris sur la livraison.

Oui pour **trois vérifications locales**. Elles coûtent une demi-heure à mettre en place et rattrapent exactement les erreurs qui font perdre une journée. Le jour où quelqu'un d'autre rejoint le projet, ce `Makefile` devient un fichier de CI en dix lignes.

```makefile
COMPOSE ?= docker compose -f ops/docker-compose.dev.yml
PSQL    ?= docker exec -i epavillon-postgres psql -U postgres -d epavillon

check: check-db check-front check-back

# LA vérification qui compte le plus : une migration qui ne passe pas sur une
# base vierge se découvre au déploiement, au pire moment.
check-db:
	$(COMPOSE) down -v && $(COMPOSE) up -d && sleep 12
	@$(PSQL) -t -A -c "SELECT count(*) FROM platform.cross_module_fk_report WHERE NOT is_compliant;"
	@$(PSQL) -t -A -c "SELECT analytics.refresh_all(true);" > /dev/null
	@$(PSQL) -t -A -c "SELECT count(*) FROM analytics.refresh_log WHERE NOT succeeded;"

check-front:
	cd frontend && npm run typecheck && npm run build

check-back:
	cd backend && cargo fmt --check && cargo clippy -- -D warnings && cargo test
```

Les deux compteurs de `check-db` doivent afficher **0**.

---

## `.gitignore`

La règle qui compte : **`docs/database/` est du code source, pas une sauvegarde.** Une exclusion `*.sql` trop large mettrait tout le modèle de données hors du dépôt — c'est arrivé sur l'ancien projet.

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

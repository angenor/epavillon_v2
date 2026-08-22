# ePavillon v2 — vérifications locales
#
# Pas d'intégration continue : développeur seul, contrainte de temps. Trois
# vérifications locales à la place, décrites dans docs/ENVIRONNEMENT_LOCAL.md.
#
#   make check          les trois — DÉTRUIT la base (voir ci-dessous)
#   make check-db-safe  les mêmes assertions SQL, sans rien détruire
#   make up / down      services locaux
#   make garage-init    layout, bucket et clé S3 — rejoué tout seul par `check-db`
#
# ############################################################################
# #  AVERTISSEMENT : `check-db` — donc `make check` — commence par `down -v`. #
# #  Cela SUPPRIME le volume pgdata et TOUTES les données saisies à la main   #
# #  depuis le dernier chargement du schéma. Sans confirmation, sans          #
# #  sauvegarde. Pendant le développement, utiliser `make check-db-safe`.     #
# ############################################################################

# Un `.env` à la racine, s'il existe, fournit les ports hôtes (POSTGRES_PORT…).
ENV_FILE := $(wildcard .env)
COMPOSE  ?= docker compose $(if $(ENV_FILE),--env-file $(ENV_FILE),) -f ops/docker-compose.dev.yml
PSQL     ?= docker exec -i epavillon-postgres psql -U postgres -d epavillon
GARAGE   ?= docker exec epavillon-garage /garage

# Identifiants S3 lus dans .env. `down -v` efface le layout de Garage ET sa clé :
# une clé engendrée au hasard rendait donc FAUSSES les valeurs du .env après
# chaque `make check`, et le point de contrôle manuel du stockage échouait pour
# une raison sans rapport avec le code. On IMPORTE la clé que le .env déclare
# plutôt que d'en créer une (B6, R31).
S3_KEY_ID     ?= $(shell sed -n 's/^S3_ACCESS_KEY_ID=//p'     $(ENV_FILE) 2>/dev/null | tail -1)
S3_KEY_SECRET ?= $(shell sed -n 's/^S3_SECRET_ACCESS_KEY=//p' $(ENV_FILE) 2>/dev/null | tail -1)

.PHONY: help check check-db check-db-safe assert-db assert-init-logs check-front check-back \
        up down wait-db logs-db garage-init garage-info

# `make` tout court affiche l'aide et ne détruit rien : la première cible d'un
# Makefile est celle qu'on exécute par mégarde, et `check` efface la base.
.DEFAULT_GOAL := help

help:
	@echo 'Cibles disponibles :'
	@echo '  make up             démarre les cinq services et attend le chargement du schéma'
	@echo '  make down           arrête les services (conserve les volumes)'
	@echo '  make check-db-safe  assertions sur la base en place — aucune perte'
	@echo '  make check          les trois vérifications — DÉTRUIT la base (down -v)'
	@echo '  make garage-init    layout, bucket et clé S3 — rejoué tout seul par check-db'
	@echo '  make garage-info    état du bucket'
	@echo '  make logs-db        journaux PostgreSQL'

check: check-db check-front check-back

# ---------------------------------------------------------------------------
# Services
# ---------------------------------------------------------------------------
up:
	$(COMPOSE) up -d
	@$(MAKE) --no-print-directory wait-db

# `down` seul conserve les volumes ; `down -v` les efface (voir check-db).
down:
	$(COMPOSE) down

# Attend que PostgreSQL soit prêt, plutôt qu'un `sleep` arbitraire : le
# chargement des fichiers SQL de docs/database/ prend un temps variable.
#
# PIÈGE MESURÉ : le healthcheck passe au vert AVANT la fin du chargement.
# Pendant l'initialisation, le serveur écoute déjà sur la socket locale, donc
# `pg_isready` répond « accepting connections » alors que la moitié des fichiers
# n'est pas encore jouée. On attend donc en plus le dernier objet créé par le
# dernier fichier (910_migration_v1.sql → legacy.id_map).
wait-db:
	@printf 'Attente de PostgreSQL '
	@for i in $$(seq 1 90); do \
	  etat=$$(docker inspect -f '{{.State.Health.Status}}' epavillon-postgres 2>/dev/null || echo absent); \
	  if [ "$$etat" = "healthy" ] \
	     && [ "$$($(PSQL) -t -A -c "SELECT to_regclass('legacy.id_map') IS NOT NULL" 2>/dev/null)" = "t" ]; then \
	    echo ' prêt, schéma chargé.'; exit 0; \
	  fi; \
	  printf '.'; sleep 2; \
	done; \
	echo ' ÉCHEC : schéma toujours pas chargé après 180 s — voir `make logs-db`.'; exit 1

# Une erreur d'initialisation n'empêche pas toujours le conteneur de tourner :
# elle laisse une base incomplète et silencieuse. On relit donc les journaux.
#
# Deux messages sont écartés parce qu'ils sont normaux : pendant
# l'initialisation, l'entrypoint arrête puis relance le serveur temporaire, et
# le healthcheck qui frappe à ce moment-là récolte « the database system is
# shutting down / starting up ». Ce sont des FATAL sans conséquence.
assert-init-logs:
	@echo '--- Journaux d'\''initialisation'
	@test -z "$$($(COMPOSE) logs postgres 2>&1 | grep -Ei 'ERROR:|FATAL:|PANIC:' | grep -v 'system is shutting down' | grep -v 'system is starting up')" \
	  || ($(COMPOSE) logs postgres 2>&1 | grep -Ei 'ERROR:|FATAL:|PANIC:' | grep -v 'system is shutting down' | grep -v 'system is starting up' | head -20; \
	      echo 'ÉCHEC : le chargement du schéma a produit des erreurs'; exit 1)

logs-db:
	$(COMPOSE) logs postgres

# ---------------------------------------------------------------------------
# Base de données
# ---------------------------------------------------------------------------

# LA vérification qui compte le plus : une migration qui ne passe pas sur une
# base vierge se découvre au déploiement, au pire moment.
#
# ATTENTION : `down -v` DÉTRUIT le volume pgdata et tout ce qu'il contient, ainsi
# que les volumes garage-meta / garage-data (d'où le rappel `make garage-init`).
check-db:
	@echo '################################################################'
	@echo '#  check-db va exécuter `down -v` :                             #'
	@echo '#  le volume PostgreSQL est DÉTRUIT, toutes les données saisies #'
	@echo '#  à la main sont PERDUES, ainsi que le layout Garage.          #'
	@echo '#  Pour vérifier sans rien détruire : make check-db-safe        #'
	@echo '################################################################'
	$(COMPOSE) down -v
	$(COMPOSE) up -d
	@$(MAKE) --no-print-directory wait-db
	@$(MAKE) --no-print-directory assert-init-logs
	@$(MAKE) --no-print-directory assert-db
	@$(MAKE) --no-print-directory garage-init

# Les mêmes assertions, SANS détruire la base : à lancer pendant qu'on développe.
check-db-safe: assert-db

# Les assertions elles-mêmes. Un compteur non nul fait ÉCHOUER la cible : c'est
# ce qui distingue un portail d'un affichage que personne ne lit.
assert-db:
	@echo '--- Schémas attendus'
	@test "$$($(PSQL) -t -A -c "SELECT count(*) FROM information_schema.schemata WHERE schema_name IN ('platform','reference','identity','org','event','programme','live','publication','negotiation','engagement','media','tool','content','training','analytics','legacy')")" = "16" \
	  || (echo 'ÉCHEC : schémas manquants — le chargement du modèle est incomplet, voir `make logs-db`'; exit 1)
	@echo '--- Frontières de modules (FK inter-modules nommées xmod_fk_*)'
	@test "$$($(PSQL) -t -A -c 'SELECT count(*) FROM platform.cross_module_fk_report WHERE NOT is_compliant;')" = "0" \
	  || ($(PSQL) -c 'SELECT source_schema, source_table, target_schema, target_table, constraint_name FROM platform.cross_module_fk_report WHERE NOT is_compliant;'; \
	      echo 'ÉCHEC : clés étrangères inter-modules non conformes'; exit 1)
	@echo '--- Rafraîchissement des projections analytiques'
	@$(PSQL) -t -A -c "SELECT analytics.refresh_all(true);" > /dev/null
	@test "$$($(PSQL) -t -A -c 'SELECT count(*) FROM analytics.refresh_log WHERE NOT succeeded;')" = "0" \
	  || ($(PSQL) -c 'SELECT view_name, started_at, error_code, error_message FROM analytics.refresh_log WHERE NOT succeeded ORDER BY started_at DESC LIMIT 20;'; \
	      echo 'ÉCHEC : vues analytiques en échec — voir analytics.refresh_log'; exit 1)
	@echo 'Base : conforme.'

# ---------------------------------------------------------------------------
# Front et API — inertes tant que les dossiers n'existent pas
# ---------------------------------------------------------------------------
check-front:
	@if [ -d frontend ]; then \
	   cd frontend && npm run typecheck && npm run build; \
	 else echo 'frontend/ absent — rien à vérifier (prompt A0.1)'; fi

# SQLx vérifie ses requêtes À LA COMPILATION : DATABASE_URL doit être renseignée
# et la base démarrée, sinon `cargo build` échoue au premier fichier.
#
# --all-targets ET --all-features ne sont pas décoratifs : sans eux, clippy ne
# voit ni les tests d'intégration ni `kernel/src/testing.rs`, qui vit derrière
# la caractéristique `testing` — le seul fichier du dépôt à composer du SQL
# dynamiquement échapperait au portail.
check-back:
	@if [ -d backend ]; then \
	   cd backend && cargo fmt --all --check \
	   && cargo clippy --workspace --all-targets --all-features -- -D warnings \
	   && cargo test --workspace --all-features; \
	 else echo 'backend/ absent — rien à vérifier (prompt B1)'; fi

# ---------------------------------------------------------------------------
# Stockage objet
# ---------------------------------------------------------------------------

# Un nœud Garage neuf refuse toute écriture tant que son layout n'est pas
# assigné : le conteneur démarre, l'API S3 répond, et chaque dépôt échoue.
# À refaire après chaque `down -v`, qui efface garage-meta et garage-data —
# `check-db` s'en charge lui-même depuis B6.
garage-init:
	@test -n "$(S3_KEY_ID)" && test -n "$(S3_KEY_SECRET)" \
	  || { echo 'ÉCHEC : S3_ACCESS_KEY_ID / S3_SECRET_ACCESS_KEY absentes de .env — faire `cp .env.example .env`.'; exit 1; }
	@node=$$($(GARAGE) status | grep -Eo '^[0-9a-f]{8,}' | head -1); \
	 test -n "$$node" || { echo 'ÉCHEC : nœud Garage introuvable — le conteneur est-il démarré ?'; exit 1; }; \
	 echo "Nœud : $$node"; \
	 $(GARAGE) layout assign -z dc1 -c 1G $$node; \
	 version=$$($(GARAGE) layout show | awk -F': ' '/Current cluster layout version/ {print $$2 + 1; exit}'); \
	 $(GARAGE) layout apply --version $${version:-1}
	@$(GARAGE) bucket create epavillon || true
	@$(GARAGE) key import --yes -n epavillon-dev $(S3_KEY_ID) $(S3_KEY_SECRET) > /dev/null || true
	@$(GARAGE) bucket allow --read --write epavillon --key $(S3_KEY_ID)
	@echo 'Garage : bucket « epavillon » ouvert à la clé du .env — rien à recopier.'

garage-info:
	@$(GARAGE) bucket info epavillon

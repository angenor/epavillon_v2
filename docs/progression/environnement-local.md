# Environnement local — ce qui a été vérifié, et les écarts

> Extrait de la [progression](../PROGRESSION.md). Le montage est décrit dans [ENVIRONNEMENT_LOCAL.md](../ENVIRONNEMENT_LOCAL.md).

## Ce qui a été vérifié le 16/08, et comment

| Contrôle | Résultat |
|---|---|
| `docker compose up -d` — cinq services | Démarrés ; images `pgvector/pgvector:pg17`, `valkey:8-alpine`, `jaeger:1.60`, `mailpit:latest`, `garage:v1.0.1` |
| Chargement intégral du schéma | `docker compose logs postgres` : **zéro `ERROR:` / `FATAL:` / `PANIC:`** hors messages bénins du healthcheck. Les deux `NOTICE` attendus de `900_seed.sql` sont présents, dont « Frontières de modules conformes » |
| Schémas attendus | 15 présents (`legacy` compris) ; **142 tables** hors partitions — le compte annoncé, à condition de compter les tables partitionnées parentes |
| Extensions | `pgcrypto`, `citext`, `pg_trgm`, `unaccent`, `btree_gist`, `vector`, `pg_stat_statements` ; `shared_preload_libraries = pg_stat_statements` effectif |
| `cross_module_fk_report WHERE NOT is_compliant` | **0** |
| `analytics.refresh_all(true)` | 7 vues matérialisées rafraîchies, **0 échec** dans `refresh_log` |
| Le portail échoue-t-il vraiment ? | Testé : ligne d'échec factice insérée dans `refresh_log` → `make check-db-safe` sort en **code 2**. Ligne retirée ensuite |
| `make check` de bout en bout | Vert, code de sortie 0 (`down -v` → rechargement complet → assertions) |
| Mailpit | Interface HTTP 200 ; courriel envoyé sur `localhost:1025` et **capturé** (1 message, sujet correct) |
| Jaeger | Interface HTTP 200 ; ports OTLP 4317/4318 publiés |
| Valkey | `PING` → `PONG` |
| Garage | Layout assigné, bucket `epavillon` créé, clé `epavillon-dev` en lecture/écriture. **Écriture réelle prouvée** : `PUT` puis `GET` d'un objet signé SigV4 → 200 / 200 |

---

## Écarts entre `ENVIRONNEMENT_LOCAL.md` et ce qui a réellement démarré

Relevés le 16/08 en montant l'environnement. Tous corrigés dans le document et dans les fichiers.

| Ce que décrivait le document | Ce qui s'est passé | Correction retenue |
|---|---|---|
| `check-db: … up -d && sleep 12` | Le healthcheck `pg_isready` passe au vert **en 2 s**, alors que les 18 fichiers SQL sont encore en cours d'exécution : pendant l'initialisation, le serveur temporaire écoute déjà la socket locale. Un `sleep` fixe est un pari sur la vitesse de la machine | Cible `wait-db` : conteneur sain **et** `legacy.id_map` présente — le dernier objet du dernier fichier chargé |
| Rien sur les journaux d'initialisation | Une erreur de chargement laisse une base incomplète sans arrêter le conteneur — le piège annoncé. Aucune assertion ne le couvrait | Cible `assert-init-logs`, intégrée à `check-db` |
| — | Le healthcheck déclenché pendant l'arrêt du serveur temporaire laisse un `FATAL: the database system is shutting down` **bénin** qui faisait échouer à tort l'assertion précédente | `shutting down` et `starting up` explicitement écartés du filtre |
| Ports fixes `5432`, `3900`, `3903`… | Occupés sur la machine de développement par deux autres projets (`uafricas_postgres`, `kaya-objets`) | Chaque port publié devient `${VAR:-défaut}` ; la liste est en fin de `.env.example`, les valeurs par défaut sont inchangées |
| — | `docker compose` lit le `.env` du dossier du fichier compose (`ops/`), pas celui de la racine : les ports auraient été ignorés en silence | Le `Makefile` passe `--env-file .env` quand ce fichier existe |
| « [Garage] demande une initialisation manuelle », procédure donnée en quatre commandes | La procédure **était déjà écrite** dans le document, contrairement à ce qu'annonçait la consigne de session. Ce qui manquait : son automatisation et deux pièges | Cible `make garage-init` ; le document renvoie vers elle |
| `garage layout apply --version 1` | Rejouer la commande avec un numéro erroné donne `Invalid new layout version` — et l'identifiant du nœud **change à chaque `down -v`** | La cible lit « Current cluster layout version » et ajoute 1 ; elle relève le nœud elle-même |
| `check-front: cd frontend && …` | `frontend/` et `backend/` n'existent pas encore : `make check` échouait sur une absence de dossier, pas sur un défaut | Les deux cibles annoncent l'absence et rendent la main sans échouer |
| `.env.example` : `S3_BUCKET=epavillon`, `SMTP_HOST`/`SMTP_PORT` | La consigne de session demandait `S3_BUCKET=epavillon-dev` et un `SMTP_URL` unique | Le document a été suivi : `epavillon` est le nom que crée `garage bucket create`, et deux façons de configurer le SMTP auraient été une ambiguïté de plus. `epavillon-dev` reste le nom de la **clé** d'accès |

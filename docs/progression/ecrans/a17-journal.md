# A17 : journal du back-office (`/admin/journal`)

Maquette validée : [`docs/maquettes/journal/index.html`](../../maquettes/journal/index.html). Modèle : `010_platform.sql` § 2 bis et § 2 ter, migration `ops/migrations/2026-11-02-journal.sql` (jouée le 02/11).

## Livré le 02/11

- **Capture côté API** (`api/src/middleware/request_context.rs`, fonction pure `classer`) : 5xx (panne), 401/403/429, 409/422 sur `/api/admin/*`, 404 d'une personne connectée (refus), réponse < 400 de plus d'une seconde (lente). Route = modèle de route, `*` sans correspondance ; code de l'`ApiError` ; détail seulement pour une panne, nettoyé des courriels et des jetons (`kernel::ops_log::sanitize_detail`), 2 Ko.
- **Écriture par lots** (`kernel::ops_log::OpsLog`) : canal borné (2 000), tâche qui insère toutes les 2 s par `jsonb_to_recordset` ; canal plein ou lot refusé, lignes perdues et comptées (`dropped` du résumé). Ouvert par le binaire seulement : les tests d'intégration n'écrivent rien.
- **Worker** : chaque essai en échec d'un travail écrit une ligne `job_failed` hors de la transaction du travail (`error` si mort, `warn` sinon, `request_id` = `job:<id>`). Nouveau travail `platform.maintain_journals` (quotidien, armé au démarrage avec un passage immédiat par jour) : partitions de `platform.audit_log` et `platform.ops_log` (mois courant + 3), `platform.purge_ops_log(3)`. **Les partitions de l'audit n'étaient préparées par aucun travail** avant cela.
- **Routes** `/admin/journal/*` (crate `api`, `routes/journal/`) : `summary`, `entries`, `entries.csv` (5 000 lignes, cellules de formule neutralisées), `requests/{request_id}` (journal + audit + outbox de la demande), `changes` (audit global, valeurs sensibles masquées en base par `platform.audit_is_sensitive`, textes > 300 caractères tronqués), `failures`, `jobs/{id}/retry` (`platform.retry_job`), `events/{id}/retry` (disponible tout de suite + `pg_notify`). Lecture : `identity.platform.journal` en portée globale ; relance : en plus `identity.platform.maintenance`.
- **Front** : page, onglets `?onglet=`, recherche `?demande=`, menu (permission `identity.platform.journal`), liens depuis `HealthPanel`, i18n fr/en (libellés des tâches et des événements), données d'exemple.

## Écarts

- Quatrième chiffre de l'en-tête : travaux et événements **à relancer** (`failing_jobs + failing_events`), libellé de la maquette, et non les `job_failed` des 24 h : une tâche réussie au deuxième essai n'aurait rien à montrer dans l'onglet ouvert.
- Colonne « Pour » de la maquette absente : la charge utile d'un travail ne sort jamais (elle peut porter un jeton).
- Objets de l'onglet Modifications nommés par leur table et un identifiant court, pas par leur titre (« Dossier CDP31-00128 » de la maquette) : il faudrait lire chaque module.
- Le sélecteur d'édition du layout reste affiché sur cette page globale.
- Le cache SQLx des quinze requêtes a été **écrit à la main** (types lus dans le SQL) : le tunnel vers la production ne répondait plus. À refaire par une préparation contre la base dès qu'elle est joignable ; la compilation hors ligne, clippy et les tests unitaires passent.

## Vérifié / non vérifié

- Vérifié : `cargo check` et `clippy` (kernel, api, worker), tests unitaires (classement, nettoyage du détail, CSV, pagination, prochaine occurrence, journal du middleware), syntaxe des quinze requêtes par libpg_query, `make openapi` et `check-api-contract`, `npm run typecheck`, page hors ligne dans un navigateur (sous-agent).
- Non vérifié : aucune requête exécutée contre une base, aucun test d'intégration ; parcours complet contre l'API.

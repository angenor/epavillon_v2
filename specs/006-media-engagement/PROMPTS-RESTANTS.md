# Prompts des sessions restantes — B6

> Deux sessions neuves. La première livre la phase 11 (US9) et atteint le jalon 3 ;
> la seconde fait le polissage transverse et referme le jalon. Chacune se copie
> telle quelle après `/speckit-implement`.

---

## Session A — `/speckit-implement`

```text
phase 11 Contexte : B6 (Média + Engagement). Les phases 1 à 10 sont livrées —
T001 à T184, cochées dans specs/006-media-engagement/tasks.md. Les jalons 1 et 2
sont atteints. La phase 11 est US9 : le disque ne se remplit pas tout seul. À son
terme, le JALON 3 est atteint et les neuf histoires sont livrées.

Avant de coder, lis dans cet ordre :
  - docs/PROGRESSION.md, puis docs/progression/ecrans/b6-media-engagement.md
  - docs/progression/journal/2026-08-21.md et decisions/2026-08-21.md (les
    dernières entrées portent ce qui a été tranché en phases 7 à 10)
  - specs/006-media-engagement/tasks.md, phase 11 (T185 à T198)
  - docs/database/050_media.sql § 6 — find_orphan_assets(), schedule_asset_purge(),
    reconcile_storage_quotas(), organization_storage_usage(), rendition_bytes()

Huit choses déjà en place que la phase 11 ne doit pas refaire :

  1. Les cinq fonctions du modèle existent et sont complètes. Aucune n'est à
     écrire, aucune n'est à réimplémenter en Rust : find_orphan_assets() range
     déjà du plus lourd au plus léger et exclut ce qui est rattaché ;
     reconcile_storage_quotas() fait le réalignement ; organization_storage_usage()
     est la vérité contre laquelle T196 mesure.

  2. schedule_asset_purge() ÉMET DÉJÀ « media.asset.purge_scheduled ». C'est le
     piège n° 1 du découpage, pour la troisième fois de ce jalon. Le service ne
     redouble pas : il appelle la fonction, un point. Le seul événement que le
     module Média émet est « media.asset.purged » (T187), quand l'objet a
     RÉELLEMENT quitté le stockage — schedule annonce l'intention, jamais
     l'exécution. contracts::media::{ASSET_PURGED, AssetPurged} portent déjà sa
     forme.

  3. Le patron du travail récurrent est celui de B1, et il est écrit trois fois
     dans le dépôt : identity::jobs::purge, org::jobs::duplicates,
     event::jobs::autoclose. Chacun expose planifier(&mut tx, moment) -> bool
     avec une clé d'unicité PORTANT LE JOUR, et worker/src/main.rs les RÉARME au
     démarrage — armer_les_recurrents(). Les trois chaînes nouvelles s'y
     ajoutent de la même façon : dix redémarrages dans la journée ne produisent
     pas dix purges, et c'est ce que T197 mesure.

  4. media::job_handlers(db, &config) rend UN gestionnaire, ProcessAsset, sur la
     file « media ». C'est là que purge et reconcile s'ajoutent. La file est une
     DÉCLARATION du gestionnaire — JobHandler::queue() — et le worker n'écoute
     que celles que ses gestionnaires nomment : le défaut trouvé en phase 4
     laissait les travaux s'empiler sans erreur ni trace.

  5. engagement::job_handlers rend SendReminder. Le travail des partitions (T190)
     s'y ajoute, sur la file par défaut. config.engagement.partition_interval
     existe déjà.

  6. Le contrat de stockage porte delete(&self, key) et head(&self, key). Les
     tests tournent sur le stockage SUR FICHIERS, jamais sur Garage :
     make check-db efface le layout, et media/tests/stockage_s3.rs est ignoré
     par défaut pour cette raison. T194 doit donc vérifier l'absence sur le
     stockage de test, ce qui est le vrai chemin.

  7. Le harnais media/tests/commun/mod.rs sait déposer un fichier, passer le
     worker par le vrai chemin — passer_le_worker(), worker_tue_apres_le_travail(),
     worker_relance() —, compter les déclinaisons et lire la consommation.

  8. T205 EST DÉJÀ FAITE : docs/progression/modele.md porte l'entrée du 08-21 sur
     engagement.session_reminder_schedule(). Vérifie-la, coche, n'y touche pas.

Une leçon de la phase 9 à ne pas réapprendre : dans un test, l'heure vient de la
BASE, jamais de l'horloge de la machine. Le conteneur dérive de quelques
secondes, et un run_at posé depuis Rust peut tomber dans le futur de la base :
claim_jobs() ne réserve rien, et le test échoue une fois sur deux sur une chaîne
qui marche. Pour vieillir un objet, écris « created_at = now() - interval … » en
SQL.

Ce que cette phase doit oser toucher :
  - le test de montage passe de 4 à 7 routes pour media ;
  - worker/src/main.rs gagne trois chaînes récurrentes à réarmer.

Les trois tests qui comptent en phase 11 :
  - un objet purgé A QUITTÉ LE STOCKAGE, vérifié sur le stockage lui-même et non
    sur une relecture en base, et la consommation a baissé de son poids
    DÉCLINAISONS COMPRISES ;
  - la suppression d'un objet rattaché à deux fiches est refusée EN DISANT DEUX
    — c'est l'écart n° 128 : le même fichier déposé par deux organisations donne
    une ligne, et une suppression par la première ferait disparaître l'image de
    la seconde ;
  - les trois travaux récurrents se replanifient, et le démarrage du worker
    réarme la chaîne SANS créer de doublon.

Termine par : cargo fmt --check, clippy --all-targets --all-features -D warnings,
cargo test --workspace --all-features, les contrôles mécaniques du quickstart
(frontières de crates, écritures hors schéma reference et content comprises,
fichiers de plus de mille lignes, émission et file), et la mise à jour de la
progression (journal du jour, fichier d'écran B6, ligne de suivi dans
PROGRESSION.md, décisions du jour).
```

---

## Session B — `/speckit-implement`

```text
phase 12 Contexte : B6 (Média + Engagement). Les onze phases sont livrées —
T001 à T198 — et le jalon 3 est atteint : les neuf histoires sont là. La phase 12
n'ajoute aucune fonctionnalité. Elle vérifie que ce qui a été promis est vrai, et
elle referme le jalon.

Avant de commencer, lis dans cet ordre :
  - docs/PROGRESSION.md, puis docs/progression/ecrans/b6-media-engagement.md
  - docs/progression/journal/2026-08-21.md et decisions/2026-08-21.md
  - specs/006-media-engagement/tasks.md, phase 12 (T199 à T209)
  - specs/006-media-engagement/contracts/routes.md et errors.md — les deux
    comptes que T199 et T200 opposent au code
  - specs/006-media-engagement/quickstart.md — les sept parcours de T207

Six choses à savoir avant d'ouvrir quoi que ce soit :

  1. Le compte est de TRENTE-TROIS routes au contrat, et il se vérifie à deux
     endroits qui ne se lisent pas l'un l'autre : sur la vraie application
     (T199) et dans l'OpenAPI engendrée (T200). Deux comptes écrits à deux
     endroits, comme en B4 — c'est le dispositif qui a attrapé trois routes
     muettes en B2 et un scope non monté en B4.

  2. La trente-troisième route est HORS SESSION et son montage dépend de la
     configuration : POST /internal/mail-events n'existe que si
     MAIL_WEBHOOK_TOKEN est renseigné. Elle est déjà éprouvée à part, par
     api/tests/routes_media_engagement.rs::la_porte_dingestion_suit_son_jeton —
     montée elle réclame son jeton, absente elle rend 404 et jamais 401. Ne la
     compte pas avec les autres et ne la réécris pas.

  3. T201 attend EXACTEMENT deux émissions dans les deux crates, et zéro mise en
     file : media.asset.purged (phase 11) et engagement.email.suppressed
     (phase 10). Tout le reste est émis et enfilé par la base — deux
     déclencheurs et une fonction. Un troisième appel signale un doublon qui ne
     se verrait qu'en production.

  4. T205 EST DÉJÀ FAITE : docs/progression/modele.md porte l'entrée du 08-21.
     Vérifie, coche, n'y touche pas.

  5. T207 est le SEUL moment où le stockage S3 réel est exercé. Il se fait à la
     main, sur Garage, et media/tests/stockage_s3.rs — ignoré par défaut — se
     lance par « cargo test -p media --test stockage_s3 -- --ignored ». Note que
     make check-db efface le layout de Garage : make garage-init d'abord.

  6. T208 lance make check EN ENTIER depuis la racine, ce qui commence par
     down -v. La base est détruite et rechargée. C'est obligatoire ici : le
     schéma a changé une fois dans ce jalon.

Ce que T206 doit inscrire aux points bloqués, et qui n'y est pas encore : les
obligations que ce jalon lègue à B7 — le champ de description d'image sur les
trois écrans qui téléversent, la forme attendue d'un rôle que le contrat du front
ne porte pas, et l'écart n° 138, aucune page de séance n'existant dans le front,
si bien qu'un lien de courriel ou de notification mène à la page de l'édition.

Si un contrôle échoue, ne le contourne pas : soit le code a tort et on le
corrige, soit le contrat a tort et on l'amende en le datant. Un compte ajusté
pour faire passer un test ne vérifie plus rien.

Termine par la mise à jour de la progression — journal du jour,
ecrans/b6-media-engagement.md, decisions du jour, docs/progression/api.md, et la
ligne de suivi dans PROGRESSION.md. B6 est alors livré ; dis quel est le prompt
suivant d'après docs/PROMPTS_DEVELOPPEMENT.md.
```

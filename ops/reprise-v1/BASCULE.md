# Reprise v1, phase 1 : la bascule

Principe 1 du [plan](../../docs/progression/reprise-v1/plan.md) poussé au bout : la production reçoit **la base répétée elle-même**. On sauvegarde la production, on reconstruit le banc depuis cette sauvegarde, on y joue la reprise, on contrôle, puis on renvoie la base obtenue. Rien ne s'écrit en production entre la sauvegarde et le retour : maintenance allumée, worker et API arrêtés.

Seul `postgres` possède les objets (vérifié le 27/10) ; local 17.10, production 17.11.

## Avant

- Lots 00 à 60 verts sur un clone du banc ; rôles relus par le commanditaire (`sauvegardes/reprise-v1/roles-a-relire.txt`).
- Feu vert du commanditaire.

## Gestes

Chaque geste sur le serveur mesure le voisin avant et après (`curl -s -o /dev/null -w '%{http_code}' https://financedurable.francophonie.org/`).

1. **Images d'abord** : `./deploy.sh push`, puis sur le serveur `$COMPOSE build api worker front`, sans redémarrer (le code D1 attend la colonne `password_reset_required_at`).
2. **Geler** : `$COMPOSE stop worker`. La maintenance, déjà allumée, fait refuser les écritures par l'API ; elle reste debout pour servir la page d'attente, et le commanditaire n'agit pas au back-office jusqu'au geste 8.
3. **Sauvegarder** : `./deploy.sh backup` → `sauvegardes/epavillon-<horodatage>.sql.gz`.
4. **Répéter sur cette sauvegarde** (poste) :
   - `COPIE_PROD=sauvegardes/epavillon-<horodatage>.sql.gz ops/reprise-v1/repeter.sh` ;
   - clone `epavillon_finale` du banc ; `ops/migrations/2026-10-27-compte-sans-mot-de-passe.sql` (à la place de `lots/cale-d1.sql`) ; lots 10 à 50 ; partie SQL du lot 60 ;
   - comptes rendus de chaque lot : zéro écart non expliqué ;
   - outbox et travaux du lot : événements marqués publiés, travaux `live.*` et rappels annulés, **`media.process_asset` et `org.trust_score.recompute` gardés** (sans ce dernier, le score de confiance des 234 fiches reste à 0) ;
   - exporter `legacy.garage_a_purger` (objets d'essai à effacer) puis la supprimer ; supprimer les schémas `v1`, `v1_auth`, `v1_storage` (ils portent les empreintes bcrypt) ; `legacy.id_map` reste.
5. **Fichiers vers la Garage de production** (tunnel SSH vers l'adresse du conteneur `garage` sur le réseau Docker du serveur, clés lues dans `.env.prod`, jamais écrites) : déposer les objets cités par le lot 60, effacer les objets d'essai exportés.
6. **Renvoyer la base** : `pg_dump --clean --if-exists` de `epavillon_finale` (pg_dump du conteneur local), `scp`, puis sur le serveur : `$COMPOSE stop api` (quelques minutes d'erreur pour les visiteurs, la page d'attente ayant besoin de l'API), `DROP DATABASE epavillon`, `CREATE DATABASE epavillon`, `psql < dump`.
7. **Repartir** : `$COMPOSE up -d api worker front` (nouvelles images) ; `./deploy.sh sante`.
8. **Vérifier en administrateur** (site toujours en maintenance) : propositions COP31 au back-office, une organisation, une activité COP30 et sa page, images, connexion d'un compte repris (refus avec le message D1). Puis éteindre `platform.maintenance` depuis le back-office.

## Retour arrière

Avant le geste 6, rien n'a changé en production hors Garage. Après : `./deploy.sh restore epavillon-<horodatage>.sql.gz` (la sauvegarde du geste 3), `$COMPOSE up -d api worker front` avec les images d'avant si le code D1 pose problème.

## Après (phase 2, site ouvert)

**À traiter avant toute publication du programme CdP31** : les 78 dossiers en évaluation ont une séance `planned` non publiée (leur créneau v1). Rejeter un de ces dossiers ne retire pas sa séance (`service/transition.rs` ne touche pas aux séances) : à faire à la main au planificateur, ou règle à écrire. Les 16 propositions privées de leurs intervenants (D16) les retrouvent avec l'extension D16.

Évaluations, inscriptions, rediffusions, journées spéciales, annexes, fichiers de l'ancien serveur, extensions du modèle (D7, D16, D17) : la v1 brute sera reposée en production le temps de ces lots, puis retirée.

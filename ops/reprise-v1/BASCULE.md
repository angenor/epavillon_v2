# Reprise v1, phase 1 : la bascule

Principe 1 du [plan](../../docs/progression/reprise-v1/plan.md) poussé au bout : la production reçoit **la base répétée elle-même**. On sauvegarde la production, on reconstruit le banc depuis cette sauvegarde, on y joue la reprise, on contrôle, puis on renvoie la base obtenue. Rien ne s'écrit en production entre la sauvegarde et le retour : maintenance allumée, worker et API arrêtés.

Seul `postgres` possède les objets (vérifié le 27/10) ; local 17.10, production 17.11.

**Jouée le 27/10, gestes 1 à 7** : sauvegarde `epavillon-20261027-134931.sql.gz`, archive `sauvegardes/reprise-v1/epavillon_finale-20261027-1405.sql.gz` (restauration en production : 6 min 30). Détail : journal du 27/10.

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
   - `ops/reprise-v1/jouer-phase1.sh` : clone `epavillon_finale`, migration D1 (à la place de `lots/cale-d1.sql`), lots 10 à 50, partie SQL du lot 60, bilan (répétition générale du 27/10 : 74 s) ;
   - comptes rendus de chaque lot : zéro écart non expliqué ;
   - outbox et travaux du lot : événements marqués publiés, travaux `live.*` et rappels annulés, **`media.process_asset` et `org.trust_score.recompute` gardés** (sans ce dernier, le score de confiance des 234 fiches reste à 0) ;
   - `legacy.garage_a_purger` et `legacy.fichiers_a_deposer` servent au geste 5, lu sur cette base.
5. **Fichiers vers la Garage de production** (tunnel SSH vers l'adresse du conteneur `garage` sur le réseau Docker du serveur ; le poste n'a pas de `.env.prod` : ses lignes `S3_*` sont copiées dans un fichier temporaire en mode 600, effacé après le geste 7) : `ENV_GARAGE=<copie temporaire> S3_ENDPOINT=http://127.0.0.1:<port du tunnel> lots/60-fichiers.sh deposer epavillon_finale`. Ensuite `jouer-phase1.sh epavillon_finale envoi` retire les schémas `v1*` (empreintes bcrypt) et les deux tables de travail, et écrit l'archive ; `legacy.id_map` reste.
6. **Renvoyer la base** : l'archive du geste 5, `scp`, puis sur le serveur : `$COMPOSE stop api` (quelques minutes d'erreur pour les visiteurs, la page d'attente ayant besoin de l'API), `DROP DATABASE epavillon`, `CREATE DATABASE epavillon`, `psql < dump`.
7. **Repartir** : `$COMPOSE up -d api worker front` (nouvelles images) ; `./deploy.sh sante`. Puis, même tunnel, `lots/60-fichiers.sh purger epavillon_repetition` efface de Garage les objets d'essai (`legacy.garage_a_purger`, lu sur le banc que l'envoi ne touche pas) : plus tôt, un retour arrière rendrait une base dont les images manquent.
8. **Vérifier en administrateur** (site toujours en maintenance) : propositions COP31 au back-office, une organisation, une activité COP30 et sa page, images, connexion d'un compte repris (refus avec le message D1). Puis éteindre `platform.maintenance` depuis le back-office.

## Retour arrière

Avant le geste 6, rien n'a changé en production hors Garage. Après : `./deploy.sh restore epavillon-<horodatage>.sql.gz` (la sauvegarde du geste 3), puis `ops/migrations/2026-10-27-compte-sans-mot-de-passe.sql` sur la base restaurée. Les images d'avant ne sont plus récupérables (le stockage containerd retire l'ancienne image dès que la construction réattribue `latest`), et la migration D1, additive, suffit au nouveau code pour tourner sur les anciennes données.

## Après (phase 2, site ouvert)

**À traiter avant toute publication du programme CdP31** : les 78 dossiers en évaluation ont une séance `planned` non publiée (leur créneau v1). Rejeter un de ces dossiers ne retire pas sa séance (`service/transition.rs` ne touche pas aux séances) : à faire à la main au planificateur, ou règle à écrire. Les 16 propositions privées de leurs intervenants (D16) les retrouvent avec l'extension D16.

Évaluations, inscriptions, rediffusions, journées spéciales, annexes, fichiers de l'ancien serveur, extensions du modèle (D7, D16, D17) : la v1 brute sera reposée en production le temps de ces lots, puis retirée.

**Lot 70, témoignages (27/10)**, sans v1 brute (ils vivent dans le code de la v1) :
1. clone local de la production, `lots/70-temoignages.sh <clone>` ;
2. tunnel et clés comme au geste 5, `FICHIERS_V1=<photos> lots/60-fichiers.sh deposer <clone>` ;
3. `lots/70-temoignages.sh flux | ssh <serveur> "cd /opt/epavillon && $COMPOSE exec -T postgres psql -U postgres -d epavillon -X -q -v ON_ERROR_STOP=1"`.

**Lot 80, inscriptions (27/10)**, même voie sans fichier : `lots/80-inscriptions.sh <clone>` puis `lots/80-inscriptions.sh flux | ssh …`. Le flux ne porte que des identifiants et des dates, lus au banc. Vérifié après coup en production : outbox vide, aucune notification, aucun rappel planifié, aucun participant Zoom, aucun consommateur n'a reçu les événements.

**Lot 90, journées spéciales de la CdP30 (27/10)**, même voie : `lots/90-journees-speciales.sh <clone>` puis `lots/90-journees-speciales.sh flux | ssh …`. Le flux porte le relevé public des deux journées et les correspondances écrites dans le lot ; il refuse une seconde passe (fil déjà présent). Joué après une sauvegarde (`epavillon-20261027-192649`) ; vérifié : outbox vide, ni notification, ni rappel, ni participant Zoom ; l'API sert les deux fils et leurs 15 séances.

**Lot 91, complément du lot 90 (27/10)** : `ssh … < lots/91-journees-speciales-complement.sql`, après la sauvegarde `epavillon-20261027-195115`. Vérifié : 45 intervenants et 3 visuels sur les 14 séances, outbox vide, aucune notification.

**Lot 92, intervenants sans adresse (D16)**, dans cet ordre : sauvegarde ; `ssh … < ops/migrations/2026-10-27-intervenant-sans-adresse.sql` ; `./deploy.sh update` (l'API lit les nouvelles colonnes) ; `lots/92-intervenants-sans-adresse.sh flux | ssh …`. Le flux porte l'identité des seuls écartés (civilité, nom, fonction, organisation), aucune adresse. Joué le 27/10 après la sauvegarde `epavillon-20261027-203403` : 143 repris, 6 fusionnés, 64 dossiers complets.

**Lot 93, photos des intervenants sans adresse**, dans cet ordre : sauvegarde ; `ssh … < ops/migrations/2026-10-27-photo-intervenant-sans-adresse.sql` ; `./deploy.sh update` ; sur un clone local de la production portant les lots 90 à 92, `lots/93-photos-intervenants.sh <clone>` ; tunnel et clés comme au geste 5, `lots/60-fichiers.sh deposer <clone>` (les trois fichiers) ; `lots/93-photos-intervenants.sh flux | ssh …`. Les identifiants d'objets sont fixés dans le lot : ceux du clone sont ceux de la production. Joué le 27/10 après la sauvegarde `epavillon-20261027-214757` : 3 photos déposées, rattachées et traitées.

**Lot 94, évaluations**, même voie sans fichier : `lots/94-evaluations.sh <clone>` puis `lots/94-evaluations.sh flux | ssh …`. Le flux porte notes et messages avec leur texte (la donnée reprise), rien d'autre de personnel.

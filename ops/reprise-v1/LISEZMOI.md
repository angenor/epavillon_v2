# Reprise v1 : le banc de répétition

Plan : [`docs/progression/reprise-v1/plan.md`](../../docs/progression/reprise-v1/plan.md) (§§ 2, 5, 7). Contrat des lots : [`lots/CONTRAT.md`](lots/CONTRAT.md).

## Lancer

```bash
ops/reprise-v1/repeter.sh
```

Une dizaine de secondes. Repart de zéro à chaque lancement : rien à nettoyer avant. Ne touche qu'aux bases `epavillon_repetition`, `epavillon_passage_v1` (passage, supprimée en fin de script) et `epavillon_neuve` (réservée à la comparaison avec une base neuve, à la main) ; tout autre nom est refusé par `commun.sh`.

Entrées, hors dépôt (`sauvegardes/`, ignoré par Git) : la copie de la production v2 (`COPIE_PROD`) et l'archive Supabase (`EXTRACTION_V1`), réglables par variable d'environnement. Il faut le conteneur `epavillon-postgres` et le `pg_restore` 18 du poste (`/opt/homebrew/opt/libpq/bin/`) : l'archive vient d'un `pg_dump` 18, que celui du conteneur (17) refuse.

## Ce que ça produit

`epavillon_repetition`, dans l'ordre où la bascule jouera les mêmes gestes :

1. `01-copier-production.sh` : la copie de la production v2, restaurée telle quelle.
2. Migrations de schéma de la reprise (`ops/migrations/2026-10-27-reprise-v1-correspondance.sql` : l'ancien 910 remplacé par `legacy.id_map`).
3. `03-purger-essais.sql` : purge des données d'essai (D2). Restent le commanditaire (personne, compte, rôles, sa demande d'adhésion à l'IFDD), le compte d'administration et la fiche IFDD semés par `900_seed.sql`, et tout ce que le schéma sème ou que l'administration règle. Les clés des objets Garage des médias effacés sont relevées dans `legacy.garage_a_purger`.
4. `02-poser-v1-brute.sh` : la v1 brute, schémas `v1` (son `public`), `v1_auth`, `v1_storage`. Tables, types ENUM, données, clés primaires, uniques et étrangères, commentaires. Ni fonctions, ni vues, ni déclencheurs, ni politiques, ni index, ni séquences, ni valeurs par défaut, ni droits Supabase. Les tables de jetons et journaux d'`auth` (sessions, jetons de rafraîchissement, journal d'audit…) et `storage.migrations` sont créées vides.
5. `lots/00-socle.sql` : la personne technique « Reprise v1 ».

La base sert ensuite de modèle : chaque lot travaille sur `CREATE DATABASE … TEMPLATE epavillon_repetition`. Le script vérifie qu'aucune connexion n'y reste.

## Ce que ça vérifie (le script échoue sinon)

- **Comptages v1** : chaque table posée contre [`comptes-v1-attendus.txt`](comptes-v1-attendus.txt), liste fermée dans les deux sens, plus les 105 inscriptions hors PACO. PACO vit encore sur Supabase (D15) : une nouvelle extraction fera bouger ses tables, à recaler dans le fichier après avoir vérifié que l'écart ne touche qu'elles.
- **Purge, avant d'effacer** : toute table non vide doit figurer au classement du script (purgée, partielle ou gardée, avec sa raison) ; le commanditaire et la fiche IFDD doivent exister.
- **Purge, après** : tables purgées vides, aucune clé étrangère orpheline dans toute la base (les déclencheurs et cascades sont coupés pendant la purge, pour n'émettre ni audit, ni outbox, ni notification).

## À la bascule (plan § 7)

- Jouer en production, après `./deploy.sh backup` : la migration, `03-purger-essais.sql`, la pose de la v1 (même chemin, base cible changée), puis les lots.
- **Garage** : effacer de son stockage les objets listés dans `legacy.garage_a_purger` (seau, clé) une fois la purge validée, puis supprimer la table. La purge SQL ne les touche pas.
- Une base de production qui aurait changé depuis la copie du 27/10 peut faire échouer la garde du classement : c'est voulu, classer la table puis relancer.

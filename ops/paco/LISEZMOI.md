# PACO : l'application v1 servie à côté de la v2

Le cycle de webinaires PACO n'est pas repris dans la v2 (décision D9, relevé complet : `docs/progression/reprise-v1/paco.md`). En attendant sa vraie migration, l'Apache du serveur sert **l'application v1 elle-même**, inchangée à l'écran et toujours branchée sur Supabase, sur `/paco`, `/paco/join`, `/paco/admin` et les pages de connexion de la v1 (`/login`, `/403`, `/forgot-password`, `/reset-password`). Tout le reste va à Nuxt.

| Fichier | Rôle |
|---|---|
| `garde-routeur.patch` | seul écart avec le code v1 : toute navigation hors des chemins ci-dessus recharge la page, que sert alors la v2 |
| `construire.sh` | copie l'arbre de travail v1, pose la garde, écrit les trois variables, construit et vérifie |

## Reconstruire

```bash
ops/paco/construire.sh /chemin/de/travail     # défaut : $TMPDIR/paco-build
```

- Lit le dépôt v1 voisin (`../epavillonvue`, ou `PACO_V1=…`) **sans y écrire**, arbre de travail compris (fichiers non commités : c'est ce qui tournait en production).
- Variables : seulement `VITE_SUPABASE_URL` et `VITE_SUPABASE_ANON_KEY`, reprises de `epavillonvue/.env.local` sans être affichées, et `VITE_APP_URL`. Les clés d'IA de la v1 restent dehors ; le script échoue si l'une d'elles, ou une valeur `sk-…` ou `sb_secret_…`, se retrouve dans le build, ou si la clé Supabase n'est pas la clé publique `anon`.
- Résultat : `<travail>/epavillon-paco/`, seuls fichiers à déposer (`index.html`, `assets/`, `images/`, `fonts/`, `logos/`, `favicon.ico`, `logo-ifdd.png`, `version.json`), dans `/var/www/epavillon-paco/` sur le serveur. Hors de `/opt/epavillon`, que `deploy.sh` synchronise avec `--delete`.
- Après chaque séance (passer la séance en `completed` avec sa rediffusion, ajouter la suivante) : modifier la v1, relancer le script, redéposer le dossier.

## Retirer PACO

1. Supprimer les lignes PACO du vhost 443 (`ProxyPassMatch`, `AliasMatch`, bloc `<Directory "/var/www/epavillon-paco">`), puis `apachectl configtest && systemctl reload httpd`.
2. Supprimer `/var/www/epavillon-paco/` sur le serveur.
3. Supprimer ce dossier.

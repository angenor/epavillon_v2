# Vérification — Guide Négo 0a

Ce guide prouve les trois critères de recette. Contrats : [contracts/](contracts/). Décisions : [research.md](research.md).

## Préalables

- Services locaux montés (`make up`). **Ne jamais lancer `make check` ni `down -v`.**
- Le drapeau, sur la base déjà montée — une fois :

```sql
INSERT INTO platform.feature_flags (key, description, is_enabled, rollout_percent)
VALUES ('guide_nego.enabled', 'Guide Négo : l''application mobile entière.', false, 0)
ON CONFLICT (key) DO NOTHING;
```

- API et front : `cd backend && cargo run -p api` · `cd frontend && npm run dev`. Ouvrir sur l'adresse exacte d'`APP_PUBLIC_URL`.
- Le hors-connexion se vérifie sur une **construction**, pas sur le serveur de développement : `npm run build && node .output/server/index.mjs`, une fois sans préfixe, une fois avec `NUXT_APP_BASE_URL=/v2/`.
- Sur téléphone : un service worker exige HTTPS — tunnel, ou `chrome://inspect` avec renvoi de port.

## 1 — Le drapeau ouvre et ferme, sans redéployer

Sans être connecté à aucun compte.

| Geste | Attendu |
|---|---|
| Drapeau éteint, ouvrir `guide-nego/` puis `guide-nego/composants` | Page fermée, au design de Guide Négo, les deux fois |
| `UPDATE platform.feature_flags SET is_enabled = true, rollout_percent = 100 WHERE key = 'guide_nego.enabled';` puis recharger | L'application |
| `rollout_percent = 50`, recharger | Fermée — pas de déploiement progressif sans compte |
| Remettre 100, recharger, **arrêter l'API**, recharger | Toujours ouverte ; « Hors connexion — lu à… » |
| Relancer l'API, `is_enabled = false`, recharger | Fermée |
| Basculer `negotiation.enabled` | Rien ne change dans Guide Négo ; `/negociations` du site suit son drapeau |
| Allumer `negotiation.channels` (à 100), recharger | Cinq onglets, « Échanges » en quatrième |

## 2 — Installée, elle s'ouvre en mode avion

Sur un téléphone Android, drapeau allumé.

1. Ouvrir `guide-nego/installer`, installer, lancer depuis l'icône : plein écran, icône et nom justes.
2. Écran d'ouverture → « Continuer en visiteur ». **Attendre le message « Prête hors connexion »** — sans avoir visité les onglets. Fermer.
3. Mode avion. Relancer depuis l'icône (le lendemain, ou en avançant l'horloge : « lu hier à … ») : l'application s'ouvre, bandeau jaune une fois, rappel dans l'en-tête ensuite ; tous les écrans, la police (« œ », « Œ », « É »), les pictogrammes.
4. Couper le mode avion : « Synchronisé à HH:MM » sans recharger.
5. Thème : forcer « Sombre », fermer, rouvrir en mode avion — sombre d'emblée, sans éclair. Le site, dans le navigateur, garde son thème.
6. iPhone : « Installer » mène aux étapes manuelles ; l'installation par Partager fonctionne.
7. Nouvelle version : reconstruire, relancer deux fois avec réseau, puis en mode avion — elle s'ouvre, et `caches.keys()` ne montre plus qu'un cache `gn-*`.
8. Téléphone réglé en anglais, et cookie `epavillon_locale=en` posé par le site : Guide Négo est en français ; le site reste en anglais après la visite.

### Réseau lent + nouvelle construction

Dans Chrome, application gardée : reconstruire et redémarrer le serveur (nouvel identifiant de construction), puis Réseau → débit bridé « 3G lente » (ou un profil à 50 kb/s).

| Geste | Attendu |
|---|---|
| Ouvrir l'application | Elle s'ouvre en moins de deux secondes, sur la version gardée ; aucun écran vide |
| Application → Service workers | La nouvelle version est « en cours d'installation », puis « en attente » ; deux caches `gn-*` |
| Couper le réseau pendant l'installation | L'installation échoue ; l'ancienne version sert toujours ; un seul cache complet |
| Débit rétabli, fermer puis rouvrir deux fois | La nouvelle version sert ; l'ancien cache a disparu |
| Pendant tout l'essai, éteindre le drapeau | L'application se ferme dès la réponse de l'API, sans attendre la mise à jour |

Dans l'onglet Réseau, une fois l'application gardée : aucune requête vers une autre origine.

## 3 — La page des composants est fidèle

`guide-nego/composants` à côté de `docs/AppNego/design/ecrans/01-systeme.html`, fenêtre à 360 px, en clair puis en sombre, sections 1 à 7 : couleurs, échelle de texte, mesures, 57 pictogrammes, composants livrés et leurs états. Puis à 390 px : barre à cinq onglets sans trou ni débordement. Puis à 320 px, et à 360 px avec la police à 130 % : la barre seule défile, l'onglet actif est dans la vue, aucun libellé tronqué ni sur deux lignes ; à 320 px avec quatre onglets, rien ne défile. Au clavier : ordre de focus, anneau visible. Avec « réduire les animations » : arc et squelette fixes.

## 4 — Contrôles automatiques

```bash
make check-safe
```

`check-front` y gagne trois contrôles : `check-guide-nego` (bornage, imports, jetons, mille lignes, « Programme »), `guide-nego-contrastes` (100 % des paires aux seuils), `node --test frontend/tests/guide-nego/`.

## 5 — Le site n'a pas bougé

Accueil, une page du back-office et `/negociations` : identiques, en clair et en sombre. Aucun service worker enregistré hors de `guide-nego/` (Application → Service workers).

## En partant

`docs/AppNego/progress.md` (état, journal) ; `docs/progression/modele.md` pour la ligne de semis ; ADR-019.

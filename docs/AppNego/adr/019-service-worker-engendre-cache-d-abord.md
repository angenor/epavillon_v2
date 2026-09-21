# ADR-019 — Un service worker engendré à la construction, cache d'abord

**Statut** : accepté — 21/09/2026

## Contexte

Guide Négo doit s'ouvrir sans réseau ([ADR-003](003-tout-ce-qui-se-lit-se-lit-hors-connexion.md)), sous le même toit Nuxt que le site ([ADR-002](002-web-installable-d-abord-capacitor-avant-les-echanges.md)). Le site est redéployé souvent, y compris pendant la COP, et chaque déploiement change l'identifiant de construction. Une navigation « réseau d'abord » donnerait, dans une salle au réseau saturé mais vivant, une page neuve dont les fichiers n'arrivent jamais : écran vide, alors qu'une version complète est dans le téléphone.

`@vite-pwa/nuxt` vise l'application entière : portée à la racine, et un précache qui prendrait tous les fichiers du site, faute de savoir lesquels sont à Guide Négo.

## Décision

- **Un module local**, sur le crochet `build:manifest`, calcule la liste exacte des fichiers de Guide Négo — fermeture des imports de l'entrée, de sa mise en page, de ses pages et du paquet de locale `fr` — et l'inscrit, avec la version, dans `guide-nego/sw.js`. Adresses relatives : le préfixe `/v2/` est sans objet.
- **Portée bornée à `guide-nego/`** : il ne peut contrôler aucune page du site.
- **Tout ou rien à l'installation** : un service worker actif a toujours un cache complet.
- **Navigation cache d'abord, toujours.** L'application s'ouvre sur la version gardée sans attendre le réseau.
- **La nouvelle version se garde sans s'exécuter**, en arrière-plan ; pas de `skipWaiting` ; elle sert à une ouverture suivante, et efface alors l'ancienne.
- **Le drapeau reste lu par l'API** à chaque ouverture : l'arrêt d'urgence n'attend pas la mise à jour.
- La première garde complète se dit à la personne : « Prête hors connexion ».

## Conséquences

- Aucune dépendance ajoutée à l'exécution ; ni module PWA, ni Workbox.
- Une correction déployée n'arrive qu'à la **deuxième** ouverture avec réseau. C'est le prix de « s'ouvre toujours ».
- Pas de service worker en développement : le hors-connexion se vérifie sur une construction, avec et sans préfixe.
- Les réponses d'API ne passent jamais par ce cache : les données gardées vivent dans la garde des lectures, qui sait dire leur heure.
- À revoir avec Capacitor, où la coquille embarque déjà ses fichiers.

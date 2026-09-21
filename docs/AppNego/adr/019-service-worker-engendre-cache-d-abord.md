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
- **La nouvelle version se garde sans s'exécuter**, en arrière-plan. Elle prend la main **au chargement d'une page**, et seulement là : la page écrit `prendre-la-main` au worker en attente, il répond par `skipWaiting()`, et la page se recharge une fois. Sans cela, une version installée attendrait que l'application soit fermée pour de bon — sur un téléphone, ce jour peut ne pas venir. C'est sûr : un worker en attente a, par construction, son cache complet, et une version arrivée pendant l'usage attend le chargement suivant.
- **Les caches de la coquille s'appellent `gn-coquille-<version>`**, et le ménage ne vise que ce préfixe : les données que garderont les étapes suivantes — documents téléchargés — vivent dans leurs propres caches et survivent aux déploiements du site.
- **Ne se retélécharge que ce qui a changé** : à l'installation, un fichier sous `_nuxt/` (nom à empreinte) se copie d'un cache de coquille plus ancien s'il le porte ; seuls la page vide, les traductions, le manifeste, les icônes et les fichiers nouveaux partent au réseau. Une installation interrompue reprend où elle s'était arrêtée. Le tout-ou-rien est conservé.
- **La liste se vérifie contre un serveur réel** : `frontend/scripts/guide-nego-verifier-garde.mjs` demande chaque adresse et refuse toute réponse autre qu'un 200 franc — une redirection comprise, qu'un navigateur refuse de servir à une navigation. Sans ce contrôle, une seule adresse cassée empêcherait toute garde, en silence.
- **Le drapeau reste lu par l'API** à chaque ouverture : l'arrêt d'urgence n'attend pas la mise à jour.
- La première garde complète se dit à la personne : « Prête hors connexion ».

## Conséquences

- Aucune dépendance ajoutée à l'exécution ; ni module PWA, ni Workbox.
- Une correction déployée n'arrive qu'au **deuxième chargement** avec réseau : le premier l'installe, le second la sert. C'est le prix de « s'ouvre toujours », et le drapeau couvre l'urgence.
- Pas de service worker en développement : le hors-connexion se vérifie sur une construction, avec et sans préfixe.
- Les réponses d'API ne passent jamais par ce cache : les données gardées vivent dans la garde des lectures, qui sait dire leur heure.
- À revoir avec Capacitor, où la coquille embarque déjà ses fichiers.

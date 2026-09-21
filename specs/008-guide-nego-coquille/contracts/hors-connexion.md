# Contrat hors connexion — Guide Négo 0a

## Manifeste — `public/guide-nego/manifest.webmanifest`

`name` et `short_name` « Guide Négo » · `lang: fr` · `id`, `start_url`, `scope` : `./` · `display: standalone` · `orientation: portrait` · `background_color: #FFFFFF` · `theme_color: #233400` · icônes relatives : `icones/192.png`, `icones/512.png` (`any`), `icones/192-masque.png`, `icones/512-masque.png` (`maskable`). Aucun chemin absolu.

## Service worker — `guide-nego/sw.js`, engendré à la construction

Écrit par `frontend/modules/guide-nego-garde.ts` (crochet `build:manifest`) depuis `frontend/guide-nego/sw.modele.js` : il y inscrit `VERSION` (l'identifiant de construction) et `LISTE`. Absent en développement. Enregistré par la mise en page : `register(assetUrl('guide-nego/sw.js'), { scope: assetUrl('guide-nego/') })`, puis `registration.update()` à chaque ouverture. Jamais enregistré depuis une page du site.

**`LISTE`** : fermeture des imports — statiques, dynamiques, `css`, `assets` — de l'entrée, de `layouts/guide-nego`, de `pages/guide-nego/**` et du paquet de locale `fr` (`en` n'est pas servi à cette étape) ; plus la page vide (`./`), le manifeste et les icônes. Adresses relatives au service worker.

| Requête | Règle |
|---|---|
| Navigation dans la portée | **Cache d'abord** : la page vide de la version active. Le réseau n'est pas attendu |
| Adresse de `LISTE` | Cache d'abord |
| Autre fichier de même origine | Réseau ; non gardé |
| Appels d'API, autre origine, méthode autre que `GET` | Laissés passer, jamais gardés |

Cycle : `install` garde **toute** `LISTE` dans `gn-<VERSION>`, ou échoue — l'ancienne version continue alors de servir, et le navigateur réessaiera · pas de `skipWaiting` : une version installée attend la fermeture de l'application · `activate` efface les caches `gn-*` des autres versions et prend les pages ouvertes.

**Première garde, dite à la personne** : quand `navigator.serviceWorker.ready` se résout et que `gn.garde-annoncee` est absente, la mise en page affiche le message éphémère « Prête hors connexion » et pose la clé.

Garanties :

- un service worker actif a toujours un cache complet ;
- une nouvelle version en ligne ne ralentit ni ne casse l'ouverture : elle se garde en arrière-plan, sans s'exécuter, et sert à une ouverture suivante ;
- l'arrêt par le drapeau ne dépend pas de cette mise à jour.

## Garde des lectures — `useGnLecture(cle, lire)`

Rend `{ valeur, luA, source, etat }`. `source` ∈ `reseau` · `garde` · `aucune`. N'échoue jamais : un échec de lecture se traduit par `source: 'garde'` ou `'aucune'`. Délai de lecture : 5 s. Ne garde que des objets simples.

## Drapeaux — `resoudreDrapeau(reponse, garde, cle)`

Table de vérité : `research.md` R7. Si la garde porte un état, il vaut aussitôt ; la lecture de l'API se fait en parallèle et s'applique à l'arrivée. Seule la toute première ouverture attend. Garantie : **seule une réponse réussie qui dit « éteint » ferme l'application.**

## Ce que le site ne voit pas

Aucun service worker hors de `guide-nego/` · aucune clé de stockage sans préfixe `gn.` ou hors de la base `guide-nego` · aucun cookie, ni lu ni écrit — `epavillon_locale` compris, donc jamais de `setLocale` · le cookie `epavillon_theme` et l'attribut `data-theme` de `<html>` ne sont ni lus ni écrits.

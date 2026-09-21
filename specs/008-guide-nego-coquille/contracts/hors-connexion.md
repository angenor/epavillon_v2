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

Cycle : `install` garde **toute** `LISTE` dans `gn-coquille-<VERSION>`, ou échoue — l'ancienne version continue alors de servir, et le navigateur réessaiera · `activate` efface les autres caches **de la coquille** et prend les pages ouvertes.

**Le nom des caches est un contrat.** Ceux de la coquille, et eux seuls, s'appellent `gn-coquille-<VERSION>` ; le ménage d'`activate` ne vise que ce préfixe. Une étape qui gardera ses propres données — les documents téléchargés de l'étape 1, par exemple — les met dans un cache à elle, sous un autre nom (`gn-documents…`), qui survit à chaque déploiement du site. **Ne jamais nommer un cache de données `gn-coquille-…`** : il partirait à la mise en ligne suivante.

**La reprise entre versions.** Un déploiement du site change `VERSION` sans changer les fichiers : à l'installation, toute adresse sous `_nuxt/` — nom à empreinte, donc même adresse = même contenu — se copie depuis un cache `gn-coquille-*` plus ancien s'il la porte. Ne partent au réseau que la page vide, les traductions, le manifeste, les icônes et les fichiers nouveaux. Ce qui est déjà dans le cache de la version en cours n'est pas redemandé : une installation interrompue par le réseau reprend où elle s'était arrêtée. Le tout-ou-rien est conservé.

**La prise de main, au chargement d'une page et jamais ailleurs.** Sans `skipWaiting`, une version installée attend que l'application soit fermée pour de bon — sur un téléphone, ce jour peut ne pas venir. Donc : au chargement, si l'inscription porte un worker en attente **et** que la page a déjà un contrôleur, la page lui écrit `prendre-la-main` ; il répond par `skipWaiting()` ; au `controllerchange`, la page se recharge **une seule fois**. C'est sûr — un worker en attente a, par construction, son cache complet — et jamais brutal : une version arrivée pendant l'usage attend le chargement suivant, donc aucune saisie ne se perd aux étapes à venir. La toute première ouverture n'a pas de contrôleur : il n'y a rien à remplacer.

**Vérification.** `frontend/scripts/guide-nego-verifier-garde.mjs <adresse de guide-nego/>` lit `sw.js`, en tire `LISTE` et demande chaque adresse : toute réponse autre que 200, **toute redirection comprise**, échoue. Une réponse redirigée, gardée puis servie à une navigation, est refusée par le navigateur. À lancer sur la construction locale, avec et sans `/v2/`, et après chaque mise en ligne.

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

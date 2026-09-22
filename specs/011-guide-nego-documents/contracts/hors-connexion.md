# Contrat — ce que le téléphone garde, et ce qui l'efface

Il complète celui de 0c (`specs/010-guide-nego-accueil-profil/contracts/hors-connexion.md`) **sans le réécrire**. La garde des lectures (`useGnLecture`), la file d'écritures (`utils/guide-nego/file.ts`) et la mesure de la place (`useGnPlace`) restent ce qu'elles sont.

## Ce qui est gardé, et où

| Donnée | Où | Clé | Effacée par |
|---|---|---|---|
| La liste de la bibliothèque, avec son vocabulaire | Garde des lectures, magasin `lectures` | `documents` | Jamais par « Tout retirer » ; remplacée à chaque lecture réussie |
| Les notes de correction | Garde des lectures | `corrections` | Idem |
| Les favoris du compte | Garde des lectures | `mes-favoris` | La déconnexion, comme tout ce qui est propre au compte |
| **La forme lisible et les images d'un document public** | Cache **`gn-documents-publics`** | Adresse d'API de la ressource | « Tout retirer », « Retirer du téléphone », le document dépublié |
| **Idem pour un document réservé** | Cache **`gn-documents-reserves`** | Idem | Les mêmes, **plus la déconnexion et la perte d'accès** |
| La fiche de chaque copie : version, `reading_etag`, date, place, réservé ou non, pages d'origine gardées | IndexedDB `guide-nego` **v3**, magasin **`copies`** | Identifiant du document | En même temps que la copie |
| Les téléchargements demandés sans réseau | Magasin **`a-telecharger`** | Identifiant du document | Au succès, à l'annulation, à la déconnexion pour un réservé |
| Taille du texte, progression, documents ouverts, derniers ouverts | `localStorage`, avec `try/catch` | `gn-lecture-taille`, `gn-lecture-progression`, `gn-documents-ouverts`, `gn-documents-recents` | Jamais par « Tout retirer » ; la progression d'une version disparue s'oublie à la relecture de la liste |

Les caches ne portent **pas** le préfixe `gn-coquille-`. Le ménage du service worker ne les touche pas, et ils survivent à un déploiement (ADR-019). Le test `sw-garde` gagne le cas nommé.

## Télécharger

1. Si le réseau manque, l'entrée va dans `a-telecharger`, et la fiche dit « Se téléchargera au retour du réseau ».
2. Sinon, la forme lisible se lit en flux. La progression vient des octets reçus sur `Content-Length`. Les images des pages à bloc d'origine se lisent ensuite ; en mode « tel quel », toutes les images.
3. **Rien n'est écrit dans le cache avant le dernier octet reçu.** Les écritures se font ensuite dans le cache du bon côté, public ou réservé, puis dans `copies`.
4. `POST …/downloads` part une fois, sans attendre et sans rien réessayer.
5. Une annulation (`AbortController`) ou une coupure ne laisse **aucune** entrée.
6. `a-telecharger` part sur les déclencheurs de la file de 0c : ouverture, `online`, retour au premier plan. Ce n'est **pas** la file de 0c : ce sont des lectures, sans compte et sans empreinte à protéger.

Il n'y a pas de compte exigé, sauf pour un réservé, qui demande l'accès.

## Une copie est-elle la bonne ?

À chaque lecture réussie de la liste :
- une copie dont le `reading_etag` diffère de celui servi est une copie **d'une autre version du fichier**. Cela n'arrive pas pour un document publié, dont le fichier est figé (FR-007) : on ne retélécharge rien en silence, et la fiche dit « Remplacé par… » si c'est le cas ;
- une copie d'un document **absent** de la liste (dépublié) s'efface ;
- une copie d'un document devenu réservé, alors que la personne n'a pas l'accès, s'efface (FR-034).

## Les effacements

| Geste ou événement | Effacé | Jamais effacé |
|---|---|---|
| « Tout retirer du téléphone » | Les deux caches, `copies`, `a-telecharger` | Les lectures, les réglages, la progression, la file de 0c |
| « Retirer du téléphone » sur une fiche | Les entrées de ce document dans son cache, et sa ligne `copies` | Le reste |
| **Déconnexion** | `gn-documents-reserves` entier, les lignes `copies` et `a-telecharger` réservées ; **puis** la file de 0c, **puis** la fermeture de session (`useGnSession.ts`) | Les copies publiques |
| Accès perdu, lu par `useGnAcces` | Comme la déconnexion, sans fermer la session | Les copies publiques |
| API injoignable, `/auth/refresh` en panne | **Rien** — reprise 1 de 0c | Tout |

**La place** : `useGnPlace` relit l'estimation **après** l'effacement. La baisse est immédiate pour Cache Storage (SC-005).

## Les textes qui changent

- **Déconnexion**, dans `guide-nego.reglages.json` : « Vos téléchargements restent sur le téléphone » devient **« Les documents publics téléchargés restent sur le téléphone ; les documents réservés s'effacent. »** La phrase de la boîte de confirmation change de la même façon.
- **« À propos »**, confidentialité (`guide-nego.a-propos.json`) : « Les téléchargements restent sur ce téléphone. Vos thématiques, vos favoris et les accords que vous donnez suivent votre compte. »
- **« 02 Ouverture »** (`guide-nego.ouverture.json`), écart 41 : `avec-compte.favoris` devient « Favoris, quiz », et le groupe « sans compte » annonce « Documents de négociation, à lire sans réseau ».
- **« Mes téléchargements »** (`guide-nego.telechargements.json`) devient l'entrée de « Mes documents ».

## Tests sans navigateur (`tests/guide-nego/`)

Sur de faux `caches` et un faux IndexedDB, comme `sw-garde` et `file-*` :
- un téléchargement interrompu ne laisse rien ;
- la déconnexion efface les réservés **avant** la session, et garde les publics ;
- la perte d'accès fait de même ;
- une API muette n'efface rien ;
- « Tout retirer » laisse les lectures et la progression ;
- une copie dépubliée s'efface à la relecture ;
- `a-telecharger` ne part qu'une fois, et survit à une fermeture ;
- « Nouveau » : moins de sept jours **et** jamais ouvert ; ouvert, il ne l'est plus ;
- le ménage du service worker laisse `gn-documents-*`.

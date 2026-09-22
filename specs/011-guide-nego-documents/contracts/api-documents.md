# Contrat — les routes publiques des documents

Famille `/negotiation`, crate `negotiation`. **Lecture publique, écriture gardée** ([03-api.md](../../../docs/AppNego/03-api.md)). Les formes de corps sont désignées par leur nom TypeScript, défini dans `frontend/app/types/negotiation-documents.ts`. Le contrat des chemins, verbes et codes est **engendré** par `make openapi`, puis vérifié par `make check-api-contract`.

« Accès » veut dire `identity.has_permission(personne, 'negotiation.space.access', global)`, la règle de 0b.

## Lectures

| Verbe et chemin | Compte | Réponse | Empreinte | Notes |
|---|---|---|---|---|
| `GET /negotiation/documents` | facultatif | `DocumentLibrary` | oui, `304` | La liste entière des documents publiés, avec leur vocabulaire. Varie selon la personne : `Cache-Control: private, no-cache` |
| `GET /negotiation/documents?q=<texte>` | facultatif | `DocumentTextHits` | non | Recherche dans le texte (R8). Pour un réservé sans accès : l'identifiant seul, **ni page ni extrait** |
| `GET /negotiation/documents/{id}/reading` | selon l'accès | `DocumentReading` | oui, figée | La forme lisible entière : pages, sommaire, mode. Réservé sans accès : `403 NEGOTIATION_DOCUMENT_RESTRICTED` |
| `GET /negotiation/documents/{id}/pages/{index}/image` | selon l'accès | `image/jpeg` | oui, figée | L'image d'une page, lue dans le bucket privé et servie par l'API. `Cache-Control: private` pour un réservé, `public, max-age` sinon |
| `GET /negotiation/documents/corrections` | facultatif | `CorrectionNoteList` | oui, `304` | Les notes vivantes de tous les documents publiés. Celles d'un réservé ne vont qu'à qui a l'accès |
| `GET /negotiation/me/bookmarks` | exigé | `DocumentBookmarkList` | oui, `304` | Les identifiants des favoris et leur date |

### `DocumentLibrary`

- **`documents[]`** — un élément par document publié :
  - `id`, `slug`, `version` ;
  - `title` et `summary`, déjà **résolus** dans la langue demandée avec repli sur le français, `summary` valant `null` pour un réservé sans accès ;
  - `type` (code du terme) ;
  - `themes[]` (codes), **vide et masqué** pour un réservé sans accès : le champ `themes_hidden` vaut alors `true` ;
  - `cop`, l'identifiant de l'édition, ou `null` ;
  - `issued_on`, `published_at`, `publisher`, `locale` ;
  - `source` : `"file"` ou `"link"` ;
  - `external_url`, pour un lien ;
  - `link_host`, par exemple `enb.iisd.org` ;
  - `restricted` et `accessible` ;
  - `page_count`, `reading_bytes` et `mode` (`"reflow"` ou `"as_is"`), pour un fichier ;
  - `superseded_by`, soit `{ id, title, published_at, page_count }`, le bout publié de la chaîne (R18), soit `null` ;
  - `reading_etag`, l'empreinte de la forme lisible, qui permet au téléphone de savoir si sa copie est la bonne.
- **`vocabulary`** — les libellés des seules valeurs citées, résolus :
  - `types` : `{ code, label, sort_order }` ;
  - `themes` : `{ code, label, sort_order }`, actifs seulement ;
  - `cops` : `{ id, label, city }`.
- **`served_at`** — l'heure du serveur.

### `DocumentReading`

`{ id, version, mode, page_count, outline: OutlineEntry[], pages: ReadingPage[] }`, dans la grammaire de [forme-lisible.md](forme-lisible.md).

En mode `as_is`, `pages[]` ne porte que `index`, `label` et `image`. En mode `reflow`, `image` n'est présent que sur les pages à bloc d'origine (R2).

## Écritures

| Verbe et chemin | Compte | Corps | Réponse | Notes |
|---|---|---|---|---|
| `POST /negotiation/documents/{id}/downloads` | **aucun** | — | `204` | Compteur (FR-035). Rien de la personne n'est gardé. Réservé sans accès : `403` |
| `PUT /negotiation/me/bookmarks/{document_id}` | exigé | — | `204` | Idempotent. Document inconnu ou non publié : `404 NEGOTIATION_DOCUMENT_NOT_FOUND` |
| `DELETE /negotiation/me/bookmarks/{document_id}` | exigé | — | `204` | Idempotent, même si le favori n'existe pas |

Un favori sur un réservé sans accès est permis : il ne dévoile rien que la liste ne montre déjà.

## Codes d'erreur nouveaux (catalogue de `kernel`)

| Code | HTTP | Message |
|---|---|---|
| `NEGOTIATION_DOCUMENT_NOT_FOUND` | 404 | « Ce document n'existe pas, ou n'est plus publié. » |
| `NEGOTIATION_DOCUMENT_RESTRICTED` | 403 | « Ce document est réservé aux négociatrices et négociateurs. Saisissez votre code d'invitation pour l'ouvrir. » |
| `NEGOTIATION_DOCUMENT_NOT_READABLE` | 409 | « Ce document ne se lit pas dans l'application : ouvrez-le dans le navigateur. » — demande de forme lisible pour un lien |

## Tests exigés (sur base réelle)

- **Chemin nominal** de chaque route.
- **`304`** sur `If-None-Match` pour la liste, les notes et les favoris.
- **Réservé** :
  - sans compte, avec un compte sans accès, et par identifiant forgé ;
  - la liste masque le résumé et les thématiques ;
  - `reading`, `image` et `downloads` rendent `403` ;
  - la recherche ne rend ni page ni extrait.
- **Remplacement** : un document remplacé par un brouillon n'est **pas** marqué remplacé ; une chaîne de trois mène au bout.
- **Dépublié** : absent de la liste ; son `reading` rend `404`.
- **Favori** : `PUT` deux fois, `DELETE` deux fois, aucune erreur.

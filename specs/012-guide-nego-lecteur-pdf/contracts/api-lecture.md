# Contrat — le PDF servi, la lecture, le choix « Texte agrandi »

Ce contrat **modifie** [api-documents.md](../../011-guide-nego-documents/contracts/api-documents.md) et [api-admin-documents.md](../../011-guide-nego-documents/contracts/api-admin-documents.md) de l'étape 1. Ce qu'il ne nomme pas ne change pas. Il est engendré dans `frontend/app/types/api.ts` par `make openapi`, et vérifié par `make check-api-contract`.

---

## Route nouvelle — le fichier

### `GET /negotiation/documents/{id}/file` · `HEAD` identique sans corps

| | |
|---|---|
| Accès | Comme `reading` : publié, sinon `404 NEGOTIATION_DOCUMENT_NOT_FOUND` ; réservé sans `negotiation.space.access` (portée globale), `403 NEGOTIATION_DOCUMENT_RESTRICTED` ; lien externe, `409 NEGOTIATION_DOCUMENT_NOT_READABLE`. **Vérifié à chaque requête**, morceau compris (FR-030) |
| Sans `Range` | `200`, corps entier, `Content-Length` — c'est le téléchargement de la copie |
| `Range: bytes=a-b`, `bytes=a-`, `bytes=-n` | `206`, `Content-Range: bytes a-b/total`, `Content-Length` de la plage. Une seule plage ; plusieurs plages → la première seule, comme le permet la RFC 9110 |
| Plage hors du fichier | `416`, `Content-Range: bytes */total`, code **`NEGOTIATION_DOCUMENT_RANGE_INVALID`** (nouveau) |
| `If-None-Match` égal à l'empreinte | `304` |
| En-têtes, toujours | `Content-Type: application/pdf` · `Accept-Ranges: bytes` · `ETag: "<asset_id>"` (fort : un fichier publié ne change jamais en place) · `Content-Disposition: inline` · **aucun `Content-Encoding`** |
| Cache | Public : `Cache-Control: private, max-age=3600, no-transform`. Réservé : `Cache-Control: private, no-store, no-transform` (FR-029) |
| Lecture du stockage | `kernel::storage::ObjectStore::get_range` (nouveau), jamais l'objet entier pour une plage |

Le compteur de téléchargements reste `POST …/downloads`, envoyé par le client une fois la copie gardée ; la route du fichier ne compte rien, puisque pdf.js l'appelle par dizaines de morceaux.

**Origine croisée (poste de développement)** : le contrôle d'origine accepte `Range` et `If-None-Match` en préflight et expose `Content-Range`, `Accept-Ranges`, `Content-Length`, `ETag`. En production, site et API partagent l'hôte.

---

## Routes changées

### `GET /negotiation/documents` — `DocumentLibrary`

Pour un fichier, `mode` disparaît ; paraissent :

| Champ | Type | Règle |
|---|---|---|
| `has_text` | `boolean` | `negotiation.document_reading_modes().has_text` |
| `large_text` | `boolean` | `…large_text` — « Texte agrandi » offert |
| `reading_bytes` | `number` | PDF + JSON de lecture (R5) : la taille annoncée de la copie |

L'empreinte de la liste change avec eux.

### `GET /negotiation/documents/{id}/reading` — `DocumentReading`

`{ id, version, has_text, large_text, page_count, outline, pages }`.

- `mode` disparaît ; `pages[].image` disparaît.
- `pages[]` porte `index`, `label`, `blocks` — `blocks` vide pour une page sans texte. Toujours servie, même sans texte extrait : les numéros imprimés en viennent.
- Le bloc `origin` garde `reason` et `text?` ; le client en fait un renvoi vers la page (R8).
- L'empreinte : `(id, asset_id, extracted_at, large_text_choice)`.

### `GET /negotiation/documents/{id}/pages/{index}/image` — **supprimée**

Plus aucune image de page ne va au téléphone (FR-039). Les tests de l'étape 1 qui la visent passent sur la route du fichier.

---

## Back-office

### `PUT /admin/negotiation/documents/{id}/large-text` — remplace `…/as-is`

| | |
|---|---|
| Permission | `negotiation.document.publish`, portée globale, filtrage par périmètre comme les autres routes |
| Corps | `{ "choice": true \| false \| null }` — `null` rend la main au verdict |
| Réponse | `AdminDocument`, avec l'extraction à jour |
| Refus | `422` s'il n'y a pas d'extraction (inchangé) ; pas de condition sur le statut, un brouillon se règle avant publication |
| Écriture | Contexte d'écriture posé, audit par `tg_audit` ; le téléphone suit à la prochaine lecture de la liste, dont l'empreinte change |

### `GET /admin/negotiation/documents/{id}/preview` — `AdminDocumentPreview`

`extraction.serve_as_is` devient `extraction.large_text_choice: boolean | null`, et s'ajoutent `extraction.has_text` et `extraction.large_text`, lus par la même fonction. Les images de page restent (R11). Un `failed` porte `failure_reason`, que l'aperçu affiche ; un PDF protégé par mot de passe le nomme (R12).

### `POST /admin/negotiation/documents/{id}/publish`

Inchangé : une extraction `ready` exigée. La mention « ou tel quel avec images » du contrat de l'étape 1 n'a jamais été écrite dans le code ; elle disparaît du contrat.

---

## Code d'erreur nouveau (catalogue de `kernel`)

| Code | HTTP | Message |
|---|---|---|
| `NEGOTIATION_DOCUMENT_RANGE_INVALID` | 416 | « La partie demandée du document n'existe pas. » |

## Codes retirés

Aucun. `NEGOTIATION_DOCUMENT_NOT_READY` garde son sens.

---

## Tests exigés (sur base réelle)

- **Plages** : `206` et `Content-Range` justes pour `a-b`, `a-`, `-n` ; `200` sans `Range` ; `416` au-delà ; `304` sur l'empreinte ; aucun `Content-Encoding` ; `Accept-Ranges` présent ; `HEAD` sans corps.
- **Réservé, à chaque morceau** : sans session, session sans accès, adresse forgée d'un brouillon, document dépublié entre deux morceaux — `403` ou `404`, jamais un octet (SC-009).
- **Cache** : `no-store` pour un réservé, jamais pour un public.
- **Liste et lecture** : `has_text`, `large_text` pour les quatre cas de la fonction ; `mode` et `image` absents.
- **`large-text`** : `true`, `false`, `null` ; relance d'extraction qui garde le choix ; URL forgée d'un administrateur d'événement refusée.
- **`get_range`** : sur le stockage de fichiers de test, bornes comprises.

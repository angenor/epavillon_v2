# Contrat — le back-office des documents et des notes

Routes plates `/admin/negotiation/...`, crate `negotiation`, comme celles de 0b (`lib.rs:54-56`). **Portée globale** : un administrateur d'événement ne passe pas, et un identifiant forgé non plus. Les formes sont définies dans `frontend/app/types/admin-negotiation-documents.ts`.

Chaque écriture ouvre `state.db().write(&ctx)`, qui pose `app.actor_id` et `app.request_id`, et laisse une trace d'audit.

## Qui peut quoi

| Permission (portée globale) | Détenue par | Ouvre |
|---|---|---|
| `negotiation.document.publish` | `admin`, `super_admin` (existe) | Tout le tableau ci-dessous, sauf poser et retirer une note |
| `negotiation.correction.post` | `expert`, `super_admin` (nouveau) | Lire la liste et l'aperçu ; poser une note |
| `negotiation.correction.withdraw` | `expert`, `super_admin` (nouveau) | Retirer une note |

Les deux lectures ouvertes à l'une **ou** l'autre permission se testent dans le gestionnaire, par `has_permission` en portée globale. Un test de source, sur le patron de `tests/perimetre_url_forgee.rs`, vérifie que **chaque** route déclare sa garde.

## Documents

| Verbe et chemin | Garde | Corps → réponse | Notes |
|---|---|---|---|
| `GET /admin/negotiation/documents` | publish **ou** correction.post | → `AdminDocumentList` | État (brouillon, publié, dépublié), type, version, remplacement, extraction |
| `POST /admin/negotiation/documents` | publish | `AdminDocumentInput` → `AdminDocument` | Crée un **brouillon** sans source (R5) |
| `GET /admin/negotiation/documents/{id}` | publish **ou** correction.post | → `AdminDocument` | Avec l'état d'extraction |
| `PATCH /admin/negotiation/documents/{id}` | publish | `AdminDocumentInput` partiel → `AdminDocument` | Titres, résumé, type, thématiques (remplacement en bloc), COP, version, date, éditeur, langue, remplace, réservé, marqueur, lien externe |
| `PUT /admin/negotiation/documents/{id}/file` | publish | `{ asset_id }` → `AdminDocument` | Attache le PDF déposé par la garde média, et met l'extraction en file dans la même transaction. Refusé sur un document publié : `NEGOTIATION_DOCUMENT_FILE_LOCKED` |
| `POST /admin/negotiation/documents/{id}/extraction` | publish | → `202` | Relance l'extraction |
| `PUT /admin/negotiation/documents/{id}/as-is` | publish | `{ serve_as_is: bool }` → `AdminDocument` | « Ouvrir tel quel », sans republier |
| `POST /admin/negotiation/documents/{id}/publish` | publish | → `AdminDocument` | Exige une source, et pour un fichier une extraction `ready` ou « tel quel » avec images : sinon `NEGOTIATION_DOCUMENT_NOT_READY` |
| `POST /admin/negotiation/documents/{id}/unpublish` | publish | → `AdminDocument` | Pose `unpublished_at` |
| `POST /admin/negotiation/documents/{id}/new-version` | publish | → `AdminDocument` | Brouillon prérempli, désigné comme remplaçant ; version à saisir |
| `DELETE /admin/negotiation/documents/{id}` | publish | → `204` | **Brouillon jamais publié seulement** ; efface ses liens de thématiques dans la même transaction (R16). Sinon `NEGOTIATION_DOCUMENT_PUBLISHED_UNDELETABLE` |
| `GET /admin/negotiation/documents/{id}/preview` | publish **ou** correction.post | → `AdminDocumentPreview` | Verdict, indicateurs, sommaire, et pour chaque page : `index`, `label`, `blocks`, l'adresse de l'image |
| `GET /admin/negotiation/documents/{id}/file` | publish **ou** correction.post | → `application/pdf` | Le PDF d'origine, lu dans le bucket privé |
| `GET /admin/negotiation/documents/{id}/pages/{index}/image` | publish **ou** correction.post | → `image/jpeg` | L'image d'une page, **y compris d'un brouillon** — la route publique refuse un document non publié |


## Notes de correction

| Verbe et chemin | Garde | Corps → réponse | Notes |
|---|---|---|---|
| `GET /admin/negotiation/documents/{id}/corrections` | publish **ou** correction.post | → `AdminCorrectionNoteList` | Vivantes **et** retirées, avec les auteurs et les dates |
| `POST /admin/negotiation/documents/{id}/corrections` | correction.post | `{ page_index, passage?, body: { fr, en? } }` → `AdminCorrectionNote` | Page hors du document : `NEGOTIATION_CORRECTION_PAGE_UNKNOWN`, traduit du déclencheur |
| `POST /admin/negotiation/corrections/{note_id}/withdraw` | correction.withdraw | → `AdminCorrectionNote` | Idempotent. Jamais de suppression |

## Dépôt du PDF (garde média, pas une route nouvelle)

`POST /media/assets` avec `owner_schema=negotiation`, `owner_table=documents`, `owner_id=<brouillon>`, sans rôle — `media.attachment_role` n'en a aucun pour une source, et la garde n'en demande pas —, un fichier `application/pdf`. La garde exige `negotiation.document.publish` et impose `visibility=private`, donc le bucket privé (R4).

Le champ `FileField` du back-office s'en charge, puis appelle `PUT …/file`.

## Codes d'erreur nouveaux

| Code | HTTP | Message |
|---|---|---|
| `NEGOTIATION_DOCUMENT_SOURCE_BOTH` | 422 | « Un document est un fichier ou un lien, jamais les deux. » — traduit de `ck_documents_source_at_most_one` |
| `NEGOTIATION_DOCUMENT_SOURCE_MISSING` | 422 | « Déposez un fichier ou indiquez un lien avant de publier. » — traduit de `ck_documents_published_has_source` |
| `NEGOTIATION_DOCUMENT_NOT_READY` | 409 | « L'extraction n'est pas terminée. Attendez-la, ou choisissez « ouvrir tel quel ». » |
| `NEGOTIATION_DOCUMENT_FILE_LOCKED` | 409 | « Le fichier d'un document publié ne change pas. Publiez une nouvelle version. » |
| `NEGOTIATION_DOCUMENT_ALREADY_SUPERSEDED` | 409 | « Ce document est déjà remplacé par « {titre} ». » — traduit de `ux_documents_supersedes` |
| `NEGOTIATION_DOCUMENT_SUPERSEDE_CYCLE` | 409 | « Ce remplacement formerait une boucle. » |
| `NEGOTIATION_DOCUMENT_UNKNOWN_THEME` | 400 | « Cette thématique n'existe pas : {code}. » |
| `NEGOTIATION_DOCUMENT_UNKNOWN_TYPE` | 400 | « Ce type de document n'existe pas. » — traduit de `tg_check_term_taxonomy` |
| `NEGOTIATION_DOCUMENT_PUBLISHED_UNDELETABLE` | 409 | « Un document publié ne se supprime pas : dépubliez-le. » |
| `NEGOTIATION_CORRECTION_PAGE_UNKNOWN` | 422 | « Cette page n'existe pas dans le document. » |

Codes réutilisés : `NEGOTIATION_DOCUMENT_NOT_FOUND`, et les codes génériques d'autorisation.

## Événements

**Aucun événement d'outbox à cette étape** (principe IV, sans objet) : ni l'assistant ni les notifications n'existent. L'extraction est un **travail** (`platform.jobs`), pas un événement.

À l'étape 3b, « nouveau document » et « note posée » pourront être émis sans rien défaire.

## Tests exigés (sur base réelle)

- **Chemin nominal** : brouillon, dépôt, extraction simulée sur un petit PDF de test, aperçu, publication, nouvelle version, dépublication.
- **Refus par URL forgée** et administrateur d'événement refusé, **sur chaque route**.
- **Chaque invariant traduit** : les deux, source manquante, deux successeurs, boucle, thématique inconnue, page inconnue, fichier figé.
- **L'expert** lit l'aperçu, pose et retire une note, et **ne peut ni publier ni modifier**. L'administrateur publie, mais **ne pose pas** de note.
- **Suppression d'un brouillon** : ses liens `entity_terms` ont disparu.
- **Audit** : chaque écriture laisse son auteur.

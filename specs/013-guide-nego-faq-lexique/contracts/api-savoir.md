# Contrat — le savoir côté application

Routes plates `/negotiation/...`, crate `negotiation`. Formes TypeScript dans `frontend/app/types/negotiation-savoir.ts`. Lecture publique, écriture gardée (03-api.md). Toute écriture ouvre `state.db().write(&ctx)`.

## Le paquet

| Verbe et chemin | Garde | Réponse | Notes |
|---|---|---|---|
| `GET /negotiation/knowledge` | aucune | `KnowledgeBundle` | Tout le publié, `complete: true`, textes dans la langue demandée. `ETag` = `negotiation.knowledge_fingerprint()` et la langue ; `Vary: Accept-Language` ; `If-None-Match` → `304` ; `Cache-Control: public, no-cache` |
| `GET /negotiation/knowledge?since=<served_at>` | aucune | `KnowledgeBundle` | `complete: false` : entrées changées depuis `since − 5 min`, identifiants sortis dans `removed` (seulement les entrées déjà publiées une fois : un brouillon jamais servi n'a rien à retirer). Parcours, vocabulaires et `most_read` toujours entiers (R2) |

`KnowledgeBundle` :

```text
served_at, complete
faq_sections[]      { code, label, icon, sort_order }
glossary_families[] { code, label, sort_order }
faq[]               { id, section_code, question, answer, status, verified_on,
                      sources[], related_ids[], updated_at }
glossary[]          { id, slug, family_code, term, acronym, variants[], translation,
                      definition, heard_in_room, sources[], related_ids[], status, updated_at }
pathway             { groups[] { id, label, sort_order,
                      steps[] { id, label, detail, origin_label, link, sort_order } } }
                    link : { kind: document | faq | glossary, target_id, page, section, label } | null
most_read[]         identifiants d'entrées de FAQ, trois au plus : lectures des 30 derniers jours (jour de Paris), puis editorial_rank
removed             { faq[], glossary[] }   — vide si complete
```

`status` vaut `published` ou `to_review` : les brouillons ne sortent jamais. `related_ids` porte **tous** les liens, publiés ou non : le téléphone les filtre sur les entrées qu'il a, puisqu'une différence ne renvoie pas le parent quand une entrée liée change d'état. `sources[]` : `{ document_id?, document_title?, external_title?, external_url?, section_label?, page_from?, page_to?, quote? }` — le titre du document est joint pour la lecture sans la bibliothèque ; une clé nulle est omise.

## Lectures et retours sur la FAQ

| Verbe et chemin | Garde | Corps → réponse | Notes |
|---|---|---|---|
| `POST /negotiation/faq/{id}/read` | aucune | → `204` | Compteur du jour. Entrée non publiée : `204` quand même, rien n'est compté |
| `PUT /negotiation/faq/{id}/feedback` | session | `{ helpful, missing_reason? }` → `FaqFeedback` | Écrase la voix de la personne. `outdated` ouvre aussi un signalement `from_feedback`, une fois |
| `POST /negotiation/faq/{id}/reports` | session | `{ client_ref, reasons[], details? }` → `201 FaqReportReceipt` | Rejoué avec le même `client_ref` : `200`, même reçu. Plafond : `NEGOTIATION_REPORT_LIMIT` (429) |
| `GET /negotiation/me/faq-feedback` | session | → `MyFaqFeedback` sous `ETag` | Les voix de la personne, pour réafficher « Merci. » |

## Questions aux experts

| Verbe et chemin | Garde | Corps → réponse | Notes |
|---|---|---|---|
| `POST /negotiation/me/questions` | `negotiation.space.access` | `{ client_ref, theme_code, body, consent_to_faq }` → `201 MyQuestion` | Rejeu : `200`. Sans l'accès : `403` du verrou de 0b |
| `GET /negotiation/me/questions` | `negotiation.space.access` | → `MyQuestionList` sous `ETag` | État, réponse, date et signature « Nom, expert IFDD », comme les notes de correction. Vidé du téléphone à la déconnexion |

## Parcours et favoris

| Verbe et chemin | Garde | Corps → réponse | Notes |
|---|---|---|---|
| `GET /negotiation/me/pathway` | session | → `MyPathway { step_ids[] }` sous `ETag` | |
| `PUT /negotiation/me/pathway/{step_id}` | session | → `204` | Idempotent (R7) |
| `DELETE /negotiation/me/pathway/{step_id}` | session | → `204` | Idempotent |
| `GET /negotiation/me/glossary-favorites` | session | → `MyGlossaryFavorites { entry_ids[] }` sous `ETag` | |
| `PUT` / `DELETE /negotiation/me/glossary-favorites/{entry_id}` | session | → `204` | Idempotents |

## Termes proposés

| Verbe et chemin | Garde | Corps → réponse | Notes |
|---|---|---|---|
| `POST /negotiation/glossary/proposals` | session | `{ client_ref, term, context? }` → `201 ProposalReceipt` | S'ajoute à une proposition en attente du même terme normalisé. Terme déjà publié : `NEGOTIATION_GLOSSARY_TERM_EXISTS` (409) avec son `slug`. Plafond : `NEGOTIATION_PROPOSAL_LIMIT` (429) |

## Codes d'erreur nouveaux

`NEGOTIATION_FAQ_NOT_FOUND` 404 · `NEGOTIATION_GLOSSARY_NOT_FOUND` 404 · `NEGOTIATION_PATHWAY_STEP_NOT_FOUND` 404 · `NEGOTIATION_REPORT_LIMIT` 429 · `NEGOTIATION_PROPOSAL_LIMIT` 429 · `NEGOTIATION_GLOSSARY_TERM_EXISTS` 409 · `NEGOTIATION_REPORT_REASON_REQUIRED` 422 · `NEGOTIATION_TEXT_TOO_LONG` 422 (600 caractères) · plus les neuf du back-office — dix-sept en tout. Messages en français au catalogue de `kernel/src/error.rs`, section « Guide Négo, savoir (étape 2) ».

## Côté téléphone

- `useGnSavoir()` : `useGnLecture('savoir', …)`, fusion par `utils/guide-nego/savoir.ts::fusionner` (R3).
- Écritures par la file de 0c : `savoir.retour.<id>` (dernière voix gardée), `savoir.signalement.<client_ref>`, `savoir.question.<client_ref>`, `savoir.terme.<client_ref>`, famille `parcours.<step_id>` et `lexique.favori.<entry_id>` (R7, R8).
- `POST …/read` hors file, une fois par entrée, par jour et par téléphone (R12).

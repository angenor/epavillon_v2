# Contrat — le back-office du savoir et la file des experts

Routes plates `/admin/negotiation/...`, **portée globale** (`Requires<P>`), comme l'étape 1. Formes dans `frontend/app/types/admin-negotiation-savoir.ts`. Chaque écriture : `state.db().write(&ctx)`, audit. Un test de source vérifie que **chaque** route déclare sa garde, et un test d'URL forgée refuse un administrateur d'événement.

## Qui peut quoi

| Permission (globale) | Détenue par | Ouvre |
|---|---|---|
| `negotiation.knowledge.publish` | `admin`, `expert`, `super_admin` | Rédiger, publier, dépublier, mettre « À revoir », supprimer un brouillon jamais publié : FAQ, parcours, lexique |
| `negotiation.knowledge.review` | `expert`, `super_admin` | Dater une vérification ; la file : signalements, questions, termes proposés |

Les lectures des listes et des fiches sont ouvertes à l'une **ou** l'autre (extracteur `LectureSavoir`, patron de `LectureDocuments`).

## FAQ

| Verbe et chemin | Garde | Corps → réponse | Notes |
|---|---|---|---|
| `GET /admin/negotiation/faq` | publish ou review | `?q=&section=&status=` → `AdminFaqList` | `q` par trigramme sur `question_norm` ; compte des signalements ouverts par entrée |
| `POST /admin/negotiation/faq` | publish | `AdminFaqInput` → `AdminFaqEntry` | Brouillon |
| `GET /admin/negotiation/faq/{id}` | publish ou review | → `AdminFaqEntry` | Avec sources, liées, retours « Oui / Non » comptés, signalements |
| `PATCH /admin/negotiation/faq/{id}` | publish | `AdminFaqInput` partiel → `AdminFaqEntry` | Sources et liées remplacées en bloc |
| `POST /admin/negotiation/faq/{id}/verify` | review | `{ verified_on? }` → `AdminFaqEntry` | Défaut : aujourd'hui, fuseau de Paris ; pose `verified_by` ; une entrée « À revoir » revient `published` |
| `POST /admin/negotiation/faq/{id}/publish` | publish | → `AdminFaqEntry` | Sans vérification : `NEGOTIATION_FAQ_UNVERIFIED` (traduit de `ck_faq_entries_verified`) |
| `POST /admin/negotiation/faq/{id}/to-review` | publish | → `AdminFaqEntry` | Reste visible (tranché le 25/09) |
| `POST /admin/negotiation/faq/{id}/unpublish` | publish | → `AdminFaqEntry` | Revient `draft` |
| `DELETE /admin/negotiation/faq/{id}` | publish | → `204` | Jamais publiée seulement : sinon `NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE` |

## Lexique

Mêmes routes sous `/admin/negotiation/glossary`, sans `verify` : `GET` (liste, `?q=&family=&status=`), `POST`, `GET {id}`, `PATCH {id}`, `POST {id}/publish`, `POST {id}/to-review`, `POST {id}/unpublish`, `DELETE {id}`. Le `slug` est rendu, jamais accepté en entrée. `NEGOTIATION_GLOSSARY_SLUG_TAKEN` (409) si deux termes se normalisent pareil.

## Parcours

| Verbe et chemin | Garde | Corps → réponse |
|---|---|---|
| `GET /admin/negotiation/pathway` | publish ou review | → `AdminPathway` (groupes et étapes, publiés ou non) |
| `POST /admin/negotiation/pathway/groups` · `PATCH …/groups/{id}` · `DELETE …/groups/{id}` | publish | groupe ; suppression refusée s'il porte des étapes |
| `POST /admin/negotiation/pathway/steps` · `PATCH …/steps/{id}` · `DELETE …/steps/{id}` | publish | étape ; une étape déjà cochée se dépublie plutôt que se supprimer |
| `PUT /admin/negotiation/pathway/order` | publish | `{ groups: [{ id, step_ids[] }] }` → `AdminPathway` |

## La file des experts

| Verbe et chemin | Garde | Corps → réponse | Notes |
|---|---|---|---|
| `GET /admin/negotiation/queue` | review | `?kind=reports|questions|proposals` → `ExpertQueue` | Les plus anciens d'abord ; comptes par sorte. **Aucun auteur** (R9) |
| `POST /admin/negotiation/queue/reports/{id}/close` | review | `{ outcome }` → `AdminFaqReport` | `revised | confirmed | dismissed` |
| `POST /admin/negotiation/queue/questions/{id}/answer` | review | `{ answer }` → `AdminQuestion` | Met en file `negotiation.expert_question.answered_email` dans la même transaction (R11) |
| `POST /admin/negotiation/queue/questions/{id}/promote` | review | → `AdminFaqEntry` | Brouillon de FAQ né de la question et de la réponse ; `status = added_to_faq`. Sans consentement : `NEGOTIATION_QUESTION_NO_CONSENT` |
| `POST /admin/negotiation/queue/proposals/{id}/accept` | review | `AdminGlossaryInput` → `AdminGlossaryEntry` | Crée l'entrée en brouillon ; les auteurs reçoivent leur courriel **à sa publication** |
| `POST /admin/negotiation/queue/proposals/{id}/reject` | review | `{ reason }` → `AdminProposal` | |
| `GET /admin/negotiation/queue/proposals/{id}` | review | → `AdminProposal` | Contextes des auteurs, sans les auteurs ; entrées proches par `similarity` (R5) |

Élément déjà traité : `NEGOTIATION_QUEUE_ITEM_CLOSED` (409).

## Codes d'erreur du back-office

`NEGOTIATION_FAQ_UNVERIFIED` 422 (`ck_faq_entries_verified`) · `NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE` 409 · `NEGOTIATION_GLOSSARY_SLUG_TAKEN` 409 · `NEGOTIATION_QUESTION_NO_CONSENT` 422 (`ck_expert_questions_promotion`) · `NEGOTIATION_QUEUE_ITEM_CLOSED` 409 · `NEGOTIATION_SOURCE_TARGET_INVALID` 422 (`ck_knowledge_sources_owner`, `ck_knowledge_sources_target`) · `NEGOTIATION_PATHWAY_LINK_INVALID` 422 (`ck_pathway_steps_link`) · `NEGOTIATION_PATHWAY_GROUP_NOT_EMPTY` 409 · `NEGOTIATION_RELATED_SELF` 422 (`ck_faq_related_not_self` et son pendant du lexique). Avec les huit de [api-savoir.md](api-savoir.md) : **dix-sept codes**.

## Écrans

`pages/admin/negociations/faq/{index,[id]}.vue`, `lexique/{index,[id]}.vue`, `parcours/index.vue`, `file/index.vue` ; composants dans `components/admin/negotiation/`, bâtis sur `ui/` (`UiTable`, `UiSearchInput`, `UiSelect`, `UiFormField`, `UiInput`, `UiTextarea`, `UiCombobox`, `UiDatePicker`, `UiBadge`, `UiModal`, `UiAlert`, états vide, erreur, refusé, chargement). Menu : section « Guide Négo » de `layouts/admin.vue`, entrées gardées par les deux permissions.

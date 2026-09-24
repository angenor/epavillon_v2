# Modèle de données — Guide Négo, étape 1b

**Source de vérité** : `docs/database/100_negotiations.sql`, modifié **avant** le code. La base locale se migre par `migration.sql` (rejouable, jouée deux fois, schémas comparés), **à la suite** de celle de l'étape 1 ; aucune n'est encore en production. `docs/progression/modele.md` le consigne (ADR-017).

Le changement est petit : une colonne change de sens, une fonction naît, un poids se recalcule. Rien ne se crée pour les positions du texte (R7) ni pour le mode de lecture, qui reste sur l'appareil.

---

## 1. `negotiation.document_renditions` — « ouvrir tel quel » devient « Texte agrandi »

| Avant (étape 1) | Après | Note |
|---|---|---|
| `serve_as_is boolean NOT NULL DEFAULT false` | `large_text_choice boolean NULL` | **NULL = suit le verdict de l'extraction** ; vrai ou faux = choix de l'administratrice (arbitrage du 24/09). Modifiable sans republier. Une relance d'extraction le garde ; un nouveau fichier fait une nouvelle ligne, qui repart de NULL |
| `reading_bytes` = texte + images des pages à tableau ou figure | `reading_bytes` = **octets du PDF + octets du JSON de lecture** | La taille de la copie gardée (R5), annoncée sur la fiche. Calculée à la fin de l'extraction |

Migration : `ALTER … RENAME COLUMN serve_as_is TO large_text_choice`, `DROP NOT NULL`, `DROP DEFAULT`, puis `UPDATE … SET large_text_choice = NULL` — les choix « tel quel » des postes d'essai n'ont pas de sens dans le nouveau modèle. `COMMENT ON COLUMN` réécrit. `reading_bytes` se recalcule par une relance d'extraction des documents existants (`POST …/extraction`), pas en SQL : le poids du JSON est celui que l'API sert.

`is_reflowable`, `quality`, `outline`, `status` et les contraintes ne changent pas.

## 2. `negotiation.document_reading_modes(p_document_id uuid)` — nouvelle

Fonction `STABLE`, `SECURITY INVOKER`, qui rend une ligne :

| Colonne | Règle | Sert |
|---|---|---|
| `has_text` | une page au moins du document a `plain_text <> ''` | recherche, sommaire, recherche « un mot du texte » (FR-017, FR-018) |
| `large_text` | `has_text AND coalesce(large_text_choice, is_reflowable, false)` | « Texte agrandi » offert (FR-022) |

Écrite une fois, lue par la liste, la lecture et l'aperçu : la règle ne se réimplémente pas dans le service (principe VIII). Un test SQL couvre les quatre cas — verdict vrai, verdict faux, choix contraire, sans texte malgré un choix vrai.

## 3. `negotiation.document_pages` — inchangée

`image_key`, `image_bytes` et `has_origin_block` **restent**, pour l'aperçu du back-office seul (R11). Leur `COMMENT ON` précise « aperçu du back-office — jamais servi au téléphone depuis l'étape 1b ».

## 4. `negotiation.correction_notes` — inchangée

Page et extrait cité suffisent au repérage sur la couche de texte (R7).

## 5. Ce qui ne va pas en base

| Sur l'appareil | Où | Note |
|---|---|---|
| Le mode de lecture | `localStorage` `gn.lecture-mode` : `pages` \| `texte` | Défaut `pages` |
| L'annonce vue | `localStorage` `gn.lecture-mode-annonce` : `1` | Une fois par téléphone |
| La copie gardée | Caches `gn-documents-publics` / `gn-documents-reserves` ; fiche dans IndexedDB `guide-nego` v3, magasin `copies` | **Format 2** : voir [contracts/copie-gardee.md](contracts/copie-gardee.md) |
| Progression, taille du texte, documents ouverts | Inchangés depuis l'étape 1 | La progression est commune aux deux modes |

## 6. Ce qui change dans les réponses de l'API

Détaillé dans [contracts/api-lecture.md](contracts/api-lecture.md) :

- `mode: 'reflow' | 'as_is'` disparaît de la liste et de la lecture, remplacé par `has_text` et `large_text` ;
- `pages[].image` disparaît de la lecture ;
- l'empreinte de lecture prend `large_text_choice` à la place de `serve_as_is`.

## Relations

Inchangées depuis [l'étape 1](../011-guide-nego-documents/data-model.md#relations).

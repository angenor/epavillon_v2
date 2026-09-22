# Modèle de données — Guide Négo, étape 1

**Le SQL d'abord.** Tout ce qui suit s'écrit dans `docs/database/` **avant** le code. La base locale se migre par `specs/011-guide-nego-documents/migration.sql`, un script rejouable :
- `IF NOT EXISTS`, `ON CONFLICT DO NOTHING`, `DROP TRIGGER IF EXISTS` puis `CREATE TRIGGER` ;
- il se joue deux fois, puis on compare les schémas ;
- **jamais `down -v`**.

Le changement se consigne dans `docs/progression/modele.md`, ce qu'ADR-017 permet puisque le modèle est commun.

Les noms ci-dessous sont ceux du plan. Leur forme exacte, contraintes comprises, est fixée par le SQL.

---

## 1. `negotiation.documents` — ce qui change

| Élément | Avant | Après | Pourquoi |
|---|---|---|---|
| Source | `ck_documents_source_xor` : exactement un de `asset_id`, `external_url` | `ck_documents_source_at_most_one` : **au plus un**, et `ck_documents_published_has_source` : `published_at IS NULL OR num_nonnulls(asset_id, external_url) = 1` | Le brouillon naît sans source (R5) ; « jamais les deux » reste tenu à tout instant |
| COP | — | `event_id uuid NULL`, `CONSTRAINT xmod_fk_documents_event REFERENCES event.events(id) ON DELETE RESTRICT` ; index partiel | FR-002, R17. Patron de `negotiation.meetings.event_id` |
| Date du document | — | `issued_on date NULL` | Distincte de `published_at` (R17) |
| Éditeur | `external_publisher text` | **renommé** `publisher text` | Vaut pour toute source ; aucune ligne de code ne le cite (R17) |
| Assistant | `is_rag_eligible DEFAULT true` | `DEFAULT false` | ADR-011 : seul ce qu'un humain a choisi est lu (FR-008) |
| Dépublication | — | `unpublished_at timestamptz NULL` | Distingue un document retiré d'un brouillon, dans la liste du back-office (FR-009) |
| Remplacement | `supersedes_id`, sans garde | `ux_documents_supersedes` : unique partiel `WHERE supersedes_id IS NOT NULL` ; déclencheur `tg_documents_no_supersede_cycle` | Un seul successeur, pas de boucle (FR-006, R18) |

Sans changement : `space_id` (NULL pour Guide Négo, document transversal), `is_restricted` (vrai = réservé), `published_at` (NULL = brouillon ou dépublié), `download_count` et `negotiation.register_document_download()`, `track_term_id` (non exposé, R16), `search_vector` (qui suit le renommage), l'audit.

**États d'un document** :

| État | Condition |
|---|---|
| Brouillon | `published_at IS NULL`, jamais publié |
| Publié | `published_at IS NOT NULL` |
| Dépublié | `published_at IS NULL` et `unpublished_at IS NOT NULL` — colonne nouvelle `unpublished_at timestamptz NULL`, posée au retrait, effacée à la republication |

**Validation côté service** (le reste est tenu par la base) :
- un fichier est un PDF (le type est vérifié par la garde média) ;
- une publication exige une extraction `ready` **ou** le mode « tel quel » avec des images de page (R6) ;
- le fichier d'un document publié est figé (FR-007) : `NEGOTIATION_DOCUMENT_FILE_LOCKED`.

## 2. `reference.taxonomy_terms` — deux types de document

Dans `020_reference.sql`, taxonomie `document_type` : on ajoute les termes **`summary`** (fr « Résumé », en « Summary ») et **`bulletin`** (fr « Bulletin », en « Bulletin »), avec leur ordre d'affichage. Le libellé fr de `negotiation_guide` se relit : « Guide ».

## 3. Les thématiques d'un document — `reference.entity_terms`

Aucune table nouvelle. Une ligne `('negotiation', 'documents', <id du document>, <id du terme>, 'primary', sort_order)` par thématique, et seulement des termes de `negotiation_theme` (R16).

- Écriture par remplacement en bloc, qui vérifie la taxonomie et `is_active`.
- Suppression d'un brouillon : ses liens s'effacent dans la même transaction.

## 4. `negotiation.rendition_status` — ENUM, machine à états

`pending` → `extracting` → `ready` | `failed`. Relancer l'extraction ramène à `pending`.

## 5. `negotiation.document_renditions` — une ligne par document fichier

| Colonne | Type | Note |
|---|---|---|
| `document_id` | uuid PK | → `negotiation.documents(id)` ON DELETE CASCADE |
| `asset_id` | uuid | `xmod_fk_document_renditions_asset` → `media.assets(id)` : le fichier extrait. Un autre fichier donne une autre extraction |
| `status` | `negotiation.rendition_status` | |
| `page_count` | integer NULL | Pages du document (R7), > 0 une fois `ready` |
| `outline` | jsonb NULL | Sommaire en grammaire close ([forme-lisible.md](contracts/forme-lisible.md)) |
| `is_reflowable` | boolean NULL | Verdict de l'extraction |
| `quality` | jsonb NULL | Indicateurs : part des pages avec texte, colonnes détectées, blocs `origine`, faux termes filtrés |
| `serve_as_is` | boolean NOT NULL DEFAULT false | **Le choix de l'administratrice** : « ouvrir tel quel » (FR-005 bis). Modifiable sans republier |
| `reading_bytes` | bigint NULL | Poids de la copie gardée selon R2, qui est la taille annoncée sur la fiche |
| `extractor` | text NULL | Outil et version, par exemple `pdfium-render 0.8 / pdfium 6xxx` |
| `failure_reason` | text NULL | Motif lisible d'un échec |
| `attempts` | integer NOT NULL DEFAULT 0 | |
| `extracted_at`, `created_at`, `updated_at` | timestamptz | |

Contraintes :
- `ck_document_renditions_ready` : `status <> 'ready' OR page_count > 0` ;
- `ck_document_renditions_failed` : `status <> 'failed' OR failure_reason IS NOT NULL`.

Déclencheur `updated_at`.

## 6. `negotiation.document_pages` — une ligne par page

| Colonne | Type | Note |
|---|---|---|
| `document_id` | uuid | → `negotiation.documents(id)` ON DELETE CASCADE |
| `page_index` | integer | À partir de 1 ; PK `(document_id, page_index)` |
| `label` | text NOT NULL | Étiquette imprimée, « 59 » |
| `blocks` | jsonb NOT NULL DEFAULT '[]' | Blocs de la page ([forme-lisible.md](contracts/forme-lisible.md)) ; vide en mode « tel quel » |
| `plain_text` | text NOT NULL DEFAULT '' | Texte brut, pour la recherche |
| `search_vector` | tsvector GENERATED ALWAYS AS (`to_tsvector('french', plain_text)`) STORED | Index GIN `ix_document_pages_search` |
| `image_key` | text NULL | Clé de l'image de la page dans le bucket privé |
| `image_bytes` | integer NULL | |
| `has_origin_block` | boolean NOT NULL DEFAULT false | Vrai si la page porte un tableau ou une figure : son image part avec la copie gardée (R2) |

Une nouvelle extraction remplace toutes les lignes du document, dans une transaction. Les images obsolètes se purgent par un travail différé.

## 7. `negotiation.correction_notes`

| Colonne | Type | Note |
|---|---|---|
| `id` | uuid PK DEFAULT `platform.uuid_v7()` | |
| `document_id` | uuid NOT NULL | → `negotiation.documents(id)` ON DELETE CASCADE : seul un brouillon se supprime |
| `page_index` | integer NOT NULL | Borné par `tg_correction_notes_page_exists` : la page existe dans `document_pages` (principe VIII) |
| `passage` | text NULL | Extrait cité du passage visé ; NULL = tête de page |
| `body` | `platform.i18n_text` NOT NULL | Français exigé |
| `author_id` | uuid NOT NULL | `xmod_fk_correction_notes_author` → `identity.people(id)` |
| `created_at` | timestamptz NOT NULL DEFAULT now() | |
| `withdrawn_at` | timestamptz NULL | |
| `withdrawn_by` | uuid NULL | `xmod_fk_correction_notes_withdrawer` → `identity.people(id)` |

Contraintes et index :
- `ck_correction_notes_withdrawal` : `(withdrawn_at IS NULL) = (withdrawn_by IS NULL)` ;
- index `ix_correction_notes_live` sur `(document_id, page_index) WHERE withdrawn_at IS NULL` ;
- audit par `platform.tg_audit()`.

**Jamais de suppression** d'une note : un retrait se date (FR-049).

La qualité imprimée — « expert IFDD » — vient du rôle, pas d'une colonne. Si la maquette en exige une autre plus tard, elle s'ajoutera.

## 8. Le rôle `expert` et ses permissions

Dans `100_negotiations.sql`, à la suite de `space_lead` (§ rôles et permissions), le tout en `ON CONFLICT DO NOTHING` :

| Élément | Valeur |
|---|---|
| Permission | `negotiation.correction.post` — « Poser une note de correction » |
| Permission | `negotiation.correction.withdraw` — « Retirer une note de correction » |
| Rôle | `expert` — « Expert », `allowed_scopes {global}`, `is_system` vrai. Son commentaire nomme les étapes 2, 7 et 8, qui y ajouteront leurs permissions |
| Rôle → permissions | `expert` → les deux ci-dessus |

`super_admin` les reçoit par le déclencheur existant. `admin` ne les reçoit pas : corriger le fond relève de l'expert, et le lexique d'interface le distingue de l'administrateur.

## 9. `media` — le bucket privé et le contrat de lecture

| Élément | Où | Note |
|---|---|---|
| Réglage `media.private_bucket` = `"epavillon-prive"` | `050_media.sql` (semence des réglages) | Lu par le dépôt pour la visibilité `private` (R4) |
| Fonction `media.object_location(p_asset_id uuid)` | `050_media.sql` | Rend `bucket`, `object_key`, `status`, `byte_size`, `mime_type` d'un objet non supprimé. STABLE. Contrat de lecture offert aux autres modules, comme `media.object_url()` |
| Garde `("negotiation","documents")` | Rust (`media/src/domain/guards.rs`), pas SQL | Permission `negotiation.document.publish`, type `application/pdf`, visibilité imposée `private` |

## 10. Ce qui ne va pas en base

- **Sur l'appareil** : les copies gardées, la progression, la taille du texte, les documents ouverts et les derniers ouverts ([hors-connexion.md](contracts/hors-connexion.md)).
- **Les favoris** : `negotiation.document_bookmarks` existe telle quelle.
- **Le compteur de téléchargements** : il existe.

## Relations

```text
event.events ◄──xmod── documents ──xmod──► media.assets (PDF, bucket privé)
                          │  ▲
             supersedes_id│  │ (un seul successeur)
                          ▼  │
                       documents
documents 1 ── 0..1 document_renditions ──xmod──► media.assets
documents 1 ── 0..n document_pages
documents 1 ── 0..n correction_notes ──xmod──► identity.people (auteur, retrait)
documents 1 ── 0..n reference.entity_terms (negotiation_theme)
documents 1 ── 0..n document_bookmarks ──xmod──► identity.people
```

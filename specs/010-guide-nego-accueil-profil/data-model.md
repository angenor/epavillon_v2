# Modèle de données — étape 0c

**Date** : 2026-09-22 · **Spécification** : [spec.md](spec.md) · **Décisions** : [research.md](research.md)

> **`docs/database/*.sql` est la source de vérité.** Ce document dit ce qui s'y ajoute et pourquoi ;
> il ne le remplace pas. Le SQL se modifie d'abord, la base se recharge, puis le code s'écrit.

---

## 1. Ce qui existe déjà et qu'on ne touche pas

| Objet | Où | Ce qu'il porte pour 0c |
|---|---|---|
| `reference.taxonomies` · `reference.taxonomy_terms` | `020_reference.sql:92` et `:105` | Le vocabulaire et ses termes, libellés `fr`/`en`, ordre, état actif, dépréciation par `superseded_by` |
| `identity.people` | `030_identity.sql:37` | Nom affiché (`display_name`, colonne générée), `country_id`, `preferred_locale`, `timezone` — ce que le profil montre |
| `reference.countries` | `020_reference.sql:53` | Le libellé du pays, résolu par `platform.t(name, locale)` |
| `identity.consents` · `identity.current_consents` | `030_identity.sql:480` et `:492` | La consignation d'un accord et son état courant. **Rien n'y est semé** : aucun accord n'est offert à cette étape |
| `negotiation.network_memberships` | `100_negotiations.sql:318` | Le patron suivi par la table nouvelle, et l'appartenance au réseau affichée par « Mon accès » (étape 0b) |
| `negotiation.tg_check_term_taxonomy` | `100_negotiations.sql:63` | Le trigger générique de garde de vocabulaire, employé six fois dans le module |
| `event.events.timezone` | `060_events.sql:101` | **Pas employé à cette étape** (R11) ; l'étape 3a s'en servira |
| `identity.negotiator_profiles` | `030_identity.sql:447` | **Pas employé, pas modifié** — voir R1 |

---

## 2. Le vocabulaire `negotiation_theme`

**Fichier : `docs/database/020_reference.sql`.** Deux ajouts, aux deux endroits prévus.

### 2.1 La déclaration

Une ligne dans l'`INSERT INTO reference.taxonomies` (l. 267-279), posée avant le
`ON CONFLICT (code) DO NOTHING`, précédée d'un commentaire de justification — comme `negotiation_network`
l'a été à l'étape 0b :

| Colonne | Valeur |
|---|---|
| `code` | `negotiation_theme` |
| `label` | `{"fr":"Thématiques de négociation","en":"Negotiation themes"}` |
| `description` | `{"fr":"Filières suivies par une négociatrice ou un négociateur : adaptation, finance, genre…","en":"Tracks followed by a negotiator: adaptation, finance, gender…"}` |
| `is_multi_select` | `true` — on en suit plusieurs |
| `is_hierarchical` | `false` |
| `is_system` | `true` — il commande une règle métier (les alertes de 3b), aucun écran ne le modifie |

**Le commentaire à poser au-dessus, en substance** : les thématiques de négociation ne sont pas celles du
Pavillon. `activity_theme` classe ce que l'ePavillon programme ; `negotiation_theme` dit ce qu'une
personne suit en salle. Les mêler ferait voir « Élevage durable » à une négociatrice et « Article 6 » dans
les filtres du site.

### 2.2 Les dix termes

À la fin de l'`INSERT INTO reference.taxonomy_terms` (avant le `ON CONFLICT` l. 361), `sort_order` par
pas de dix. La liste et les libellés sont arrêtés en [research.md § R13](research.md). `color_hex` et
`icon` restent nuls, comme pour tous les vocabulaires semés.

**Codes** : `adaptation`, `mitigation`, `finance`, `loss_and_damage`, `article_6`, `transparency`,
`gender`, `just_transition`, `agriculture`, `technology`. Cinq coexistent avec un homonyme sous
`activity_theme` : c'est légal — la clé est `(taxonomy_code, code)` — et voulu.

---

## 3. La table nouvelle : `negotiation.theme_subscriptions`

**Fichier : `docs/database/100_negotiations.sql`**, à la suite de `network_memberships`, dont elle reprend
le patron trait pour trait.

```
negotiation.theme_subscriptions
├── id              uuid        PK DEFAULT platform.uuid_v7()
├── person_id       uuid        NOT NULL  → identity.people(id) ON DELETE CASCADE
│                               contrainte : xmod_fk_theme_subscriptions_person
├── theme_term_id   uuid        NOT NULL  → reference.taxonomy_terms(id) ON DELETE RESTRICT
├── followed_at     timestamptz NOT NULL DEFAULT now()
└── left_at         timestamptz NULL
```

| Élément | Nom | Raison |
|---|---|---|
| Contrainte | `ck_theme_subscriptions_period` — `left_at IS NULL OR left_at >= followed_at` | Un suivi ne se ferme pas avant d'avoir commencé |
| Index unique partiel | `ux_theme_subscriptions_active` sur `(person_id, theme_term_id) WHERE left_at IS NULL` | Un seul suivi vivant par personne et par thématique, sans interdire l'historique |
| Index inverse | `ix_theme_subscriptions_theme` sur `(theme_term_id, followed_at DESC) WHERE left_at IS NULL` | « Qui suit la finance ? » — ce dont 3b aura besoin pour ses alertes |
| Trigger d'audit | `tg_theme_subscriptions_audit` → `platform.tg_audit()` | Qui a retiré un suivi, et quand. C'est ce qui manque à `reference.entity_terms` |
| Trigger de vocabulaire | `tg_theme_subscriptions_check_theme` → `negotiation.tg_check_term_taxonomy('theme_term_id', 'negotiation_theme')` | Un terme d'un autre vocabulaire est refusé **par la base**, pas par le code (principe VIII) |

**Commentaires à porter** (`COMMENT ON`, en français, comme tout le modèle) :

- Sur la table : les thématiques qu'une personne choisit de suivre. Elles n'ouvrent aucun droit :
  elles commandent ce qu'on lui montre en premier et ce dont on l'avertit. Choix révocable, jamais une
  compétence attestée — celle-là vit dans `identity.negotiator_profiles`.
- Sur `left_at` : un suivi se ferme, il ne se supprime pas. Ce qu'une personne a suivi pendant une COP
  reste lisible dans l'historique, et l'index unique ne porte que sur le suivi vivant.

**Pas de `updated_at`** : la table ne se modifie pas en place, sauf pour poser `left_at`. L'audit porte
le reste.

**Aucune vue** : la lecture est une jointure de deux tables sur une personne. Une vue n'y ajouterait rien.

---

## 4. Ce qui sort du modèle : la version de la politique

**Aucune table.** La version vient désormais du fichier de texte embarqué dans l'API
([research.md § R6](research.md)), et non plus du réglage `PRIVACY_POLICY_VERSION`.

Ce qui change, hors `docs/database/` :

| Où | Quoi |
|---|---|
| `backend/crates/kernel/src/config.rs` | Le champ brut `privacy_policy_version`, son défaut `2026-01`, sa validation et `ProgrammeConfig.privacy_policy_version` disparaissent |
| `.env.example` | La ligne `PRIVACY_POLICY_VERSION=2026-01` disparaît |
| `backend/crates/modules/programme/src/service/registration.rs:122` | Lit la version depuis `kernel` — une ligne |
| `backend/crates/modules/programme/src/repo/consents.rs:26-31` | Le commentaire de doctrine est réécrit : la version ne vient plus de la configuration |

**Ce qui ne change pas** : `identity.consents` et sa vue, la signature d'`accorder`, la finalité
`registration_sensitive_data`, la source `registration_form`, le code d'erreur
`REGISTRATION_CONSENT_REQUIRED`, et les lignes déjà écrites sous `2026-01` — un historique ne se réécrit
pas.

---

## 5. Ce qui vit sur l'appareil, et nulle part ailleurs

Ces données ne sont pas dans le modèle, et c'est voulu. Le paragraphe de confidentialité de l'écran
« À propos » les nomme telles quelles.

| Donnée | Où | Pourquoi pas en base |
|---|---|---|
| Le thème — clair, sombre, système | `localStorage`, clé `gn.theme` (étape 0a) | Il s'applique avant toute connexion, et dépend de l'appareil |
| Les données lues, pour le hors-connexion | IndexedDB, magasin `lectures` | C'est un cache, pas une source |
| La file d'écritures en attente | IndexedDB, magasin `ecritures` **(nouveau)** | Elle se vide au retour du réseau ; rien n'y survit |
| La place occupée | Mesurée par l'appareil, jamais stockée | Une mesure, pas une donnée |
| Les clés de parcours (ouverture vue, thématiques proposées) | `localStorage`, préfixe `gn.` | Elles disent où en est **ce téléphone**, pas la personne |

---

## 6. Migration

Un script rejouable, `specs/010-guide-nego-accueil-profil/migration.sql`, sur le modèle de celui de
l'étape 0b : `CREATE TABLE IF NOT EXISTS`, gardes `DO $$ … EXCEPTION WHEN duplicate_object` sur les
triggers, `INSERT … ON CONFLICT DO NOTHING` pour le vocabulaire et ses termes. Vérifié en comparant les
schémas avec `pg_dump --schema-only` après une double exécution.

La modification de `docs/database/` se consigne dans `docs/progression/modele.md` — seule exception à la
règle qui veut que Guide Négo n'écrive pas dans la progression de l'ePavillon (ADR-017).

**`make check` et `make check-db` ne se lancent pas** : ils détruisent la base locale.

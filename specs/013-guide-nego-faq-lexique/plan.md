# Implementation Plan: Guide Négo — FAQ, parcours « Ma première COP » et lexique (étape 2)

**Branch**: `013-guide-nego-faq-lexique` | **Date**: 2026-09-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/013-guide-nego-faq-lexique/spec.md`

**Artefacts** : [research.md](research.md) · [data-model.md](data-model.md) · [contracts/](contracts/) — [api-savoir.md](contracts/api-savoir.md), [api-admin-savoir.md](contracts/api-admin-savoir.md), [resolution-lexique.md](contracts/resolution-lexique.md) · [quickstart.md](quickstart.md)

---

## Summary

Une négociatrice trouve *contact group* sans réseau en tapant « contact grup » ; chaque réponse de la FAQ dit quand un expert l'a vérifiée ; le parcours « Ma première COP » garde ses coches sur le téléphone et les suit sur le compte ; la recherche globale ouvre ; la feuille du lecteur ouvre l'entrée du lexique. Les experts tiennent une file : signalements, questions, termes proposés.

**L'approche, en une phrase par pièce.**

- **Tout dans `negotiation`**, section « §10 Savoir » du modèle, domaine `savoir` du crate ; aucun nouveau crate ([R1](research.md)).
- **Un seul paquet** FAQ + parcours + lexique, sous empreinte, relu par différence (`?since=`, chevauchement de cinq minutes, identifiants sortis) ; une entrée publiée ne se supprime jamais ([R2](research.md)).
- **La garde des lectures de 0a** porte le paquet ; une fusion pure l'applique ([R3](research.md)).
- **La recherche du téléphone est maison**, sur le repli du lecteur, avec une distance d'édition bornée ; le serveur a `pg_trgm` pour le back-office et le rapprochement des termes proposés ([R4](research.md), [R5](research.md)).
- **Désignation stable** : le `slug` du terme anglais ; **résolution** d'un texte par terme, sigle, variante, `slug`, jamais approchée — une adresse, une fonction, une fonction SQL ([R6](research.md), [resolution-lexique.md](contracts/resolution-lexique.md)).
- **Coches et favoris ligne à ligne**, idempotents, par la file de 0c : le dernier geste gagne sans `412` ([R7](research.md)).
- **Une fois et une seule** : `client_ref` sur chaque écriture différée ; plafonds par jour ([R8](research.md)).
- **Anonymat tenu par les routes** : l'auteur reste en base pour le plafond et le courriel, aucune route du back-office ne le rend ([R9](research.md)).
- **Deux permissions globales** : `knowledge.publish` (admin, expert), `knowledge.review` (expert) ([R10](research.md)).
- **Deux courriels** par la file de travaux de 0b ([R11](research.md)).

---

## Technical Context

**Langage / version** : Rust stable (API, worker) · TypeScript strict (Nuxt 4, aucun `any`)

**Dépendances principales** : Actix Web, SQLx vérifié, PostgreSQL 17 (`pg_trgm`, `unaccent` déjà chargés), Nuxt 4, TailwindCSS v4. **Aucune dépendance nouvelle**, ni côté serveur ni côté client.

**Stockage** :
- **PostgreSQL**, `negotiation` : 15 tables, 4 ENUM, 5 fonctions, 2 permissions ; `reference` : 2 vocabulaires ([data-model.md](data-model.md)).
- **Sur l'appareil** : garde `savoir` dans IndexedDB (`lectures`, sans nouvelle version de la base) ; file d'écritures de 0c ; `localStorage` pour les coches et favoris sans compte, les derniers termes consultés, les lectures comptées du jour.

**Tests** :
- `cargo test -p negotiation` sur base réelle : chaque route, URL forgée, invariants traduits, anonymat (SC-009), rejeu d'un `client_ref`, `since` et `removed`, courriels mis en file ;
- `node --test` : recherche floue, fusion du paquet, résolution, normalisation comparée à `platform.normalize_label` sur un jeu commun ;
- écrans à 320, 360 et 390 px, deux thèmes.

**Plateforme cible** : PWA, rendu côté navigateur, 360 px de référence, hors connexion. Back-office sur écran large.

**Objectifs de performance** : recherche du lexique < 1 s après la dernière frappe hors connexion (SC-001), sans décalage perceptible sur 500 entrées (SC-004) ; le `304` coûte zéro octet ; une différence ne porte que ce qui a changé.

**Contraintes** : préfixe `/v2` ; aucun composant ni jeton du site dans l'application ; aucun fichier de plus de mille lignes ; jamais `make check`, jamais `down -v`.

**Échelle et périmètre** :
- **Application** : 14 écrans ou feuilles — FAQ, rubrique, entrée, « Qu'est-ce qui manque », « Dépassé ou faux », question, question envoyée, « Mes questions », parcours, lexique (ouverture, recherche, aucun résultat), liste, entrée, favoris, proposer un terme, recherche globale — et la feuille du lecteur.
- **Back-office** : 6 écrans — FAQ (liste, fiche), lexique (liste, fiche), parcours, file.
- **Routes** : 14 publiques ou personnelles, 32 d'administration.
- **Composants nouveaux** : `GnLigneQuestion`, `GnSource`, `GnRetourUtile`, `GnEtapeParcours`, `GnLigneTerme`, `GnRailAlphabet`, `GnGroupeResultats`, `GnFeuilleTerme`, `GnLoupe` — sur la planche.
- **Volume** : quelques centaines d'entrées au plus.

---

## Constitution Check

*Porte à franchir avant la recherche, et à repasser après la conception.*

| Principe | Verdict | Comment il est tenu |
|---|---|---|
| **I — Le modèle fait autorité** | ✅ | §10 de `100_negotiations.sql` et les deux vocabulaires de `020_reference.sql` s'écrivent **avant** le code ; `migration.sql` rejouable, joué deux fois, schémas comparés ; consigné dans `docs/progression/modele.md` |
| **II — Frontières de modules** | ✅ | Tout est dans `negotiation` ; les sources pointent vers `negotiation.documents`, même schéma. Le thème d'une question lit `reference`, comme 0c |
| **III — `xmod_fk_*`** | ✅ | Chaque colonne vers `identity.people` : `xmod_fk_faq_entries_{author,verifier}`, `_faq_feedback_person`, `_faq_reports_{reporter,handler}`, `_expert_questions_{asker,answerer}`, `_glossary_*`, `_pathway_checks_person` |
| **IV — Effets de bord par l'outbox** | ✅ | Aucun effet inter-modules. Les courriels sont des **travaux** (`platform.jobs`) mis en file dans la transaction, comme 0b ; un événement d'audit à la réponse d'une question |
| **V — Permission et portée** | ✅ | `knowledge.publish`, `knowledge.review` en **portée globale** ; poser une question exige `space.access` ; chaque route a son test d'URL forgée et d'administrateur d'événement refusé |
| **VI — SQLx vérifié, pas d'ORM** | ✅ | Requêtes statiques ; la différence `since` est une requête écrite, pas composée |
| **VII — Contexte d'écriture** | ✅ | `Db::write(&ctx)` sur chaque écriture ; test de l'auteur dans l'audit sur une écriture publique et une du back-office |
| **VIII — Invariants non réimplémentés** | ✅ | En base et traduits : vérification avant publication, entrée publiée non supprimable, source à une cible, lien d'étape cohérent, promotion sans consentement, proposition unique par terme en attente, `client_ref` unique. Le service ne compte que les plafonds |
| **IX — Erreurs à code stable** | ✅ | Dix-sept codes nouveaux, section « Guide Négo, savoir (étape 2) » du catalogue |
| **X — Tests sur base réelle** | ✅ | Chaque route : nominal, URL forgée, invariant traduit ; anonymat relu dans chaque réponse de la file |
| **XI — Hors connexion d'abord** | ✅ | Le paquet entier dans la garde, l'heure de lecture affichée ; toutes les écritures par la file de 0c, une seule fois |
| **XII — Confiance** | ✅ | « Vérifié le » obligatoire pour publier ; « À revoir » dit qu'un expert relit ; un signalement ne modifie jamais l'entrée ; la résolution d'un terme n'est jamais approchée ; rien d'IA |
| **XIII — Un design propre et borné** | ✅ | Neuf composants dans le dossier de Guide Négo, sur la planche ; le back-office sur `ui/` ; la loupe de l'en-tête inscrite en écart dans 05-design.md |
| **XIV — Une seule porte** | ✅ *(sans objet)* | Aucun service annexe |
| **Trois agendas** | ✅ | Le parcours cite « Réunions de la Francophonie » comme origine, sans lien d'agenda ; « Programme » seul n'apparaît nulle part |
| **Garde-fou des 1000 lignes** | ✅ | `useApi.ts` (905) : deux lignes de montage ; `lire.vue` (806) perd sa feuille au profit de `GnFeuilleTerme` ; Rust découpé par famille de routes ([R17](research.md)) |
| **Le suivi vit dans `progress.md`** | ✅ | Sauf `docs/progression/modele.md` (ADR-017) |

**Aucune violation à justifier.** *Repassée après la conception* : aucune arête entre modules, chaque route a sa garde, chaque invariant son code.

---

## Project Structure

### Documentation (this feature)

```text
specs/013-guide-nego-faq-lexique/
├── spec.md · plan.md · research.md · data-model.md · quickstart.md
├── contracts/  api-savoir.md · api-admin-savoir.md · resolution-lexique.md
├── migration.sql          rejouable, recopie du §10
├── donnees-essai.sql      données de donnees-savoir.md et donnees-lexique.md, local seulement
└── tasks.md               (/speckit-tasks)
```

### Source Code (repository root)

```text
docs/database/
├── 020_reference.sql          + faq_section, glossary_family
└── 100_negotiations.sql       + §10 Savoir ; §9 : deux permissions

backend/crates/kernel/src/error.rs              + 17 codes
backend/crates/modules/negotiation/
├── src/domain/permissions.rs   + KnowledgePublish, KnowledgeReview
├── src/domain/savoir.rs        formes, validation, empreinte
├── src/repo/savoir_{paquet,faq,lexique,parcours,file}.rs
├── src/service/savoir_{paquet,retours,questions,parcours,lexique,admin,file}.rs
├── src/routes/savoir.rs · admin_savoir.rs · admin_file.rs · openapi.rs (+ paths)
├── src/jobs/emails.rs · src/mail.rs   + deux courriels
├── src/lib.rs                  montage des routes et des travaux
└── tests/savoir_*.rs · commun/savoir.rs

frontend/app/
├── types/negotiation-savoir.ts · admin-negotiation-savoir.ts
├── composables/api/guide-nego-savoir.ts · admin-negotiation-savoir.ts   (+2 lignes dans useApi.ts)
├── composables/guide-nego/useGnSavoir.ts · useGnParcours.ts · useGnFavorisLexique.ts · useGnQuestions.ts
├── utils/guide-nego/savoir.ts · recherche-floue.ts · lexique.ts          (+ tests node --test)
├── mocks/negotiation-savoir.ts
├── components/guide-nego/Gn{LigneQuestion,Source,RetourUtile,EtapeParcours,LigneTerme,
│                           RailAlphabet,GroupeResultats,FeuilleTerme,Loupe}.vue
├── components/admin/negotiation/  FaqForm · GlossaryForm · SourcesField · PathwayEditor · QueueItem
├── pages/guide-nego/
│   ├── lexique.vue (remplacée) · lexique/[slug].vue · lexique/liste.vue · lexique/favoris.vue
│   ├── ressources/faq/index.vue · rubrique/[code].vue · [id].vue · question.vue · mes-questions.vue
│   ├── ressources/parcours.vue
│   └── recherche.vue
├── pages/guide-nego/ressources/documents/[id]/lire.vue    feuille → GnFeuilleTerme
├── pages/admin/negociations/  faq/{index,[id]} · lexique/{index,[id]} · parcours/index · file/index
└── layouts/admin.vue          + quatre entrées « Guide Négo »

frontend/i18n/locales/{fr,en}/pages/   un fichier par écran (R17)
docs/AppNego/05-design.md               écart : la loupe de l'en-tête
docs/DEPLOIEMENT.md § 15                essais sur appareil réel de l'étape 2
```

**Structure Decision** : celle des étapes 0b à 1b — un domaine de plus dans le crate `negotiation`, un sous-arbre de plus sous `pages/guide-nego/` et `pages/admin/negociations/`.

---

## Ordre de construction

1. **Modèle** : §10, vocabulaires, permissions ; base jetable ; `migration.sql` sur la base locale ; `donnees-essai.sql`.
2. **API publique et personnelle**, puis **back-office et file**, avec leurs tests ; `make openapi`.
3. **Plomberie du téléphone** : types, méthodes d'API, `useGnSavoir`, fusion, recherche floue, résolution — tout testé sans navigateur.
4. **Lexique** (critère de sortie), puis **FAQ**, **retours**, **questions**, **parcours**, **recherche globale et feuille du lecteur**, **termes proposés**.
5. **Back-office** : FAQ, lexique, parcours, file.
6. **Recette** ([quickstart.md](quickstart.md)), écart dans 05-design.md, § 15 de DEPLOIEMENT.md.

## Complexity Tracking

Aucune violation de la constitution à justifier.

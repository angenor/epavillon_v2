# Implementation Plan: Guide Négo — signalements et notifications (étape 3b)

**Branch**: `015-guide-nego-signalements` | **Date**: 2026-09-25 | **Spec**: [spec.md](spec.md)

**Artefacts** : [research.md](research.md) · [data-model.md](data-model.md) · [contracts/](contracts/) ·
[quickstart.md](quickstart.md)

---

## Summary

Une négociatrice signale en trois gestes, même sans réseau ; l'administration valide en un geste
depuis Guide Négo, avec « Annuler » six secondes qui ne laisse jamais partir de notification ; le
signalement validé se pose **par-dessus** la donnée officielle, sans nom, et se retire quand la source
rattrape ou que la session finit ; une réunion non annoncée vit à part. Les changements — importés ou
signalés — préviennent les personnes qui suivent la session, dans l'application et par courriel.

**L'approche, en une phrase par pièce.**

- **Deux tables à part** : les signalements, les réunions non annoncées — rien n'écrit sur les
  sessions importées ([R1](research.md), [R2](research.md)).
- **La publication est différée** de dix secondes et relit l'état : une validation annulée n'émet rien
  ([R3](research.md)).
- **Le rattrapage** se calcule dans la transaction de l'import de 3a ([R7](research.md)).
- **Les notifications** passent par l'outbox vers `engagement`, les courriels par un travail de
  `negotiation` qui regroupe par fenêtre de dix minutes et respecte l'accord « Notifications »
  ([R8](research.md) à [R10](research.md)).
- **L'application** étend les écrans de 3a et ouvre la cloche, le centre et la validation.

---

## Technical Context

**Langage / version** : Rust stable · TypeScript strict (Nuxt 4, aucun `any`)

**Dépendances** : aucune nouvelle.

**Stockage** : `negotiation` (un ENUM, trois tables, deux colonnes), `engagement` (quatre types
semés), `identity` (une permission, l'accord dans `identity.consents`). Appareil : garde IndexedDB
version 3 inchangée, quatre clés ; la file de 0c.

**Tests** : `cargo test -p negotiation -p engagement` sur base réelle ; `node --test` pour les règles
pures ; recette au navigateur sur la version construite, 360 px, deux thèmes ; courriels dans Mailpit.

**Contraintes** : un signalement ne modifie jamais une session importée (SC-003) ; aucune notification
d'une validation annulée (SC-004) ; aucun nom d'autrice hors de la validation (SC-008) ; aucune
notification poussée ; aucun fichier > 1000 lignes (`useApi.ts` ≈ 907 : lignes de montage seulement).

**Échelle** : six écrans nouveaux ou complétés, quatre composants, onze routes, trois travaux, quatre
types de notification.

---

## Constitution Check

| Principe | Verdict | Comment |
|---|---|---|
| **I — Modèle d'abord** | ✅ | SQL dans `docs/database/` puis `migration.sql` rejouable ; ligne dans `docs/progression/modele.md` |
| **II — Frontières** | ✅ | `negotiation` calcule les destinataires et compose l'avis ; `engagement` écrit les avis reçus par une branche générique, sans lire ni appeler `negotiation` ; aucune arête entre crates |
| **III — `xmod_fk_*`** | ✅ | auteur, décideur, édition, personne des agendas |
| **IV — Outbox** | ✅ | quatre événements ; courriels par travail posé dans la transaction |
| **V — Permission et portée** | ✅ | `negotiation.report.validate` globale ; signaler : `negotiation.space.access` globale |
| **VII — Contexte d'écriture** | ✅ | routes `Db::write(&ctx)`, travaux `job.context()` |
| **VIII — Invariants en base** | ✅ | doublon, cible, motif de refus, longueur : contraintes ; le code traduit |
| **IX — Codes stables** | ✅ | six codes |
| **X — Base réelle** | ✅ | SC-001, SC-003, SC-004, SC-006, SC-008 en tests d'intégration |
| **XI — Hors connexion** | ✅ | signaler en file avec référence client ; centre, « Mes signalements », encarts gardés |
| **XII — Confiance** | ✅ | par-dessus, jamais dedans ; rien avant validation ; l'administration voit la source du moment |
| **XIII — Design borné** | ✅ | composants dans `components/guide-nego/`, sur `passation/`, à la planche |
| **Trois agendas** | ✅ | la réunion non annoncée reste une « Session de négociation », marquée « Non annoncée » |

Aucune violation.

---

## Project Structure

```text
docs/database/100_negotiations.sql     # report_status, session_reports, network_meetings,
                                       # network_agenda_entries, notify_changes, permission
docs/database/110_engagement.sql       # quatre notification_types
backend/crates/contracts/src/negotiation.rs          # constantes d'événements
backend/crates/modules/negotiation/src/
├── {domain,repo,service,routes}/reports.rs · admin_reports.rs · notifications.rs
├── import/rattrapage.rs               # pur : signalements rattrapés par la lecture
├── jobs/import.rs                     # + rattrapage, + meeting.changed, + travail de courriel
├── jobs/publish.rs · jobs/change_email.rs
├── mail.rs                            # + gabarits de changement
└── tests/                             # signalements_*, validation_*, rattrapage, notifications_*
backend/crates/modules/engagement/src/{consumers/notifications.rs, repo/notifications.rs, routes/notifications.rs}   # branche générique, replace, filtre module
backend/crates/kernel/src/error.rs     # + six codes
frontend/app/
├── pages/guide-nego/negociations/{index,[id],agenda}.vue   # étendus
├── pages/guide-nego/negociations/{signalements,non-annoncee}.vue
├── pages/guide-nego/validation/signalements.vue · notifications.vue
├── pages/guide-nego/ressources/{reglages,a-propos}.vue      # étendus
├── components/guide-nego/{GnEncartSignalement,GnFeuilleSignaler,GnCloche,GnLigneNotification}.vue
├── composables/api/{negotiation-reports,notifications}.ts
├── composables/guide-nego/{useGnSignalements,useGnValidation,useGnNotifications,useGnReglageNotifications}.ts
├── utils/guide-nego/signalements.ts   # encart affiché, fin, repère ; tests
└── types/negotiation-reports.ts
```

---

## Séquencement

| # | Phase | Livre |
|---|---|---|
| 1 | **Le modèle** | SQL, migration rejouable, base `epavillon_dev2` migrée, schémas comparés |
| 2 | **Signaler (API)** | `POST /negotiation/reports`, « mes signalements », `can_validate_reports`, codes ; tests idempotence, accès, doublon |
| 3 | **Valider et afficher (API)** | file, valider, annuler, refuser, retirer ; travail de publication ; `network_reports`/`network_meetings` dans la réponse des sessions ; rattrapage à l'import ; tests SC-003, SC-004, rattrapage, coupure |
| 4 | **Notifier (API)** | `meeting.changed` à l'import, branches du consommateur, types semés, travail de courriel, accord et réglage par thématique ; tests destinataires, regroupement, fenêtre, accord |
| 5 | **Plomberie client** | types, clients, composables, règles pures et tests |
| 6 | **US1 — Signaler (écrans)** | feuille, précision, envoi, « Votre signalement », réunion non annoncée, « Mes signalements » |
| 7 | **US2, US3 — Valider et afficher (écrans)** | encart, repère, lignes non annoncées, écran de validation |
| 8 | **US4, US5 — Notifier (écrans)** | cloche, centre, réglages par thématique, accord sur « À propos » |
| 9 | **Recette** | quickstart au navigateur ; écart 40 mis à jour ; écarts nouveaux ; § 15 de `DEPLOIEMENT.md` |

La phase 5 suit la phase 4 (elle a besoin de toutes les formes) ; 6, 7, 8 se suivent (mêmes écrans).

## Complexity Tracking

*Aucune violation à justifier.*

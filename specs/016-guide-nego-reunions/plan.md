# Implementation Plan: Guide Négo — Réunions de la Francophonie (étape 4)

**Branch**: `016-guide-nego-reunions` | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

**Artefacts** : [research.md](research.md) · [data-model.md](data-model.md) · [contracts/](contracts/) ·
[quickstart.md](quickstart.md)

## Summary

L'IFDD saisit au back-office ses trois rendez-vous — atelier préparatoire, concertation des
négociatrices et négociateurs, concertation ministérielle — et peut les lier à une activité du
Pavillon. L'application les montre dans l'onglet « Francophonie », section « Réunions », hors connexion
compris ; on s'y inscrit, même sans réseau, avec liste d'attente tenue par la base ; le lien de
visioconférence n'est servi qu'aux inscrites. « Ma journée » gagne les lignes des sessions et des
réunions du jour.

**L'approche** : les tables existent (`meetings`, `meeting_registrations`) — on les complète au lieu de
les doubler ([R1](research.md)) ; l'inscription reprend le patron éprouvé du Pavillon, en base
([R4](research.md)) ; la visioconférence ne quitte l'API que pour qui y a droit ([R6](research.md)) ; le
Pavillon n'est qu'une clé ([R7](research.md)) ; les avis passent par la mécanique de 3b ([R8](research.md)).

## Technical Context

Rust stable, SQLx vérifié, Nuxt 4 TS strict. Aucune dépendance nouvelle. Stockage : `negotiation`
(colonnes, triggers, trois fonctions), `reference` (un vocabulaire), `engagement` (deux types). Appareil :
deux clés de garde, la file de 0c. Tests : `cargo test -p negotiation -p engagement` sur base réelle,
`node --test`, recette au navigateur (version construite, 360 px, deux thèmes). Contraintes :
`useApi.ts` ≈ 911 lignes (montage seulement) ; le lien visio jamais dans une réponse publique ; aucune
écriture dans `programme`.

## Constitution Check

| Principe | Verdict | Comment |
|---|---|---|
| I — Modèle d'abord | ✅ | SQL puis `migration.sql` rejouable ; ligne dans `docs/progression/modele.md` |
| II — Frontières | ✅ | `negotiation` lit `programme.sessions` et `live.meetings_public` en SQL (patron `live/repo/cross`), n'écrit que chez lui ; aucune arête entre crates |
| III — `xmod_fk_*` | ✅ | `xmod_fk_negotiation_meetings_pavilion_session` (`ON DELETE SET NULL`) |
| IV — Outbox | ✅ | avis par la charge `notification` (3b), courriels par la file de `negotiation` |
| V — Permission et portée | ✅ | `Requires<MeetingManage>` sur la portée globale (décision du 21/09, jamais `RequiresAnyScope`) ; inscription : `space.access` globale (R9) |
| VIII — Invariants en base | ✅ | capacité, fenêtre, liste d'attente, promotion : trigger et fonction ; le code traduit |
| X — Base réelle | ✅ | concurrence sur la dernière place (SC-003), rejeu (SC-002), visio non servie |
| XI — Hors connexion | ✅ | liste, fiches, inscriptions et liens des inscrites gardés ; inscription en file |
| XII — Confiance | ✅ | brouillon jamais servi ; « Complet » dit au retour, jamais caché |
| XIII — Design borné | ✅ | composants dans `components/guide-nego/`, planche |
| Trois agendas | ✅ | « Réunions de la Francophonie » partout ; jamais une liste mêlée hors « Ma journée » |
| Onglet partagé | ✅ | `GnSectionPavillon.vue` et `GnJourneeLignePavillon.vue` vides, seuls points de l'étape 5 |

## Project Structure

```text
docs/database/020_reference.sql · 100_negotiations.sql · 110_engagement.sql
backend/crates/modules/negotiation/src/
├── {domain,repo,service,routes}/meetings.rs · meeting_registrations.rs · admin_meetings.rs
├── repo/cross.rs                       # lecture de programme.sessions et live.meetings_public
├── notifications/avis.rs · emission.rs # + réunion changée, annulée, place obtenue
└── jobs/change_email.rs                # + cible « réunion »
backend/crates/kernel/src/error.rs      # + six codes
frontend/app/
├── pages/guide-nego/francophonie/{index.vue, reunions/[id].vue}
├── components/guide-nego/francophonie/{GnSectionReunions,GnSectionPavillon,GnLigneReunion}.vue
├── components/guide-nego/{GnEtiquettePavillon.vue, journee/GnJourneeLigne{Sessions,Reunions,Pavillon}.vue}
├── composables/api/negotiation-meetings.ts · admin-negotiations.ts (+ réunions)
├── composables/guide-nego/useGnReunions.ts · useGnInscriptionsReunions.ts
├── utils/guide-nego/reunions.ts        # + tests
├── pages/admin/negociations/reunions/{index,nouvelle,[id]}.vue + components/admin/negotiation/Meeting*.vue
└── types/negotiation-meetings.ts
```

## Séquencement

| # | Phase | Livre |
|---|---|---|
| 1 | Le modèle | vocabulaire, colonnes, trigger, fonctions, types, migration, base migrée |
| 2 | Lire et s'inscrire (API) | liste publique, mes inscriptions et liens, s'inscrire, se désinscrire et promouvoir ; tests (concurrence, rejeu, visio) |
| 3 | Back-office (API + écrans) | saisir, publier, annuler, lier, inscrites ; sélecteur d'activités ; tests de garde |
| 4 | Prévenir (API) | annulation, changement, place obtenue : avis et courriels |
| 5 | Plomberie client | types, clients, composables, règles pures |
| 6 | US1 — L'onglet et la fiche | page, sélecteur, sections, liste, fiche, étiquette Pavillon, hors connexion |
| 7 | US2 — S'inscrire | bouton et états, file, refus dits, liens visio |
| 8 | US4 — Ma journée, recherche | trois lignes du bloc ; groupe de recherche si l'étape 2 est dans `main` |
| 9 | Recette | quickstart, écarts, § 15 de `DEPLOIEMENT.md` |

## Complexity Tracking

*Aucune violation.*

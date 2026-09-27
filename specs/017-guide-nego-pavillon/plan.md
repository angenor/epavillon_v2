# Implementation Plan: Guide Négo — Pavillon de la Francophonie (étape 5)

**Branch**: `017-guide-nego-pavillon` (partie de `016-guide-nego-reunions`) | **Date**: 2026-09-26 | **Spec**: [spec.md](spec.md)

**Artefacts** : [research.md](research.md) · [contracts/](contracts/) · [quickstart.md](quickstart.md) — pas de
data-model : aucune donnée nouvelle, seulement une vue publique enrichie de colonnes déjà en base.

## Summary

La section Pavillon de l'onglet Francophonie montre les activités du stand, lues dans l'API publique du
programme, enrichie **par ajouts seulement** de ce que la base a déjà (rediffusion, noms, langue, liste
d'attente, empreinte). On s'y inscrit par le mécanisme d'inscription existant, formulaire compris, même
hors connexion. « Ma journée » gagne sa ligne Pavillon. La recette joue le scénario qui clôt le MVP.

## Technical Context

Rust, SQLx, Nuxt 4. Aucune dépendance. Stockage : la vue `programme.v_public_schedule` (colonnes ajoutées
en fin) et les lectures du détail ; aucune table. Appareil : trois clés de garde, la file de 0c. Tests :
`cargo test -p programme` (+ `-p api` pour les routes), `test:site`, `test:guide-nego`, recette au
navigateur. Contraintes : site inchangé ; aucune donnée privée rendue publique ; `useApi.ts` ≈ 913 lignes.

## Constitution Check

| Principe | Verdict | Comment |
|---|---|---|
| I — Modèle d'abord | ✅ | la vue dans `075_programme_sessions.sql` d'abord, migration rejouable ; ligne dans `modele.md` |
| II — Frontières | ✅ | `programme` lit `live.streams`, `media.assets`, `identity.people`, `org` en SQL pour sa vue et son détail (lectures, patron existant) ; aucune arête entre crates |
| V — Permission | ✅ | lectures publiques inchangées ; inscription : les règles existantes de `programme` |
| VIII — Invariants | ✅ | capacité, fenêtre, liste d'attente : la base du Pavillon, inchangée |
| X — Base réelle | ✅ | non-régression des clés, absence de données privées, empreinte |
| XI — Hors connexion | ✅ | édition, fiches, inscriptions gardées ; inscription en file |
| XII — Confiance | ✅ | rien de privé exposé ; « lu à » partout |
| XIII — Design borné | ✅ | composants de Guide Négo |
| Trois agendas | ✅ | « Pavillon de la Francophonie, et ses activités » ; jamais filtré par mes thématiques |

## Project Structure

```text
docs/database/075_programme_sessions.sql     # v_public_schedule : colonnes hors diffusion ajoutées en fin (fenêtre, attente, langue)
docs/database/080_live.sql                   # v_public_schedule redéfinie en entier après live.streams : + rediffusion en fin
backend/crates/modules/programme/src/
├── repo/public_schedule.rs · repo/session_parts.rs   # champs ajoutés, lecture publique des noms
├── routes/public_schedule.rs · routes/registrations.rs   # ETag/304
└── repo/cross/mod.rs                       # + live.streams, media.assets au registre des lectures hors schéma
backend/crates/modules/programme/tests/pavillon_ajouts.rs   # non-régression, confidentialité, empreinte, nombre de lignes
frontend/app/
├── types/views.ts · types/programme/session.ts   # champs facultatifs ajoutés
├── composables/api/pavillon.ts · composables/guide-nego/useGnPavillon.ts · useGnInscriptionsPavillon.ts
├── utils/guide-nego/pavillon.ts (+ tests)
├── components/guide-nego/francophonie/{GnSectionPavillon,GnBlocLieu,GnLigneActivite,GnFormulaireInscription}.vue
├── components/guide-nego/journee/GnJourneeLignePavillon.vue · GnEtiquettePavillon (destination)
└── pages/guide-nego/francophonie/pavillon/[slug].vue
```

## Séquencement

| # | Phase | Livre |
|---|---|---|
| 1 | L'API du programme, par ajouts | vue enrichie, détail enrichi, empreintes, migration, tests de non-régression et de confidentialité, site vert |
| 2 | Plomberie client | types, client, composables, règles pures |
| 3 | US1 — Lire | section Pavillon, fiche, étiquette, hors connexion |
| 4 | US2 — S'inscrire | bouton, formulaire, file, refus dits |
| 5 | US3 — Ma journée | ligne Pavillon |
| 6 | Recette | quickstart § 0 à 4 et 6 ; § 5 (scénario du MVP) après fusion de 2 et 4 |

## Complexity Tracking

*Aucune violation.*

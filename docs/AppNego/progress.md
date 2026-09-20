# Suivi de Guide Négo

> **Ce fichier est la mémoire de Guide Négo entre deux sessions.** Il remplace, pour ce projet, la progression de l'ePavillon : on n'écrit ni dans `docs/PROGRESSION.md` ni dans `docs/progression/` — [ADR-017](adr/017-le-suivi-vit-dans-progress-md.md). Toute session le lit en arrivant et le met à jour en partant.

## État

| Étape | État | Note |
|---|---|---|
| Cadrage et documentation | ✅ 18/09/2026 | Ce dossier |
| Maquette — direction artistique | ✅ 19/09/2026 | « Typographique », Atkinson Hyperlegible Next, quatre onglets et bouton « Aa » — [ADR-018](adr/018-direction-typographique-quatre-onglets.md) |
| Maquette | ✅ 20/09/2026 | 18 pages dans [design/ecrans/](design/ecrans/), jetons et composants dans [design/passation/](design/passation/), écarts tranchés dans [05-design.md](05-design.md) ; police embarquable fournie, icône provisoire : le symbole inversé de l'ePavillon ; le back-office se construit sur celui de l'ePavillon |
| Amendement de la constitution (1.1.0) | ✅ 20/09/2026 | Section « Guide Négo », principes XI à XIV, deux contraintes ; I à X inchangés |
| Correctif du modèle : vecteurs | ⏳ À faire | Avant toute indexation |
| 0a — Coquille et système de design | ⏳ À faire | |
| 0b — Compte et admission | ⏳ À faire | |
| 0c — Thématiques, « Ma journée », profil | ⏳ À faire | |
| 1 — Documents | ⏳ À faire | |
| 2 — FAQ et lexique | ⏳ À faire | Ouvre la recherche globale |
| 3a — Sessions : l'agenda et son import | ⏳ À faire | L'import se construit sans attendre l'accord |
| 3b — Sessions : signalements et notifications | ⏳ À faire | Ouvre le centre de notifications |
| 4 — Réunions de la Francophonie | ⏳ À faire | |
| 5 — Pavillon de la Francophonie | ⏳ À faire | API existante |
| 6 — Échanges | — | Après le MVP ; Capacitor d'abord |
| 7 — Assistant IA | — | Après le MVP |
| 8 — Formations et quiz | — | Après le MVP |
| 9 — Restitutions | — | À confirmer sur le terrain |

## Préalables

| Préalable | État |
|---|---|
| Accord écrit du secrétariat de la CCNUCC | Demande en cours — ne bloque pas |
| Droits d'indexation et de quiz sur les guides | ✅ Acquis |
| Comptes Apple et Google de l'OIF | ✅ Ouverts |
| Experts validateurs | ✅ Désignés et disponibles |
| Vidéos des 16 modules de formation | ✅ Disponibles |

## Points ouverts

- Les lignes « En attente » du journal de maquette : des données à fournir, pas du code.
- Les restitutions et leur partage par cercle : proposés, à confirmer par entretiens.
- Les noms de dossiers du client (`guide-nego/`) et le drapeau du module : à fixer à la spécification du socle.

## Journal

| Date | Fait | À suivre |
|---|---|---|
| 2026-09-20 | Constitution amendée en 1.1.0 : section « Guide Négo » et principes XI à XIV (hors connexion, confiance, design borné, une seule porte) ; contraintes des trois agendas et du suivi ; un amendement né de Guide Négo se consigne ici. Gabarits vérifiés : rien à y modifier, ils lisent la constitution à l'exécution | Correctif du modèle (vecteurs), puis 0a. À part : la constitution exige encore `make check` avant commit, contre `CLAUDE.md` — correctif 1.1.1 à décider |
| 2026-09-20 | Feuille de route : le socle scindé en 0a, 0b, 0c et les sessions en 3a, 3b — une spécification par chantier ; la recherche globale s'ouvre à l'étape 2, le centre de notifications à l'étape 3b | `/speckit-constitution`, puis 0a |
| 2026-09-20 | Seconde livraison de la maquette rangée : page « Échanges », dossier `passation/` (jetons, thème, mesures, pictogrammes, composants), 29 écarts tranchés ; barre à cinq onglets à l'ouverture des Échanges | — |
| 2026-09-20 | Maquette terminée et rangée dans `design/ecrans/` ; feuille de route réécrite avec un prompt Spec Kit par étape, un prompt de plan commun et l'amendement de la constitution | `/speckit-constitution`, puis l'étape 0 — Socle |
| 2026-09-19 | Direction artistique choisie dans Claude Design, qui a posé la question avant tout prompt : « Typographique », Atkinson Hyperlegible Next, quatre onglets et bouton « Aa ». Écran de référence rangé dans `design/ecrans/`, prompts 2, 3 et 7 adaptés, ADR-018 | Prompt 2 : le système de design, avec cinq points à reprendre — rôle du jaune, relief de « En cours », filets trop pâles, pictogrammes d'état, tailles sous 15 px |
| 2026-09-18 | Cadrage : notes du porteur confrontées au modèle de l'ePavillon et à deux recherches (processus de la CCNUCC, offre de l'IFDD) ; dix arbitrages. Dossier écrit : sept documents, dix-huit ADR, le lexique de l'interface et les prompts de maquette | Maquette dans Claude Design, puis le socle |

## Mettre à jour ce fichier

1. La ligne de l'étape dans **État**.
2. Une ligne au **Journal** : ce qui a été fait, ce qui vient.
3. Un **ADR** de plus si quelque chose a été tranché ; un point ouvert de moins s'il est résolu.

Rien d'autre. Ce fichier se lit en entier à chaque session : il reste court.

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
| 0a — Coquille et système de design | 🟡 Spécifiée et planifiée le 21/09/2026 | [spec](../../specs/008-guide-nego-coquille/spec.md) · [plan](../../specs/008-guide-nego-coquille/plan.md) · [tâches](../../specs/008-guide-nego-coquille/tasks.md) — 73 tâches, branche `008-guide-nego-coquille` ; **phases 1 à 3 livrées** (27 tâches) : drapeau, fondations du design, garde des lectures ; reste US1, US2, US4, US5 |
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
- `make check-safe` : deux tests Rust de `identity` (`toute_ecriture_laisse_son_auteur`) ont échoué une fois le 21/09 pendant que l'API locale et un serveur de développement tournaient, puis passé seuls et dans la porte complète, API arrêtée. Aucun Rust n'avait été touché. À surveiller : ils semblent sensibles à une activité concurrente sur la base.

## Journal

| Date | Fait | À suivre |
|---|---|---|
| 2026-09-21 | 0a, phases 1 à 3 livrées, `make check-safe` au vert, un commit par phase. `guide_nego.enabled` semé (et inséré à la main en local) ; `/guide-nego/**` rendu côté navigateur ; R3 vérifié — la route s'affiche en `fr`, mais `@nuxtjs/i18n` réécrivait le cookie de langue du site : `plugins/guide-nego-langue.client.ts` le rétablit. Fondations : thème, mesures, police, sprite, huit rôles sombres mesurés (tableau dans `05-design.md`), `#gn-portail`. Contrôles `check-guide-nego` (bornage, feuilles et `<style>`), contrastes (50 paires), `node --test` (21). Drapeau : registre du site étendu (`closedPath`, `lastKnown`), page fermée au design de Guide Négo, verdict immédiat d'après la garde puis appliqué dans les deux sens ; API arrêtée, l'application reste ouverte — vérifié au navigateur. T041 et T042 (connexion, « hier à… ») faites en avance : la garde en dépend | US1 (T026 à T040), puis US2, US4, US5. En local le drapeau est resté **allumé à 100 %** pour la suite |
| 2026-09-21 | Tâches de 0a (73) et analyse de cohérence, corrections appliquées : tout style de composant s'écrit sous `[data-app="guide-nego"]` et rien ne se téléporte dans `<body>` (`#gn-portail`) ; « lu à » et « Synchronisé à » sans fuseau, « hier à… », « le 11 nov. à… » — écart 32 ; la barre d'onglets seule défile quand la place manque, essai à 320 px ; « Prête hors connexion » dit à la personne (FR-012 bis, lexique) ; seul le paquet `fr` est gardé | `/speckit-implement` par tranches : phases 1 à 3, puis US1, US2, US4 et US5 ; `make check-safe` et un commit par phase |
| 2026-09-21 | Plan de 0a repris après relecture : Guide Négo s'affiche en français quel que soit le téléphone et ne touche pas au cookie de langue du site ; service worker engendré à la construction par un module local (liste exacte des fichiers, précache complet, navigation cache d'abord, mise à jour en arrière-plan) — le préchauffage est abandonné ; le drapeau vaut aussitôt d'après la garde et se relit en parallèle ; écarts 30 et 31 inscrits à `05-design.md` ; branche `008-guide-nego-coquille` créée | `/speckit-tasks`, `/speckit-analyze`, puis `/speckit-implement` |
| 2026-09-21 | Plan de 0a : `specs/008-guide-nego-coquille/plan.md`, recherche, modèle, trois contrats, guide de vérification. Tranché : sept dossiers `guide-nego`, composants `Gn*` ; manifeste et service worker statiques en chemins relatifs, bornés à `guide-nego/` ; service worker écrit à la main, sans module PWA (ADR-019 à écrire) ; garde des lectures en IndexedDB ; registre des modules fermés du site étendu de deux champs ; thème hors `<html>` et hors cookie du site ; `theme.css` repris avec son bloc `:root` rentré sous `[data-app]` ; aucun cadre de test ajouté, `node --test` et trois scripts dans `check-front` | `/speckit-tasks`, `/speckit-analyze`, puis `/speckit-implement` |
| 2026-09-21 | Spécification de 0a écrite puis relue par le commanditaire : `specs/008-guide-nego-coquille/` — cinq récits, aucune clarification en attente. Tranché : drapeau `guide_nego.enabled`, allumé = activé et à 100 %, une panne d'API ne ferme jamais l'application ; cinquième onglet sur `negotiation.channels`, sans nouveau drapeau ; thème dans « Profil et réglages » dès 0a ; fondations entières, mais seuls les composants de la coquille et ceux que toute étape emploie — les autres arrivent avec leur étape (prompt 0a corrigé dans la feuille de route) | `/speckit-plan` de 0a avec le prompt commun. Le correctif des vecteurs reste à faire avant toute indexation |
| 2026-09-20 | Constitution 1.1.1, correctif : la porte de qualité est `make check-safe` ; `make check` et `make check-db`, qui détruisent la base locale, exigent l'accord du commanditaire — comme `CLAUDE.md` | Correctif du modèle (vecteurs), puis 0a |
| 2026-09-20 | Constitution amendée en 1.1.0 : section « Guide Négo » et principes XI à XIV (hors connexion, confiance, design borné, une seule porte) ; contraintes des trois agendas et du suivi ; un amendement né de Guide Négo se consigne ici. Gabarits vérifiés : rien à y modifier, ils lisent la constitution à l'exécution | Correctif du modèle (vecteurs), puis 0a. |
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

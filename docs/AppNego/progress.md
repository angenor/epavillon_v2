# Suivi de Guide Négo

> **Ce fichier est la mémoire de Guide Négo entre deux sessions.** Il remplace, pour ce projet, la progression de l'ePavillon : on n'écrit ni dans `docs/PROGRESSION.md` ni dans `docs/progression/` — [ADR-017](adr/017-le-suivi-vit-dans-progress-md.md). Toute session le lit en arrivant et le met à jour en partant.
>
> C'est le **point central** : il ne porte que l'essentiel. Le détail vit dans [`progression/`](progression/LISEZMOI.md) — un fichier par étape, les points ouverts, un journal par jour ; **on n'en ouvre que le fichier utile à la tâche du jour**. Ce fichier se lit en entier à chaque session : il reste court — cent lignes et 15 Ko au plus, mode d'emploi dans [`LISEZMOI.md`](progression/LISEZMOI.md).

## État

| Étape | État | En une phrase |
|---|---|---|
| Cadrage et documentation | ✅ 18/09/2026 | Ce dossier — [détail](progression/etapes/cadrage-et-maquette.md) |
| Maquette — direction artistique | ✅ 19/09/2026 | « Typographique », Atkinson Hyperlegible Next, quatre onglets et bouton « Aa » — [ADR-018](adr/018-direction-typographique-quatre-onglets.md) |
| Maquette | ✅ 20/09/2026 | 18 pages dans [design/ecrans/](design/ecrans/), écarts tranchés dans [05-design.md](05-design.md) — [détail](progression/etapes/cadrage-et-maquette.md) |
| Amendement de la constitution (1.1.0) | ✅ 20/09/2026 | Section « Guide Négo », principes XI à XIV — [détail](progression/etapes/cadrage-et-maquette.md) |
| Correctif du modèle : vecteurs | ⏳ À faire | Avant toute indexation |
| 0a — Coquille et système de design | 🟡 21/09/2026 | Recette faite sauf l'appareil réel : reste l'installation sur Android puis iPhone, et le débit bridé — [détail](progression/etapes/0a-coquille.md) |
| 0b — Compte et admission | 🟡 22/09/2026 | Les cinq récits sont entiers et la recette faite ; reste T112 sur un Android et un iPhone réels — [détail](progression/etapes/0b-compte-admission.md) |
| 0c — Thématiques, « Ma journée », profil | 🟡 22/09/2026 | Close côté code ; la mise en ligne et l'appareil réel (T096 à T098) attendent le § 15 de [DEPLOIEMENT.md](../DEPLOIEMENT.md) — [détail](progression/etapes/0c-accueil-profil.md) |
| 1 — Documents | 🟡 24/09/2026 | Close côté code, critère de sortie atteint sur poste ; restent la recette des réservés (T102) et l'appareil réel (T116) — [détail](progression/etapes/1-documents.md) |
| 1b — Le lecteur montre le PDF d'origine | 🟡 25/09/2026 | Close côté code et dans `main`, le guide se lit sur ses pages ; reste T084 sur téléphones réels — [détail](progression/etapes/1b-lecteur-pdf.md) |
| 2 — FAQ et lexique | 🟡 26/09/2026 | Fusionnée dans `main` (6d16b49) : *contact group* se trouve hors connexion, chaque réponse porte sa date de vérification ; reste l'appareil réel, § 15.4 de [DEPLOIEMENT.md](../DEPLOIEMENT.md) — [détail](progression/etapes/2-faq-lexique.md) |
| 3a — Sessions : l'agenda et son import | 🟡 25/09/2026 | Fusionnée dans `main` (95f82e7), `make check-safe` au vert ; reste l'appareil réel — [détail](progression/etapes/3a-sessions-agenda.md) |
| 3b — Sessions : signalements et notifications | 🟡 26/09/2026 | Fusionnée dans `main` (72b38cf) ; la suite Rust complète passera au `check-safe` de fin de MVP, puis l'appareil réel — [détail](progression/etapes/3b-signalements.md) |
| 4 — Réunions de la Francophonie | 🟡 26/09/2026 | Fusionnée dans `main` (27adc35) ; reste l'appareil réel, § 15.4 de [DEPLOIEMENT.md](../DEPLOIEMENT.md) — [détail](progression/etapes/4-reunions.md) |
| 5 — Pavillon de la Francophonie | 🟡 27/09/2026 | Fusionnée dans `main` (27adc35) ; reste l'appareil réel, § 15.4 de [DEPLOIEMENT.md](../DEPLOIEMENT.md) — [détail](progression/etapes/5-pavillon.md) |
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

Une ligne chacun ; le détail, et les points levés, dans [`points-ouverts.md`](progression/points-ouverts.md).

- [La réinitialisation du site atteint Guide Négo](progression/points-ouverts.md#la-réinitialisation-du-site-atteint-guide-négo) — La préface de Tailwind du site touche aussi l'application, contre FR-026 de 0a ; non corrigée.
- [Confier un espace de négociation à une personne qui n'administre pas la plateforme](progression/points-ouverts.md#confier-un-espace-de-négociation-à-une-personne-qui-nadministre-pas-la-plateforme) — Rien de construit, délibérément ; une spécification à part si l'IFDD le demande.
- [Les lignes « En attente » du journal de maquette](progression/points-ouverts.md#les-lignes--en-attente--du-journal-de-maquette) — Des données à fournir, pas du code.
- [Les deux textes qui engagent](progression/points-ouverts.md#les-deux-textes-qui-engagent) — Politique de confidentialité et conditions d'utilisation, à fournir par l'IFDD.
- [L'hébergeur à nommer dans « À propos »](progression/points-ouverts.md#lhébergeur-à-nommer-dans--à-propos-) — Le commanditaire le dira ; d'ici là, l'écran n'en nomme aucun.
- [L'accord du secrétariat de la CCNUCC](progression/points-ouverts.md#laccord-du-secrétariat-de-la-ccnucc) — Demande en cours pour la reprise du calendrier des sessions.
- [Une clé de traduction manquante sur le site](progression/points-ouverts.md#une-clé-de-traduction-manquante-sur-le-site) — `aside.description` de `pages/home.json` existe en anglais, pas en français ; sans effet sur Guide Négo.
- [« Réservé aux négociateurs » sur la page de maintenance du site](progression/points-ouverts.md#-réservé-aux-négociateurs--sur-la-page-de-maintenance-du-site) — Un texte du site, hors périmètre, non corrigé.
- [Créer un compte ne consigne aucun consentement](progression/points-ouverts.md#créer-un-compte-ne-consigne-aucun-consentement) — À trancher par le commanditaire : consigner l'accord, ou corriger le commentaire.
- [La mise en ligne de 0a, 0b et 0c](progression/points-ouverts.md#la-mise-en-ligne-de-0a-0b-et-0c) — Couvre désormais 0a à 3b, huit migrations ; préparée au § 15 de DEPLOIEMENT.md, pas exécutée ; le commanditaire l'exécute.
- [Les objets désignés par une colonne sans être rattachés](progression/points-ouverts.md#les-objets-désignés-par-une-colonne-sans-être-rattachés) — Seuls les documents de Guide Négo sont déclarés ; déclarer les autres avant tout écran des orphelins.
- [Les restitutions et leur partage par cercle](progression/points-ouverts.md#les-restitutions-et-leur-partage-par-cercle) — Proposés, à confirmer par entretiens.
- [Étape 1 — limites connues de l'extraction](progression/points-ouverts.md#étape-1--limites-connues-de-lextraction) — Filigrane répété, fond dessiné en tracé ; le vrai guide n'a ni l'un ni l'autre.
- [Étape 1 — le jeu d'exemple des documents](progression/points-ouverts.md#étape-1--le-jeu-dexemple-des-documents) — Il ne simule ni les permissions, ni les images de page, ni le PDF.
- [Étape 1 — ce que `check-api-contract` ne vérifie pas](progression/points-ouverts.md#étape-1--ce-que-check-api-contract-ne-vérifie-pas) — Ni les verbes, ni une réponse à deux formes, ni `backend/.sqlx`.
- [Étape 1 — relevés de la recette du back-office (23/09)](progression/points-ouverts.md#étape-1--relevés-de-la-recette-du-back-office-2309) — Dates au fuseau du serveur puis du navigateur ; entrées refusées visibles à l'expert. Hors de Guide Négo.
- [Étape 1 — relevés de la phase 8](progression/points-ouverts.md#étape-1--relevés-de-la-phase-8) — « Ajouté » sans le fuseau de la COP, panne de recherche muette, sous-titre de Ressources trop large.
- [Étape 1 — un refus de l'API compté comme une panne](progression/points-ouverts.md#étape-1--un-refus-de-lapi-compté-comme-une-panne) — `useGnLecture` passe l'application hors connexion sur un 401 ou un 403.
- [Étape 1 — la recette des réservés (T102)](progression/points-ouverts.md#étape-1--la-recette-des-réservés-t102) — Faute d'un compte admis par code ; désormais une série de cases du § 15.4 de DEPLOIEMENT.md.
- [Étape 1 — relevés de la phase 13](progression/points-ouverts.md#étape-1--relevés-de-la-phase-13) — Thématique en code brut depuis une adresse ; ports pris pendant la recette.
- [Étape 1 — relevés de la phase 12](progression/points-ouverts.md#étape-1--relevés-de-la-phase-12) — Signature « expert IFDD » sans accord, extrait sur une puce, fiche de l'expert trop active.
- [Étape 1 — relevés de la phase 11](progression/points-ouverts.md#étape-1--relevés-de-la-phase-11) — `/guide-nego` sans barre finale (corrigé à l'étape 2, T077) ; place comptée sur toute l'origine.
- [Étape 1 — relevés de la phase 10](progression/points-ouverts.md#étape-1--relevés-de-la-phase-10) — Section d'un passage, recherche de deux lettres, cibles sous 44 px.
- [Étape 1 — relevés de la phase 9](progression/points-ouverts.md#étape-1--relevés-de-la-phase-9) — Page à l'italienne qui glisse en « tel quel » ; téléphones réels à faire.
- [« Au nom de {nom} » n'élide pas](progression/points-ouverts.md#-au-nom-de-nom--nélide-pas) — « Au nom de Aïssatou » ; `zoneElides()` en donne la règle.
- [Étape 1 — deux contre-examens interrompus (23/09)](progression/points-ouverts.md#étape-1--deux-contre-examens-interrompus-2309) — Les tests passent ; à relancer pour la même assurance qu'ailleurs.
- [Deux tests de `identity` sensibles à la concurrence](progression/points-ouverts.md#deux-tests-de-identity-sensibles-à-la-concurrence) — Un échec isolé le 21/09, API lancée ; à surveiller.
- [`make check-safe` en fin de cycle seulement](progression/points-ouverts.md#make-check-safe-en-fin-de-cycle-seulement) — Entre-temps : `check:guide-nego`, `test:guide-nego`, `typecheck`.
- [Étape 5 — rien n'écrit `live.streams`](progression/points-ouverts.md#étape-5--rien-nécrit-livestreams) — Sans outil de saisie, la rediffusion d'une activité ne paraîtra pas en production.
- [Étape 5 — le back-office du site n'édite pas les formulaires d'inscription](progression/points-ouverts.md#étape-5--le-back-office-du-site-nédite-pas-les-formulaires-dinscription) — Une activité à formulaire complet ne se prépare qu'en base.
- [Deux fuseaux sur un même écran](progression/points-ouverts.md#deux-fuseaux-sur-un-même-écran) — « Lu à » suit le téléphone, « validé à » la COP ; relevé à la recette de 5, non corrigé.

## Dernières nouvelles

- 27/09 — **MVP clos côté code** : `make check-safe` complet au vert sur `main` (27adc35), 319 lots, 1 555 tests, aucun échec ; reste l'appareil réel, § 15.4 de [DEPLOIEMENT.md](../DEPLOIEMENT.md) — [journal](progression/journal/2026-09-27.md).
- 27/09 — Étapes 4 et 5 fusionnées dans `main` (27adc35) ; le scénario qui clôt le MVP est joué de bout en bout — [étape 5](progression/etapes/5-pavillon.md).
- 27/09 — Les essais sur téléphones réels de 0a à 3b forment une seule liste à cocher, § 15.4 de [DEPLOIEMENT.md](../DEPLOIEMENT.md).
- 27/09 — Le suivi devient ce point central et le dossier [`progression/`](progression/LISEZMOI.md), demandé le 26/09 — [journal](progression/journal/2026-09-27.md).
- 26/09 — Étape 2 fusionnée dans `main` (6d16b49) et poussée ; le blocage des tests venait du contrôle de macOS sur chaque binaire neuf — [étape 2](progression/etapes/2-faq-lexique.md).

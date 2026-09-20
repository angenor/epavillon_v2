# AppNégo — Document de cadrage

> Version 1 — 18 septembre 2026. Sources : [note2.md](note2.md), le modèle de données de l'ePavillon, deux recherches documentaires, les arbitrages du jour. « AppNégo » est un nom de travail ; « négociatrices » inclut les négociateurs : mêmes droits.

## 1. En bref

AppNégo est l'application mobile des négociatrices francophones. D'abord : les documents de l'IFDD lisibles sans connexion, une FAQ, un lexique français-anglais, le programme du jour en français. Ensuite : les échanges du réseau, un assistant IA qui cite ses sources, des quiz.

- **Sa place.** Rien n'offre aujourd'hui, *en français et hors connexion*, le quotidien d'une session adossé à un savoir validé. Les outils officiels sont en anglais ; les guides sont des PDF.
- **Son atout.** C'est la face mobile de modules déjà dessinés dans l'ePavillon (Négociations, Formations, Outils) : le modèle de données couvre l'essentiel, mais **aucun code n'existe derrière**.
- **Son échéance.** Première version à la COP31 (Antalya, 9–20 novembre 2026), puis les modules suivants sans attendre.

## 2. Le besoin

| Difficulté citée par une négociatrice, confirmée par la recherche | Réponse de l'application |
|---|---|
| Ne pas connaître les programmes, surtout quand on débute | Programme du jour en français, filtré par thématiques suivies, avec alertes |
| L'anglais — « leur plus grand défi », écrit l'IFDD | Lexique hors connexion ; plus tard, résumés en français des textes en anglais |
| Faible encadrement ; trouver le coordonnateur de son groupe | FAQ vivante, parcours « ma première COP », questions aux experts |
| Restitution quotidienne au ministère | Aide privée à la rédaction ; partage par cercle choisi |
| Financement de la participation | Hors de portée d'une application |

## 3. Ce que la recherche a recadré

1. **La Francophonie n'est pas un groupe de négociation.** Ses pays siègent dans des groupes opposés (Union européenne, Canada, Groupe africain, PMA), et elle ne tient que trois rendez-vous par COP, plus le pavillon. Le module « réunions de la Francophonie » sera léger, et une restitution ne peut pas être lisible par toute l'application.
2. **Le programme officiel n'a pas de flux public** : ni RSS ni API, seulement un fichier JSON non documenté, propre à chaque session. Les conditions d'utilisation interdisent de compiler le site : **le reprendre exige un accord écrit du secrétariat** — déjà partenaire du programme des négociatrices.
3. **Les réunions décisives ne sont pas toutes annoncées.** « Informal informals » et apartés (*huddles*) ne figurent dans aucun programme : le signalement par le réseau est la seule source.
4. **Hors plénière, tout se passe en anglais.** Le français arrive après coup : un document du 21 novembre 2025 n'a paru en français que le 4 décembre, après son adoption.
5. **Le guide paraît une semaine avant la COP**, et ses droits sont partagés avec son rédacteur (Climate Analytics Africa), sans licence ouverte. Il faut publier en une journée, et régler les droits avant de le découper pour l'IA.
6. **L'IFDD a déjà eu une application** (2018-2019) : une centaine d'installations, jamais mise à jour, retirée des magasins. Un lecteur de PDF ne retient personne.
7. **Des ressources existent en français** — bulletin ENB (financé par l'IFDD), lexique de l'IIED, guide « Au nom de ma délégation » de l'IISD, guides ecbi : on les intègre, on ne les refait pas.

## 4. Public et accès

Public : 597 négociatrices formées depuis 2018, leurs homologues masculins, et tout visiteur.

| Niveau | Qui | Ce qu'il peut faire |
|---|---|---|
| Visiteur | Tout le monde, sans compte | Documents publics, FAQ, lexique, programme |
| Compte | Toute personne inscrite | Favoris, thématiques suivies, hors connexion, inscriptions, quiz |
| Négociatrice | Admise par code d'invitation | Échanges, documents réservés, signalements, assistant IA, restitutions |
| Réseau des négociatrices | Code diffusé dans leur groupe WhatsApp | En plus : leur canal réservé |
| Expert, administrateur | Désignés par l'IFDD | Valident FAQ, sources, quiz, signalements |

- **Mêmes droits pour les femmes et les hommes.** Ce qui distingue une négociatrice n'est pas un champ « genre » mais son appartenance au réseau, portée par le code utilisé.
- **L'admission est un réglage** : code, approbation par un administrateur, ou les deux. Un code se révoque, et l'on sait qui est entré avec lequel.
- **Un seul compte pour l'ePavillon et l'application** — deux couperaient l'historique. Ce qui diffère : le droit (négociateur ou non) et la session, qui note « application mobile » et l'appareil.

## 5. Modules et ordre de livraison

| # | Module | Ce que la personne fait | Modèle de données | Cible |
|---|---|---|---|---|
| 0 | Socle | Installer, entrer avec son code, choisir ses thématiques, lire sans réseau | Comptes prêts ; codes, thématiques suivies, appareils à ajouter | COP31 |
| 1 | Documents | Chercher, télécharger, mettre en favori, voir qu'un document est remplacé | Prêt ; plusieurs thématiques par document à ajouter | COP31 |
| 2 | FAQ et lexique | Trouver une réponse validée ; comprendre un terme anglais en salle | À créer | COP31 |
| 3 | Programme | Réunions du jour de ses thématiques, Francophonie, pavillon ; alertes ; signalements | Réunions prêtes ; source, salle, « déplacée », signalements à ajouter | COP31 (§ 6) |
| 4 | Échanges | Canaux par thématique et promotion, questions aux experts, messages privés, proposer un document | Prêt | Après la COP31 |
| 5 | Assistant IA | Une réponse qui cite le document et la page, ou la vidéo et la minute | Prêt ; transcriptions, obsolescence à ajouter | Ensuite |
| 6 | Quiz | S'exercer sur un document ou une vidéo | Prêt ; provenance IA, validation à ajouter | Avec le 5 |
| 7 | Restitutions | Rédiger son compte rendu ; le partager à un cercle choisi | À créer | À confirmer à la COP31 |

Deux boucles font la valeur de l'ensemble : une question posée à un expert devient une entrée de FAQ, qui nourrit l'assistant ; un document partagé est classé avec l'aide de l'IA, validé, puis rejoint le corpus.

## 6. Règles de confiance

Une erreur peut tromper tout un continent. Six règles :

1. **La source officielle fait foi.** Chaque réunion importée montre sa source, l'heure de dernière lecture et un lien vers l'original. Si la lecture échoue, l'application le dit et renvoie au programme officiel — jamais de donnée périmée en silence.
2. **Un signalement ne s'affiche qu'une fois validé** par un administrateur, en un geste sur son téléphone. Il se pose *par-dessus* la donnée officielle, sans l'écraser.
3. **Un corpus à deux étages.** « Référence », validé par un expert, est le seul que lit l'assistant. « Arrivée » reçoit tout ce qui est capté, marqué non vérifié. L'IA propose le classement ; un humain promeut.
4. **Toute source porte un état** — valide, à vérifier, dépassée, retirée — et un « remplacée par ». Chaque réponse a un bouton « dépassé ou faux » qui alimente la file des experts. Un expert peut poser une note de correction sur une page ou un intervalle de vidéo. À chaque nouveau cycle, les sources du précédent repassent « à vérifier ».
5. **Aucun quiz publié sans relecture d'un expert** ; chaque question garde son passage source. Un quiz régénéré par une négociatrice reste privé et marqué « non relu ».
6. **L'assistant ne répond que depuis la référence**, cite toujours, sait dire « je ne sais pas », et ne conseille jamais la position d'un pays. On le juge sur 30 à 50 vraies questions validées par les experts, rejouées à chaque changement de modèle.

## 7. Architecture

```
Application mobile (Nuxt, installable, hors connexion)
      │
   API Rust — sessions, droits, quotas ──── PostgreSQL, la base de l'ePavillon
      │  file de travaux et d'événements
      ├── Service IA Python, interne — schéma tool + pgvector ──── OpenRouter
      └── Worker Rust — import du programme officiel
```

- **Même base, SQL d'abord.** Compléments au § 5, plus trois transverses : client et appareil sur la session, appareils pour les notifications, états d'obsolescence. Côté code, tout est à écrire : crates `negotiation` puis `training`, et le service IA.
- **Une seule porte, l'API Rust.** Le service Python (FastAPI) reste interne ; il possède le schéma `tool`, conçu pour être détaché (ADR-11), et reçoit les documents par la file d'événements.
- **OpenRouter.** Le modèle qui rédige (Gemini) se change en une ligne ; changer celui qui vectorise (voyage-4) impose de tout réindexer. À corriger dans le SQL **avant le premier document indexé** : la dimension des vecteurs (1536 figés, voyage-4 en produit 1024) et une colonne nommant le modèle d'origine. Écarter dans OpenRouter les fournisseurs qui entraînent sur les données reçues.
- **Mobile : oui au dossier de pages dédié** dans le Nuxt existant, avec sa mise en page, rendu côté navigateur, installable (PWA). Piège : le préfixe `/v2` a déjà produit trois échecs silencieux ; la portée du service worker est le prochain candidat.
- **Capacitor avant le module Échanges** : remplacer WhatsApp exige des notifications poussées, donc les magasins. La session par cookie n'y passe plus : le client mobile aura une session par jeton — le seul endroit où l'authentification distingue l'application du site.

## 8. Feuille de route

**Cette semaine — rien de technique, tout est critique :**

1. Demander au secrétariat de la CCNUCC l'accord écrit, et si possible un flux.
2. Régler les droits d'indexation et de quiz sur les guides.
3. Ouvrir les comptes Apple et Google au nom de l'OIF (plusieurs semaines).
4. Nommer les experts et leur disponibilité pendant la COP.
5. Récupérer les vidéos des 16 modules de formation.

**Avant le 2 novembre** : modules 0 à 3 installés ; le guide COP31, attendu vers le 3, publiable en une journée. *Critère de sortie* : une négociatrice entre avec le code reçu sur WhatsApp, lit le guide en salle sans réseau, trouve « contact group » dans le lexique, voit les réunions du jour de ses thématiques, et signale une annulation que l'administrateur valide depuis son téléphone.

**Pendant la COP31** : vous êtes au stand — le seul test terrain de l'année. Cinq entretiens par jour : qu'avez-vous cherché, trouvé, manqué ? Rédigez-vous une restitution, pour qui ? En coulisse, capter les documents de session.

**Ensuite** : modules 4 à 7, chacun dès que le précédent est stable. Bornes : Bonn (7–17 juin 2027), COP32 à Addis-Abeba (8–19 novembre 2027). Climat d'abord ; les deux autres conventions n'exigent aucun changement de structure.

**Mesures** : installations, personnes actives par jour de COP, téléchargements, recherches au lexique, signalements validés, questions sans réponse.

## 9. Risques et parades

| Risque | Parade |
|---|---|
| Flux officiel inconnu avant l'ouverture, ou accord refusé | Par défaut, un lien vers le programme officiel ; l'import ne s'allume qu'une fois vérifié et se coupe seul s'il échoue |
| Le sort de l'application de 2018 | Un contenu qui change chaque jour, promotion par les ateliers, mesures d'usage, web installable avant les magasins |
| WhatsApp à moitié remplacé | Commencer par ce que WhatsApp ne fait pas ; ne fermer le groupe que si 80 % des membres sont actifs dans l'application. Dire que l'IFDD héberge et modère |
| Réseau saturé sur le site | Tout ce qui se lit se lit sans réseau |
| Validation intenable depuis le stand | Un geste sur téléphone ; experts joints en direct |
| Positions de pays exposées | Restitutions privées, partage par cercle : délégation, groupe de négociation, réseau |

## 10. Décisions et points ouverts

**Décidé le 18/09** : petite version à la COP31 sur un socle prêt pour la suite · ouverte à tous, modules réservés aux négociatrices · code d'invitation, approbation en secours · WhatsApp remplacé à terme · lexique et import automatique du programme dès la COP31 · OpenRouter, Gemini et voyage-4 · même base, service IA en Python.

**Ouvert** :

- L'accord du secrétariat de la CCNUCC — bloque l'import du module 3.
- Les droits sur les guides — bloquent les modules 5 et 6.
- Qui sont les experts, et combien de temps ils donnent pendant une COP.
- Les restitutions et leur partage par cercle : aucune source publique ne décrit ce besoin, à confirmer par entretiens à la COP31.
- Le nom de l'application.

## Sources principales

[Dates des COP](https://unfccc.int/sites/default/files/resource/COP30_2g_dates_venues_auv.pdf) · [conditions d'utilisation de la CCNUCC](https://unfccc.int/this-site/terms-of-use) · [types de réunions et langues](https://unfccc.int/files/resource_materials/application/pdf/20170919_guide_for_presiding_officers_final.pdf) · [guide CdP30](https://www.ifdd.francophonie.org/publications/guide-des-negociations-cdp30/) · [ateliers des négociatrices](https://www.ifdd.francophonie.org/ateliers-de-formation-des-negociatrices-francophones/) · [l'application de 2018](https://web.archive.org/web/20200619034655/https://play.google.com/store/apps/details?id=com.ifdd.app) · [enquête IIED 2025](https://www.iied.org/sites/default/files/pdfs/2025-06/22603iiedfr.pdf) · [« Au nom de ma délégation »](https://www.iisd.org/system/files/2024-03/au-nom-de-ma-delegation-deuxieme-edition.pdf) · [voyage-4 sur OpenRouter](https://openrouter.ai/voyageai/voyage-4)

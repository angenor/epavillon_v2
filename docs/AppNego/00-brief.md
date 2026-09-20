# 00 — Brief

> Guide Négo en une lecture. Le reste du dossier : [01-stack](01-stack.md) · [02-domaine](02-domaine.md) · [03-api](03-api.md) · [04-roadmap](04-roadmap.md) · [05-design](05-design.md) · [06-apres-mvp](06-apres-mvp.md) · [progress](progress.md) · [adr/](adr/). « Négociatrices » inclut toujours les négociateurs : mêmes droits.

## En bref

Guide Négo est l'application mobile de l'IFDD pour les négociatrices francophones des COP. D'abord : les documents de négociation lisibles sans connexion, une FAQ, un lexique anglais-français, les sessions de négociation du jour en français, les réunions de la Francophonie et les activités du Pavillon. Ensuite : les échanges du réseau, un assistant IA qui cite ses sources, les formations en vidéo, des quiz, des restitutions.

- **Sa place.** Rien n'offre aujourd'hui, *en français et hors connexion*, le quotidien d'une session adossé à un savoir validé. Les outils officiels sont en anglais ; les guides sont des PDF.
- **Son atout.** C'est la face mobile de modules déjà dessinés dans l'ePavillon : même base, même API, même compte. Le modèle de données couvre l'essentiel ; aucun code n'existe encore derrière.
- **Son cap.** Première sortie visée : la COP31 (Antalya, 9–20 novembre 2026). Aucun calendrier n'est imposé : l'ordre de la [feuille de route](04-roadmap.md) suffit.
- **Qui le porte.** Le développeur de l'ePavillon, présent à chaque COP dans l'équipe technique du stand de l'OIF : il valide depuis son téléphone et voit l'application servir.

## D'où vient le besoin

À la question « quelle est la première difficulté d'une négociatrice à la COP ? », l'une d'elles répond : la méconnaissance des programmes, surtout chez les nouvelles ; l'anglais ; le faible encadrement de la délégation par certains points focaux ; le financement de la participation des points focaux genre ; les restitutions à rendre chaque jour ou tous les deux jours au groupe pays ou au ministère ; et, pour une nouvelle, la nécessité de se rapprocher du *lead* de son groupe.

| Difficulté | Réponse de l'application |
|---|---|
| Ne pas connaître les programmes | Sessions de négociation du jour en français, filtrées par thématiques suivies, avec alertes |
| L'anglais — « leur plus grand défi », écrit l'IFDD | Lexique hors connexion ; plus tard, résumés en français |
| Faible encadrement ; trouver son coordonnateur | FAQ vivante, parcours « Ma première COP », questions aux experts, annuaire |
| Restitution quotidienne | Aide privée à la rédaction ; partage par cercle choisi |
| Financement de la participation | Hors de portée d'une application |

## Trois agendas, jamais confondus

| Nom complet | Ce que c'est |
|---|---|
| **Sessions de négociation** | Les réunions officielles de la CCNUCC, importées de la source officielle |
| **Réunions de la Francophonie** | Atelier préparatoire, concertation des négociateurs, concertation ministérielle |
| **Pavillon de la Francophonie** | Les activités du stand OIF/IFDD, déjà gérées par l'ePavillon |

Le mot « Programme » seul n'apparaît nulle part — [ADR-008](adr/008-trois-agendas-jamais-confondus.md).

## Ce que la recherche a recadré

1. **La Francophonie n'est pas un groupe de négociation.** Ses pays siègent dans des groupes opposés (Union européenne, Canada, Groupe africain, PMA) ; elle ne tient que trois rendez-vous par COP, plus le Pavillon. Une restitution ne peut donc pas être lisible par toute l'application.
2. **Le programme officiel n'a pas de flux public** : ni RSS ni API, un fichier JSON non documenté par session, et des conditions d'utilisation qui exigent un accord écrit du secrétariat pour le reprendre.
3. **Les réunions décisives ne sont pas toutes annoncées.** *Informal informals* et apartés (*huddles*) ne figurent dans aucun programme : le signalement par le réseau est la seule source.
4. **Hors plénière, tout se passe en anglais**, et le français arrive après coup — parfois après l'adoption.
5. **Le guide paraît une semaine avant la COP** : il faut pouvoir le publier en une journée.
6. **L'IFDD a déjà eu une application** (2018-2019) : une centaine d'installations, jamais mise à jour, retirée des magasins. Un lecteur de PDF ne retient personne.
7. **Des ressources existent en français** — bulletin ENB, lexique de l'IIED, « Au nom de ma délégation » de l'IISD, guides ecbi : on les intègre, on ne les refait pas.

## Public et accès

597 négociatrices formées par l'IFDD depuis 2018, leurs homologues masculins, et tout visiteur.

| Niveau | Qui | Ce qu'il peut faire |
|---|---|---|
| Visiteur | Tout le monde, sans compte | Documents publics, FAQ, lexique, sessions, Francophonie |
| Compte | Toute personne inscrite | Favoris, thématiques suivies, hors connexion, inscriptions, quiz |
| Négociatrice | Admise par code d'invitation | Échanges, documents réservés, signalements, assistant, restitutions |
| Réseau des négociatrices | Code diffusé dans leur groupe WhatsApp | En plus : leur canal réservé |
| Expert, administrateur | Désignés par l'IFDD | Valident FAQ, sources, quiz, signalements |

## Ce que Guide Négo n'est pas

- **Pas un remplaçant de l'application officielle de la CCNUCC** : la source officielle fait foi, et l'application y renvoie dès qu'elle doute.
- **Pas un conseiller de position** : ni l'assistant ni l'IFDD ne disent à un pays quoi défendre.
- **Pas un lecteur de PDF de plus** : c'est ce qui change chaque jour qui fait revenir.
- **Pas un second ePavillon** : ni base, ni compte, ni back-office à part.

## Préalables

| Préalable | État au 18/09/2026 |
|---|---|
| Accord écrit du secrétariat de la CCNUCC pour reprendre les sessions | Demande en cours, la direction est confiante. **Le mécanisme se construit sans attendre** |
| Droits d'indexation et de quiz sur les guides | Acquis |
| Comptes Apple et Google au nom de l'OIF | Ouverts |
| Experts validateurs | Désignés et disponibles |
| Vidéos des 16 modules de formation | Disponibles |

## Risques et parades

| Risque | Parade |
|---|---|
| Flux officiel inconnu avant l'ouverture, ou accord tardif | L'import se coupe seul s'il échoue et renvoie au programme officiel ; essais sur les données archivées de la COP30 |
| Le sort de l'application de 2018 | Un contenu qui change chaque jour, promotion par les ateliers, mesures d'usage |
| WhatsApp à moitié remplacé | Commencer par ce que WhatsApp ne fait pas ; ne fermer le groupe que sur critère |
| Réseau saturé sur le site | Tout ce qui se lit se lit sans réseau |
| Validation intenable depuis le stand | Un geste sur téléphone ; experts joints en direct |
| Positions de pays exposées | Restitutions privées, partage par cercle |

## Décisions

Chaque décision a son [ADR](adr/) : [000](adr/000-guide-nego-est-la-face-mobile-de-l-epavillon.md) face mobile de l'ePavillon · [001](adr/001-un-seul-compte-la-session-distingue-le-mobile.md) un seul compte · [002](adr/002-web-installable-d-abord-capacitor-avant-les-echanges.md) web installable d'abord · [003](adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) hors connexion · [004](adr/004-rust-en-facade-python-au-sidecar.md) Rust en façade, Python au sidecar · [005](adr/005-openrouter-et-embedding-versionne.md) OpenRouter · [006](adr/006-admission-par-code-approbation-en-reglage.md) admission par code · [007](adr/007-le-reseau-distingue-pas-le-genre.md) le réseau, pas le genre · [008](adr/008-trois-agendas-jamais-confondus.md) trois agendas · [009](adr/009-la-source-officielle-fait-foi.md) la source officielle fait foi · [010](adr/010-un-signalement-se-pose-par-dessus.md) signalements · [011](adr/011-un-corpus-a-deux-etages.md) corpus à deux étages · [012](adr/012-toute-source-porte-un-etat.md) obsolescence · [013](adr/013-aucun-quiz-publie-sans-relecture.md) quiz relus · [014](adr/014-whatsapp-est-remplace-a-terme.md) WhatsApp · [015](adr/015-restitutions-privees-partage-par-cercle.md) restitutions · [016](adr/016-vert-et-jaune-fonces-police-hors-charte.md) couleurs et police · [017](adr/017-le-suivi-vit-dans-progress-md.md) suivi · [018](adr/018-direction-typographique-quatre-onglets.md) direction typographique, quatre onglets.

## Sources

[Dates des COP](https://unfccc.int/sites/default/files/resource/COP30_2g_dates_venues_auv.pdf) · [conditions d'utilisation de la CCNUCC](https://unfccc.int/this-site/terms-of-use) · [types de réunions et langues](https://unfccc.int/files/resource_materials/application/pdf/20170919_guide_for_presiding_officers_final.pdf) · [guide CdP30](https://www.ifdd.francophonie.org/publications/guide-des-negociations-cdp30/) · [ateliers des négociatrices](https://www.ifdd.francophonie.org/ateliers-de-formation-des-negociatrices-francophones/) · [l'application de 2018](https://web.archive.org/web/20200619034655/https://play.google.com/store/apps/details?id=com.ifdd.app) · [enquête IIED 2025](https://www.iied.org/sites/default/files/pdfs/2025-06/22603iiedfr.pdf) · [« Au nom de ma délégation »](https://www.iisd.org/system/files/2024-03/au-nom-de-ma-delegation-deuxieme-edition.pdf) · [coordonnateurs des groupes](https://unfccc.int/sites/default/files/resource/unfccc_negotiating_group_chairs_and_coordinators_web_version_16_march_2026.pdf)

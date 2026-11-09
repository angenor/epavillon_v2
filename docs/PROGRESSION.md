# Progression
**Ce fichier est la mémoire du projet entre deux sessions Claude Code.** Toute session commence par le lire et se termine par le mettre à jour.
Il ne porte que ce qui se lit **en arrivant** : les derniers faits, l'état général, l'avancement par prompt, ce qui bloque. Le détail vit dans [`progression/`](progression/) — **on n'en ouvre que le fichier utile à la tâche du jour.**

## Dernière mise à jour : 8 novembre 2026
- 08/11 : **intervenants en ligne** : lien d'inscription d'intervenant par activité (page `/intervenant`, sans compte), lien Zoom personnel qui ouvre l'application, rangée « Intervenants en ligne » sur la carte de la régie (inscrire, renvoyer les liens, envoyer le lien au porteur), copie Angenor et Issa ; envois d'office (porteur à J-2, intervenants à J-1) éteints jusqu'à validation. Migration `2026-11-08-intervenants-en-ligne` à jouer, puis aperçu des courriels. Pas encore en ligne. Voir [journal](progression/journal/2026-11-08.md).
- 07/11 : **journée en présentiel seulement** : interrupteur « Diffusée en ligne » de la page d'un fil (ni inscription, ni direct, réunion Zoom annulée) ; l'atelier de la CdP31 du 08/11 l'éteint ; courriel aux inscrits prêt (aperçu, puis envoi)
- 06/11 : **programme de la CdP31** : le 13/11, « Enjeux de financements pour les régions et les municipalités » passe à 13 h, l'activité de la RDC sur la budgétisation climat à 11 h 30 (production, sauvegarde faite)
- 05/11 : **tutoriel vidéo « image de couverture »** (HyperFrames, voix ElevenLabs par OpenRouter, données d'exemple) et ce que le tournage a révélé : « À faire » rappelle la couverture manquante d'une activité retenue, le lien descend jusqu'à elle, « Déposer une image » sans image, recadrage imposé en 16:9 (migration `2026-11-05-couverture-activite-16-9` jouée). Page d'un fil : les documents acceptent aussi un **lien vers une page de téléchargement** (`event.programme_track_links`, migration `2026-11-05-liens-documents-fil` jouée). Pas encore en ligne. Voir [journal](progression/journal/2026-11-05.md), [tutoriels](tutoriels/LISEZMOI.md).
- 04/11 : **espace organisation en direction condensée et annulation par l'organisation** : « Mon organisation » et la fiche d'un dossier refaites (bandeau sur la photo de l'édition, cartes qui remontent, « Votre dossier » avant « Le jour J », « Format et créneau » masqué une fois retenu) ; l'organisation annule elle-même une activité retenue avant son début, motif obligatoire et jamais publié, inscrits prévenus, courriel aussitôt à l'équipe (adresses de `engagement.mail.automatic_cc`) avec la personne en copie. Décision du back-office : permission de la règle exigée. Migration `2026-11-04-annulation-par-lorganisation` jouée. Pas encore en ligne. Voir [journal](progression/journal/2026-11-04.md).

## État général
| Domaine | État |
|---------|------|
| Modèle de données | ✅ Complet et validé, chargé sur PostgreSQL 17 + pgvector ; le décompte exact vit dans [`README.md`](README.md), les changements dans [`progression/modele.md`](progression/modele.md) |
| Cadrage et décisions | ✅ Complet — 14 ADR, périmètre du jalon arrêté ([`CADRAGE.md`](CADRAGE.md)) |
| Environnement local | ✅ Monté et vérifié le 16/08 — [`progression/environnement-local.md`](progression/environnement-local.md) |
| Front | ✅ Raccordé à l'API depuis le 22/08 ; plus aucun écran ne lit d'exemples depuis le 27/08 ; ajustements d'écrans en septembre à la demande du commanditaire |
| Outillage de la phase B | ✅ Spec Kit installé et constitution ratifiée en 1.0.0 le 20/08 — [B0](progression/ecrans/b0-constitution.md) |
| API | ✅ B0 à B9 livrés ; `make check-api-contract` compte zéro route en attente — [`progression/api.md`](progression/api.md) |
| Mise en ligne | ✅ Servie à la racine de `epavillonclimatique.francophonie.org` depuis le 26/10 (sous `/v2` du 02/09 au 26/10) ; reste : redirections des adresses de la v1 — [`DEPLOIEMENT.md`](DEPLOIEMENT.md) § 17 |

## Front — suivi des prompts
Un écran = un fichier : ce qui a été livré, les écarts entre le modèle et l'interface, ce qui a été vérifié. Un écart se tranche, il ne se contourne pas. Les prompts sont dans [PROMPTS_DEVELOPPEMENT.md](PROMPTS_DEVELOPPEMENT.md).

| Prompt | Écran | État | Détail |
|--------|-------|------|--------|
| A0.1 | Socle Nuxt, Tailwind, i18n, jetons | ✅ 16/08 | [écarts et vérifications](progression/ecrans/a0.1-socle.md) |
| A0.2 | Types TypeScript dérivés du SQL | ✅ 16/08 | [écarts et vérifications](progression/ecrans/a0.2-types.md) |
| A0.3 | Données simulées | ✅ 16/08 | [écarts et vérifications](progression/ecrans/a0.3-donnees-simulees.md) |
| A0.4 | Composants d'interface + page de guide de style | ✅ 16/08 · navigation latérale du back-office refondue 04/09 · barre publique en aplat bleu nuit 04/10, en production le 16/10 | [écarts et vérifications](progression/ecrans/a0.4-composants.md) |
| A1 | Authentification | ✅ 17/08 | [écarts et vérifications](progression/ecrans/a1-authentification.md) |
| A2 | Rattachement à une organisation | ✅ 17/08 · texte allégé 28/09 · rejoindre ne bloque plus le dépôt 28/09 · en production 29/09 | [écarts et vérifications](progression/ecrans/a2-organisation.md) |
| A3 | Page publique de l'événement | ✅ 17/08 · refondue 19/08 · corrigée 27/08 · bandeau de programmation 16/09 · en production · affiche et page d'activité 30/09 · page d'activité resserrée 02/10, présentation détaillée et inscription allégée 03/10, programmations en liste éditoriale 04/10, page d'activité en bandeau 05/10, en production le 16/10 · direction condensée (programmation, calendrier, journée, activité, événement) 01/11 | [écarts et vérifications](progression/ecrans/a3-evenement-public.md) |
| A4 | Formulaire de soumission | ✅ 17/08 · ajusté 15/09 et 16/09 | [écarts et vérifications](progression/ecrans/a4-soumission.md) |
| A5 | Espace organisation | ✅ 17/08 · menu du compte « Mon espace » 05/10, en production le 16/10 · une édition à la fois 28/10 · échanges flottants, profil, pastilles 30/10 · pastilles vues, nature des échanges, accès du super-administrateur 30/10 · éditeur de couverture 31/10 · assistance sur chaque écran 01/11 | [écarts et vérifications](progression/ecrans/a5-espace-organisation.md) |
| A6 | Tableau de bord back-office | ✅ 17/08 | [écarts et vérifications](progression/ecrans/a6-tableau-de-bord.md) |
| A7 | Liste des propositions | ✅ 18/08 · révisé 15/09 | [écarts et vérifications](progression/ecrans/a7-propositions.md) |
| A8 | Fiche d'évaluation | ✅ 18/08 · évaluation flottante 16/09 · en production · correction par l'équipe 17/09, en production · titre et présentation reportés sur la séance 03/10, en production le 16/10 · destinataires et lu / non lu des messages 28/10, en production · annulation d'un dossier retenu 28/10, en production | [écarts et vérifications](progression/ecrans/a8-evaluation.md) |
| A9 | Planificateur de créneaux | ✅ 18/08 · stand par défaut 29/09, en production · dossiers acceptés seuls publiés 28/10, en production · séance commune 01/11 | [écarts et vérifications](progression/ecrans/a9-planificateur.md) |
| A10 | Gestion des événements | ✅ 18/08 · téléversement des visuels 26/08 · liste en rangées 16/09 · fuseau cherchable et brouillon local 16/09 · appel : formats, revues et grille facultatifs 27/09 · en production | [écarts et vérifications](progression/ecrans/a10-evenements.md) |
| A11 | Organisations et fusion | ✅ 18/08 · l'équipe accepte une demande et désigne le référent 31/10 | [écarts et vérifications](progression/ecrans/a11-organisations-fusion.md) |
| A12 | Utilisateurs et rôles | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a12-utilisateurs-roles.md) |
| A13 | Messages d'incident | ✅ 18/08 | [écarts et vérifications](progression/ecrans/a13-incidents.md) |
| A13 bis | Envoi de courriels du back-office | ✅ 28/10, en production | [écarts et vérifications](progression/ecrans/a13-envoi-de-courriels.md) |
| A14 | Page « En cours de maintenance » | ✅ 18/08 · plateforme entière en maintenance, basculée du back-office 16/10, en production, **allumée** | [écarts et vérifications](progression/ecrans/a14-maintenance.md) |
| A15 | Accueil public et vitrine administrable | ✅ 19/08 · panneau « À venir » refondu 24/08 · section d'appel en affiche 16/09 · frise sans salle ni thématiques, carte d'événement masquée une fois son programme publié, témoignage du bandeau en bas de cadre, éditions en affiches ou en liste, oiseau qui suit le curseur 03/10, rail et transitions du bandeau, bouton de programmation au bas d'« À venir » 05/10, en production le 16/10 · direction condensée, temps forts 01/11 | [écarts et vérifications](progression/ecrans/a15-accueil.md) |
| A16 | Régie du jour | ✅ 01/11, en production le 02/11 · réunion créée au démarrage, lien d'animateur en grand 02/11 | [écarts et vérifications](progression/ecrans/a16-regie.md) |
| A17 | Journal du back-office | ✅ 02/11, pas encore en ligne | [écarts et vérifications](progression/ecrans/a17-journal.md) |
| B7 | Raccordement du front à l'API | ✅ 22/08 | [écarts et vérifications](progression/ecrans/b7-raccordement.md) |
| B8 | Module Vitrine (`content`) | ✅ 24/08 · téléversement des images 05/09 | [écarts et vérifications](progression/ecrans/b8-vitrine.md) |
| B9 | Direct + Tableaux de bord (`live`, `analytics`) | ✅ 27/08 | [écarts et vérifications](progression/ecrans/b9-direct-tableaux-de-bord.md) |

## API
B0 à B9 sont livrés, du socle d'identité au raccordement du site (B7, 22/08) puis à `content`, `live` et `analytics` (27/08). Le tableau des prompts et les obligations d'API relevées en écrivant les écrans : [`progression/api.md`](progression/api.md).

## Guide Négo
L'application mobile des négociatrices a son propre dossier, [`AppNego/`](AppNego/), et son propre suivi, [`AppNego/progress.md`](AppNego/progress.md).
Une session Guide Négo n'écrit pas ici — [ADR 017](AppNego/adr/017-le-suivi-vit-dans-progress-md.md).
Seule exception : une modification de `docs/database/` se consigne dans [`progression/modele.md`](progression/modele.md).

## Où trouver le reste
| Question | Fichier |
|----------|---------|
| Qu'a fait la session d'hier ? | [`progression/journal/`](progression/journal/) — un fichier par jour, nommé par sa date |
| Pourquoi cette décision d'interface, de nommage, de découpage ? | [`progression/decisions/`](progression/decisions/) — un fichier par jour |
| Quelles tables ont bougé, et pourquoi ? | [`progression/modele.md`](progression/modele.md) |
| Quels écarts sur l'écran que je reprends ? | [`progression/ecrans/`](progression/ecrans/) — le fichier de son prompt |
| Qu'est-ce qui attend une réponse du commanditaire ? | [`progression/points-bloques.md`](progression/points-bloques.md) |
| Qu'est-ce qui a déjà coûté une erreur ? | [`progression/pieges.md`](progression/pieges.md) |
| Que disait ce fichier avant d'être raccourci, le 17/09 ? | [`progression/archive/`](progression/archive/PROGRESSION-2026-09-17.md) — l'historique détaillé d'août et de septembre |

## Points bloqués
Sept questions au commanditaire attendent, et quelques points restent en suspens : [`progression/points-bloques.md`](progression/points-bloques.md).

## Mettre à jour ce dispositif en fin de session
1. **Le journal du jour** — `progression/journal/<date>.md` : ce qui a été fait, ce qui bloque, ce qui vient. Le fichier se crée s'il n'existe pas.
2. **Le fichier de l'écran travaillé** — `progression/ecrans/<prompt>.md` : livré, écarts, vérifications. Un nouveau prompt ouvre un fichier, déclaré dans le suivi ci-dessus.
3. **Les décisions du jour** — `progression/decisions/<date>.md`, si quelque chose a été tranché.
4. **`progression/modele.md`** — si un fichier de `docs/database/` a changé.
5. **Ici** — l'état général, la ligne de suivi du prompt, et « Dernière mise à jour ».

**« Dernière mise à jour » ne garde que les cinq derniers faits, une ligne chacun — le plus ancien sort quand un nouveau entre ; le détail va dans le journal.** **Ne pas regrossir ce fichier** : il se lit en entier à chaque session, c'est ce qui lui donne sa valeur.

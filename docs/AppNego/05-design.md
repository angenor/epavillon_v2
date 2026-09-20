# 05 — Design

> Ce que l'interface de Guide Négo doit être, et pourquoi. La maquette se fait dans Claude Design avec [design/prompts-claude-design.md](design/prompts-claude-design.md) ; ses livrables reviennent dans [design/](design/). Le vocabulaire de l'interface est dans [design/lexique.md](design/lexique.md).

## Intention

**Agréable, belle, et lisible en plein soleil.** Une application qu'on a plaisir à ouvrir dix fois par jour : chaleureuse, sûre d'elle, à la hauteur d'une déléguée qui représente son pays.

**Guide Négo a sa propre identité.** Elle ne reprend rien du design de l'ePavillon — ni son cyan, ni sa mise en page, ni ses composants, ni sa retenue — [ADR-016](adr/016-vert-et-jaune-fonces-police-hors-charte.md). Seules les couleurs viennent de la charte de l'IFDD.

**Direction retenue le 19/09 : « Typographique »** — le blanc dominant, le vert très foncé comme encre, une seule police. La beauté vient de la typographie, du rythme et de la justesse des espaces. L'écran de référence est dans [design/ecrans/00-direction-retenue.html](design/ecrans/00-direction-retenue.html), la décision dans [ADR-018](adr/018-direction-typographique-quatre-onglets.md).

## Les conditions d'usage commandent tout

- Debout dans un couloir, une main prise, entre deux salles ; ou dehors, en plein soleil, entre deux bâtiments.
- Tard le soir en salle : les séances se prolongent la nuit.
- Téléphone Android de milieu de gamme, écran de 360 px de large, réseau saturé ou absent.
- Peu de temps : trouver une salle, un terme anglais ou un document en moins de dix secondes.
- Des lectrices de documents de négociation : la densité ne les rebute pas, le flou si.

## Couleurs

Dominante **vert foncé**, seconde **jaune foncé** : des nuances foncées du vert et du jaune de la [charte graphique de l'IFDD](../CHARTE_GRAPHIQUE.md), contrastes mesurés. Ce sont des repères : la maquette peut ajuster la clarté d'une nuance si le contraste tient, jamais la teinte.

| Nuance | Valeur | Contraste | Usage pressenti |
|---|---|---|---|
| Vert très foncé | `#233400` | 13,4:1 sur blanc | En-têtes, navigation, grands aplats |
| Vert foncé | `#3C5404` | 8,5:1 sur blanc | Action principale, liens |
| Vert moyen | `#557607` | 5,3:1 sur blanc | Pictogrammes, bordures franches |
| Vert de charte | `#8FBF2F` | 2,2:1 sur blanc | Accent **sur vert foncé** ; jamais en texte sur clair |
| Jaune foncé | `#D3B011` | 7,8:1 avec le noir | Aplat de mise en avant, texte noir dessus |
| Jaune très foncé | `#816B06` | 5,2:1 sur blanc | Texte et pictogrammes jaunes sur fond clair |
| Jaune de charte | `#FFD500` | 1,4:1 sur blanc | Accent **sur vert foncé** ; jamais sur blanc |
| Noir | `#231F20` | — | Texte courant |
| Gris, gris pâle | `#565554`, `#D9D8D6` | 7,4:1 pour le gris | Texte secondaire ; fonds et séparations |
| Teintes claires | `#ECFFD3`, `#FFF7DA` | — | Fonds de bloc vert et jaune |

**Les états d'une session**, décidés avec la direction : En cours vert `#557607` · Déplacée cyan `#0C6792` · Annulée rouge `#C7101D` · Non annoncée violet `#732F85` · Terminée gris `#565554` · Prévue noir. Le système de design étend cette correspondance aux autres objets. Une règle ne bouge pas : **un état = un pictogramme + un mot + une couleur**, jamais la couleur seule.

## Typographie

**Ni Helvetica ni NeueMaverick** : les polices de la charte ne conviennent pas à une application mobile. Critères du choix :

- licence ouverte, fichiers embarqués dans l'application — aucune police ne se télécharge en salle ;
- diacritiques français complets, chiffres tabulaires pour les horaires ;
- grande hauteur d'x, formes ouvertes, `I`, `l` et `1` distincts, `O` et `0` aussi ;
- graisses de 400 à 700, jamais moins de 400.

**Police retenue : Atkinson Hyperlegible Next**, seule, en 400, 600 et 700. Conçue pour la basse vision — c'est exactement la contrainte du plein soleil —, à licence ouverte. La maquette la charge depuis Google Fonts ; l'application l'embarque.

## Lisible en plein soleil

1. Thème clair par défaut, fonds opaques. Texte courant à **7:1** au moins, texte secondaire à 4,5:1 — aucun gris clair.
2. Corps de 17 px, interligne 1,5. L'heure et la salle d'une session : 20 px et graisse 600 au moins.
3. Deux blocs voisins se distinguent sans effort : aplat, bordure franche ou espace — jamais une ombre légère seule.
4. Jamais de texte posé sur une photo, un dégradé ou une surface translucide.
5. Le jaune ne s'écrit jamais sur du clair.
6. Cibles tactiles de **48 px**, action principale en bas de l'écran, à portée de pouce.
7. Thème sombre pour les séances de nuit : pas une inversion, vert et jaune désaturés.
8. Aucune animation n'est nécessaire pour comprendre.

Trois points de l'écran de référence ne tiennent pas encore ces règles, et se règlent au système de design : des filets de liste sous 2:1, des textes à 14 px dont un italique, et des signes d'état qui sont des caractères et non des pictogrammes.

## Navigation

**Décidé le 19/09, complété le 20/09** : une barre basse avec pictogrammes — **Accueil · Négociations · Francophonie · Ressources**, et **Échanges** en quatrième position dès que le module ouvre ; onglets à largeur de libellé. Le lexique n'est pas un onglet : un bouton « Aa », permanent, en haut à droite de chaque écran, l'ouvre d'un geste.

**Trois agendas, jamais confondus** ([ADR-008](adr/008-trois-agendas-jamais-confondus.md)) : « Sessions de négociation », « Réunions de la Francophonie », « Pavillon de la Francophonie ». Le mot « Programme » seul n'apparaît nulle part. Ils ne se mêlent que dans « Ma journée », sur l'accueil, où chaque ligne porte son origine.

Trois éléments reviennent sur tous les écrans : l'état de connexion (« Hors connexion — lu à 14:05 »), l'origine d'une donnée importée (« Source officielle »), et le verrou d'un module réservé, qui invite à saisir son code.

## Inventaire des écrans

| Page de maquette | Écrans | Forme |
|---|---|---|
| Socle | Installation, compte, code d'invitation, attente d'approbation, thématiques, accueil « Ma journée », recherche globale, notifications, profil, états transverses | Groupe |
| Documents | Bibliothèque, fiche, mes documents | Groupe |
| Lecteur | Lecture d'un document, sommaire, recherche | Dédiée |
| Savoir | FAQ, entrée de FAQ, question à un expert, parcours « Ma première COP » | Groupe |
| Lexique | Recherche, entrée | Dédiée |
| Sessions de négociation | Liste du jour, filtres, état de l'import | Dédiée |
| Détail d'une session | Détail, signalements posés par-dessus | Dédiée |
| Signaler, mon agenda | Signalement, suivi, agenda personnel | Groupe |
| Francophonie | Réunions de la Francophonie, Pavillon | Groupe, deux sections distinctes |
| Validation | Files de l'administrateur et de l'expert, sur téléphone | Groupe |
| Échanges | Canaux, annuaire, questions aux experts, proposer un document | Groupe — après le MVP |
| Conversation | Canal, conversation privée, annonce | Dédiée — après le MVP |
| Assistant | Réponse citée, « Je ne sais pas », résumé en français | Dédiée — après le MVP |
| Formations | Modules, fiche, lecteur vidéo avec transcription | Groupe — après le MVP |
| Quiz | Liste, question, correction, quiz personnel | Groupe — après le MVP |
| Restitutions | Rédaction, cercle, fil partagé, synthèse | Groupe — à confirmer |
| Back-office | Gestion sur poste | Groupe, projet séparé |

La maquette couvre **tous** les écrans, y compris ceux d'après le MVP : un système de design se juge sur l'ensemble.

**Le back-office n'est pas l'application.** Ses écrans s'ajoutent au back-office existant de l'ePavillon, où l'équipe gère déjà tout le reste, et en gardent l'apparence. Ils se maquettent à part, pour que rien ne déteigne sur Guide Négo.

## La maquette livrée

Dans [design/ecrans/](design/ecrans/) : dix-huit pages HTML autonomes (00 à 17), leur `index.html`, le système `01-systeme.html` et le `journal-de-maquette.html` — à lire dans l'ordre du [README](design/ecrans/README.md). Dans [design/passation/](design/passation/) : `tokens.json`, `theme.css`, `mesures.css`, `pictogrammes.svg`, `composants.md`, `mouvement.md`, la notice de la police, et [ecarts.md](design/passation/ecarts.md). Dans le code, ce système vit dans le dossier de Guide Négo et ne touche à aucun jeton du site.

Deux décisions prises en maquette : **le jaune signale ce qui vous concerne à l'instant** — session en cours, non-lu, mention — et rien d'autre (une marque de rôle est une étiquette bordée) ; **la barre a cinq onglets** à largeur de libellé dès que les Échanges sont ouverts, quatre avant.

## Les écarts, tranchés

Numéros de [ecarts.md](design/passation/ecarts.md). Le code suit ces décisions, pas les pages qui les contredisent.

| Écart | Décision |
|---|---|
| 1 | `#2E3F0E` entre au système, pour le squelette sombre seulement ; jamais en filet |
| 2, 11 | Badges de maquette et rayon 6 px : hors application |
| 3 | Un seul vert d'état en sombre : `#B5D66A` |
| 4, 27 | Les rôles sans valeur sombre se définissent à l'étape 0, contrastes mesurés ; aucun écran ne sort sans son thème sombre |
| 5 | Pictogramme Document : vert moyen dans une liste, vert très foncé dans un menu ou un bouton |
| 6, 15 | Pas de 16 px : onglets de filtre et titres de ligne à 17 ; hauteur d'onglet de filtre 48 |
| 7 | Page 00 : planche d'archive, rien à en reprendre sous 15 px |
| 8, 9 | « Aa » et « GN » sont des glyphes de bouton, hors échelle de texte |
| 10 | 13 px : libellés d'onglets, marque de rôle, compteur, jour de la bande — rien d'autre |
| 12 | Barre à largeur de libellé partout ; les pages à quatre onglets égaux sont antérieures |
| 13, 14 | Précisions d'écriture : 48 px pour une ligne de liste, 56 px pour une ligne à réglage, à cocher ou à cercle |
| 16, 17, 18 | `pause`, `chevUp` et `minus` entrent dans la famille ; le compte est celui de `pictogrammes.svg` |
| 19 | Le bouclier coché dit « validé par un expert », sous ses trois libellés : confirmé |
| 20 | Croix rouge = annulé ; triangle rouge = erreur ou lecture impossible |
| 21, 22, 24, 25 | Données et phrases : une seule phrase de verrou, celle de la page 12 ; « Synchronisé à » porte l'heure de dernière lecture, distincte de l'instant |
| 26 | Icône provisoire : `frontend/public/logos/svg/epavillon-symbole-inverse.svg`, à la place du monogramme « GN » |
| 28, 29 | À faire à l'étape 0 : ordre de focus et annonces d'état, vérification à 390 px |

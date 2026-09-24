# Contrat — l'écran du lecteur

Ce que l'écran `pages/guide-nego/ressources/documents/[id]/lire.vue` et ses composants promettent. Les écarts avec la maquette sont les écarts 43 et 44 de [05-design.md](../../../docs/AppNego/05-design.md).

## Les deux modes

| | « Pages » (défaut) | « Texte agrandi » |
|---|---|---|
| Composant | `GnLecteurPages` (nouveau, client seulement) | `GnLecteurTexte` (sorti de `lire.vue`, comportement de l'étape 1) |
| Source | PDF : copie (`getDocument({ data })`) ou route du fichier par plages | Lecture (`blocks`) : copie ou `…/reading` |
| Offert | Toujours, pour un fichier | Si `large_text` |
| Pied | « Page {label} sur {page_count} · {section} » et jauge — `GnBarreLecture`, inchangé | Idem |
| Recherche | Liste de l'étape 1 ; passage marqué sur la couche de texte (R7) | Étape 1 |
| Notes | `GnMargeNote` en marge, panneau non modal | `GnNoteCorrection` sur le bloc, étape 1 |
| Taille du texte, termes touchables | Non | Oui |

Passer d'un mode à l'autre garde l'index de page. Le mode choisi vit en `localStorage` (`gn.lecture-mode`) ; un document sans « Texte agrandi » s'ouvre en pages sans changer ce choix et l'annonce une fois par `GnAnnonce`.

## La barre dépliée — quatre emplacements

| Emplacement | Contenu | Condition |
|---|---|---|
| 1 | « Sommaire » (`toc`) | `has_text` et sommaire non vide |
| 2 | « Rechercher » (`search`) | `has_text` |
| 3 | `GnChoixMode` « Pages · Texte » — à la place de « Marquer » | `large_text` |
| 4 | « Réglages » (`sliders`, **nouveau pictogramme**) | Toujours — la feuille porte au moins le thème |

« Aa », dans l'en-tête, n'ouvre que le lexique (ADR-018) ; aucun autre bouton de l'écran ne le ressemble.

## La feuille « Réglages » (`GnReglagesLecture`)

1. `GnChoixMode`, si `large_text`.
2. Le thème — Clair, Sombre, Système — réglage de 0a.
3. La taille — Normale, Grande, Très grande, 17 / 20 / 24 px — **en mode texte seulement**.
4. La phrase de l'étape 1 : le réglage vaut pour tous les documents et se retrouve dans le profil.

## Gestes en mode « Pages »

| Geste | Effet |
|---|---|
| Défilement vertical | Pages suivies, `pagechanging` tient le pied et la progression |
| Pincement | Grossissement autour du point, de la largeur à 4 × la largeur ; redessin net de la zone visible après le geste |
| Double toucher | Double l'échelle autour du point ; au-delà de la largeur, revient à la largeur |
| Toucher simple | Déplie ou replie la barre, 300 ms après, si aucun second toucher n'est venu |
| Appui long | Sélection de la couche de texte (navigateur) |

## Composants nouveaux (dossier de Guide Négo, planche des composants)

| Composant | Rôle |
|---|---|
| `GnLecteurPages` | Enveloppe de `PDFViewer` : gestes, couche de texte, surlignages, marges des notes, repli d'appareil trop ancien (R1) |
| `GnLecteurTexte` | Le rendu recomposé de l'étape 1, sorti de la page |
| `GnChoixMode` | Deux segments « Pages · Texte », 48 px, dans la barre et dans la feuille |
| `GnMargeNote` | Filet de 3 px, triangle, cible de 48 px ; déplié en panneau non modal de 40 % de la hauteur au plus |
| `GnAttentePages` | L'attente de la première page en ligne : jauge « reçu sur demandé », puis au bout de 3 s « Lire le texte en attendant » et « Télécharger pour lire sans réseau » (FR-009 bis) |

## États

| État | Rendu |
|---|---|
| Chargement, copie gardée | `GnChargement` jusqu'à la première page rendue |
| Chargement en ligne | `GnAttentePages` : la forme lisible se lit d'abord, puis le PDF par le transport de plages. La jauge montre les octets reçus sur les octets demandés, et suit les demandes nouvelles de pdf.js. **Au bout de 3 s sans page**, deux boutons : « Lire le texte en attendant », si `large_text`, qui ouvre `GnLecteurTexte` à la même page, et « Télécharger pour lire sans réseau », le téléchargement de l'étape 1. Quand la page est prête, elle prend la place du texte, **sauf si la personne a touché « Rester sur le texte »** (ou changé de mode elle-même) ; la décision vit dans l'instance du lecteur, le mode gardé ne change pas |
| Pas sur le téléphone, sans réseau | Écran de l'étape 1, inchangé |
| Page non encore reçue, réseau tombé | La page dit qu'il faut le réseau et propose « Télécharger au retour du réseau » |
| Appareil qui n'affiche pas les pages | Décidé par **détection des fonctions manquantes** — jamais par l'identifiant du navigateur —, **ou par la seconde sécurité** : erreur de pdf.js, ou première page non dessinée après 8 s de travail de pdf.js, réglable — **le délai ne court que lorsqu'aucune plage demandée n'est en route** : le réseau lent ne fait jamais basculer (arbitré après l'essai). L'événement est compté sur le téléphone (`gn.lecture-bascules`), et les pages ne se réessaient qu'à l'ouverture suivante. « Texte agrandi » s'il est offert, avec une ligne qui le dit ; sinon `GnEtatErreur` qui dit que ce téléphone ne peut pas afficher ce document. Jamais le visionneur du téléphone |
| Réservé sans accès, 404, erreur | Étape 1, inchangés |

## Accessibilité

- La couche de texte porte le texte de la page pour le lecteur d'écran ; le lecteur annonce la page en cours comme à l'étape 1.
- `GnChoixMode` est un groupe de deux boutons à `aria-pressed`, `GnMargeNote` un bouton à `aria-expanded`.
- Cibles de 48 px, 44 au minimum ; aucune dans la zone de page n'empêche la sélection.

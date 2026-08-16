# Prompt — Guide de style ePavillon v2 (Claude Design)

Un seul prompt, à coller tel quel dans Claude Design. Il produit le **guide de style** : la page de référence qui fixe les jetons de design, la typographie, les composants et leurs états.

Ce guide n'est pas un livrable décoratif — c'est le contrat visuel que Claude Code appliquera ensuite à chaque page. Une fois validé, il devient un fichier `frontend/app/assets/css/design-tokens.css` et un composant `pages/style-guide.vue` conservé dans le projet.

---

## Le prompt

```
Tu produis le GUIDE DE STYLE d'une plateforme institutionnelle réelle : une page
unique qui montre tous les jetons de design, tous les composants et tous leurs
états. Ce n'est pas une maquette d'écran : c'est le référentiel qui servira à
construire toutes les pages ensuite.

Réponds en français. Produis un artefact HTML autonome avec TailwindCSS
(tout en ligne, aucune ressource externe).

=== LE PRODUIT ===
ePavillon est la plateforme numérique de l'IFDD (Institut de la Francophonie
pour le développement durable), organe de l'Organisation internationale de la
Francophonie. Elle sert les grandes conférences des Nations unies — COP climat,
biodiversité, désertification — ainsi que des webinaires et un espace réservé
aux négociateurs francophones.

Les utilisateurs sont des professionnels : agents ministériels, responsables
d'ONG, chercheurs, négociateurs climat. Ils viennent d'Afrique de l'Ouest et
centrale, d'Europe, du Maghreb, d'Haïti, du Vietnam, du Liban, du Canada.
Beaucoup consultent depuis un mobile, sur une connexion irrégulière.

=== CHARTE GRAPHIQUE IFDD (officielle, à respecter strictement) ===
Source : Charte_graphique_Ifdd_couleurs.ai, 1er septembre 2023.

Couleurs principales
  cyan    #00A1E4
  rouge   #E63132
  jaune   #FFD500
  vert    #8FBF2F
  violet  #732F85
  gris    #565554

Complémentaires
  bleu riche #1D1A5B
  gris pâle  #D9D8D6

Accents (versions claires, pour fonds et pastilles)
  cyan   #8CD5F2
  rouge  #F28385
  jaune  #FFF08C
  vert   #ADD476
  violet #906CAD

Neutres
  blanc #FFFFFF
  noir  #231F20

Polices : NeueMaverick pour les titres, Helvetica pour le texte courant.
NeueMaverick n'étant pas disponible en ligne, utilise une grotesque géométrique
de substitution pour les titres et note ce choix en commentaire.

=== CE QUE TU DOIS DÉCIDER, PAS SEULEMENT AFFICHER ===
La charte donne six couleurs de marque, pas une interface. Ton travail est de
construire à partir d'elles un système utilisable, en respectant ces règles :

1. Les couleurs de marque ne sont pas des couleurs d'interface. Le cyan #00A1E4
   sur fond blanc ne passe pas le contraste AA en texte. Dérive les nuances
   nécessaires (une échelle de 50 à 900 par teinte) et dis clairement lesquelles
   sont utilisables en texte, lesquelles en fond, lesquelles en bordure.
2. Sémantique explicite : succès, avertissement, erreur, information, neutre.
   Rattache chacune à une couleur de la charte plutôt que d'introduire des
   couleurs étrangères.
3. Le gris #565554 et le gris pâle #D9D8D6 portent la structure : fonds,
   bordures, séparateurs, texte secondaire. Construis-en une échelle complète.
4. Thème sombre : ce n'est pas un inversement. Le vert #8FBF2F et le jaune
   #FFD500 deviennent agressifs sur fond noir — désature ou assombris.
   Définis la palette claire sur `:root`, redéfinis-la sous
   `@media (prefers-color-scheme: dark)` gardé par
   `:root:not([data-theme="light"])` ET sous `:root[data-theme="dark"]`.
5. Tous les jetons sont des variables CSS nommées en clair
   (`--color-surface`, `--color-text-muted`, `--space-4`, `--radius-md`…),
   pas des valeurs en dur répétées.

=== TON ET DIRECTION ARTISTIQUE ===
Institutionnel et sérieux, mais vivant. Ni tableau de bord SaaS générique, ni
site d'ONG militant.

Références de posture : le site des Nations unies pour la rigueur, une revue
scientifique en ligne pour la lisibilité, une billetterie de festival pour
l'énergie de la programmation.

À FAIRE
- Hiérarchie typographique forte : les titres portent le sens, pas les icônes.
- Densité informationnelle assumée. Ces gens lisent des documents de
  négociation ; un tableau bien composé ne les rebute pas.
- Beaucoup de blanc entre les blocs, peu à l'intérieur des blocs.
- La couleur distingue des états et des thématiques ; elle ne décore pas.
- Coins légèrement arrondis (6 à 8 px), ombres très discrètes ou bordures fines.

À ÉVITER ABSOLUMENT
- Dégradés, effets de verre dépoli, néons, halos flous.
- Illustrations 3D, blobs, formes organiques flottantes.
- Emoji en guise d'icônes fonctionnelles.
- Les tournures marketing (« Boostez », « Révolutionnez ») et le mot « Bienvenue ».

=== CONTENU DU GUIDE (dans cet ordre) ===

1. JETONS
   - Palette complète : chaque nuance avec son nom de variable, sa valeur
     hexadécimale, et son rapport de contraste calculé sur fond clair et sur
     fond sombre. Marque visiblement celles qui échouent en AA pour du texte.
   - Échelle typographique : 7 niveaux maximum, avec taille, graisse, hauteur de
     ligne et usage prévu. Montre un vrai paragraphe français de 60 mots à
     chaque niveau de corps de texte.
   - Échelle d'espacement (base 4 px), rayons, épaisseurs de bordure, ombres.

2. COMPOSANTS DE BASE, chacun avec TOUS ses états
   (repos, survol, focus clavier, actif, désactivé, chargement)
   - Boutons : principal, secondaire, discret, danger, avec icône, taille
     normale et compacte. Rappel : en Tailwind v4 les boutons n'ont pas
     `cursor-pointer` par défaut, ajoute-le.
   - Champs de formulaire : texte, zone de texte, liste déroulante, recherche,
     case à cocher, bouton radio, interrupteur, sélecteur de date.
     Montre les états : vide, rempli, focus, erreur avec message, aide
     contextuelle, désactivé, lecture seule.
   - Jetons et pastilles : thématique (couleur par thème), statut, filtre
     retirable, compteur.
   - Bandeaux d'alerte : information, succès, avertissement, erreur — avec titre,
     corps, action, et bouton de fermeture.

3. COMPOSANTS COMPOSÉS
   - Carte d'activité : créneau avec fuseau horaire, titre, organisation avec
     pays, pastilles thématiques, format, salle, jauge d'inscription, état
     temporel (à venir / en cours / en direct / terminé / reporté / annulé).
     C'est le composant le plus utilisé de la plateforme : soigne-le.
   - Tableau de données : en-têtes triables, ligne sélectionnable, pagination,
     état vide, état de chargement en squelettes. Montre-le avec 12 lignes de
     données réalistes, pas 3.
   - Navigation : barre principale avec sélecteur de langue (FR/EN) et bascule
     de thème, fil d'Ariane, onglets, navigation latérale du back-office.
   - Panneau latéral, boîte de dialogue de confirmation, menu contextuel.
   - Frise de progression (suivi d'un dossier : déposé → en évaluation →
     décision) et barre d'étapes d'un formulaire long.

4. MOTIFS TRANSVERSAUX
   - Les quatre états d'un écran : chargement (squelettes), vide, erreur,
     accès refusé. Traite-les comme des composants de plein droit, pas comme
     des cas particuliers — ce sont eux qu'on oublie et qui font mauvaise
     impression en production.
   - Affichage d'une date avec fuseau : « 14:30 — 16:00 (heure de Belém, UTC−3) ».
   - Bandeau d'incident sur trois niveaux de gravité.
   - Pastille « en direct », avec sa règle d'usage : une seule activité peut la
     porter à un instant donné, l'IFDD ne diffuse jamais deux directs en même
     temps.

5. RÈGLES D'USAGE, écrites
   Une courte liste de phrases qui tranchent : quand utiliser un bouton
   secondaire plutôt qu'un lien, quelle couleur pour quel statut, comment
   composer une pastille thématique, quelle largeur maximale pour un paragraphe.

=== CONTRAINTES TECHNIQUES ===
- TailwindCSS v4 : pas de `bg-opacity-*`, utiliser `bg-cyan/50`.
- Responsive réel : le guide doit tenir à 375 px. Les blocs larges défilent dans
  leur propre conteneur `overflow-x: auto` ; le corps de page ne défile jamais
  horizontalement.
- Accessibilité : contraste AA minimum, focus visible au clavier, libellés de
  formulaire associés, zones cliquables d'au moins 44 px.
- Bilingue français / anglais : laisse respirer les libellés, les textes anglais
  étant plus courts d'environ 20 %, mais certaines tournures françaises étant
  30 % plus longues.
- Une bascule de thème clair/sombre fonctionnelle dans la page.

=== DONNÉES ===
Jamais de lorem ipsum. Utilise des contenus réalistes en français : titres
d'activité plausibles, noms d'organisations francophones FICTIFS (« Réseau
ouest-africain pour l'adaptation », « Institut national de l'environnement du
Bénin »), pays réels, dates cohérentes. N'attribue aucun propos à une personne
réelle.

Thématiques à utiliser pour les pastilles : Atténuation · Adaptation ·
Pertes et préjudices · Finance climatique · Genre · Agriculture et alimentation ·
Biodiversité · Désertification.

=== LIVRABLE ===
Une page HTML unique, autonome, avec un sommaire ancré permettant de naviguer
entre les sections. À la fin de la page, un bloc `<pre>` contenant toutes les
variables CSS définies, prêt à être copié dans un fichier de jetons.

Avant de coder, écris en cinq lignes les décisions que tu prends sur les
nuances dérivées et sur le thème sombre, et pourquoi.
```

---

## Après génération

1. **Vérifier les contrastes annoncés.** Claude calcule les rapports, mais recoupez au moins le cyan et le vert sur fond blanc — ce sont les deux pièges de cette charte.
2. **Récupérer le bloc de variables CSS** en fin de page → `frontend/app/assets/css/design-tokens.css`.
3. **Conserver la page** dans le projet comme `pages/style-guide.vue`. Elle sert de référence pendant tout le développement et de test de non-régression visuelle.
4. **La citer dans chaque prompt de page** adressé ensuite à Claude Code : voir [PROMPTS_DEVELOPPEMENT.md](PROMPTS_DEVELOPPEMENT.md).

## Prompts de reprise

| Situation | Prompt |
|-----------|--------|
| Palette dérivée bancale | « Les nuances dérivées du vert #8FBF2F tirent vers le kaki. Reprends l'échelle en conservant la teinte et en jouant sur la saturation et la luminosité. » |
| Contrastes optimistes | « Recalcule les rapports de contraste et marque en rouge toute combinaison sous 4,5:1 pour du texte courant et sous 3:1 pour du texte large. » |
| Thème sombre inversé | « Le thème sombre est un simple inversement. Reprends-le : le vert et le jaune de la charte sont agressifs sur fond noir, désature-les ; les surfaces doivent être des gris chauds dérivés du #231F20, pas du noir pur. » |
| États manquants | « Il manque les états de focus clavier et de chargement sur plusieurs composants. Complète, sans exception. » |
| Guide trop maigre | « Montre chaque composant avec des données réalistes et en quantité : 12 lignes de tableau, 6 cartes d'activité, pas un exemplaire de chaque. » |

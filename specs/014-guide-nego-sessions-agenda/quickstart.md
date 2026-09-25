# Quickstart — recette de l'étape 3a

Dossier `/Users/mac/Documents/projets/IFDD/epavillon_v2-dev2`, base `epavillon_dev2`, API sur
**8096**, site sur **3005** (`APP_PUBLIC_URL=http://localhost:3005`). Un seul worker sur cette base.
Version **construite** du site (`npm run build && node .output/server/index.mjs`), pas le serveur de
développement : le service worker n'existe qu'en construction.

## 0. Préalables

1. Base migrée par [migration.sql](migration.sql) ; `cargo run -p api` et `cargo run -p worker`.
2. Un compte administrateur de la plateforme ; un compte admis dans Guide Négo, qui suit
   « Adaptation » et « Genre ».
3. `OPENROUTER_API_KEY` : faire la recette **une fois sans** (anglais seul), puis avec si la clé est
   posée — une trentaine de titres traduits, pas davantage.

## 1. Import éteint (SC-004)

Ouvrir `/guide-nego/negociations` → aucune session, renvoi au programme officiel de la CCNUCC.

## 2. Allumer l'import (US2, SC-001)

Back-office → Négociations → Import : lecteur « archive », jeu `cop30/lecture-1`, premier jour de
l'archive = **aujourd'hui** ; allumer. Dans les cinq secondes : l'état dit « lu à … », N sessions,
N écarts. L'application montre la bande des jours et la liste du jour.

« Lire maintenant » sans rien changer → **0 écart** ; le journal compte une lecture de plus.

## 3. Les écarts (SC-002)

Choisir `cop30/lecture-2` (une session avancée, une changée de salle, une disparue, une nouvelle) →
lire maintenant → **4 écarts** au journal. Dans l'application : la déplacée porte « Déplacée » et
l'ancienne heure barrée ; la changée de salle, « était … salle … » ; la nouvelle paraît. **La disparue
est encore là** : « Lire maintenant » une seconde fois → elle passe « Annulée », retirée du programme
officiel.

## 4. La coupure (US2 sc. 5, SC-003)

Seuil = 2. Choisir `injoignable` → lire deux fois. Après la seconde : l'application affiche
« Lecture impossible », « n'a pas répondu depuis … », aucune session, le lien vers le programme
officiel et « Réessayer ». Revenir à `lecture-2` → lire → la liste revient seule.

Arrêter le worker, attendre `seuil × intervalle + 1 min` → l'application se coupe aussi.

## 5. La liste (US1)

- Filtre « Mes thématiques » : Adaptation, Genre, les sessions « Thématique non précisée », toutes
  les coordinations (aucun groupe coché). Cocher « Groupe africain » sur l'écran des thématiques →
  seules ses coordinations restent, marquées « Mon groupe ».
- Back-office → Ordre du jour : rattacher un point à « Finance » → ses sessions quittent le filtre
  au rafraîchissement.
- Chaque heure porte « heure d'Antalya » ; le téléphone réglé sur un autre fuseau n'y change rien.
- Un jour sans session de ses thématiques : l'état vide, « Voir toutes les sessions », « Modifier mes
  thématiques ».

## 6. La fiche (US3)

Déplacée : heure et salle barrées → nouvelles ; jour et fuseau sous l'heure. Type : toucher
« contact group » ouvre la feuille avec ce texte. « Hors ordre du jour officiel » sur une session sans
point. Aucune section de documents (la source n'en donne pas). Annulée : « Ajouter à mon
agenda » désactivé.

## 7. Mon agenda (US4)

Revenir à `lecture-1` et lire. Connecté : ajouter deux sessions qui se chevauchent, dont **celle qui
disparaît dans `lecture-2`**, et une troisième → les deux portent « Chevauche … — … ». Choisir
`lecture-2` et lire deux fois → la disparue est annulée, plus de chevauchement, plus de rappel. Armer « Me
rappeler 15 minutes avant » sur une session qui commence dans moins de quinze minutes (archive
décalée à aujourd'hui) → le bandeau paraît en tête, une fois ; la ligne dit « Rappel 15 minutes
avant » ; sous l'interrupteur, la phrase « dans l'application ouverte, sans sonnerie ».
« Ma journée » : le bloc « Votre prochaine session de négociation » montre la prochaine de l'agenda.

Sans compte : « Ajouter à mon agenda » propose de se connecter.

## 8. Hors connexion (US5, SC-007, SC-008)

Lire la liste en ligne, puis couper le réseau (outils du navigateur) et recharger : tous les jours,
toute fiche, « Mon agenda », « Hors connexion — lu à … » et le bandeau à la première ouverture.
Ajouter une session sans réseau → visible aussitôt ; réseau rétabli → **une** ligne en base.

## 9. Écrans

360 px, clair et sombre, contre 07, 08, 09 et le bloc de 02 : aucun défilement horizontal, états
reconnaissables sans la couleur (SC-006), cibles de 48 px.

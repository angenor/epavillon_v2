# Quickstart — recette de l'étape 5

Dossier dev2, base `epavillon_dev2`, API 8096, site 3005, version **construite**, Mailpit. Seuls ses PID.

## 0. Préalables

Programme de `cop31` publié avec quelques activités : une aujourd'hui, une en cours, une passée hier avec
une rediffusion (`live.streams` de type `replay` — s'il n'en existe pas, en poser une par le back-office
du site ou, à défaut, le dire), une pleine avec liste d'attente, une dont le formulaire demande une
question de plus que le pays. Un lieu de type « pavilion » avec son adresse et son plan. Comptes de
recette de l'étape 4.

## 1. Lire

Segment Pavillon : bloc de lieu, « Aujourd'hui » et compteur, « Hier — rediffusions », « Les jours
suivants ». Toutes les activités, quelles que soient mes thématiques. Détail : organisateurs par leur
nom, langue, intervenantes par leur nom, rediffusion « · N min » qui s'ouvre. Hors connexion : « lu à ».

## 2. S'inscrire

D'un geste sur une activité au formulaire simple (pays prérempli) ; formulaire complet sur l'autre ;
pleine → liste d'attente ; annuler ; hors connexion → une seule inscription au retour ; « Complet »,
« Inscriptions closes », « Pas encore ouvertes » dits. Sans compte → connexion.

## 3. Liens et « Ma journée »

Depuis une réunion liée, l'étiquette ouvre l'activité. « Ma journée » : ligne Pavillon avec l'activité du
jour, ou « Prochaine : … ».

## 4. Le site n'a pas changé

`npm run test:site`, `make check-api-contract` ; la page `programme` du site s'affiche comme avant.

## 5. Le scénario qui clôt le MVP (après fusion de 2 et 4 dans `main`, puis `main` dans cette branche)

Code d'invitation (reçu « sur WhatsApp ») → installer → télécharger le guide → le lire en mode avion →
« contact group » dans le lexique → sessions du jour de mes thématiques avec « lu à » → signaler une
annulation → l'administrateur valide depuis son téléphone → l'encart paraît, la personne qui suit est
prévenue. Version construite, 360 px.

## 6. Écrans

360 px, clair et sombre, contre 10 (1c, 1d) et 02 (07).

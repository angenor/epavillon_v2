# Quickstart — recette de l'étape 4

Dossier dev2, base `epavillon_dev2`, API 8096, site 3005, version **construite**, Mailpit. On n'arrête que
ses propres PID.

## 0. Préalables

Base migrée ; API et worker ; comptes : administrateur `recette.docs.admin@example.org`, admise
`p9-agenda-1790357034@exemple.test`, sans accès `p9b-sans-acces@exemple.test`, une seconde admise pour la
liste d'attente. Une activité du Pavillon existe dans l'édition `cop31` (sinon, la créer au back-office
du site).

## 1. Saisir et publier (US3, SC-001)

Back-office → Négociations → Réunions : saisir les trois natures ; l'une en ligne avec un lien, l'une à
accès limité (« ministres et chefs de délégation »), l'une avec capacité 1 et liée à l'activité du
Pavillon, l'une avec capacité 1 **sans** liste d'attente. En brouillon : invisibles dans l'application. Publiées : visibles, avec l'étiquette Pavillon.

## 2. Lire (US1)

Onglet « Francophonie » : sélecteur, titre « Réunions de la Francophonie », trois lignes avec fuseau ;
fiche ; segment « Pavillon » → titre « Pavillon de la Francophonie », section vide ; l'étiquette ouvre
cette section. Sans compte : tout se lit, aucun lien visio. Hors connexion : liste et fiches avec
« lu à ».

## 3. S'inscrire (US2, SC-002, SC-003)

Admise : « M'inscrire » → « Inscrite » ; le lien « Rejoindre à distance » paraît, et reste hors
connexion. Seconde admise sur la réunion à capacité 1 : « Complet », « Rejoindre la liste d'attente » →
« Liste d'attente — position 1 ». Première se désinscrit → la seconde passe « Inscrite », avis dans la
cloche et courriel (Mailpit). Hors connexion : s'inscrire, rétablir → une ligne ; si la réunion s'est
remplie entre-temps (sans liste d'attente), « Complet » est dit. Sans accès : mène à l'accès.

## 4. Annuler, modifier (US3, FR-020)

Changer l'heure d'une réunion publiée → inscrites prévenues ; l'annuler avec un motif → « Annulée » et
avis. Supprimer l'activité du Pavillon liée (ou la délier) → l'étiquette disparaît, la réunion reste.
Un compte sans la permission : écrans refusés, y compris par adresse.

## 5. Ma journée, recherche

Une réunion du jour : ligne « Réunions de la Francophonie » ; aucune : « Rien aujourd'hui. Prochaine :
… ». Ligne « Sessions de négociation » ; ligne Pavillon vide. Recherche (si l'étape 2 est fusionnée) :
groupe « Réunions de la Francophonie ».

## 6. Écrans

360 px, clair et sombre, contre 10 (1a, 1b) et 02 (07) ; une marque par ligne ; cibles 48 px. **À la fin :
réunions de recette laissées en base, import éteint.**

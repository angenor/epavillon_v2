# Recette — étape 2 : FAQ, parcours et lexique

Sur la **version construite**, au navigateur, à 360 px, thème clair puis sombre. Ports de dev1 : API 8095, site 3004, base `epavillon`. Ce qui ne se prouve que sur un téléphone réel va au § 15 de [DEPLOIEMENT.md](../../docs/DEPLOIEMENT.md) ; **ne jamais dire l'avoir testé sur téléphone**.

## 0. Préparer

1. Migrer sans détruire : `psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f specs/013-guide-nego-faq-lexique/migration.sql`, **deux fois** ; comparer `pg_dump --schema-only` avec une base jetable chargée depuis `docs/database/` (§ 13 de DEPLOIEMENT.md). Jamais `down -v`.
2. Un compte `expert`, un compte `admin`, un compte admis par code (négociatrice), un compte sans accès.
3. Charger les données d'essai : `psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f specs/013-guide-nego-faq-lexique/donnees-essai.sql`, deux fois — le second passage n'ajoute rien.
4. `npm run build` puis servir la version construite ; API sur 8095.

## 1. Le lexique hors connexion — critère de sortie (récit 1)

1. Ouvrir Guide Négo avec le réseau, puis passer hors connexion (outils du navigateur) et recharger.
2. Depuis « Ma journée », Ressources et un document : « Aa » ouvre le lexique, champ actif, « 19 entrées, sans réseau ». La croix ramène à l'écran d'origine.
3. Taper `contact grup` → *contact group* en tête, en moins d'une seconde. `groupe de contact` → la même entrée, « Aussi dans les traductions françaises ». `braketed` → « Vous cherchiez peut-être : *bracketed text* ». `GGA` → *global goal on adaptation*. `placeholder text` → « 0 entrée sur 19 », « Proposer ».
4. Ouvrir *contact group* : famille, traduction, définition, « Entendu en salle », sources, termes liés (chacun s'ouvre). Favori ; retrouver l'entrée dans « Mes termes favoris ».
5. Liste : lettres et comptes, rail (toucher, glisser), lettres vides grisées, filtre « Réunions ».
6. `/guide-nego/lexique?terme=Contact%20Group` ouvre `/guide-nego/lexique/contact-group` ; `?terme=GGA` ouvre l'entrée du GGA ; `?terme=foo` tombe sur « aucun résultat ».
7. **SC-004** : gonfler la garde à 500 entrées (script de la console fourni par la phase), mesurer le temps par frappe.

## 2. La FAQ (récits 2 et 5)

1. FAQ : nombre de réponses, ligne de synchronisation, rubriques et comptes, parcours, « Les plus lues », « Poser une question ».
2. L'entrée complète : « Vérifié le … », réponse, deux sources dont une citation ; la source du guide ouvre le document à la page citée (copie gardée hors connexion, sinon l'écran « pas sur votre téléphone »).
3. Hors connexion, chercher `contact` dans la FAQ : l'entrée paraît.
4. Au back-office, mettre l'entrée « À revoir » : au retour du réseau, le téléphone l'affiche avec « Un expert relit cette réponse » et sa date. Corriger la réponse : seule elle est relue (réseau des outils : `?since=`, `304` à la relecture suivante).
5. Sans compte : « Non » et « Dépassé ou faux » mènent à la connexion. Connecté : « Oui » → « Merci. » ; « Non » → « Trop vague » ; « Non » → « Dépassée ou fausse » ; hors connexion, « Dépassé ou faux » avec deux motifs et une précision, puis retour du réseau : le signalement arrive **une fois** dans la file, sans nom.

## 3. Questions aux experts (récit 6)

1. Compte sans accès : le verrou, « Saisir mon code d'invitation ».
2. Négociatrice : question avec consentement, puis une sans, la seconde hors connexion. Écran « Envoyé » sans délai promis.
3. Expert : répondre aux deux ; un courriel arrive dans Mailpit (`http://localhost:8025`) pour chacune ; « Mes questions » les montre répondues. Promouvoir la première : brouillon de FAQ sans nom. La seconde refuse la promotion.
4. Se déconnecter : « Mes questions » quitte le téléphone.

## 4. Parcours (récit 3)

1. Sans compte : cocher trois étapes ; progression, compte par groupe, libellé gris non barré ; « Lire : Guide, chapitre 3 » ouvre le guide au chapitre.
2. Se connecter : les trois coches rejoignent le compte.
3. Hors connexion, cocher deux étapes ; bandeau « Vos coches restent sur le téléphone » ; retour du réseau.
4. Second navigateur, même compte : les cinq coches. Décocher une étape sur chacun ; au retour du premier, la ligne « mises à jour depuis un autre appareil » paraît et l'état retenu est le dernier geste.

## 5. Recherche globale, feuille du lecteur, termes proposés (récits 4, 7, 8)

1. La loupe de « Ma journée » ouvre « Rechercher » ; `contact` : Lexique, FAQ, Documents, avec comptes et mots marqués ; la ligne « Pas encore ici : … ». Hors connexion : la mention du texte des documents non téléchargés.
2. Guide gardé, « Texte agrandi », hors connexion : toucher *global goal on adaptation* → l'entrée, « Dans ce guide : p. … », « Ouvrir dans le lexique ». Toucher un terme absent → « pas encore dans le lexique », « Proposer ».
3. Proposer `placeholder text` hors connexion depuis « aucun résultat », et le même terme d'un second compte : **une** proposition, deux contextes, dans la file. L'expert l'accepte, la publie : deux courriels.

## 6. Back-office et droits (récit 9)

1. Admin : rédiger une entrée, tenter de la publier sans vérification → refus en français ; l'expert date, l'admin publie. Admin : pas d'accès à la file.
2. Supprimer une entrée publiée → refus ; la dépublier → elle quitte le téléphone à la relecture.
3. Parcours : ajouter, ordonner, dépublier une étape ; le téléphone suit.
4. URL forgée : un administrateur d'événement et un compte sans rôle sur chaque page et chaque route → refus.

## 7. Contrôles

`npm run check:guide-nego`, `npm run test:guide-nego`, `npm run typecheck`, `cargo test -p negotiation`, `make openapi` sans écart, puis `make check-safe` sous verrou (protocole).

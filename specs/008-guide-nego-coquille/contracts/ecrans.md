# Contrat des écrans — Guide Négo 0a

Adresses relatives à `<baseURL>` (`/` en local, `/v2/` en ligne). Aucune n'a de variante `/en/`, et l'interface s'affiche en français quel que soit le téléphone (R3). Toutes sont rendues côté navigateur et fermées par `guide_nego.enabled`, sauf `fermee`.

| Adresse | Écran | Maquette | Barre d'onglets |
|---|---|---|---|
| `guide-nego/` | Accueil — état vide | — | oui |
| `guide-nego/negociations` | Négociations — état vide ; titre « Sessions de négociation » | — | oui |
| `guide-nego/francophonie` | Francophonie — état vide ; nomme « Réunions de la Francophonie » et « Pavillon de la Francophonie » | — | oui |
| `guide-nego/echanges` | Échanges — état vide ; renvoie à l'accueil si `negotiation.channels` est éteint | — | oui |
| `guide-nego/ressources` | Ressources — état vide + ligne « Profil et réglages » | 02 · 14c | oui |
| `guide-nego/ressources/reglages` | Profil et réglages — « Affichage → Thème » seulement | 02 · 11 | non, retour |
| `guide-nego/lexique` | Lexique — état vide, ouvert par « Aa » | — | non, retour |
| `guide-nego/ouverture` | Ouverture sans compte | 02 · 02 | non |
| `guide-nego/installer` | Installation | 02 · 01 | non |
| `guide-nego/fermee` | Application fermée | — | non |
| `guide-nego/composants` | Page interne ; aucun lien n'y mène ; `noindex` | 01-systeme | non |

## Règles de parcours

- Première venue (`gn.ouverture-vue` absente) sur `guide-nego/` → `ouverture`. « Continuer en visiteur » pose la clé et mène à l'accueil.
- Dans un navigateur, non installée : `installer` est proposé par un lien de l'écran d'ouverture, jamais imposé. En mode installé, `installer` renvoie à l'accueil ; dans un navigateur, elle s'affiche toujours. Elle dit d'ouvrir l'application une fois avec réseau.
- Fermée → toute adresse mène à `fermee`. Rouverte → `fermee` mène à l'accueil.
- Le retour du téléphone suit l'historique ; chaque onglet se recharge sur lui-même.

## En-tête

Titre, sous-titre facultatif, filet, « Aa » à droite, ligne de connexion au-dessus du titre — heure du téléphone, sans fuseau : « à 14:05 » le jour même, « hier à 23:10 », puis « le 11 nov. à 23:10 ». **Ni notifications, ni avatar** à cette étape. Sur un écran secondaire : retour à gauche.

## Les quatre états par écran

Ces écrans ne lisent aucune donnée : « vide » est leur état nominal. Chargement = l'arc, pendant la résolution du drapeau à la toute première ouverture. Erreur = sans objet tant qu'aucun contenu n'est lu ; le composant existe et figure à la page des composants. Accès refusé = `fermee`.

## Textes

Tous par i18n ; `fr` affiché, `en` fourni (règle du dépôt) et non servi à cette étape. Un fichier par écran : `pages/guide-nego.accueil.json`, `.negociations`, `.francophonie`, `.echanges`, `.ressources`, `.reglages`, `.lexique`, `.ouverture`, `.installer`, `.fermee`, `.composants` ; et `components/gn-barre-onglets.json`, `gn-entete.json`, `gn-connexion.json`. Libellés imposés par `design/lexique.md` : « Négociations », « Francophonie », « Hors connexion », « Synchronisé à ». Le mot « Programme » seul n'apparaît nulle part.

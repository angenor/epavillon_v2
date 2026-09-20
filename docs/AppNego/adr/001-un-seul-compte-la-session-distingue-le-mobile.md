# ADR-001 — Un seul compte ; la session distingue le mobile

**Statut** : accepté — 18/09/2026

## Contexte

Il fallait pouvoir distinguer, à l'authentification, une personne venue du site d'une personne venue de l'application, tout en gardant ses informations et son historique rattachés.

## Décision

Un seul compte pour l'ePavillon et Guide Négo. Deux choses distinguent l'usage, et aucune n'est le compte :

- le **droit** : la permission de négociateur, portée par espace ;
- la **session** : elle note le type de client (site ou application) et l'appareil.

## Conséquences

- Une personne inscrite sur le site entre dans l'application sans se réinscrire, et l'inverse.
- `identity.sessions` reçoit le type de client et l'appareil — à ajouter au SQL.
- On sait qui utilise l'application, sans jamais dédoubler une personne.
- Dans la coquille Capacitor, la session passe par jeton et non par cookie — [ADR-002](002-web-installable-d-abord-capacitor-avant-les-echanges.md).

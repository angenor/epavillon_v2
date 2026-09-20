# ADR-002 — Web installable d'abord, Capacitor avant les Échanges

**Statut** : accepté — 18/09/2026

## Contexte

L'application de 2018 est morte dans les magasins, faute d'entretien. Mais remplacer WhatsApp exige des notifications poussées fiables, que seule une application de magasin garantit sur tous les téléphones. Les comptes Apple et Google de l'OIF sont ouverts.

## Décision

Le client est un sous-arbre de pages du Nuxt existant, avec sa propre mise en page, rendu côté navigateur, installable depuis le navigateur (PWA). Capacitor l'emballe pour les magasins **avant** le module Échanges.

## Conséquences

- Aucune validation de magasin ne retarde le MVP ; une correction se déploie dans l'heure.
- Le code s'écrit dès le départ pour être emballé : aucun rendu serveur dans ce sous-arbre, aucun chemin absolu.
- Dans la coquille Capacitor, l'origine change : session par jeton, origine à autoriser côté API.
- Piège : le préfixe `/v2` s'applique à la portée du service worker et au manifeste.

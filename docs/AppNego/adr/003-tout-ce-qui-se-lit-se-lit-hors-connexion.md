# ADR-003 — Tout ce qui se lit se lit hors connexion

**Statut** : accepté — 18/09/2026

## Contexte

Sur le site d'une COP, le réseau est saturé ; l'itinérance coûte cher aux déléguées africaines. Une application qui attend le réseau ne sert pas en salle.

## Décision

Le hors-connexion est la règle, pas une option. Lexique et FAQ sont gardés en entier ; les trois agendas, pour les jours de l'événement ; les documents, à la demande. Seuls l'assistant et les échanges exigent le réseau.

## Conséquences

- Toute donnée affichée hors connexion dit **quand** elle a été lue.
- Un signalement, un favori ou une inscription faits sans réseau partent à son retour.
- La place occupée est visible et se libère en un geste.
- Les documents réservés s'effacent du téléphone à la déconnexion.
- L'API sert des listes « modifiées depuis », avec empreinte — [03-api](../03-api.md).

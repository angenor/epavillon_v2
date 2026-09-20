# ADR-008 — Trois agendas, jamais confondus

**Statut** : accepté — 18/09/2026

## Contexte

Trois choses portent des horaires et se confondent vite sous le mot « programme » : les réunions officielles de négociation, les rendez-vous de la Francophonie, et les activités du stand de l'OIF et de l'IFDD. Elles n'ont ni la même origine, ni le même public, ni la même fiabilité.

## Décision

Trois agendas, trois noms complets, trois modules : **Sessions de négociation**, **Réunions de la Francophonie**, **Pavillon de la Francophonie**. Le mot « Programme » seul est banni de l'interface, de l'API et des documents.

## Conséquences

- Dans le modèle : `negotiation.meetings` pour les deux premiers, distingués par leur nature ; `programme.sessions` pour le Pavillon.
- Dans l'API : une famille `/negotiation`, distincte de `/sessions` et `/schedule` qui servent le Pavillon.
- Dans l'interface : jamais une liste mêlée, sauf « Ma journée », où chaque ligne porte son origine.
- Une réunion de la Francophonie tenue au Pavillon est un **lien** entre deux objets, pas une fusion.

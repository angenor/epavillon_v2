# ADR-014 — WhatsApp est remplacé à terme

**Statut** : accepté — 18/09/2026

## Contexte

Les négociatrices échangent dans un groupe WhatsApp. Il sert bien la conversation, mais ne se façonne pas : les documents partagés s'y perdent, rien ne se classe, rien ne se retrouve. Les coalitions elles-mêmes vivent sur WhatsApp et Signal : l'habitude est forte.

## Décision

Le module Échanges vise à **remplacer** le groupe, pas à vivre à côté : canaux par thématique et par promotion, annonces, conversations privées, questions aux experts.

## Conséquences

- C'est le plus gros chantier, et le plus risqué : une conversation coupée en deux est pire qu'un seul groupe imparfait.
- Notifications poussées obligatoires, donc Capacitor et les magasins d'abord — [ADR-002](002-web-installable-d-abord-capacitor-avant-les-echanges.md).
- On commence par ce que WhatsApp ne fait pas ; le groupe ne ferme que sur critère — 80 % de ses membres actifs dans l'application pendant une session.
- On dit sans détour que l'IFDD héberge et modère : ce n'est pas le chiffrement de WhatsApp.
- Le modèle existe : canaux, messages partitionnés par mois, modération, conversations privées.

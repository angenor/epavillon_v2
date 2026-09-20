# ADR-006 — Admission par code d'invitation, approbation en réglage

**Statut** : accepté — 18/09/2026

## Contexte

L'application est ouverte à tous, mais certains modules sont réservés aux négociatrices et négociateurs. Il faut les admettre sans charger l'équipe, et pouvoir durcir si des usurpateurs apparaissent.

## Décision

On entre avec un **code d'invitation** diffusé dans le groupe WhatsApp. Le mode d'admission est un réglage du back-office : code seul, approbation par un administrateur, ou les deux. Les deux chemins existent dès le départ.

## Conséquences

- En temps normal, aucune charge pour l'équipe.
- Un code se révoque ; on sait qui est entré avec lequel, et l'on peut retirer ces accès.
- En cas d'abus, la bascule vers l'approbation se fait sans redéployer.
- Les essais de code sont limités ; codes et demandes d'accès s'ajoutent au SQL.
- L'accès lui-même reste une permission avec sa portée, jamais un nom de rôle.

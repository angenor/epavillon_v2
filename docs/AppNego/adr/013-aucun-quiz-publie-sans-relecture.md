# ADR-013 — Aucun quiz publié sans relecture

**Statut** : accepté — 18/09/2026

## Contexte

Les quiz — choix multiple, vrai ou faux — sont générés par l'IA depuis un document ou un enregistrement. Ils valent résumé : ils disent ce qu'il faut retenir. Mais l'IA peut halluciner une question comme sa réponse.

## Décision

Un administrateur demande la génération ; **un expert ou un administrateur relit et valide avant toute publication**. Chaque question garde le passage source qui la justifie. Une négociatrice peut régénérer un quiz pour elle — questions reformulées, ou portant sur d'autres passages : il reste **privé**, marqué « Non relu », et peut être proposé à la relecture.

## Conséquences

- La correction d'une question montre sa source : on vérifie soi-même.
- Le modèle des quiz existe (`training`) ; s'y ajoutent le rattachement à un document, la provenance IA, la relecture et le quiz personnel.
- La génération personnelle est bornée par un quota.
- Seuls des documents de la référence donnent des quiz publiés — [ADR-011](011-un-corpus-a-deux-etages.md).

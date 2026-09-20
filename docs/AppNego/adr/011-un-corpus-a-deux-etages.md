# ADR-011 — Un corpus à deux étages

**Statut** : accepté — 18/09/2026

## Contexte

Beaucoup de documents passent pendant une session : ceux du groupe WhatsApp, ceux de la CCNUCC, des courriels. Tout capter nourrirait l'assistant de fausses informations ; tout choisir à la main ne tiendrait pas pendant une COP.

## Décision

Deux étages. **L'arrivée** reçoit tout ce qui est proposé ou capté : cherchable, mais marqué « non vérifié ». **La référence** ne contient que ce qu'un expert a validé, et c'est **le seul étage que lit l'assistant**. L'IA propose le classement — type, thématiques, doublon probable ; un humain promeut, en un geste.

## Conséquences

- Capter largement ne coûte rien à la fiabilité des réponses.
- Le marqueur « utilisable par l'assistant » existe déjà sur `negotiation.documents`.
- Les natures de sources restent ouvertes : documents, transcriptions, FAQ, et d'autres demain.
- Une réponse d'expert promue en FAQ entre dans la référence : la boucle se ferme.

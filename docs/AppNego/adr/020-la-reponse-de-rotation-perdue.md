# ADR-020 — La réponse de rotation perdue n'est pas un vol

**Statut** : accepté — 22/09/2026, tranché par le commanditaire. **Nuance** R3 de [specs/001-socle-identite/research.md](../../../specs/001-socle-identite/research.md#r3--rotation-du-jeton-de-rafraîchissement-et-détection-de-rejeu).

## Contexte

Chaque renouvellement révoque la session et en ouvre une neuve ; un jeton déjà tourné qui revient est tenu pour un vol, et **toutes** les sessions de la personne tombent (R3). R3 a écarté toute tolérance en ne pensant qu'aux requêtes concurrentes, que le client ne produit pas.

Il existe pourtant une explication innocente, et c'est le cas courant d'un téléphone en salle : **la réponse de `/auth/refresh` se perd au retour** — réseau saturé, passage du Wi-Fi à la 4G, téléphone verrouillé au mauvais moment. Le serveur a déjà tourné le jeton ; le navigateur n'a jamais reçu le nouveau cookie, et représente l'ancien. Sous R3 strict, la négociatrice perd toutes ses sessions, sur tous ses appareils, pour une coupure d'une seconde.

## Décision

- **L'ancien jeton, présenté dans les 60 s qui suivent sa rotation, et tant que la session qui l'a remplacé n'a jamais été renouvelée**, révoque cette remplaçante (motif `response_lost`) et ouvre une session neuve : **une seule session vivante**.
- **Hors de ces bornes, rien ne change** : tout est coupé (`reuse_detected`).
- **Retrouver la remplaçante sans deviner** : la ligne tournée la nomme, `identity.sessions.replaced_by`, tenue à jour — après une reprise, elle désigne la nouvelle. Deux réponses perdues de suite se reprennent donc chacune.
- **« Jamais renouvelée »** se lit dans l'état de la remplaçante : vivante. Si elle a servi, le navigateur l'avait reçue, et l'ancien jeton n'a plus d'explication innocente. `last_seen_at` ne suit pas l'usage (R3 : aucune écriture par requête) ; seul le renouvellement compte comme usage.
- **Le jeton d'une remplaçante écartée pour réponse perdue** n'a jamais été remis au navigateur : s'il se présente, c'est un vol, et tout est coupé.
- **Deux requêtes concurrentes** portant le même jeton restent un rejeu quand la révocation conditionnelle les départage ; l'invariant tenu est qu'il n'y a jamais deux sessions vivantes.
- **La durée est un réglage** : `AUTH_REFRESH_GRACE`, 60 s par défaut. **Zéro rend la règle stricte** de R3.

## Conséquences

- Un jeton volé et présenté dans la minute qui suit une rotation légitime, avant que la remplaçante ne serve, ouvre une session au voleur et éteint celle de la personne — qui, à son prochain renouvellement, présentera un jeton écarté : tout tombe alors. La fenêtre est d'une minute, et suppose un vol au moment exact d'une rotation.
- Une colonne ajoutée au modèle (`030_identity.sql`, migration rejouable, `docs/progression/modele.md`).
- Tests : `identity/tests/rejeu_reponse_perdue.rs` (reprise, deux reprises, rejeu après la minute, rejeu après usage, jeton écarté) et `api/tests/routes_auth.rs` (la reprise en HTTP pose de nouveaux cookies).

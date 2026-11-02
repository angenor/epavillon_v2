# ADR-024 : Plusieurs onglets ne déconnectent jamais

**Statut** : accepté, 02/11/2026, exigence du commanditaire après un incident en production. **Remplace** la reprise d'[ADR-020](020-la-reponse-de-rotation-perdue.md) (révocation de la remplaçante, « une seule session vivante ») ; garde sa fenêtre et son réglage.

## Contexte

Le 02/11, le commanditaire, la régie du back-office et une page publique ouvertes, est déconnecté de partout : `jeton de rafraîchissement rejoué : toutes les sessions sont coupées`.

Les onglets partagent les cookies, donc le même jeton d'accès : toutes les lignes temps réel reçoivent `reauth` au même instant, et chaque onglet rafraîchit avec le même jeton. ADR-020 tenait ce cas pour impossible (« le client n'en envoie qu'un à la fois », vrai pour un onglet, faux pour plusieurs) :
- deux requêtes simultanées : la seconde perd la révocation conditionnelle et coupe tout ;
- deux requêtes à la suite : la seconde révoque la remplaçante de la première ; si la réponse de la première arrive en dernier, le navigateur garde un cookie révoqué, et le renouvellement suivant coupe tout.

## Décision

- **Navigateur** : une seule rotation à la fois entre onglets (`navigator.locks`, `utils/rotation-entre-onglets.ts`). L'onglet qui passe après une rotation réussie ailleurs depuis moins de 10 s ne rafraîchit pas : il rejoue sa requête avec les cookies neufs. Sans verrou, la marque seule (`localStorage`) évite la plupart des doublons.
- **API** : le jeton d'une session tournée depuis moins de `AUTH_REFRESH_GRACE` (60 s) ouvre une **session sœur** ; celle déjà émise **reste vivante**, car c'est peut-être son cookie que le navigateur garde. Le cas simultané y est ramené : la requête qui perd la révocation relit la ligne et passe par la même règle.
- **Une lignée close ne se rouvre pas** : on suit `replaced_by` (qui nomme la dernière sœur) ; si son bout a été fermé (déconnexion, mot de passe changé, suspension, vol), le jeton rend « session expirée », sans rien couper d'autre.
- **Hors de la minute, rien ne change** : tout est coupé (`reuse_detected`). Zéro garde la règle stricte de R3.
- `response_lost` n'est plus écrit ; une ligne qui le porte reste un vol si son jeton revient.

## Conséquences

- Un jeton volé et présenté dans la minute qui suit une rotation légitime ouvre une session au voleur **sans que rien ne le détecte** (ADR-020 éteignait la victime, ce qui finissait par tout couper). La fenêtre reste d'une minute et suppose un vol au moment exact d'une rotation ; ne jamais déconnecter en pleine COP l'emporte.
- Des sessions sœurs restent vivantes sans navigateur qui les porte, jusqu'à leur échéance (héritée sur le site, 90 jours glissants pour l'application). Aucune liste d'appareils ne les montre aujourd'hui.
- Plafond du temps réel (cinq lignes par session) inchangé : la sixième ligne ferme la plus ancienne, qui passe au pouls toutes les 15 s et rouvre sa ligne au retour sur l'onglet ; un onglet caché ne tient pas de ligne.
- Tests : `identity/tests/rejeu_reponse_perdue.rs`, `rejeu_du_jeton.rs` (deux renouvellements simultanés réussissent), tests unitaires de `service/session.rs`, `frontend/tests/site/rotation-entre-onglets.test.ts`.

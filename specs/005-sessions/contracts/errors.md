# Contrat — Erreurs

**Fonctionnalité** : Sessions (B5) · **Date** : 2026-08-21

> Le catalogue vit dans `backend/crates/kernel/src/error.rs` et l'OpenAPI est **engendrée** depuis lui. Ce fichier dit ce que B5 y ajoute, et comment les refus de PostgreSQL se traduisent.

---

## La règle, inchangée depuis B1

Un refus **exprimé par le contrat du front** sort en **200**, avec son discriminant. Un refus **non exprimé** sort en erreur HTTP, avec un code stable et un message français.

**Le partage passe ici entre le hasard et la faute.** Une personne peut arriver une minute après la clôture, ou trouver la salle pleine : ce sont des issues normales d'une tentative bien formée, et elles portent leur valeur.

| Refus | Où il vit dans le contrat | Statut |
|---|---|---|
| Inscriptions closes | `RegistrationResult` — `{ status: 'closed', closed_at }` | 200 |
| Pas encore ouvertes | `RegistrationResult` — `{ status: 'not_open_yet', opens_at }` | 200 |
| Jauge atteinte, sans liste d'attente | `RegistrationResult` — `{ status: 'full', capacity }` | 200 |
| Déjà inscrit | `RegistrationResult` — `{ status: 'already_registered', registration }` | 200 |
| Bascule en liste d'attente | `RegistrationResult` — `{ status: 'waitlisted', position }` | 200 — **ce n'est pas un refus**, c'est une place |

**Le planificateur, lui, ne refuse rien.** `PlannerMutationResult` ne porte aucun discriminant de refus, et c'est le contrat le plus important de ce module : aucun chevauchement ne produit d'erreur, à aucun statut.

---

## Les huit codes ajoutés au catalogue

| Code | Statut | Quand | Ce que l'écran en fait |
|---|---|---|---|
| `SESSION_DERIVED_FIELD` | 422 | Une écriture envoie `time_range`, `enforce_room_exclusivity`, ou un canal alors que la diffusion est **retirée** | `field` nomme le champ. Le message dit que la valeur est déduite, pas saisie |
| `SESSION_UNKNOWN_REFERENCE` | 422 | Salle, canal, journée ou fil **inexistant, désactivé, ou d'une autre édition** | `field` nomme le champ ; le contrôle recharge ses listes |
| `SESSION_TRACK_EVENT_MISMATCH` | 422 | Traduction du refus du déclencheur de rattachement | Bandeau : la journée spéciale n'appartient pas à cette édition |
| `REGISTRATION_NOT_ACCEPTED` | 422 | La séance **ne prend pas d'inscription**, ou elle est **annulée** | Le formulaire n'aurait pas dû être proposé : l'écran recharge la séance |
| `REGISTRATION_ANSWER_INVALID` | 422 | Réponse manquante, de mauvais type, hors options, hors bornes, ou clé inconnue | `field` porte le **code du champ** ; le message dit la règle enfreinte |
| `REGISTRATION_CONSENT_REQUIRED` | 422 | Une réponse est donnée à un champ **sensible** sans consentement | `field` nomme le champ ; l'écran affiche la case à cocher |
| `REGISTRATION_ACCOUNT_REQUIRED` | 401 | Le formulaire **n'admet pas** l'inscription sans compte | L'écran propose de se connecter |
| `REGISTRATION_LOCKED` | 422 | Le déclencheur refuse une **annulation** — séance annulée, ou question devenue obligatoire (écart n° 125) | Message expliquant que l'inscription ne peut plus être modifiée. Elle n'engage plus à rien |

**Huit, et pas plus.** Cinq refus métier sont déjà des membres d'union et sortent en 200 ; le reste relève des codes que le noyau porte depuis B1 — 401 sans session, 403 sans permission ou sans périmètre, 404 pour une ressource inexistante **ou hors périmètre**, 422 pour un corps mal formé.

---

## Ce que `REGISTRATION_ANSWER_INVALID` doit dire

Un seul code, parce qu'un formulaire branche sur le **champ** et non sur la nature de la faute. Le message, lui, distingue :

| Faute | Message |
|---|---|
| Réponse obligatoire absente ou vide | « Cette réponse est obligatoire. » |
| Type incompatible | « Cette valeur n'est pas un nombre / une date / une adresse électronique valide. » |
| Hors options | « Cette valeur ne fait pas partie des choix proposés. » |
| Hors bornes | « La valeur doit être comprise entre X et Y. » / « … ne doit pas dépasser N caractères. » |
| Clé inconnue | « La question « x » n'existe pas dans ce formulaire. » |
| Choix multiple obligatoire vide | « Choisissez au moins une réponse. » |

**La clé inconnue est un refus, pas un silence** (FR-075) : une réponse mal orthographiée qui disparaît sans un mot est une réponse perdue, et personne ne s'en apercevra avant l'export.

---

## La traduction des refus de PostgreSQL

| SQLSTATE | Contrainte / déclencheur | Traduction |
|---|---|---|
| `23514` | `ck_sessions_period` | 422 sur le champ de **fin** |
| `23505` | `ux_sessions_slug` | Jamais rendu : le service dérive et suffixe avant d'écrire (R7) |
| `23505` | `ux_sessions_proposal_sequence` | Jamais rendu : l'insertion est `ON CONFLICT DO NOTHING` — **c'est l'idempotence de la naissance** (R6) |
| `23505` | `ux_registrations_person_session` | 200, `{ status: 'already_registered' }` — la ligne vivante est relue et rendue |
| `23503` | clés étrangères de salle, canal, journée, fil | `SESSION_UNKNOWN_REFERENCE`, champ nommé |
| `23000` | `tg_check_session_track_event()` | `SESSION_TRACK_EVENT_MISMATCH` |
| `23001` | `tg_validate_registration()` — session annulée | `REGISTRATION_NOT_ACCEPTED` à l'inscription · `REGISTRATION_LOCKED` à l'annulation |
| `23001` | `tg_validate_registration()` — clôture | 200, `{ status: 'closed' }` — l'échéance est relue sur la séance |
| `23001` | `tg_validate_registration()` — jauge atteinte | 200, `{ status: 'full', capacity }` — la jauge est relue sur la séance |
| `23502` | `tg_validate_registration()` — réponses obligatoires | `REGISTRATION_ANSWER_INVALID` à l'inscription (filet, le service a déjà refusé) · `REGISTRATION_LOCKED` à l'annulation |

**Le même code, deux traductions, distinguées par le geste et jamais par le texte.** C'est la règle de B4 : `23001` et `23502` servent à l'inscription comme à l'annulation, et le service sait lequel des deux il est en train de faire. Brancher sur le message français du déclencheur produirait un second libellé qui se périmerait à la première évolution du SQL.

**La valeur qui accompagne un refus est relue, jamais extraite du message.** Le déclencheur écrit « Capacité atteinte (30 places). » ; le service rend `capacity` en le relisant sur la séance. Extraire un nombre d'une phrase française est un piège que B3 a déjà nommé.

---

## Ce qui **ne doit jamais** produire d'erreur

| Situation | Réponse attendue |
|---|---|
| Deux séances qui se chevauchent, quelle que soit la gravité | **Succès**, et le conflit dans la réponse |
| Deux directs simultanés | **Succès**, conflit bloquant |
| Une séance placée sur une salle déjà occupée | **Succès**, conflit bloquant |
| Une édition dont le programme n'est pas publié | Programmation publique **vide** |
| Une édition sans aucune séance | Écran du planificateur avec ses listes **vides** |
| Un dossier non retenu | Liste de séances **vide**, jamais absente |
| Une annonce de publication rejouée | Traitement **sans effet**, sans erreur |

**Aucun type de ce module ne permet d'exprimer un refus de placement.** C'est le contrat, et c'est ce qui empêche qu'un jour quelqu'un ajoute la variante.

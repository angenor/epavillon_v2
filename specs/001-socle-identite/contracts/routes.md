# Contrat — Routes

**Fonctionnalité** : Socle technique et Identité (B1) · **Date** : 2026-08-20

> **Ce document ne définit aucune forme de réponse.** Les formes sont dans `frontend/app/types/`
> et n'y ont qu'une seule source. On indique ici, pour chaque route : le verbe, le chemin,
> l'autorisation exigée, le type de requête et de réponse **par son nom TypeScript**, et la
> politique de statut HTTP.
>
> La documentation OpenAPI est **générée depuis le code** (FR-063). Ce fichier est la carte, pas
> la documentation.

---

## Préfixe et transport

Toutes les routes vivent sous **`/api`** — le préfixe que `.env.example` prescrit pour le raccordement du site. **La clé `NUXT_PUBLIC_API_BASE` y reste VIDE jusqu'à B7** : la renseigner aujourd'hui ferait basculer tous les écrans sur une API sans routes métier, et afficherait l'état d'erreur partout.

**Les en-têtes CORS sont posés** (20/08, avancés depuis B7). L'origine autorisée est celle d'`APP_PUBLIC_URL`, la même liste que l'intergiciel d'origine et normalisée par la même fonction : l'une décide ce qui a le droit d'écrire, l'autre ce que le navigateur a le droit de lire, et les deux doivent dire la même chose. **Jamais `*`** — le navigateur le refuse dès que les cookies sont autorisés. Le préalable `OPTIONS` est répondu par l'intergiciel **sans atteindre la route**, qui n'accepte souvent que `POST`. `X-Request-Id` est **exposé**, sans quoi le site ne pourrait pas le lire pour un signalement d'incident. Et **les réponses d'erreur portent les en-têtes elles aussi** : sans eux, le navigateur masque un 401 ou un 403 et l'écran affiche une panne réseau à la place du message français. Les chemins ci-dessous sont donnés **tels que le front les écrit**, sans ce préfixe.

- Toute requête porte `Accept-Language`, que le front pose déjà. La langue négociée résout les textes du modèle.
- Toute réponse porte `X-Request-Id` — repris de la requête s'il y était, engendré sinon.
- La session voyage par cookies (`epavillon_at`, `epavillon_rt`) ; le front les envoie seul (`credentials: 'include'`).
- Toute écriture vérifie l'en-tête d'origine.

---

## La politique de statut HTTP, et pourquoi elle n'est pas uniforme

**Un refus prévu par le contrat du front n'est pas une erreur HTTP.**

Le front consomme des unions discriminées : `LoginResult` porte `invalid_credentials`, `locked`, `suspended` ; `RoleWriteResult` porte `duplicate`, `scope_not_allowed`, `forbidden_scope`. Son client HTTP **lève une exception sur tout statut d'erreur** — rendre 401 sur un mot de passe faux ferait donc échouer l'écran au lieu de lui faire afficher son message.

D'où la règle, qui vaut pour tout le projet :

| Le refus est… | Réponse |
|---|---|
| **exprimé par le contrat** comme membre d'union | **200**, avec le discriminant `status` |
| **non exprimé** par le contrat | statut d'erreur HTTP + corps d'erreur (voir [`errors.md`](errors.md)) |

Ce n'est pas un adoucissement de la sécurité : un refus rendu en 200 avec `invalid_credentials` ne divulgue rien de plus qu'un 401, et il divulgue **moins** qu'un 401 qui se distinguerait d'un 404. C'est précisément ce que la règle de discrétion demande.

---

## Authentification — `/auth/*`

Aucune de ces routes n'exige de session, sauf mention contraire.

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `POST` | `/auth/login` | `LoginPayload` | `LoginResult` | **200** pour les six issues | Ordre imposé : mot de passe d'abord (FR-019). Empreinte factice si l'adresse est inconnue (FR-020) |
| `POST` | `/auth/logout` | `{}` | `{ status: 'signed_out' }` | 200 | Révoque la session portée par le cookie. **Réussit même sans session** — se déconnecter deux fois n'est pas une erreur |
| `GET` | `/auth/me` | — | `Person \| null` | **200, corps `null` si pas de session** | **Pas de 401.** Le store du front appelle cette route à chaque navigation, y compris déconnecté ; un 401 y ferait afficher un état d'erreur au lieu d'un état déconnecté. Le paramètre `personId` que le front passe encore disparaît (mock) |
| `POST` | `/auth/refresh` | `{}` | `{ status: 'renewed' \| 'expired' }` | 200 | **Route nouvelle**, absente du front : il ne la connaît pas encore et n'en a pas besoin tant qu'il tourne sur les mocks. Rotation du jeton (R3) |
| `POST` | `/auth/register` | `RegisterPayload` | `RegisterResult` | 200 | **Réponse invariable**, adresse libre ou prise (FR-035). Aucune session ouverte |
| `POST` | `/auth/verify-email` | `{ token }` | `VerifyEmailResult` | 200 | Trois refus distincts, « déjà utilisé » avant « périmé » |
| `POST` | `/auth/verify-email/resend` | `{ email }` | `ResendVerificationResult` | 200 | **Réponse invariable** (FR-036) |
| `POST` | `/auth/password-reset` | `{ email }` | `PasswordResetRequestResult` | 200 | **Réponse invariable** (FR-036) |
| `GET` | `/auth/password-reset/check?token=…` | — | `TokenCheckResult` | 200 | Contrôle **avant** d'afficher le formulaire |
| `POST` | `/auth/password-reset/confirm` | `{ token, password }` | `PasswordResetResult` | 200 · **422** si le mot de passe est refusé | Le jeton est **revérifié** ici, pas seulement au contrôle (FR-042). Révoque toutes les sessions |

**Le second facteur** : `/auth/login` peut rendre `mfa_required`. **Aucune route ne complète le défi dans ce jalon** — arbitrage du 20/08. Le champ `challenge_id` est renseigné pour que le contrat reste honorable, et l'écran affiche ce qu'il affiche déjà.

---

## Identité — `/people/*`

Toutes exigent une session.

| Verbe | Chemin | Autorisation | Réponse |
|---|---|---|---|
| `GET` | `/people` | `identity.person.read`, quelle que soit la portée | `Person[]` |
| `GET` | `/people/{id}` | soi-même, ou `identity.person.read` | `Person \| null` |
| `GET` | `/people/{id}/roles` | soi-même, ou `identity.person.read` | `RoleAssignment[]` — attributions en cours |
| `GET` | `/people/{id}/permissions` | soi-même, ou `identity.person.read` | `EffectivePermission[]` |
| `GET` | `/people/{id}/administered-events` | soi-même, ou `identity.person.read` | `AdministeredEvents` — **jamais nul, toujours une valeur pleine** |

« Soi-même » est décidé par la session, jamais par un paramètre : c'est ce qui empêche de lire le périmètre d'administration d'un autre en forgeant l'identifiant.

---

## Back-office de l'identité — `/admin/*`

Toutes exigent une session **et** une permission. Le périmètre borne les listes.

| Verbe | Chemin | Autorisation | Requête | Réponse | Statut |
|---|---|---|---|---|---|
| `GET` | `/admin/users` | `identity.person.read` | — | `UserListScreen` | 200 · **403** sans la permission ou sur périmètre vide |
| `GET` | `/admin/users/{id}` | `identity.person.read` | — | `UserDetail \| null` | 200 · `null` si inexistante. **Hors périmètre → 200 avec `in_scope: false`**, lecture seule |
| `GET` | `/admin/users/role-options` | `identity.role.assign`, sur au moins une portée | — | `RoleAssignmentOptions` | 200 · 403 |
| `GET` | `/admin/users/{id}/effective-permissions` | `identity.person.read` | — | `EffectivePermissionsView` | 200 · 403 |
| `POST` | `/admin/users/{id}/roles` | `identity.role.assign` **sur la portée visée** | `GrantRolePayload` | `RoleWriteResult` | **200** pour les six issues |
| `DELETE` | `/admin/users/roles/{assignmentId}` | `identity.role.assign` **sur la portée de l'attribution** | `RevokeRolePayload` | `RoleWriteResult` | **200** |
| `PUT` | `/admin/users/{id}/status` | `identity.person.manage` | `SetPersonStatusPayload` | `PersonWriteResult` | **200** |
| `GET` | `/admin/privacy-requests` | `identity.person.manage` **en portée globale** | — | `PrivacyQueueScreen` | 200 · **403** |
| `PUT` | `/admin/privacy-requests/{id}` | `identity.person.manage` **en portée globale** | `HandlePrivacyRequestPayload` | `PrivacyWriteResult` | **200** |

### Trois points qui se perdraient sans être écrits

**1. Les paramètres `granted` et `actorId` disparaissent.** Le front les passe encore à `grantRole`, `revokeRole`, `setStatus` et `handlePrivacyRequest` : ce sont les permissions et l'identité de l'acteur, envoyées par le client. C'est la seule façon de rejouer l'autorisation sur des données simulées, et son propre commentaire le dit — « le paramètre disparaîtra au prompt B1 : l'API lit sa propre session, et un client qui déclare ses propres droits n'est pas un contrôle d'accès ». **L'API les ignore, même s'ils sont envoyés** (FR-055).

**2. Retirer exige le même droit qu'attribuer, sur la même portée.** La permission se vérifie sur la portée de **l'attribution visée**, pas sur celle de l'acteur. Sans cela, un administrateur détaché sur une édition pourrait retirer un rôle global qu'il n'aurait jamais pu accorder.

**3. La file RGPD ne se borne pas par édition.** Une demande d'effacement porte sur la plateforme entière : elle exige la portée **globale**, ou rien. Un administrateur d'édition reçoit 403, jamais une file filtrée — filtrer donnerait l'illusion d'un traitement complet.

---

## Exploitation — `/health`, `/ready`, `/docs`

| Verbe | Chemin | Autorisation | Réponse |
|---|---|---|---|
| `GET` | `/ready` | aucune | vivacité : le processus répond et le pool de connexions est ouvert |
| `GET` | `/health` | `analytics.dashboard.read` | l'état d'exploitation, depuis `analytics.v_operational_health` — outbox en retard, travaux en échec, courriels en rebond, partitions manquantes |
| `GET` | `/docs` | aucune hors production | documentation OpenAPI générée |

`/ready` ne demande rien et ne divulgue rien : c'est l'orchestrateur qui la lit. `/health` porte des chiffres d'exploitation et se protège comme une donnée. **`/ready` existe dès les phases 1 et 2**, en talon : il rend `{"status":"ok"}` sans contrôler le pool — ce contrôle arrive avec sa tâche.

---

## Ce que le montage des routes lit au démarrage

`platform.modules` décide quels modules sont montés (principe II). Un module `disabled` n'est pas monté : ses chemins répondent **404**, pas 403 — une route absente n'est pas une route interdite, et dire « interdit » révélerait qu'elle existe.

Dans ce jalon, seul `identity` a un crate ; les autres lignes du registre — celles que sèment `010_platform.sql`, `115_content.sql` et `125_training.sql` — sont sans effet tant qu'aucun crate ne leur correspond.

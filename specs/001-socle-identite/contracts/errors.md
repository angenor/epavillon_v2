# Contrat — Erreurs

**Fonctionnalité** : Socle technique et Identité (B1) · **Date** : 2026-08-20

Principe IX : **toute erreur porte un code stable et un message français.** Le code est un identifiant machine, jamais traduit, jamais renommé sans amendement de version. Le front **branche sur le code** ; il ne compose jamais un message à partir du texte.

---

## Forme du corps d'erreur

```json
{
  "code": "IDENTITY_PASSWORD_TOO_WEAK",
  "message": "Le mot de passe doit compter au moins 8 caractères, dont une majuscule et une minuscule.",
  "field": "password",
  "request_id": "01J…"
}
```

`field` n'est renseigné que pour une erreur de validation — il désigne alors le champ fautif, **dans le nom que le front lui donne**. `request_id` est le même que l'en-tête `X-Request-Id` : c'est ce qui permet de retrouver la trace Jaeger et la ligne d'audit à partir d'une capture d'écran.

**Rappel de politique** : un refus prévu par le contrat du front n'est pas une erreur HTTP — il sort en 200 avec un discriminant. Voir [`routes.md`](routes.md). Ce catalogue ne couvre donc que ce que le contrat n'exprime pas.

---

## Catalogue des codes stables

### Transverses — portés par le noyau

| Code | Statut | Message | Quand |
|---|---|---|---|
| `VALIDATION_FAILED` | 422 | dépend du champ | Corps mal formé, champ manquant, valeur hors bornes |
| `UNAUTHENTICATED` | 401 | Votre session a expiré. Veuillez vous reconnecter. | Route protégée sans session valide |
| `FORBIDDEN` | 403 | Vous n'avez pas les droits nécessaires pour cette action. | Permission absente, ou portée hors périmètre |
| `NOT_FOUND` | 404 | La ressource demandée est introuvable. | Identifiant inconnu — **et identifiant inaccessible** (FR-037) |
| `CONFLICT` | 409 | Cette action entre en conflit avec l'état actuel de la donnée. | Écriture concurrente, contrainte d'unicité non couverte plus bas |
| `PAYLOAD_TOO_LARGE` | 413 | La requête dépasse la taille autorisée. | Garde-fou d'entrée |
| `INTERNAL` | 500 | Une erreur interne est survenue. L'incident a été enregistré. | Tout le reste. **Aucun détail technique ne sort** |
| `SERVICE_UNAVAILABLE` | 503 | Le service est momentanément indisponible. | Base injoignable, arrêt en cours |

`NOT_FOUND` couvre **deux cas volontairement indiscernables** : la ressource n'existe pas, et la ressource existe mais hors du périmètre de l'appelant. C'est le principe IX : « un identifiant inaccessible se refuse comme tel, il ne se distingue pas d'un identifiant inexistant par la forme de la réponse ».

### Session et authentification

| Code | Statut | Message | Quand |
|---|---|---|---|
| `IDENTITY_SESSION_EXPIRED` | 401 | Votre session a expiré. Veuillez vous reconnecter. | Jeton de rafraîchissement périmé. **Pas sur `/auth/refresh`** — voir ci-dessous |
| `IDENTITY_SESSION_REVOKED` | 401 | Cette session a été fermée. Veuillez vous reconnecter. | Session révoquée — déconnexion, changement de mot de passe, suspension. **Pas sur `/auth/refresh`** — voir ci-dessous |
| `IDENTITY_REFRESH_REUSED` | 401 | Par sécurité, toutes vos sessions ont été fermées. Veuillez vous reconnecter. | **Jeton rejoué** — toutes les sessions viennent d'être révoquées (R3) |
| `IDENTITY_ORIGIN_REJECTED` | 403 | Requête refusée : origine non autorisée. | Écriture dont l'en-tête d'origine n'est pas reconnu |
| `IDENTITY_PASSWORD_TOO_WEAK` | 422 · `password` | Le mot de passe doit compter au moins 8 caractères, dont une majuscule et une minuscule. | Mêmes règles que le front applique déjà |

**`IDENTITY_SESSION_EXPIRED` et `IDENTITY_SESSION_REVOKED` ne sont rendus par aucune route livrée, et c'est voulu.** `/auth/refresh` exprime les deux cas par son union — `{status:'expired'}` en 200 —, et la règle du contrat l'emporte : un refus prévu par le front n'est pas une erreur HTTP. Ils attendent une route **protégée** qui aurait besoin de distinguer « périmée » de « fermée » pour composer son message ; en attendant, une route protégée sans session valide rend `UNAUTHENTICATED`, qui porte exactement le même texte. Les garder ici coûte deux variantes inertes ; les retirer serait un changement **majeur** pour un front qui pourrait déjà brancher dessus.

### Écritures du back-office

| Code | Statut | Message | Quand |
|---|---|---|---|
| `IDENTITY_EMAIL_ALREADY_USED` | 409 · `primary_email` | Cette adresse est déjà utilisée par une autre personne. | Écriture d'administration. **Jamais rendu par l'inscription** — la discrétion l'interdit (FR-035) |
| `IDENTITY_ACCOUNT_ALREADY_EXISTS` | 409 | Cette personne a déjà un compte avec mot de passe. | Création d'un second compte mot de passe |
| `IDENTITY_ROLE_WINDOW_INVALID` | 422 · `valid_until` | La date de fin doit être postérieure à la prise d'effet. | |
| `IDENTITY_ROLE_SCOPE_MISMATCH` | 422 · `scope_id` | Une portée globale ne vise aucune cible ; une portée ciblée en exige une. | |
| `IDENTITY_ROLE_REVOCATION_INVALID` | 422 | Un motif de retrait ne peut pas être posé sur une attribution en cours. | |
| `IDENTITY_UNKNOWN_REFERENCE` | 422 · variable | La valeur choisie n'existe pas. | Pays, langue ou autre référence inconnue |
| `IDENTITY_PRIVACY_WRONG_ACTION` | 422 | L'anonymisation ne répond qu'à une demande d'effacement. | Aussi exprimé par le discriminant `wrong_type` ; le code sert aux appels hors écran |

### Exploitation

| Code | Statut | Quand |
|---|---|---|
| `MAIL_RELAY_UNREACHABLE` | — | **Jamais rendu à un client**, et délibérément **absent de l'énuméré des codes d'API** : il vit dans `kernel::mail`, d'où il part en tête de `platform.jobs.last_error` et déclenche la reprise d'essai. L'y mettre lui donnerait un statut HTTP, donc un chemin vers une réponse |

---

## Traduction des erreurs PostgreSQL

Principe VIII : **le code ne redouble pas une contrainte de la base — il traduit son refus.** La correspondance vit dans le noyau, en un seul endroit.

### Violations de contrainte

| SQLSTATE | Contrainte | Devient |
|---|---|---|
| `23505` | `ux_people_primary_email` | `IDENTITY_EMAIL_ALREADY_USED`, champ `primary_email` |
| `23505` | `ux_person_emails` | `IDENTITY_EMAIL_ALREADY_USED`, champ `email` |
| `23505` | `ux_accounts_password_per_person` | `IDENTITY_ACCOUNT_ALREADY_EXISTS` |
| `23505` | `ux_accounts_provider_subject` | `CONFLICT` |
| `23505` | `ux_role_assignments_active` | **discriminant `duplicate`** de `RoleWriteResult`, avec l'attribution en conflit |
| `23505` | unicité de `sessions.refresh_token_hash` ou de `one_time_tokens.token_hash` | **collision d'aléa** : on régénère et on rejoue une fois, puis `INTERNAL` |
| `23514` | `ck_people_suspension_window` | **discriminant `missing_deadline`** de `PersonWriteResult` |
| `23514` | `ck_accounts_password_shape` | `INTERNAL` — le service ne devrait jamais l'atteindre ; s'il l'atteint, c'est un défaut de code |
| `23514` | `ck_role_assignment_window` | `IDENTITY_ROLE_WINDOW_INVALID`, champ `valid_until` |
| `23514` | `ck_role_assignment_scope` | `IDENTITY_ROLE_SCOPE_MISMATCH`, champ `scope_id` |
| `23514` | `ck_role_assignment_revocation` | `IDENTITY_ROLE_REVOCATION_INVALID` |
| `23514` | `ck_outbox_event_type_format` | `INTERNAL` — un type d'événement mal formé est un défaut de code, pas une donnée de l'utilisateur |
| `23503` | clé étrangère vers `reference.countries` ou `reference.locales` | `IDENTITY_UNKNOWN_REFERENCE`, champ déduit du nom de la contrainte |
| `23514` | `email_check`, `timezone_name_check`, `url_check`, `slug_check` — les **domaines** de `000_bootstrap.sql` | `VALIDATION_FAILED`, **sans champ** : voir ci-dessous |
| `23514` | `i18n_text_check` | `INTERNAL` — un texte multilingue mal bâti vient du code, pas de l'utilisateur |
| `23514` | `people_first_name_check` · `people_last_name_check` | `VALIDATION_FAILED`, champ `first_name` / `last_name` |
| `23514` | `people_civility_check` · `person_emails_label_check` | `INTERNAL` — listes fermées que l'interface choisit elle-même ; une valeur hors liste est un défaut de code |
| `22P02` | conversion impossible : uuid, date ou valeur d'énumération mal formée | `VALIDATION_FAILED` |

**Un domaine à CHECK lève `23514`, jamais `22P02`** — mesuré sur la base : `SELECT 'pasunemail'::platform.email` rend `23514 / CONSTRAINT NAME: email_check`. `citext` accepte toute chaîne, donc rien ne se convertit mal ; c'est le CHECK qui refuse. Une version antérieure de ce tableau rangeait le cas sous `22P02`, et le code qui la suivait fidèlement rendait 500 sur une adresse mal écrite.

**Le refus d'un domaine ne porte ni table ni colonne** — seulement le schéma, le domaine et la contrainte. Le noyau ne peut donc pas nommer le champ fautif : `platform.email` sert `people.primary_email` comme `person_emails.email`. Il rend `VALIDATION_FAILED` sans `field`, et **le module ajoute `.field(...)`**, puisque lui seul sait d'où venait la valeur.

### Exceptions levées par les triggers du modèle

| SQLSTATE | Origine | Devient |
|---|---|---|
| `23001` (`restrict_violation`) | `identity.tg_check_role_scope()` | **discriminant `scope_not_allowed`**, portant **le message de la base tel quel** |

Le trigger écrit déjà, en français :

> « Le rôle « admin » ne peut pas être attribué sur la portée « organization » (portées autorisées : global, event). »

Ce message est **repris sans réécriture**. Le reformuler côté service produirait deux libellés pour un même refus, et le second se périmerait à la première évolution du modèle. C'est le principe VIII pris au mot.

---

## Ce qui n'est jamais rendu

Trois choses ne franchissent aucune réponse d'erreur, quelles que soient les circonstances :

1. **Le texte brut d'une erreur PostgreSQL non reconnue.** Il porte des noms de tables, de colonnes, parfois des valeurs. Une erreur non répertoriée sort en `INTERNAL` ; le détail va dans la trace, avec l'identifiant de requête.
2. **L'existence d'un compte**, sur les quatre points d'entrée publics. Aucun code de ce catalogue n'est atteignable depuis `/auth/login`, `/auth/register`, `/auth/password-reset` ou `/auth/verify-email/resend` d'une façon qui distinguerait une adresse connue d'une adresse inconnue.
3. **Un secret**, sous aucune forme : ni empreinte, ni jeton, ni fragment de l'un des deux.

---

## Ce que ce catalogue ne couvre pas encore

**Les violations d'exclusion (`23P01`)** n'y figurent pas : les trois contraintes `ex_*` du modèle vivent dans `publication` et `negotiation`, hors du périmètre de B1. Le module qui ouvrira l'une d'elles complète ce tableau — sans quoi son refus sortira en `INTERNAL`.

**Le corps mal formé et la requête trop grosse sont couverts depuis la phase 3**, en même temps que la première route qui accepte un corps. Un champ absent ou mal typé rend `VALIDATION_FAILED` (422) avec le **nom du champ**, seule chose empruntée au texte de serde — et filtrée avant de sortir ; un corps au-dessus d'un mégaoctet rend `PAYLOAD_TOO_LARGE` (413). Le texte anglais de serde ne franchit aucune réponse : il part dans la trace, avec l'identifiant de requête. Le chemin inconnu, lui, rend le corps du catalogue depuis les phases 1 et 2.

---

## Règle d'évolution

Ajouter un code est **mineur** : le front l'ignore et retombe sur son message générique. Renommer ou supprimer un code est **majeur** : le front branche dessus, et un code disparu produit un écran muet. Un code retiré se remplace donc par une période où les deux existent, jamais par une substitution sèche.

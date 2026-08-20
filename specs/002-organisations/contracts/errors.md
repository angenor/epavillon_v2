# Contrat — Erreurs

**Fonctionnalité** : Organisations (B2) · **Date** : 2026-08-20

Principe IX : **toute erreur porte un code stable et un message français.** La forme du corps, les codes transverses du noyau et la règle de statut sont ceux de B1 et ne sont pas rejoués ici — voir [`../../001-socle-identite/contracts/errors.md`](../../001-socle-identite/contracts/errors.md). Ce fichier ne porte que ce que B2 ajoute.

**Rappel de politique** : un refus prévu par le contrat du front n'est pas une erreur HTTP. Ce catalogue ne couvre donc que ce que le contrat n'exprime pas.

---

## Les six refus qui sortent en 200, et ne sont pas des erreurs

| Discriminant | Route | Ce qu'il porte |
|---|---|---|
| `already_member` | rattachement | L'organisation et l'état de l'adhésion existante |
| `name_taken` | création | La fiche en conflit, sous la forme d'un résultat de recherche — de quoi la rejoindre |
| `already_invited` | invitation | L'adhésion en vol, pour proposer de **relancer** plutôt que d'émettre une seconde invitation |
| `domain_taken` | vérification d'un domaine | **Le nom de la fiche** qui détient déjà le domaine vérifié. Sans ce nom, le refus est incompréhensible |
| `confirmation_mismatch` | fusion | Rien de plus : le nom saisi ne correspond pas à la fiche absorbée |
| `already_merged` | fusion | La fiche survivante à viser |

---

## Catalogue des codes ajoutés par ce module

| Code | Statut | Message | Quand |
|---|---|---|---|
| `ORG_NOT_MANAGER` | 403 | Seul un référent de cette organisation peut effectuer cette action. | Inviter, décider d'une demande, révoquer une adhésion sans être référent **actif de cette organisation** |
| `ORG_MEMBERSHIP_IS_INVITATION` | 422 | Cette adhésion est une invitation : elle attend la réponse de la personne, pas la vôtre. | Un référent tente d'approuver une invitation (écart n° 33). **C'est le refus qui empêche de faire entrer quelqu'un qui n'a rien accepté** |
| `ORG_MEMBERSHIP_NOT_PENDING` | 422 | Cette adhésion n'attend plus de décision. | Décision sur une adhésion active ou révoquée |
| `ORG_LAST_MANAGER` | 422 | Cette organisation n'aurait plus aucun référent. Désignez un remplaçant d'abord. | Retrait du dernier référent actif (FR-041). Contournable par la permission de gestion des organisations |
| `ORG_MERGE_FIELD_NOT_ARBITRABLE` | 422 | L'adresse de la fiche absorbée ne peut pas être reprise : elle reste la sienne, et c'est ce qui fait que ses anciens liens continuent de fonctionner. | Un arbitrage de fusion porte sur l'adresse d'URL. **Champ : `slug`** (R6) |
| `ORG_MERGE_GLOBAL_SCOPE_REQUIRED` | 403 | La fusion de deux organisations exige des droits sur l'ensemble de la plateforme. | Permission de fusion détenue sur une portée qui n'est pas globale. Distinct de `FORBIDDEN` parce que l'écran sait dire **pourquoi** : il n'existe pas de fusion limitée à une COP |
| `ORG_MERGE_SAME_ORGANIZATION` | 422 | Une organisation ne peut pas être fusionnée avec elle-même. | Source et cible identiques. La fonction de base le refuse aussi ; le service le dit avant, pour nommer le champ |
| `ORG_DOMAIN_VERIFICATION_REQUIRED` | 422 | Un rattachement automatique exige un domaine vérifié. | Activation du rattachement automatique sur un domaine non vérifié |
| `ORG_NAME_IS_DERIVED` | 422 | Le nom légal et le sigle suivent la fiche : ils ne se retirent pas à la main. | Tentative de retrait d'une dénomination posée par la base |
| `ORG_UNKNOWN_REFERENCE` | 422 | La valeur choisie n'existe pas. | Type d'organisation, pays ou langue inconnus. Le champ fautif est déduit du nom de la contrainte |
| `ORG_INVITATION_NOT_YOURS` | 403 | Cette invitation ne vous est pas adressée. | Une session existe et ne désigne pas la personne du jeton (R10) |

Onze codes. Ils sont **engendrés dans l'OpenAPI depuis le catalogue du noyau**, comme les vingt de B1 : un code ajouté apparaît au prochain démarrage, un code oublié n'existe pas.

---

## Traduction des erreurs PostgreSQL

Principe VIII : **le code ne redouble pas une contrainte de la base — il traduit son refus.** La correspondance vit dans le noyau ; ce tableau ajoute les entrées de ce module.

### Violations d'unicité (`23505`)

| Contrainte | Devient |
|---|---|
| `ux_organizations_name_country` | **discriminant `name_taken`** de `CreateOrganizationResult`, avec la fiche en conflit. En **fusion**, où le nom vient d'un arbitrage : `CONFLICT`, champ `legal_name` |
| `ux_organizations_slug` | Collision d'adresse à la création : on suffixe et on rejoue **une fois**, puis `INTERNAL`. En fusion, ce cas n'existe pas — l'arbitrage est refusé avant (R6) |
| `ux_organization_domains_verified` | **discriminant `domain_taken`** de `OrganizationWriteResult`, **portant la fiche qui détient le domaine** |
| `ux_organization_names` | Silencieux : la dénomination existe déjà pour cette fiche et ce genre, il n'y a rien à faire |
| `ux_memberships` | **Ne doit jamais remonter** : la demande de rattachement est un unique ordre avec reprise conditionnelle (R7). S'il remonte, c'est un défaut de code → `INTERNAL` |
| `ux_memberships_primary` | **Ne doit jamais remonter** : la primauté est attribuée par trigger, jamais par le service → `INTERNAL` |
| `ux_duplicate_candidates` | Absorbée par `ON CONFLICT` dans le balayage |

### Violations de vérification (`23514`)

| Contrainte | Devient |
|---|---|
| `organizations_legal_name_check` | `VALIDATION_FAILED`, champ `legal_name` |
| `organizations_acronym_check` | `VALIDATION_FAILED`, champ `acronym` — « Un sigle compte entre 2 et 32 caractères », le libellé que l'écran affiche déjà |
| `organization_names_name_check` | `VALIDATION_FAILED`, champ `name` |
| `organization_domains_domain_check` | `VALIDATION_FAILED`, champ `domain` |
| `ck_domain_autojoin_requires_verification` | `ORG_DOMAIN_VERIFICATION_REQUIRED`, champ `auto_join` |
| `ck_memberships_invitation` | `INTERNAL` — une date d'invitation sans auteur vient du code, jamais de l'utilisateur |
| `ck_organizations_merge_shape` · `ck_organizations_no_self_merge` | `INTERNAL` — seul le service pourrait les enfreindre, et il ne pose jamais ces colonnes |
| `ck_duplicate_candidates_ordered` | `INTERNAL` — l'ordre est posé par `LEAST`/`GREATEST` (R11) |
| `organizations_trust_score_check` | `INTERNAL` — la valeur vient de la fonction du modèle |
| `email_check`, `url_check`, `slug_check` — les **domaines** de `000_bootstrap.sql` | `VALIDATION_FAILED` **sans champ** ; le module ajoute le champ, lui seul sachant d'où venait la valeur. C'est la leçon de B1 : ces domaines lèvent `23514`, **jamais `22P02`** |

### Clés étrangères (`23503`)

| Contrainte | Devient |
|---|---|
| Vers `reference.countries`, `reference.locales` | `ORG_UNKNOWN_REFERENCE`, champ déduit du nom de la contrainte |
| `xmod_fk_memberships_person` | `INTERNAL` — la personne vient d'être lue ou créée dans la même transaction |

### Exceptions levées par les fonctions du modèle

Trois exceptions viennent de `040_organizations.sql`, avec les **noms de condition** que le SQL déclare. Leur SQLSTATE exact est à **relever sur la base** avant d'écrire la traduction : B1 a payé une fois d'avoir recopié un code depuis un document au lieu de le mesurer, et une adresse mal écrite sortait en 500.

| Origine | Condition déclarée | Devient |
|---|---|---|
| `org.tg_forbid_merge_chains()` | `integrity_constraint_violation` | **discriminant `already_merged`**, portant **le message de la base tel quel** : « Fusion impossible : la fiche cible … est elle-même fusionnée. Cibler la fiche finale. » |
| `org.merge_organizations()` — source et cible identiques | `invalid_parameter_value` | `ORG_MERGE_SAME_ORGANIZATION` |
| `org.merge_organizations()` — fiche introuvable ou déjà fusionnée | `no_data_found` | **discriminant `not_found`** ou `already_merged` selon la fiche visée |

**Le message du trigger est repris sans réécriture.** Le reformuler produirait un second libellé pour un même refus, et le second se périmerait à la première évolution du modèle. C'est ce qui a été fait en B1 pour le refus de portée d'un rôle, et il n'y a aucune raison de faire autrement.

---

## Ce qui n'est jamais rendu

- **Le nom d'une organisation hors du périmètre de l'appelant**, même dans un message d'erreur. Le refus est indiscernable de « inexistante ».
- **L'existence d'une personne** à qui interroge un domaine qui n'est pas le sien : le paramètre est ignoré, la question ne peut pas être posée.
- **Le détail technique d'une erreur interne** : ni nom de contrainte, ni requête, ni message PostgreSQL brut — sauf les trois messages français que le modèle écrit lui-même, qui sont faits pour être lus.

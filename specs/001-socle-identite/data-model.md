# Phase 1 — Modèle de données

**Fonctionnalité** : Socle technique et Identité (B1) · **Date** : 2026-08-20

> **Aucune table, aucune colonne, aucun type n'est créé ni modifié par ce module.**
> Ce document ne définit rien : il dit ce que le code fait de ce qui existe déjà dans
> [`docs/database/030_identity.sql`](../../docs/database/030_identity.sql) et
> [`docs/database/010_platform.sql`](../../docs/database/010_platform.sql), et où chaque règle
> de la spécification atterrit.

---

## 1. Ce que le code lit, écrit, et ne touche pas

### `identity.people` — la personne

| Colonne | Ce que le module en fait |
|---|---|
| `primary_email` | Clé de rapprochement. Domaine `citext` : **la comparaison ignore déjà la casse**, le service ne met rien en minuscules avant d'interroger |
| `email_verified_at` | **Porte à elle seule l'état « en attente de vérification »** (FR-024, écart n° 20). Nulle → connexion refusée. Aucun statut de personne n'est ajouté pour cela |
| `status`, `status_reason`, `status_changed_by`, `status_changed_at`, `suspended_until` | Écrits par le changement de statut du back-office (FR-056). `ck_people_suspension_window` exige la date de fin sur une suspension : le service **ne la revalide pas**, il traduit le refus |
| `preferred_locale`, `timezone` | Remplis à l'inscription depuis la langue de l'interface et le fuseau du navigateur — deux champs de formulaire en moins, deux colonnes `NOT NULL` remplies quand même |
| `display_name` | Colonne générée : **jamais écrite**, seulement lue |
| `search_vector` | Colonne générée : sert la recherche de la liste du back-office, n'a aucune représentation côté client |
| `primary_organization_id` | **Non écrit par ce module.** Maintenu par les triggers du module `org` (B2) |
| `civility`, `phone`, `job_title`, `biography`, `country_id`, `city`, `is_directory_visible` | Lus par la fiche du back-office. Leur écriture est le profil de la personne, hors périmètre de ce jalon |

`ux_people_primary_email` est **partiel** — il ne porte que sur les personnes non anonymisées. Une personne effacée libère donc son adresse, et une réinscription sur cette adresse est légitime.

### `identity.accounts` — le moyen de se connecter

| Colonne | Ce que le module en fait |
|---|---|
| `password_hash` | Empreinte Argon2id au format PHC. Calculée et comparée **entièrement côté service** — `COMMENT ON COLUMN` : « aucune fonction SQL ne doit vérifier de mot de passe » |
| `password_changed_at` | Posée à chaque changement, y compris par réinitialisation |
| `failed_attempts`, `locked_until` | **Écart n° 18.** Les colonnes existent, le seuil et la durée vivent dans la configuration du service. Remises à zéro par : connexion réussie, expiration du verrou, réinitialisation menée à terme (FR-015) |
| `last_login_at` | Posée à l'ouverture d'une session |
| `mfa_secret_encrypted`, `mfa_enabled_at`, `mfa_recovery_codes` | **Lues seulement.** `mfa_enabled_at` non nulle produit l'issue « second facteur requis ». Aucune route ne les écrit dans ce jalon (arbitrage du 20/08) |
| `provider`, `provider_subject` | Toujours `password` / `NULL` dans ce jalon. La fédération reste hors périmètre (décision du commanditaire, 17/08) |

`ux_accounts_password_per_person` : un seul compte mot de passe par personne. `ck_accounts_password_shape` : empreinte obligatoire et sujet nul pour `password`.

### `identity.sessions` — la session

`refresh_token_hash bytea NOT NULL UNIQUE` est **la seule empreinte de jeton que porte la table**. C'est ce fait, et pas un choix d'architecture, qui décide de la forme du jeton d'accès ([research.md § R1](research.md#r1--forme-du-jeton-daccès--cest-le-modèle-qui-tranche)).

| Colonne | Usage |
|---|---|
| `refresh_token_hash` | SHA-256 du jeton de rafraîchissement. Le clair n'existe que dans le cookie |
| `expires_at` | 12 h, ou 30 j avec « rester connecté » (FR-030) |
| `revoked_at`, `revoked_reason` | La rotation s'écrit ici, en chaînant des lignes. Motifs employés : `rotated`, `logout`, `logout_all`, `reuse_detected`, `password_changed`, `status_changed`, `anonymization` (ce dernier écrit par la base) |
| `user_agent`, `ip_address` | Renseignés à l'ouverture, pour que la personne reconnaisse ses appareils |
| `last_seen_at` | Porte l'instant d'ouverture de la ligne, et rien d'autre : la rotation en crée une neuve à chaque renouvellement, donc la date suit l'activité **sans qu'aucune écriture ne s'ajoute**. Une mise à jour par requête ferait de cette table le point chaud de la base |
| `account_id` | `ON DELETE SET NULL` : l'effacement d'un compte laisse la session historisée |

### `identity.one_time_tokens` — les liens reçus par courriel

| Colonne | Usage |
|---|---|
| `token_hash` | SHA-256. Le clair ne vit que dans le courriel et dans la charge utile du travail différé, effacée à l'envoi ([research.md § R8](research.md#r8--le-chemin-du-jeton-en-clair-jusquau-courriel)) |
| `purpose` | **Détermine la durée de validité** (FR-017, écart n° 19). Aucun appelant ne pose d'expiration |
| `expires_at` | `NOT NULL` sans valeur par défaut — c'est précisément l'écart. Dérivée de la finalité par la configuration |
| `consumed_at` | Posée par une écriture **conditionnelle** (`WHERE consumed_at IS NULL`) : deux clics simultanés n'aboutissent qu'une fois (FR-041) |
| `payload` | Libre. On y range l'adresse visée, comme le font déjà les données simulées — de quoi composer le courriel sans relire la personne. **Jamais de secret** |
| `person_id` | Nullable dans le modèle (invitation d'une personne pas encore créée). Toujours renseignée pour les deux finalités de ce jalon |

### RBAC — `permissions`, `roles`, `role_permissions`, `role_assignments`

Aucune écriture sur les trois premières : le catalogue est semé par `030_identity.sql` § 6 et administré par migration.

`role_assignments` est la seule table écrite. Quatre invariants y sont portés par la base et **ne sont pas recopiés en Rust** :

| Invariant | Ce qu'il refuse | Devient |
|---|---|---|
| `tg_role_assignments_check_scope` | un rôle attribué sur une portée que `roles.allowed_scopes` n'autorise pas | `scope_not_allowed`, **avec le message français que le trigger écrit lui-même** |
| `ux_role_assignments_active` | la même personne, le même rôle, la même portée, deux fois en cours | `duplicate`, avec l'attribution en conflit |
| `ck_role_assignment_scope` | une portée globale avec une cible, ou une portée ciblée sans cible | erreur de validation, champ `scope_id` |
| `ck_role_assignment_window` | une date de fin antérieure à la prise d'effet | erreur de validation, champ `valid_until` |
| `ck_role_assignment_revocation` | un auteur ou un motif de retrait sur une attribution **en cours** | erreur de validation |

**Une attribution n'est jamais supprimée** : le retrait pose `revoked_at`, `revoked_by`, `revoked_reason`. `note` reste le motif de l'**octroi** et ne sert jamais deux fois.

### RGPD — `consents`, `current_consents`, `privacy_requests`

`privacy_requests.due_at` porte son échéance réglementaire par valeur par défaut — trente jours. Le service ne la calcule pas. `identity.anonymize_person()` fait tout le travail d'effacement **et émet elle-même son événement** : le service ne doit pas l'émettre une seconde fois (voir [`contracts/events.md`](contracts/events.md)).

### Ce que le module lit hors de son schéma

| Objet | Schéma | Usage |
|---|---|---|
| `reference.locales` | noyau partagé | négociation de la langue de la requête |
| `reference.countries` | noyau partagé | pays d'une personne, à l'inscription et sur la fiche |
| `platform.t()`, `platform.i18n_text` | noyau partagé | résolution des textes du modèle selon la langue |
| `platform.emit_event()`, `platform.jobs`, `claim_jobs()`, `fail_job()` | noyau partagé | effets différés |
| `platform.modules` | noyau partagé | montage des routes au démarrage. **`platform.feature_flags` n'est PAS lue par l'API** : les deux tables ne sont pas au même étage — `modules` décide si un module a des routes du tout (404 quand il n'en a pas), `feature_flags` décide de ce que le SITE affiche d'un module déjà monté |
| `platform.current_actor_id()`, `current_request_id()` | noyau partagé | contexte d'écriture |
| `analytics.v_operational_health` | analytique | route de santé |

Aucune clé étrangère n'est créée, donc `platform.cross_module_fk_report` reste vide sans effort.

---

## 2. Les fonctions du modèle, et l'usage qui en est fait

| Fonction | Signature retenue | Où |
|---|---|---|
| `identity.has_permission(personne, permission, type_de_portée, portée)` | `boolean` | **Unique point de décision d'autorisation.** Jamais de test de nom de rôle |
| `identity.administered_events(personne)` | `TABLE (is_global boolean, event_ids uuid[])` | **Fonction qui retourne une table : on l'appelle, on ne la joint pas.** Renvoie toujours exactement une ligne, jamais de NULL |
| `identity.effective_permissions(personne)` | `TABLE (permission_code, scope_type, scope_id)` | Composition de l'écran « ce que cette personne peut faire » |
| `identity.anonymize_person(personne, motif)` | `void`, `SECURITY DEFINER` | Effacement RGPD. Émet son propre événement |
| `platform.emit_event(...)` | `uuid` | Émission dans la transaction du changement d'état |
| `platform.claim_jobs(file, worker, lot)` | `SETOF platform.jobs` | Réservation atomique par le worker |
| `platform.fail_job(travail, erreur)` | `void` | Replanification à délai croissant, puis file morte |
| `platform.t(champ, locale)` | `text` | Résolution d'un texte du modèle, repli sur le français |
| `platform.is_feature_enabled(clé, personne)` | `boolean` | Non utilisé par ce module — les drapeaux commandent le routage du front |

**Le filtre canonique de périmètre**, tel qu'il s'écrit partout :

```sql
SELECT is_global, event_ids FROM identity.administered_events($1);
-- puis :  WHERE is_global OR event_id = ANY(event_ids)
```

Et les trois cas, qui restent distincts dans le code :

| Retour | Signification | Réponse |
|---|---|---|
| `(true, …)` | tous les événements | accès complet, `event_ids` non signifiant |
| `(false, {…})` | les éditions listées | liste filtrée sur ces éditions |
| `(false, '{}')` | aucun droit | **refus explicite**, jamais une liste vide |

**Point clos en B1-specify** : le filtre de la fonction porte sur `programme.proposal.read_all`, et les quatre rôles attribuables sur une édition la détiennent (`admin`, `reviewer`, `programmer`, `super_admin`). Aucune modification n'est requise.

---

## 3. Les machines à états

### Session

```
        connexion réussie
              │
              ▼
        ┌──────────┐   renouvellement    ┌──────────┐
        │  ouverte │ ──────────────────► │ révoquée │  motif = rotated
        └──────────┘   (nouvelle ligne)  └──────────┘
              │
              ├── déconnexion ─────────► révoquée · logout
              ├── déconnexion totale ──► révoquée · logout_all
              ├── mot de passe changé ─► révoquée · password_changed
              ├── suspension / blocage ► révoquée · status_changed
              ├── effacement RGPD ─────► révoquée · anonymization   (écrit par la base)
              ├── jeton rejoué ────────► TOUTES révoquées · reuse_detected
              └── échéance atteinte ───► expirée (pas de révocation : la date suffit)
```

Une ligne révoquée n'est jamais supprimée : c'est l'historique des appareils.

### Jeton à usage unique

```
       créé ──► valide ──► consommé          (consumed_at posée, une seule fois)
                   │
                   ├────► périmé             (expires_at dépassée)
                   └────► invalidé           (un jeton plus récent, même finalité, même personne)
```

Trois refus, et **« déjà utilisé » l'emporte sur « périmé »** : un jeton consommé puis périmé dit que le travail est fait, là où « le lien a expiré » enverrait redemander un courriel inutile. C'est l'ordre que suivent déjà les données simulées.

### Statut d'une personne

```
    active ⇄ suspended        (suspension : date de fin OBLIGATOIRE)
      │  ⇅
      │  blocked              (exclusion durable, sans terme)
      ▼
    anonymized                IRRÉVERSIBLE — posé UNIQUEMENT par anonymize_person(),
                              depuis une demande d'EFFACEMENT. Jamais depuis un
                              panneau de modération.
```

### Demande RGPD

```
    received ──► in_progress ──► completed
        │             │
        └─────────────┴────────► rejected
```

L'acte d'anonymisation est **distinct** du changement de statut de la demande : on peut clore administrativement sans effacer, et l'effacement ne vaut que pour une demande d'effacement (FR-060).

---

## 4. Correspondance Rust ↔ base

Règles, non exhaustives mais sans exception :

| En base | En Rust |
|---|---|
| `uuid` | `uuid::Uuid`, enveloppé par agrégat (`PersonId`, `AccountId`, `SessionId`…) pour qu'on ne passe pas l'un pour l'autre |
| `timestamptz` | `time::OffsetDateTime`, sérialisé en RFC 3339 — ce que le front nomme `IsoDateTime` |
| `platform.i18n_text` (domaine sur `jsonb`) | **résolu en base** par `platform.t()` quand un seul texte suffit ; rendu tel quel quand le front en attend l'objet complet (`I18nText`) |
| `platform.email` (domaine sur `citext`) | `String`. **La comparaison insensible à la casse est faite par la base**, pas par le service |
| ENUM PostgreSQL (`person_status`, `token_purpose`, `scope_type`, `auth_provider`, `privacy_request_*`) | `enum` Rust dérivé `sqlx::Type`, avec le nom de type PostgreSQL déclaré. **Ce sont des machines à états, pas des vocabulaires ouverts** — les vocabulaires ouverts vivent dans `reference.taxonomy_terms` et n'ont jamais d'`enum` Rust |
| `text[]`, `uuid[]` | `Vec<…>` |
| `bytea` | `[u8; 32]` pour les empreintes SHA-256 |
| colonne générée (`display_name`, `search_vector`) | champ **en lecture seule** dans la structure, jamais dans un `INSERT` |
| `inet` | `std::net::IpAddr`, converti en `IpNetwork` au moment de l'écriture — c'est la forme que SQLx donne à `inet`, et l'adresse d'un poste en est le cas dégénéré. La caractéristique `ipnetwork` est activée pour cette seule colonne : sans elle, l'adresse traverserait en texte et c'est PostgreSQL qui refuserait une valeur mal formée |

**Les secrets ne franchissent aucune frontière de sérialisation.** `password_hash`, `mfa_secret_encrypted`, `mfa_recovery_codes`, `refresh_token_hash` et `token_hash` n'apparaissent dans aucune structure de réponse — c'est la règle que `frontend/app/types/identity.ts` applique déjà côté client, et elle vaut d'abord côté serveur.

---

## 5. Ce que le front attend, et d'où ça vient

Les formes de réponse ne se redéfinissent pas ici : elles sont dans `frontend/app/types/`. Ce tableau dit seulement **quelles tables composent quoi**.

| Forme attendue (front) | Composée depuis |
|---|---|
| `Person` | `identity.people`, colonnes générées comprises |
| `LoginResult` | `people` + `accounts`, selon l'ordre de contrôle imposé |
| `VerifyEmailResult`, `TokenCheckResult`, `PasswordResetResult` | `one_time_tokens` + `people` |
| `EffectivePermission[]` | `identity.effective_permissions()` |
| `AdministeredEvents` | `identity.administered_events()` |
| `UserListScreen` | `people` + `accounts` + `role_assignments` + `roles` + `reference.countries` + `org.organizations` (lecture seule inter-schéma) + `privacy_requests`, borné par le périmètre |
| `UserDetail` | tout ce qui précède, plus `person_emails`, `current_consents`, `platform.audit_log` pour l'historique des attributions |
| `RoleAssignmentOptions` | `roles` + `effective_permissions()` de l'acteur + les portées atteignables |
| `EffectivePermissionsView` | `effective_permissions()` **enrichie de l'origine** — quel rôle apporte quoi. La fonction rend `(permission, portée)` sans dire d'où ça vient : suffisant pour autoriser, insuffisant pour expliquer, et l'écran demandé est un écran d'explication |
| `PrivacyQueueScreen` | `privacy_requests` + `people`, sans borne d'édition |

### Une lecture franchit une frontière de module, et c'est assumé

`UserListRow.organization_name` et `UserDetail.organization_name` viennent de `org.organizations`, hors du schéma `identity`.

Ce n'est **ni une dépendance de crate ni un appel de module à module** : c'est une jointure SQL en lecture seule, `identity` ne dépendant toujours que de `kernel` et de `contracts`. Les deux autres lectures externes — `reference.countries` et `platform.audit_log` — relèvent du **noyau partagé**, que la constitution exempte explicitement.

Le franchissement vers `org`, lui, est déjà **acté par le modèle** : `identity.people.primary_organization_id` porte le commentaire « Cross-module assumé : nommage `xmod_fk_` pour être identifiable lors d'une extraction de service ». La colonne existe, la jointure la suit. Le jour où `org` deviendrait un service distant, `platform.generate_module_decoupling_script('org')` produit mécaniquement la liste des liens à couper, et cette jointure y figure.

**À traiter en B2**, où la question se posera dans l'autre sens : soit la lecture reste une jointure, soit elle passe par un contrat. Noté ici pour qu'elle se décide, et non qu'elle se découvre.

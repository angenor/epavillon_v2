# Phase 1 — Le modèle, et ce que le code en fait

**Fonctionnalité** : Organisations (B2) · **Date** : 2026-08-20

**Aucune table, aucune colonne, aucun type n'est créé ni modifié.** Ce document ne redéfinit pas le modèle : il dit, table par table, ce que le module lit, ce qu'il écrit, ce qu'il laisse à la base, et d'où vient chaque forme que le front attend. La source reste [`docs/database/040_organizations.sql`](../../docs/database/040_organizations.sql), qui porte sa propre documentation.

---

## 1. Ce que le code lit, écrit, et ne touche pas

### `org.organizations` — la fiche

| Colonne | Ce que le module en fait |
|---|---|
| `legal_name`, `acronym` | Écrits à la création et par les arbitrages de fusion. **Leur modification alimente les dénominations toute seule** — trigger `tg_organizations_sync_names`, jamais recopié en Rust |
| `legal_name_normalized`, `acronym_normalized` | **Jamais écrits** — colonnes engendrées. Lues seulement par la fonction de recherche |
| `slug` | Composé par l'API à la création (`platform.slugify`), rendu unique en cas de collision. **Jamais déplacé par une fusion** (R6) |
| `organization_type_code` | Code de `reference.taxonomy_terms`, taxonomie `organization_type`. Vérifié à l'écriture ; **jamais un énuméré Rust** |
| `status` | Machine à états, § 3. Le formulaire crée `candidate` ; le sceau admet en `active` ; la fusion pose `merged` — **par la fonction de base, jamais par le service** |
| `merged_into_id`, `merged_at` | **Jamais écrits par le service.** `merge_organizations()` les pose, et `tg_organizations_no_merge_chain` interdit les chaînes |
| `verified_at`, `verified_by` | Le sceau. Distinct du statut : les confondre ferait disparaître d'un écran une organisation qu'on voulait seulement ne pas mettre en avant |
| `trust_score` | Écrit **uniquement** par le travail différé de recalcul (R12), et seulement quand la valeur change |
| `created_by`, `created_at`, `updated_at` | Posés à la création ; `updated_at` par trigger |

Les vérifications que le code **ne redouble pas** : longueur du nom, longueur du sigle, forme de l'adresse d'URL et de l'adresse électronique (domaines `platform.*`), cohérence de la fusion, absence d'auto-fusion, unicité (nom normalisé, pays) sur les fiches vivantes, unicité de l'adresse d'URL. Toutes sont traduites, aucune n'est anticipée — [`contracts/errors.md`](contracts/errors.md).

### `org.organization_names` — les dénominations

Le module **écrit** les dénominations reçues d'un import ou d'un utilisateur, et **confirme** ou déconfirme une dénomination existante. Il **n'écrit jamais** celles que le trigger pose (nom légal, sigle) : la fiche complète les marque `is_derived` en comparant la dénomination normalisée à celles de la fiche, et l'API refuse de les retirer.

`is_confirmed` ne décide que de l'**affichage** : confirmée ou non, une dénomination sert la recherche. C'est ce qui permet de retrouver une fiche par une faute d'orthographe connue sans jamais l'afficher sous ce nom.

`kind` porte l'ordre de départage de deux dénominations d'égal score, que la fonction de recherche calcule déjà : légal, sigle, court, traduction, ancien, faute de frappe — et dix rangs de pénalité pour une dénomination non confirmée.

### `org.organization_domains` — les domaines

Écrits à la vérification manuelle (`verification_method = 'manual'`) et au réglage du rattachement automatique. Les deux autres méthodes de vérification existent dans le modèle et **ne sont pas livrées**.

Deux invariants tenus par la base et **traduits, jamais recopiés** : un domaine vérifié n'appartient qu'à une fiche (`ux_organization_domains_verified`) — le refus **nomme** la fiche qui le détient, sans quoi il est incompréhensible ; et le rattachement automatique exige la vérification (`ck_domain_autojoin_requires_verification`).

### `org.public_email_domains` — les messageries grand public

**Lecture seule, et jamais recopiée en Rust.** Vingt domaines semés. La fonction de recherche les neutralise elle-même ; la lecture « ce que révèle mon adresse » les interroge par jointure. Une liste en dur dans le code se périmerait le jour où l'IFDD en ajoute une.

### `org.memberships` — les adhésions

| Colonne | Ce que le module en fait |
|---|---|
| `role`, `status` | Machines à états, § 3 |
| `is_primary` | **Jamais calculé par le service.** `tg_default_primary_membership` attribue la primauté à la première adhésion active, et `tg_sync_primary_organization` la répercute sur la personne |
| `invited_by`, `invited_at` | **La direction de l'attente.** Renseignées : l'organisation a invité. Nulles : la personne a demandé. C'est ce qui sépare les deux files, et la vérification `ck_memberships_invitation` interdit d'en renseigner une seule |
| `approved_by`, `approved_at` | Posées par la décision d'un référent, ou par le rattachement automatique |
| `revoked_at` | Posée par un refus ou un retrait. **La ligne n'est jamais supprimée** |

L'unicité `ux_memberships (organization_id, person_id)` **ne connaît pas le statut** : une adhésion révoquée occupe la place. La demande de rattachement est donc un unique ordre d'insertion avec reprise conditionnelle (R7), jamais une lecture suivie d'une écriture.

### `org.duplicate_candidates` — la file

Écrite par le balayage (R11) et par les décisions du back-office. `merge_organizations()` marque elle-même la paire fusionnée : **le service ne le refait pas**.

`ck_duplicate_candidates_ordered` impose `left_id < right_id` — les identifiants sont donc ordonnés à l'écriture, et l'ordre de la paire **ne dit rien** de qui absorbe qui : c'est la désignation de l'écran de fusion, et elle n'a aucun rapport avec la place d'une fiche dans la paire.

### `org.organization_references` — le registre

**Lecture seule.** Dix-huit lignes, alimentées par huit fichiers du modèle (040, 050, 060, 070, 075, 080, 090, 125). Le module le parcourt pour chiffrer le transfert d'une fusion (R4) et pour nommer les lignes à l'écran ; il n'y écrit jamais — un module y déclare ses références par son DDL, pas par du code.

### `org.merge_log` — le journal des fusions

**Écrit par la fonction de base, relu par le service** dans la même transaction pour rendre le décompte réel à l'écran. C'est la seule façon d'obtenir `rows_reassigned` : la fonction ne rend que l'identifiant de la fiche survivante.

### Ce que le module lit hors de son schéma

Cinq lectures franchissent une frontière de schéma. Aucune n'est une dépendance de crate ni un appel de module à module — la règle qui les autorise, et ses limites, sont posées en R14.

| Lecture | Pourquoi elle est nécessaire |
|---|---|
| `identity.people` | Le nom d'un membre, d'un créateur, d'un vérificateur. La fiche du back-office les affiche |
| `identity.one_time_tokens` | L'invitation voyage par un jeton de finalité `invitation` — par `kernel::tokens`, jamais par du SQL écrit dans ce module (R8) |
| `programme.proposals`, `proposal_organizations`, `sessions` | Le filtrage par périmètre d'administration, et les activités de la fiche. Une organisation n'appartient à aucune édition : c'est **l'activité déposée** qui la rattache à un périmètre |
| `reference.taxonomy_terms`, `reference.countries` | Libellé et couleur d'un type d'organisation, nom d'un pays — des **données**, jamais des traductions d'interface |
| `analytics.mv_organization_scorecard` | La fiche de performance, projection matérialisée |
| `platform.entity_history()` | L'historique champ par champ de la fiche |

Le module **n'écrit nulle part hors de son schéma**, à une exception près et elle est portée par le noyau : la consommation d'un jeton d'invitation.

---

## 2. Les fonctions du modèle, et l'usage qui en est fait

| Fonction | Usage | Ce qu'il ne faut pas faire |
|---|---|---|
| `org.find_similar_organizations(nom, pays, courriel, site, limite)` | Les **deux** lectures de recherche et le balayage de détection | Ne pas la modifier. Ne pas recopier son score en Rust — il est déjà calculé |
| `org.resolve_organization(id)` | Porter une demande de rattachement sur la fiche vivante quand la cible a été absorbée | Ne pas suivre `merged_into_id` à la main : la fonction existe pour ça, et le trigger garantit qu'il n'y a jamais de chaîne |
| `org.merge_organizations(source, cible, motif)` | **Toute** la fusion des rattachements | **N'émet pas d'événement après elle** : elle le fait déjà. **Ne marque pas la paire** : elle le fait déjà. Ne pas lui ajouter de paramètre |
| `org.compute_trust_score(id)` | Le travail différé de recalcul | Ne pas l'appeler dans une transaction d'écriture métier : c'est un agrégat, pas un invariant |
| `platform.normalize_label`, `slugify`, `extract_domain` | Composition de l'adresse d'URL, comparaison du nom de confirmation, extraction du domaine d'une adresse | Ne pas réécrire la normalisation en Rust : la casse, les accents et la ponctuation y sont traités **comme la base les traite**, et un écart d'une virgule ferait diverger la comparaison |
| `platform.t(champ, locale)` | Résolution des libellés multilingues du modèle | Jamais `.fr` en direct, jamais recopié dans un fichier de traduction |
| `platform.entity_history('org','organizations',id)` | L'onglet Historique de la fiche | Ce n'est pas une table : c'est un sous-produit du journal d'audit |
| `identity.has_permission`, `identity.administered_events` | Par le garde du noyau, jamais appelées directement | Jamais un nom de rôle |
| `platform.emit_event`, `platform.jobs` | Les six événements du module et les trois travaux de fond | Jamais d'insertion à la main dans l'outbox |

---

## 3. Les machines à états

### Statut d'une organisation

```text
                    ┌──────────────┐
   création publique│  candidate   │
   ─────────────────>              │
                    └──────┬───────┘
                           │ sceau posé (le sceau ADMET du même geste)
                           v
                    ┌──────────────┐   fusion (fonction de base)   ┌─────────┐
                    │    active    │ ─────────────────────────────>│ merged  │
                    └──────┬───────┘                               └─────────┘
                           │ décision du back-office                  (terminal :
                           v                                           un trigger
                    ┌──────────────┐                                   interdit les
                    │ archived     │                                   chaînes)
                    │ rejected     │
                    └──────────────┘
```

Ce que le module **écrit** : `candidate` à la création, `active` en posant le sceau. Ce qu'il **n'écrit pas** : `merged`, posé par la fonction de base. `archived` et `rejected` existent dans le modèle et **ne sont pas livrés** — aucun écran ne les demande dans ce jalon.

Retirer le sceau ne change **pas** le statut : la fiche reste active, elle cesse d'être certifiée.

### Adhésion — deux entrées, deux sorties, jamais de suppression

```text
        demande spontanée              invitation émise
     (invited_at NULL)              (invited_at renseigné)
              │                              │
              v                              v
         ┌─────────┐                    ┌─────────┐
         │ pending │                    │ pending │
         └────┬────┘                    └────┬────┘
   référent   │   référent          personne │  (personne, par son jeton)
   approuve   │   refuse             accepte │
              v                              v
         ┌────────┐  retrait / refus    ┌────────┐
         │ active │ ──────────────────> │revoked │ ──> nouvelle demande : REPRISE
         └────────┘                     └────────┘      de la même ligne (R7)
```

**Le même mot recouvre deux attentes inverses**, et c'est `invited_at` qui les sépare. Un référent ne peut jamais décider d'une invitation ; une personne ne peut jamais accepter une demande par un jeton.

### Décision sur une paire de doublons

```text
   (aucune décision) ──> distinct   : retirée de la file, JAMAIS ressuscitée par le balayage
                    ──> deferred    : remise à plus tard, remise en circulation possible
                    ──> merged      : posée par la fonction de fusion, pas par le service
```

---

## 4. Correspondance Rust ↔ base

| En base | En Rust | Remarque |
|---|---|---|
| `org.organization_status` | énuméré `sqlx::Type` | Machine à états **fermée** : un énuméré est légitime |
| `org.membership_role`, `org.membership_status` | énumérés `sqlx::Type` | Idem |
| `org.name_kind` | énuméré `sqlx::Type` | Idem — et il porte l'ordre de départage |
| `organization_type_code` | `String` | **Vocabulaire ouvert** : jamais un énuméré. Il vit dans `reference.taxonomy_terms`, l'IFDD le modifie depuis le back-office |
| `match_reasons text[]` | `Vec<String>` | Le front en connaît quatre valeurs ; en figer un énuméré ferait échouer la désérialisation le jour où la fonction en ajoute une |
| `score numeric` | `rust_decimal` / `f64` sérialisé tel quel | Le front le compare à un seuil qu'il porte lui-même (85) |
| `rows_reassigned jsonb` | `serde_json::Value` | Clés `schéma.table.colonne`, produites par la fonction |
| `description platform.i18n_text` | `serde_json::Value` | Résolu à l'affichage, jamais à plat |
| `platform.email`, `platform.url`, `platform.slug` | `String` | Domaines vérifiés **par la base** ; le code traduit le refus |
| `verification_method` | `String` avec vérification `CHECK` | Trois valeurs, une seule livrée |

---

## 5. Ce que le front attend, et d'où ça vient

| Forme attendue | Fichier du front | Source |
|---|---|---|
| `SimilarOrganization` | `types/org.ts` | Les treize colonnes de `find_similar_organizations()`, telles quelles |
| `EmailDomainMatch` | `types/organization-join.ts` | `organization_domains` + `organizations` + un décompte d'adhésions actives + le test de rattachement immédiat |
| `JoinOrganizationResult` | idem | Trois issues, calculées par le service |
| `CreateOrganizationResult` | idem | Deux issues ; `name_taken` porte un `SimilarOrganization` composé depuis la fiche en conflit |
| `MemberEntry`, `InviteMemberResult` | `types/organization-workspace.ts` | `memberships` + `identity.people` + le test `invited_at IS NOT NULL` |
| `OrganizationListScreen` | `types/admin-organizations.ts` | `mv_organization_scorecard`, **quatre colonnes relues sur la table vivante**, facettes comptées sur le même jeu, compteur de paires ouvertes, drapeau de restriction |
| `DuplicateQueueScreen`, `DuplicatePair`, `DuplicateSide` | idem | `duplicate_candidates` + les deux fiches réduites à ce qui permet de trancher |
| `MergePreview`, `MergeTransferLine` | idem | Comparatif des dix champs + **le registre des références**, chiffré (R4) |
| `MergeResult` | idem | `merge_log.rows_reassigned`, relu dans la transaction |
| `OrganizationDetail` | idem | Huit lectures assemblées (R15) |
| `OrganizationWriteResult` | idem | La fiche entière recomposée après chaque écriture |

**Deux points où la forme du front ne recouvre pas exactement le modèle**, et comment ils se règlent :

- **`MergeField` range `slug` parmi les champs comparés.** L'API le compare et refuse de le déplacer (R6). Ce n'est pas une divergence de contrat : c'est un refus, exprimé par un code stable qui nomme le champ.
- **`OrganizationScorecard.statut` est du texte**, la vue le rendant ainsi ; l'énuméré Rust est reconverti au transport. Même geste qu'en B1 pour la dénomination d'une portée.

---

## 6. Ce que ce module ne touche pas, alors qu'il le côtoie

- **`identity.people.primary_organization_id`** — le registre des références le déclare, et la fusion le réaffecte **par la fonction de base**. Le service n'y écrit jamais ; le trigger d'adhésion s'en charge le reste du temps.
- **`analytics.mv_organization_scorecard`** — lue, rafraîchie par un travail (R13), jamais écrite.
- **Les tables des modules hors jalon** que le registre déclare (`media`, `publication`, `training`) — comptées à zéro aujourd'hui, comptées demain par le même code, sans modification.

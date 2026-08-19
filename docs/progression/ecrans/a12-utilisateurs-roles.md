# A12 — Utilisateurs et rôles

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 18/08. **Le modèle a été corrigé d'abord, sur deux points** : `role_assignments` gagne `revoked_by` et `revoked_reason` (plus `ck_role_assignment_revocation`), et `identity.role.assign` — qu'aucun rôle ne détenait — est accordée à `admin`. 3 pages sous `app/pages/admin/utilisateurs/` (liste, fiche à quatre onglets, file RGPD), 10 composants sous `app/components/admin/users/`, 2 utilitaires purs (`utils/role-scope.ts`, `utils/user-list.ts`), 1 fichier de contrats (`types/admin-users.ts`), 1 dossier de mocks en quatre fichiers (`mocks/admin-users/`), 2 mocks d'entité (`mocks/platform.ts`, `mocks/privacy.ts`), 1 fabrique d'API, 10 fichiers de traduction (5 × 2 locales). **Découpage collatéral** : l'espace organisation (A5) sort de `useApi.ts` dans `composables/api/organization-workspace.ts` — le fichier atteignait mille lignes

---

## Écarts relevés en écrivant les utilisateurs et les rôles (A12, 18/08)

**Deux défauts du MODÈLE, corrigés avant d'écrire les écrans** (voir le tableau plus haut).
Suivent six points qui ne se tranchent pas depuis un écran.

1. **« Les utilisateurs de la COP31 » N'EXISTE PAS comme colonne.** Une personne n'appartient à aucune édition, pas
   plus qu'une organisation (A11) : la règle métier n° 8 ne peut pas filtrer sur `event_id`. Le rattachement est un
   CALCUL, et il a fallu le poser explicitement — quatre liens comptent : une attribution de rôle portée sur
   l'édition, un dossier déposé par une organisation dont la personne est membre, une intervention annoncée, une
   inscription à l'une de ses séances. N'en retenir que le premier réduirait la liste au comité, et le responsable
   d'un webinaire ne verrait pas les personnes inscrites au sien. **À reproduire tel quel côté API (B1)** : c'est une
   décision de périmètre, pas une commodité d'écran.

2. **`identity.effective_permissions()` AUTORISE mais N'EXPLIQUE PAS.** Elle rend des lignes (permission, portée) sans
   dire d'où elles viennent. C'est assez pour un jeton d'accès, insuffisant pour l'écran que le prompt demande — « voici
   ce que cette personne peut faire, et où » est un écran d'EXPLICATION, et la réponse utile n'est pas « elle a
   `programme.proposal.decide` sur `event:01a…` » mais « elle est administratrice de la COP31 ». La composition remonte
   donc jusqu'au rôle et à l'attribution. **Ne pas ajouter de colonne à la fonction** : le jeton d'accès n'a que faire
   du rôle d'origine, et l'alourdir pour un seul écran serait payer partout ce qui ne sert qu'ici.

3. **`role_assignments.scope_id` N'A AUCUNE CLÉ ÉTRANGÈRE, et c'est délibéré** — la cible vit dans un autre module, qui
   peut devenir un service distant. Conséquence à l'écran : le nom d'une portée se résout par une jointure
   APPLICATIVE, et il peut MANQUER. Une édition supprimée laisse une attribution orpheline, qui donne des droits sur
   rien. L'interface la SIGNALE (« portée introuvable », en italique) avec de quoi la retirer, plutôt que de la taire.
   **Obligation d'API** : la même résolution, et le même aveu quand la cible a disparu.

4. **`ux_role_assignments_active` NE COUVRE QUE LES ATTRIBUTIONS NON RÉVOQUÉES — pas les attributions EXPIRÉES.** Une
   attribution arrivée à son terme n'apparaît dans aucune liste de rôles actifs, et pourtant l'index la compte encore :
   réattribuer le même rôle sur la même portée échoue. C'est le piège de cet index, et il faut le connaître des deux
   côtés — l'écran détecte le doublon sur les attributions NON RÉVOQUÉES, actives et expirées confondues.

5. **La portée `negotiation_space` est admise par le modèle et n'a AUCUNE CIBLE.** Le rôle `negotiator` la déclare dans
   `allowed_scopes`, mais le module Négociations est hors du jalon : aucune donnée ne décrit ses espaces. Le panneau
   l'affiche donc, DÉSACTIVÉE et expliquée. Masquer une portée que le modèle autorise ferait croire à un oubli ; en
   offrir une sans cible donnerait un formulaire qu'on ne peut pas valider. **À rouvrir avec le module**, pas avant.

6. **Le RGPD n'a pas de permission propre.** Traiter une demande d'export ou d'effacement passe par
   `identity.person.manage`, faute de mieux. C'est défendable — qui gère les comptes gère les demandes qui les
   concernent — mais cela empêche de confier la file RGPD à un délégué à la protection des données SANS lui ouvrir la
   suspension et le blocage. Si l'IFDD désigne un DPO, il faudra une permission de plus dans `030_identity.sql`
   (`identity.privacy.handle`), pas une règle dans un écran. **Question au commanditaire, non bloquante.**

7. **`identity.person_emails` n'a aucune donnée simulée.** Le jeu ne porte qu'une adresse par personne ; la fiche
   affiche le bloc « autres adresses » quand il est rempli et le tait sinon. Rien à trancher — c'est une lacune du jeu
   de démonstration, pas du modèle —, mais l'API le remplira et l'écran est prêt.

8. **`identity.sessions` non plus.** Une suspension annonce donc « les sessions ouvertes seront fermées » sans pouvoir
   les compter. Le modèle porte tout ce qu'il faut (`revoked_at`, `revoked_reason`) ; c'est l'API qui comptera.

---

## Ce qui a été vérifié le 18/08 sur les utilisateurs et les rôles, et comment

**Sur la base, schéma rechargé de zéro** (`down -v` puis `up -d` ; le port 5432 étant occupé par un autre projet,
`POSTGRES_PORT=5460` — le conteneur écoute toujours 5432 en interne, rien d'autre ne change) :

- **174 tables chargées, aucune erreur dans les journaux d'initialisation**, `platform.cross_module_fk_report`
  toujours vide, `make assert-db` au vert (15 schémas, frontières conformes, projections rafraîchies).
- `\d identity.role_assignments` : `revoked_by` et `revoked_reason` présentes, `ck_role_assignment_revocation` posée,
  la clé étrangère vers `people` créée.
- **Les trois garde-fous, éprouvés par une transaction jetable** plutôt que par lecture :
  - `INSERT` de `super_admin` sur une portée `event` → **REFUSÉ** par `tg_check_role_scope`, avec le message français
    attendu (« portées autorisées : global »). C'est ce qui interdit l'élévation depuis un compte détaché ;
  - pour une personne portant `admin` sur une seule édition,
    `has_permission(…, 'identity.role.assign', 'global', NULL)` et la même sur une portée `organization` rendent
    toutes deux **faux** — la permission accordée à `admin` ne vaut donc QUE sur l'édition confiée ;
  - `UPDATE … SET revoked_reason = 'essai'` sur une attribution vivante → **REFUSÉ** par
    `ck_role_assignment_revocation`.
- `role_permissions` pour `admin` porte bien les trois permissions d'identité, dont `identity.role.assign`.

**Sur l'interface, dans un navigateur, en se connectant réellement** — c'est là que les deux défauts ont été trouvés,
et aucun des deux ne se voyait à la relecture :

- **La colonne des rôles était VIDE pour tout le monde.** `.filter(isAssignmentActive)` passe l'INDEX de l'élément en
  second argument, c'est-à-dire en « maintenant » : la première attribution était comparée à l'époque Unix, et toutes
  paraissaient à venir. Corrigé par une lambda explicite aux trois endroits, avec le commentaire qui dit pourquoi.
- **Le panneau d'attribution ne s'ouvrait pas depuis la LISTE** — il s'ouvrait depuis la fiche. `UiDrawer` monté
  `open` déjà vrai ne se montrait jamais : son watcher n'avait rien vu changer. `UiModal` avait corrigé exactement
  cela au prompt A2, commentaire à l'appui ; le tiroir traînait le défaut. Corrigé de la même façon (`onMounted`).
- **Les quatre statuts s'affichaient comme quatre filtres ACTIFS.** `UiChip` dit « ce filtre est appliqué, voici
  comment l'enlever » — quatre jetons posés d'avance se lisent comme l'inverse d'une liste non filtrée. Remplacés par
  des cases à cocher.
- **Le titre complet d'une édition chassait les colonnes suivantes hors de l'écran.** La pastille porte désormais le
  SIGLE (« Administrateur · COP31 »), le titre entier restant dans l'infobulle et sur la fiche de l'édition.

**Le parcours complet, joué de bout en bout, en administratrice GLOBALE** (Aminata Bakayoko) :

- attribution de « Programmateur » sur PACO 2027 à Alizeta Kaboré : le panneau annonce **18 permissions gagnées**
  AVANT l'écriture, puis la fiche passe de 2 à 3 rôles, de 5 à 8 permissions effectives et de 2 à 3 entrées
  d'historique — la propagation à l'onglet des permissions est la vérification qui compte, c'est elle qui prouve que
  l'écran d'explication n'affiche pas un état périmé ;
- retrait de « Révisionniste · COP31 » avec motif : la carte annonce **4 permissions perdues**, et l'historique porte
  aussitôt « Rôle retiré — Révisionniste · COP31 · Aminata Bakayoko · “Fin du mandat au comité de la COP31.” » ;
- suspension d'un compte avec terme et motif : le statut, le motif, la date de fin et l'auteur s'affichent sur la
  fiche.

**LA RÈGLE MÉTIER N° 8, ÉPROUVÉE PAR CONNEXION RÉELLE, ce qui n'avait jamais été possible.** Aucun compte à périmètre
restreint n'était CONNECTABLE : Claire Perret administre la seule COP31, mais son compte exige un second facteur que
le prompt A1 n'a pas implémenté. Un compte a donc été ajouté à Estelle Ngo Bassong — responsable du cycle PACO 2027,
le cas exact que décrit le commanditaire. Connectée :

- la liste ne montre **qu'une ligne**, la sienne, et le DIT (« vous ne voyez que les personnes qui interviennent dans
  les éditions que vous administrez ») ;
- le bouton « Demandes RGPD » **n'apparaît pas** — la file exige la portée globale ;
- dans le panneau d'attribution, « Toute la plateforme » est **désactivée avec son explication**, et la liste des
  éditions n'offre que **PACO 2027** : COP29, COP30, COP31 et PACO 2026 sont présentes mais non sélectionnables — les
  taire ferait croire à un bogue, les offrir mentirait.

**Chaîne complète** : `npm run typecheck` et `npm run build` au vert, `make assert-db` au vert. Aucun fichier de
`frontend/` ne dépasse mille lignes — `useApi.ts` les avait atteintes en montant la fabrique des utilisateurs, d'où la
sortie de l'espace organisation (A5) dans `composables/api/organization-workspace.ts`, sans qu'une ligne d'écran change.

# B0 — Constitution Spec Kit

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de ce jalon est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) § PHASE B — B0.

**État** : ✅ 20/08

---

## Ce qui a été livré

`.specify/memory/constitution.md`, **ratifiée en 1.0.0 le 20/08**. Le fichier ne contenait jusque-là
que les placeholders du gabarit `constitution-template` — aucune valeur de projet, aucun amendement
antérieur à préserver : la version part donc de zéro, sans reprise.

**Dix principes**, et non les cinq emplacements du gabarit — le nombre est imposé par l'entrée B0 de
`PROMPTS_DEVELOPPEMENT.md` :

| # | Principe | Vérification mécanique |
|---|----------|------------------------|
| I | Le modèle de données fait autorité | — (revue) |
| II | Frontières de modules (`kernel`, `contracts`, `api`, `worker`, `modules/<nom>`) | graphe de dépendances des crates |
| III | Frontières vérifiables en base (`xmod_fk_*`) | `cross_module_fk_report` vide — déjà dans `make check-db` |
| IV | Effets de bord par l'outbox transactionnel | test d'intégration : écriture attendue dans `outbox_events` |
| V | Autorisation par permission ET par portée | test d'intégration : refus sur URL forgée |
| VI | SQLx vérifié à la compilation, pas d'ORM | `cargo build` |
| VII | `app.actor_id` et `app.request_id` en début de transaction | — (revue) |
| VIII | Les invariants de la base ne sont pas réimplémentés | test d'intégration : une erreur d'invariant traduite |
| IX | Erreurs d'API : code stable, message français | — (revue) |
| X | Tests d'intégration sur base réelle, sans mock de base | `cargo test` sur base jetable |

Deux sections libres du gabarit reçoivent un contenu : **« Contraintes techniques »** (pile, limite de
1000 lignes par fichier applicatif, les deux sortes de textes multilingues, périmètre du jalon et
drapeaux de module) et **« Flux de travail et portes de qualité »** (cycle Spec Kit, `make check`,
contrat d'API au service du front existant, mise à jour de la progression).

La section `Governance` porte l'autorité relative des documents, la procédure d'amendement, le
versionnage sémantique et les trois vérifications bloquantes.

---

## Écarts constatés en écrivant la constitution

1. **`platform.cross_module_fk_report` N'EST PAS DANS `010_platform.sql`.** Le prompt B0 et
   `MODELE_INDEX.md` la citent sans dire où elle vit : c'est une **vue**, définie au § 9 de
   `000_bootstrap.sql` (« Gouvernance des frontières de modules »), pas dans le fichier `platform`. La
   constitution donne le fichier et le paragraphe, pour qu'aucune session ne la cherche là où elle
   n'est pas.

2. **LE NOYAU PARTAGÉ EST EXEMPTÉ DE `xmod_fk_*`, ET CE N'EST PAS UNE TOLÉRANCE.** La vue exempte
   explicitement `platform` et `reference` (`shared_kernel`) : une FK vers `reference.countries` est
   conforme sans préfixe. Écrire le principe III sans cette exemption aurait fait renommer des
   contraintes parfaitement légitimes. C'est la vue qui décide, pas une appréciation de revue.

3. **`platform.emit_event()` REMPLIT DÉJÀ L'ACTEUR ET LA CORRÉLATION — donc le principe VII
   CONDITIONNE le principe IV.** La fonction agrège `platform.current_actor_id()` dans `metadata` et
   `platform.current_request_id()` dans `correlation_id`. Une transaction qui n'a pas positionné
   `app.actor_id` émet donc un événement **anonyme sans erreur** : rien n'échoue, la trace est
   simplement perdue. Les deux principes sont écrits séparément mais l'un ne se tient pas sans
   l'autre.

4. **L'IDEMPOTENCE DU CONSOMMATEUR N'ÉTAIT PAS DANS LE PROMPT, ET LA BASE LA PRÉVOIT.**
   `platform.inbox_events (consumer, event_id)` existe depuis le premier jour et n'est mentionnée nulle
   part dans les dix principes reçus. Un worker qui redémarre rejoue ses événements ; sans garde, il
   produit deux fois ses effets. Ajouté au principe IV comme obligation, pas comme suggestion.

5. **LA CONTRAINTE DE FORME DU TYPE D'ÉVÉNEMENT EST DÉJÀ EN BASE.**
   `ck_outbox_event_type_format` impose `^[a-z_]+\.[a-z_]+\.[a-z_]+$` — trois segments exactement. Un
   `event_type` à deux ou quatre segments échoue à l'insertion. C'est un cas d'école du principe VIII :
   le code ne revalide pas la forme, il traduit l'échec.

6. **`identity.administered_events` NE FILTRE QUE SUR UNE PERMISSION : `programme.proposal.read_all`.**
   Le périmètre d'administration est donc **celui des propositions**, pas un périmètre générique. Une
   liste de back-office qui ne porte pas sur des propositions — organisations, incidents, utilisateurs
   — s'en sert quand même comme borne d'édition, ce que fait déjà le front, mais une personne
   administrant une édition **sans** cette permission tomberait en `(false, '{}')`. **À vérifier au
   moment de B1**, en relisant le semis des rôles de `900_seed.sql` : soit tous les rôles
   d'administration d'édition portent `programme.proposal.read_all`, soit la fonction filtre sur la
   mauvaise permission. Pas tranché ici — ce n'est pas le rôle d'une constitution.

7. **`backend/` N'EXISTE PAS ENCORE, ET `make check-back` LE SAIT.** La cible est inerte tant que le
   dossier est absent (« backend/ absent — rien à vérifier (prompt B1) »). La constitution décrit donc
   une arborescence qui n'a aucun fichier : c'est voulu, elle est écrite pour B1, mais aucune de ses
   portes techniques n'est réellement exercée avant.

---

## Ce qui a été vérifié

- **Le gabarit résolu** — `.specify/scripts/bash/resolve-template.sh constitution-template --json`
  rend bien la couche `core` (aucune surcharge de projet, aucune couche de préréglage) ; c'est cette
  structure qui a été remplie.
- **Aucun placeholder résiduel** — `grep "\[[A-Z_]\{3,\}\]"` sur le fichier final ne rend rien.
- **Chaque nom cité vient d'un fichier lu**, non de mémoire : `000_bootstrap.sql` (§ 7 contexte de
  requête, § 9 frontières), `010_platform.sql` (modules, audit, outbox, jobs, drapeaux),
  `030_identity.sql` (`has_permission`, `administered_events` et son commentaire). Les signatures et
  les valeurs de retour sont recopiées du SQL.
- **Les seize schémas** annoncés dans la section « portes de qualité » sont ceux qu'assère
  `assert-db` dans le `Makefile`, pas un compte approximatif.
- **Aucune contradiction avec `CLAUDE.md`** : la limite de 1000 lignes, l'exclusion de
  `docs/database/`, la séparation textes d'interface / données métier et le périmètre des six modules
  fermés sont repris à l'identique, et la constitution déclare explicitement céder devant `CLAUDE.md`
  et `docs/database/` en cas de divergence.

**Non vérifié, et qui ne peut pas l'être aujourd'hui** : rien de ce que la constitution impose au code
Rust n'a été exercé — il n'y a pas une ligne de Rust dans le dépôt. La première mise à l'épreuve est
B1.

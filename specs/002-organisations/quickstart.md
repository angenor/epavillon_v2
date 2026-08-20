# Démarrage rapide — Organisations (B2)

**Fonctionnalité** : Organisations (B2) · **Date** : 2026-08-20

Comment lancer, **éprouver à la main**, et vérifier. Ce qui suit se joue dans un navigateur et dans un terminal, jamais dans un test seul : B1 a trouvé six défauts en jouant les parcours et zéro en les relisant.

---

## Préalables

Ceux de B1, inchangés — `cp .env.example .env`, `make up`, les trois interfaces (Mailpit, Jaeger, documentation de l'API). Voir [`../001-socle-identite/quickstart.md`](../001-socle-identite/quickstart.md).

**Trois clés nouvelles**, toutes des réglages d'exploitation :

```
ORG_DUPLICATE_SCORE_THRESHOLD=60      # entrée dans la file des doublons (FR-060)
ORG_DUPLICATE_SCAN_BATCH=200          # taille d'une tranche de balayage (R11)
ORG_SCORECARD_REFRESH_WINDOW=5m       # coalescence du rafraîchissement (R13)
```

Comme en B1, **le démarrage échoue si l'une est mal écrite** : un seuil hors de 0-175, une tranche nulle, une durée illisible. Un réglage fautif arrête le service, jamais une requête.

---

## Lancer

```bash
cd backend && cargo run -p api        # l'API
cd backend && cargo run -p worker     # relais d'outbox, file de travaux, balayage des doublons
cd frontend && npm run dev            # le site — indispensable pour que les courriels partent
```

**Les trois sont nécessaires** pour éprouver l'invitation de bout en bout : sans le worker, aucun courriel n'est mis en route ; sans le site, aucun n'est envoyé.

---

## Éprouver les parcours à la main

### D'abord, de quoi parler — et le semis en donne plus qu'on ne croit

Les treize organisations du front vivent dans `frontend/app/mocks/`, pas en base. Mais `900_seed.sql` § 5 ne sème pas qu'un nom : il sème **l'IFDD avec ses cinq dénominations** — nom légal, sigle, faute d'orthographe connue, traduction anglaise, et deux anciens noms (« Institut de l'énergie et de l'environnement de la Francophonie », « IEPF ») — **et deux domaines vérifiés**, dont `ifdd.francophonie.org` en **rattachement automatique**.

Autrement dit, deux des trois parcours s'éprouvent **sans rien semer** :

- les cinq façons de désigner une organisation, sur la fiche de l'IFDD ;
- le rattachement automatique, avec un compte créé sur une adresse `@ifdd.francophonie.org` — et la même chose **sans** rattachement automatique sur `@francophonie.org`, le second domaine étant vérifié mais non marqué.

Deux gestes restent nécessaires :

1. créer deux comptes ordinaires par l'inscription de B1 — l'un sur un domaine d'organisation, l'autre sur une adresse quelconque ;
2. semer à la main la **paire de jumelles** : deux « Observatoire du Sahel », l'une vérifiée avec `osed-sahel.org` vérifié, l'autre créée sous son seul sigle avec le **même domaine non vérifié**. C'est la paire des données simulées, et c'est le cas qui a fait naître ce module.

### La recherche, et les deux lectures — c'est le cœur de l'écart n° 23

```bash
# Ce que doit voir une personne : ce qu'elle a tapé, et rien d'autre
curl -s -b cookies.txt "$API/api/organizations/similar?name=agence%20spatiale" | jq

# Ce que doit voir le back-office : tout ce qui pourrait être la même entité
curl -s -b admin.txt  "$API/api/admin/organizations/similar?name=agence%20spatiale" | jq
```

Connecté avec une adresse `@osed-sahel.org`, **les deux réponses doivent différer** : la première ne contient aucune fiche OSED, la seconde les contient. C'est SC-003 et SC-004, et c'est la vérification qui dit si l'écart n° 23 est réglé.

Puis les cinq façons de désigner une même organisation — sigle, début du nom complet, deux lettres, traduction, ancien nom — doivent ramener **la même fiche, une seule fois**. C'est la règle métier n° 1.

Et un contrôle qui ne coûte rien : `?name=a` rend une **liste vide**, pas une erreur et pas la table entière.

### Le rattachement, et ce que le domaine décide

Connecté avec l'adresse à domaine vérifié en rattachement automatique : le bandeau propose l'organisation, et rejoindre donne une adhésion **active** immédiatement. Connecté avec l'adresse quelconque : rejoindre la même organisation donne une adhésion **en attente**.

Le contrôle qui compte, et qui n'existe que parce que la base l'impose : **faire refuser cette demande par un référent, puis la refaire**. Elle doit être acceptée et **reprendre la même ligne** — une seconde ligne violerait l'unicité, et la personne refusée ne pourrait plus jamais redemander (écart n° 72). À vérifier en base :

```sql
SELECT count(*) FROM org.memberships WHERE organization_id = :org AND person_id = :personne;
-- doit valoir 1, jamais 2
```

### La création, et le doublon qu'on ne bloque pas

Créer « Observatoire du Sahel » dans le même pays qu'une fiche vivante → **`name_taken`, en 200**, portant la fiche en cause. Créer une fiche en ayant vu des fiches proches → elle est créée, en **`candidate`**, le créateur en est **référent**, et les identifiants montrés sont conservés.

### Les deux files, et le refus qui les sépare

Inviter une adresse inconnue depuis un compte référent : une personne est créée **sans compte et sans nom**, un courriel part, visible dans Mailpit. Deux contrôles :

- **tenter d'approuver cette invitation en tant que référent** → refus `ORG_MEMBERSHIP_IS_INVITATION`. C'est le refus qui empêche de faire entrer quelqu'un qui n'a rien accepté ;
- **suivre le lien reçu** → l'adhésion devient active, et le même lien rejoué rend « déjà utilisé ».

Puis le parcours que B1 rendait impossible et que R9 débloque : **la personne invitée s'inscrit avec cette même adresse**. Elle doit obtenir un compte et un lien de vérification — avant la correction, elle recevait « vous avez déjà un compte » et ne pouvait jamais se connecter.

### Le périmètre, et l'URL forgée

Sous un compte détaché sur une seule édition : la liste ne montre que les organisations ayant déposé ou tenu une activité **dans cette édition**, et la réponse dit qu'elle est restreinte. Demander la fiche d'une organisation hors périmètre, **en forgeant l'identifiant**, doit rendre un refus indiscernable de « inexistante ».

Sous un compte sans aucun droit d'administration : **un refus**, jamais une liste vide. C'est le cas que la constitution nomme et qu'un garde testant « pas global » aurait raté.

### La fusion, et le décompte qui doit tomber juste

Sur la paire OSED : ouvrir l'aperçu, **noter le décompte ligne à ligne**, inverser le sens et constater qu'il **change**, revenir, arbitrer les champs divergents, saisir le nom de la fiche absorbée — en minuscules et sans accents, la comparaison ne doit pas en tenir compte — et fusionner.

Puis les quatre contrôles qui font la valeur de l'opération :

```sql
-- 1. le décompte annoncé et le décompte réel sont le même (SC-010)
SELECT rows_reassigned FROM org.merge_log ORDER BY performed_at DESC LIMIT 1;

-- 2. UN SEUL événement de fusion (le piège n° 1, écart n° 76)
SELECT count(*) FROM platform.outbox_events
 WHERE event_type = 'org.organization.merged' AND aggregate_id = :cible;
-- doit valoir 1, jamais 2

-- 3. la fiche absorbée survit et pointe vers la vivante
SELECT status, merged_into_id FROM org.organizations WHERE id = :source;

-- 4. l'ancien nom trouve toujours la bonne fiche
SELECT * FROM org.find_similar_organizations('observatoire du sahel');
```

Et les deux refus qu'il faut avoir vus au moins une fois : **arbitrer l'adresse d'URL vers la source** → 422 nommant le champ (R6) ; **viser une fiche déjà fusionnée** → le message du trigger, **mot pour mot**, « Cibler la fiche finale ».

### Ce que le worker tient à jour

Vérifier un domaine, puis recharger la liste : le **score de confiance doit avoir bougé** sans attendre le lendemain. Approuver dix adhésions coup sur coup, puis compter :

```sql
SELECT task, count(*) FROM platform.jobs
 WHERE task = 'org.trust_score.recompute' AND idempotency_key LIKE '%'||:org GROUP BY task;
-- un seul travail, pas dix
```

Enfin, arrêter le worker, le relancer, et constater que le balayage **reprend sa chaîne** sans la doubler — le motif de la purge récurrente de B1.

---

## Les tests

```bash
cd backend && cargo test --workspace --all-features
```

### Les quatre obligations de la constitution, et le test qui les tient

| Obligation (principe X) | Test |
|---|---|
| Chemin nominal de chaque route | `recherche_multi_signaux`, `rattachement_et_creation`, `adhesions_deux_files`, `back_office_liste_et_fiche`, `fusion_complete` |
| Refus par périmètre, **URL forgée comprise** | `perimetre_organisation_url_forgee` |
| Traduction d'un invariant de la base | `fusion_cible_deja_fusionnee` — le message du trigger, mot pour mot — et `domaine_deja_verifie_ailleurs`, qui exige que le refus **nomme** la fiche |
| Écriture des événements attendus | `outbox_une_seule_fusion` — il **compte**, il ne vérifie pas la présence |

### Les vérifications propres à ce module

| Test | Ce qu'il tient |
|---|---|
| `deux_lectures_de_recherche` | SC-003 et SC-004 : sur **la même requête**, la lecture d'utilisateur et celle de revue rendent des résultats différents, et c'est l'attendu |
| `recherche_150ms` | SC-002. Sème 5 000 organisations à distribution réaliste (R3), joue cent recherches, exige le 95ᵉ centile sous 150 ms, et **rend le plan d'exécution dans son message d'échec** |
| `adhesion_revoquee_puis_redemandee` | SC-008 : jamais plus d'une ligne par (organisation, personne) |
| `decompte_de_fusion_exact` | SC-010 : le décompte de l'aperçu et celui du journal, comparés ligne de registre par ligne de registre |
| `fusion_arbitrage_apres_lappel` | R5 : la fusion avec arbitrage du **nom légal** aboutit. Elle échouerait sur une violation d'unicité si l'ordre était inversé |
| `fusion_arbitrage_annule_tout` | SC-012 : un arbitrage qui échoue ne laisse ni fiche absorbée, ni rattachement déplacé, ni ligne au journal |
| `balayage_ne_ressuscite_pas` | SC-013 : dix passages ne créent aucune paire en double et ne ramènent aucune paire écartée |
| `score_de_confiance_coalesce` | SC-014 : cent approbations, un recalcul |
| `perimetre_vide_refuse` | Une personne sans droit d'administration reçoit **un refus**, jamais une liste vide |
| `permission_ordinaire_ne_suffit_pas` | Écart n° 73 : détenir la permission de consultation **sans** périmètre n'ouvre pas le back-office |
| `identity_apres_deplacement_des_jetons` | R8 : les tests de B1 sur la vérification d'adresse, la réinitialisation et le rejeu **restent verts**, sans avoir été réécrits |
| `invitee_peut_creer_son_compte` | R9 : une personne créée par invitation obtient un compte et son lien de vérification |

---

## Les portes à passer avant de livrer

```bash
cd backend && cargo fmt --all                                   # AVANT tout, c'est un portail
cd backend && cargo sqlx prepare --workspace -- --all-targets --all-features
make check                                                      # db, front, back
```

Trois vérifications ne sont dans aucun `Makefile` et doivent être faites à la main :

```bash
cargo tree -p org | grep -c "crates/modules"       # doit valoir 1 : la ligne racine, et rien d'autre
find backend -name '*.rs' | xargs wc -l | sort -rn | head -3    # aucun fichier au-dessus de 1000
curl -s $API/api/docs | jq '.paths | keys | length'             # les 21 chemins, et les 11 codes ajoutés
```

Et un contrôle que seul ce module impose : **le SQL composé dynamiquement passe l'analyse statique**. `cargo clippy --all-targets --all-features -- -D warnings` doit voir `repo/merge_counts.rs` — c'est précisément pour ce genre de fichier que les deux options ne sont pas décoratives, et B1 l'avait appris en découvrant que son propre fichier dynamique n'était pas analysé.

---

## Une fois que tout passe

Mettre la progression à jour — journal du jour, `progression/ecrans/b2-organisations.md`, décisions du jour, et la ligne de suivi dans `docs/PROGRESSION.md`. Une session qui ne le fait pas oblige la suivante à tout redécouvrir.

# Contrat — Événements de domaine et travaux différés

**Fonctionnalité** : Propositions (B4) · **Date** : 2026-08-20

---

## Le piège du module, énoncé une fois

**Le déclencheur d'état émet déjà.** `programme.tg_guard_proposal_status()` appelle `platform.emit_event()` à chaque transition acceptée, dans la transaction, avec le numéro de dossier, l'édition, l'organisation, les deux états et le motif. C'est l'**inverse de B3** (écart n° 87), et le retour du piège de B1 et de B2.

**Conséquence, et elle est absolue : le service n'émet aucun événement de changement d'état.** Émettre à son tour produirait deux événements par transition — donc deux courriels, deux notifications, deux incréments —, et le doublon ne se verrait qu'en production.

---

## Les huit événements émis par la base

Un par état d'arrivée, tous de la forme `programme.proposal.<état>` :

`programme.proposal.draft` (à la création) · `.submitted` · `.under_review` · `.changes_requested` · `.accepted` · `.rejected` · `.withdrawn` · `.cancelled`

**Charge utile, posée par le déclencheur** : numéro de dossier, édition, organisation, état de départ, état d'arrivée, motif.

**Ce que B4 en fait** : rien, sinon les **vérifier** — le test du principe IV lit `platform.outbox_events` après chaque transition et constate **une** ligne, pas deux.

**Ce que B6 en fera** : l'avis de dépôt à l'organisation, l'avis de décision, l'avis de correction demandée.

---

## Les trois événements émis par le service

Ils décrivent des faits que la base n'annonce pas, et chacun a un consommateur identifié.

| Type | Émis quand | Charge utile | Qui le consommera |
|---|---|---|---|
| `programme.coorganization.requested` | une organisation est associée au dossier avec un rôle autre que porteur | dossier, numéro, édition, organisation invitée, rôle, organisation porteuse | **B6** — la demande de confirmation. Le front annonce déjà « sera invitée à confirmer sa participation » |
| `programme.comment.shared` | un message est écrit en visibilité partagée avec le déposant | dossier, numéro, message, auteur, organisation porteuse, **et si c'est une demande de correction** | **B6** — l'avis au déposant. Sans lui, l'organisation découvre une correction en revenant sur son espace |
| `programme.review.assigned` | un dossier est confié à un membre du comité | dossier, numéro, édition, membre, échéance | **B6** — le rappel de revue. L'action groupée en confie douze d'un coup : **un événement par dossier**, pas un pour le lot |

**Le format respecte `module.ressource.action`**, contrainte de la base.

**Pourquoi un événement par dossier dans une action groupée** : un consommateur qui reçoit un lot doit le déplier lui-même, et son échec porte alors sur douze effets au lieu d'un. La garde de rejeu est par événement.

---

## Ce qui n'émet rien, et pourquoi

| Fait | Pourquoi aucun événement |
|---|---|
| Enregistrement d'un brouillon | aucun autre module n'en dépend, et l'écriture est fréquente — une frappe toutes les deux secondes |
| Modification d'un dossier retenu | **surtout pas** : c'est précisément ce qui ne doit rien propager vers la séance (FR-091) |
| Notation, note personnelle, points forts et faibles | internes au comité |
| Message en visibilité « comité » ou « privée » | il ne sort pas du comité, par définition |
| Accusé de lecture | un compteur |
| Résolution d'une demande de correction | l'état visible est le compteur de demandes ouvertes, relu à chaque affichage |
| Déport d'un membre du comité | l'avancement du comité est relu à chaque ouverture de la fiche ; rien n'en dépend hors du module |
| Rattachement ou détachement d'une pièce | le cycle de vie de l'objet appartient à B6, qui le connaît par ailleurs |
| Déduction des transitions v1 | une opération d'administration ponctuelle. **Émettre huit mille événements de dossiers déjà décidés déclencherait autant de courriels** — le pire effet possible d'une reprise |

Cette dernière ligne mérite un test : la déduction écrit dans le journal **sans passer par la mise à jour de l'état**, donc sans réveiller le déclencheur. C'est ce qui la rend sûre, et c'est vérifié.

---

## Travaux différés

**Aucun** (R20).

Rien dans ce module n'a d'effet à échéance : les rappels de revue et les avis de dépôt appartiennent à B6 et se déclencheront sur les trois événements ci-dessus ; la clôture automatique d'un appel échu appartient à B3 et y est livrée.

La déduction des transitions v1 est une **opération synchrone** exigeant la portée globale, et non un travail : elle est ponctuelle, et son résultat doit être lu par celui qui la lance — « 3 812 dossiers, 11 436 lignes semées », pas un identifiant de tâche.

**Le worker n'est donc pas modifié.** C'est le premier module du jalon dans ce cas, et c'est un fait à vérifier plutôt qu'un oubli à constater : le test de montage compte les gestionnaires enregistrés.

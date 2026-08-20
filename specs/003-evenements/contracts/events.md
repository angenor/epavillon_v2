# Contrat — Événements de domaine et travaux différés

**Fonctionnalité** : Événements (B3) · **Date** : 2026-08-20

Principe IV : **les effets de bord inter-modules passent par `platform.emit_event()`, appelée dans la même transaction que le changement d'état.** On appelle la fonction — on n'insère jamais à la main dans la file de sortie. Le type respecte `module.ressource.action` (`ck_outbox_event_type_format`).

---

## Le constat qui gouverne ce fichier

**Aucun déclencheur de `060_events.sql` n'émet d'événement de domaine.** Le fichier ne porte que deux déclencheurs d'audit — sur l'édition et sur l'appel — et cinq horodatages. Aucune fonction du module n'est en `SECURITY DEFINER`.

C'est **l'inverse** du piège rencontré deux fois : `identity.anonymize_person()` émettait déjà `identity.person.anonymized`, et `org.merge_organizations()` émet déjà son événement **et** marque la paire de la file. Dans les deux cas, un service qui aurait ajouté la ligne aurait produit un doublon.

Ici, la conséquence est symétrique et il faut la dire à l'endroit où l'on serait tenté de croire l'inverse : **le service émet tout lui-même, et rien n'émet à sa place.** Un changement d'état non annoncé par le code n'est annoncé par personne.

---

## Les six événements émis

| Type | Émis quand | Charge utile | Qui le consomme |
|---|---|---|---|
| `event.edition.created` | une édition est créée | identifiant, adresse d'URL, série, millésime, pavillon tenu | personne aujourd'hui. Il porte la trace d'une entité dont B4, B5 et B6 dépendent tous |
| `event.edition.updated` | une édition est modifiée | identifiant, et **ce qui a changé parmi** : période, statut, pavillon tenu, fuseau | personne aujourd'hui. La période et le fuseau intéresseront B5, dont les séances s'y rattachent |
| `event.call.opened` | un appel passe en `open` | identifiant de l'appel, édition, échéance effective | B6 (rappels) et, à terme, la vitrine. **C'est l'annonce qui ouvre le jalon** |
| `event.call.closed` | un appel passe en `closed`, à la main **ou par la clôture automatique** | identifiant, édition, échéance qui a été appliquée | B4, qui cesse d'accepter des dépôts ; B6, qui annonce |
| `event.call.deadline_extended` | `extended_until` est posée ou déplacée | identifiant, échéance initiale, nouvelle échéance | B6. **L'échéance initiale voyage avec** : c'est ce qui a été annoncé aux organisations, et un rappel qui l'ignore dit une contre-vérité |
| `event.programme.published` | la programmation d'une édition est publiée | édition, instant de publication, **le prédicat des séances à publier** et leur décompte | **B5**, qui pose la date de publication sur les séances désignées, avec garde de rejeu (R10) |

Les charges utiles vivent dans `backend/crates/contracts/event.rs` — la seule chose que les modules échangent.

---

## Ce qui n'émet RIEN, et pourquoi

**C'est une soustraction délibérée, pas un oubli.** Chaque ligne a été pesée.

| Écriture | Pourquoi aucun événement |
|---|---|
| journées du calendrier — génération, retrait, habillage | Aucun consommateur. Le détachement d'une séance est un effet de la **base** (`ON DELETE SET NULL`), pas un changement d'état à annoncer. B5 lit le rattachement, il n'en est pas notifié |
| fils de programmation — création, modification, suppression | Aucun consommateur. Leur composition, elle, vit dans `programme` et sera annoncée par B5 s'il y a lieu |
| lieux et salles | Aucun consommateur. Le caractère virtuel d'une salle est **dénormalisé par un déclencheur de `programme`**, pas par une annonce |
| canaux de diffusion | Aucun consommateur. L'affectation d'un canal à une séance est faite par un déclencheur de `programme` |
| comité de sélection | Aucun consommateur : **B4 lit `event.call_reviewers` directement**, ce qui est une lecture hors schéma sur sa propre question. Un événement ferait un second chemin pour la même information |

**La règle qui a servi** : on émet quand un **autre module** aurait à réagir, ou quand l'annonce a une valeur d'archive pour une entité pivot. Émettre « pour plus tard » remplit la file de sortie de messages que personne ne lit et qu'il faudra un jour retirer.

---

## Le travail différé

Un seul, et il se replanifie lui-même — le patron de la purge des jetons de B1, où le démarrage du worker ne fait que **réarmer** la chaîne au cas où sa dernière occurrence serait morte avant d'avoir posé la suivante.

| Clé | Ce qu'il fait | Cadence | Clé d'unicité |
|---|---|---|---|
| `event.call.autoclose` | Passe en `closed` tout appel `open` dont l'échéance effective — prolongation comprise, par `event.effective_deadline()` — est passée, et émet `event.call.closed` pour chacun | horaire, replanifié par lui-même | l'occurrence, pas l'appel : le travail balaie, il ne vise personne |

**Pourquoi ce travail est nécessaire.** `event.is_call_open()` protège la **recevabilité** — elle vérifie statut *et* fenêtre. Mais le **statut affiché** reste « ouvert » après l'échéance, sur la page publique comme dans la liste du back-office, et c'est ce statut que lit une organisation qui se demande si elle peut encore déposer.

**Il passe par la file du modèle** (`platform.jobs`, `claim_jobs()`), comme le prévoit la constitution pour un travail qui n'annonce pas lui-même un changement d'état — c'est bien l'écriture qu'il déclenche qui l'annonce.

**Ce qui n'est PAS livré ici** : le rappel d'échéance aux organisations. Les règles de rappel (`engagement.reminder_rules`) et les modèles de message multilingues appartiennent à B6 ; les recopier ici produirait un second dispositif de rappel, désynchronisé du premier au premier changement.

---

## Consommation

**Ce module ne consomme aucun événement dans ce jalon.**

Il en produit un que B5 devra consommer — `event.programme.published` — et le contrat de ce consommateur est fixé ici pour que B5 n'ait rien à deviner :

- garde de rejeu obligatoire par `platform.inbox_events (consumer, event_id)` ;
- il publie **exactement** le prédicat porté par la charge utile — séances de l'édition, statut `planned` ou `scheduled`, non encore publiées — et pas un autre ;
- il n'écrit **pas** `event.events.programme_published_at`, déjà posée par l'émetteur ;
- une seconde livraison du même événement ne publie rien de plus.

C'est la contrepartie de la frontière : l'émetteur annonce un effet précis, le consommateur ne l'interprète pas.

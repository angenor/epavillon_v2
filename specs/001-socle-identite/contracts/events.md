# Contrat — Événements de domaine et travaux différés

**Fonctionnalité** : Socle technique et Identité (B1) · **Date** : 2026-08-20

Deux mécanismes, deux usages, à ne pas confondre :

- **Un événement de domaine** annonce qu'un état a changé. Il est écrit dans la **même transaction** que le changement, par `platform.emit_event()`. Il s'adresse aux **autres modules**, présents ou futurs, et **ne porte jamais de secret**.
- **Un travail différé** exécute quelque chose plus tard. Il vit dans `platform.jobs`, avec sa clé d'unicité, sa reprise d'essai et sa file morte. Il s'adresse au **worker**.

---

## 1. Les événements émis par `identity`

La forme du type est imposée par la base : `^[a-z_]+\.[a-z_]+\.[a-z_]+$`, **trois segments exactement** (`ck_outbox_event_type_format`). Le service ne revalide pas la forme, il traduit l'échec.

| Type | Émis quand | Charge utile |
|---|---|---|
| `identity.person.registered` | une personne est créée par inscription | identifiant, langue préférée, pays |
| `identity.person.email_verified` | une adresse vient d'être vérifiée | identifiant, date de vérification |
| `identity.person.status_changed` | suspension, blocage, réactivation | identifiant, ancien statut, nouveau statut, terme, motif |
| `identity.person.anonymized` | effacement RGPD | motif — **émis par la base**, voir ci-dessous |
| `identity.account.password_changed` | mot de passe modifié, y compris par réinitialisation | identifiant de personne, canal (`reset` ou `profile`) |
| `identity.account.locked` | le seuil d'échecs est atteint | identifiant de personne, fin du verrou |
| `identity.role.granted` | un rôle est attribué | personne, rôle, type de portée, portée, auteur |
| `identity.role.revoked` | un rôle est retiré | personne, rôle, portée, auteur, motif |
| `identity.privacy_request.received` | une demande RGPD est déposée | identifiant, type, échéance |

### `identity.person.anonymized` est émis par la base — ne pas l'émettre deux fois

`identity.anonymize_person()` appelle elle-même `platform.emit_event()` avant de rendre la main. Le service qui l'invoque **n'émet rien de plus** : le faire produirait deux événements pour un seul effacement, et tout consommateur idempotent traiterait le premier puis ignorerait… le mauvais.

C'est le cas le plus facile à manquer de tout le module, parce qu'il ne casse rien : les deux lignes s'écrivent sans erreur.

### Ce que les charges utiles ne portent jamais

Aucun mot de passe, aucune empreinte, aucun jeton en clair, aucun jeton haché. `platform.outbox_events` est une table durable, indexée par agrégat, faite pour être relue et rejouée, et destinée à devenir un bus : ce qu'on y dépose est là pour longtemps et pour beaucoup de monde.

Aucune adresse de courriel non plus, sauf quand elle **est** le sujet de l'événement — et jamais dans un événement dont le seul but serait de la transmettre.

### `metadata` et `correlation_id` se remplissent seuls

`platform.emit_event()` agrège `platform.current_actor_id()` dans `metadata` et `platform.current_request_id()` dans `correlation_id`. Une transaction qui n'a pas posé son contexte émet donc un événement **anonyme, sans erreur** : rien n'échoue, la trace est simplement perdue. C'est pourquoi le noyau n'offre qu'une seule façon **recommandée** d'ouvrir une transaction en écriture, et qu'elle pose le contexte elle-même. Rien dans les types ne l'impose — le pool reste accessible, l'écoute `LISTEN/NOTIFY` et le harnais de test en exigent un vrai —, mais **aucune écriture du jalon ne passe à côté**, pas même les compteurs du chemin de la connexion, où la contrainte de temps de SC-001 aurait pu servir de prétexte : l'écriture y est lancée avant l'attente du hachage, et sa transaction se replie derrière lui.

---

## 2. Les consommateurs

Chaque consommateur inscrit `(consommateur, événement)` dans `platform.inbox_events` avant de produire son effet. Un conflit sur cette clé signifie « déjà traité » : on passe au suivant sans rien faire.

| Consommateur | Écoute | Effet |
|---|---|---|
| `telemetry` | tous | **une trace par événement de domaine.** Le compteur annoncé n'est pas livré : monter un exportateur de métriques OTLP, son intervalle et son arrêt propre pour un unique compteur coûte plus qu'il ne rapporte, alors que `count by event_type` sur les traces donne déjà le chiffre |

**Et c'est tout, dans ce jalon.** `identity` est le seul module qui existe : il n'y a personne à qui annoncer quoi que ce soit. Le premier consommateur métier arrive avec **B2**.

Ce n'est pas une raison pour repousser le mécanisme. Le consommateur de télémétrie est utile pour de vrai — il rend visible dans Jaeger ce qui se passe dans l'outbox —, et il **exerce la garde d'idempotence de bout en bout** : arrêter le worker, le relancer sur mille événements déjà traités, et vérifier qu'aucun n'est rejoué. Sans lui, FR-010 ne serait éprouvable qu'au module suivant.

---

## 3. Les travaux différés

| Tâche | Mise en file par | Clé d'unicité | Effet |
|---|---|---|---|
| `identity.send_verification_email` | inscription, renvoi de lien | identifiant du jeton | remet au serveur du site le lien de vérification |
| `identity.send_password_reset_email` | demande de réinitialisation | identifiant du jeton | remet le lien de réinitialisation |
| `identity.send_existing_account_notice` | inscription sur une adresse déjà connue | personne + date du jour | remet le rappel « vous avez déjà un compte » |
| `identity.purge_expired_tokens` | récurrente, quotidienne | date du jour | supprime les jetons périmés et consommés |

La forme du nom suit celle des événements — `module.action` — parce que `platform.jobs.task` porte l'exemple `engagement.send_reminder` dans son propre commentaire.

### Pourquoi le jeton en clair passe par la file, et pas par l'outbox

Le courriel doit contenir le jeton en clair ; la base n'en garde que l'empreinte. Le clair doit donc voyager quelque part.

- **Pas par l'outbox** : table durable, interrogeable, rejouable. Un secret réutilisable y serait un défaut permanent.
- **Par `platform.jobs`** : la constitution y range explicitement « un travail différé qui n'annonce pas un changement d'état — envoi de rappel… ». Le travail est mis en file **dans la même transaction** que la création du jeton.
- **La charge utile est vidée dès l'envoi réussi** : le travail garde sa trace — un courriel est parti, quand, après combien d'essais — sans garder son contenu.

L'événement de domaine correspondant est émis **en plus**, et sans secret. Les deux ne servent pas la même chose : l'un fait agir le worker, l'autre informera les modules à venir.

### Deux trous connus de la file, à refermer avant la phase des courriels

**Un travail mort garde sa charge utile.** `succeed()` la vide ; `platform.fail_job()` ne la vide jamais. Un travail passé en file morte conserverait donc son jeton en clair indéfiniment. Rien ne fuit aujourd'hui — `identity` ne met encore aucun travail en file — mais **c'est à trancher avant le premier envoi** : soit vider la charge utile au passage en `dead`, soit la restreindre aux tâches qui portent un secret. La perte n'est pas gratuite : pour `media`, `live` et `analytics`, la charge utile d'un travail mort est la seule matière de diagnostic. `last_error` mérite le même examen — le message d'échec d'un relais HTTP peut recopier l'adresse du destinataire.

**Un travail réservé par un worker tué revient à la file au bout de trente minutes.** Le modèle sait voir ce cas — `ix_jobs_stuck`, alerte `travaux_bloques` de `analytics.v_operational_health` — mais ne le répare pas : `claim_jobs()` ne prend que les `queued`. La reprise est donc écrite dans le worker, avec un bail délibérément plus long que le seuil d'alerte de quinze minutes : celui-ci sert à VOIR une panne, celui-là à reprendre un travail, et les confondre ferait tourner deux fois un rafraîchissement analytique un peu long. **Les essais déjà comptés ne sont pas rendus** : un travail repris trois fois meurt avant ses cinq essais annoncés. Et un travail dont la RÉUSSITE n'a pas pu être écrite sera réexécuté — la file est « au moins une fois », jamais « exactement une fois ».

### La reprise d'essai n'est pas réécrite

`platform.fail_job()` porte déjà le délai croissant plafonné à une heure et le passage en file morte au-delà de `max_attempts`. Le worker l'appelle, il ne la reproduit pas. Principe VIII.

---

## 4. La remise du courriel au serveur du site

**Contrainte d'hébergement (20/08)** : l'API et le site vivent sur deux serveurs, et **seul celui du site a le droit d'émettre du courriel**. Le worker ne parle donc jamais SMTP.

```
POST {MAIL_RELAY_URL}
X-Mail-Relay-Token: <secret partagé>

{
  "message_id": "<identifiant du travail>",
  "to":         "awa.diallo@example.org",
  "locale":     "fr",
  "subject":    "Vérifiez votre adresse — ePavillon",
  "text":       "…",
  "html":       "…"
}
```

**Le message arrive composé.** Sujet et corps sont écrits par l'API, dans la langue de `people.preferred_locale`. Le site reçoit un texte, pas un gabarit : il ouvre la connexion SMTP et envoie. Deux raisons — le texte appartient au module qui déclenche l'envoi, et en B6 la composition passera aux modèles administrables de `engagement.message_templates`. Si le site composait, il faudrait alors défaire son travail.

**Ce que le site fait de plus** : il retient les `message_id` déjà envoyés pendant quelques minutes. Une reprise d'essai après un délai d'attente dépassé — le courriel est parti, la réponse s'est perdue — ne produit alors pas un second envoi.

**Ce que le site ne fait pas** : il ne connaît aucun cas particulier, ne compose rien, ne décide de rien. Il sert **tous les courriels de la plateforme**, pas seulement ceux de l'identité, et il est écrit une fois pour toutes.

**Il est fait pour disparaître.** Le jour où le serveur de l'API obtient le droit d'émettre, l'envoi se réécrit en Rust. C'est pourquoi le noyau expose un **contrat** d'envoi et non un client HTTP : la bascule coûte une implémentation et une clé de configuration, et aucun module ne s'en aperçoit.

### Sécurité de la route privée

| Point | Traitement |
|---|---|
| Authentification | secret partagé en en-tête, comparé à temps constant |
| Requête sans secret valide | **404**, jamais 401 — une route privée ne confirme pas son existence |
| Transport | HTTPS obligatoire en production ; le jeton en clair y circule |
| Exposition | la route n'est ni documentée dans l'OpenAPI, ni référencée par une page |

### Ce qui se passe quand le site ne répond pas

Rien de particulier, et c'est le but : le travail échoue, `platform.fail_job()` le replanifie, et il passe en file morte au bout de cinq essais. Les travaux en échec remontent dans `analytics.v_operational_health`, que sert la route de santé. La panne se voit donc là où l'on regarde déjà.

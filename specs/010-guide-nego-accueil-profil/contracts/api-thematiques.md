# Contrat — les thématiques suivies

**Crate** : `negotiation` · **Décisions** : [research.md § R2, R3, R4, R16](../research.md)

> Le contrat servi est **engendré** par `make openapi`. Ce document dit ce qu'il doit porter et
> pourquoi ; il ne le remplace pas. Les corps sont **nommés**, pas décrits : leur définition vit dans
> `frontend/app/types/`, et `make check-api-contract` vérifie le lien.

---

## 1. Lire le vocabulaire — route existante, rien à écrire

```
GET /api/reference/taxonomies/negotiation_theme/terms
```

Déjà servie par `backend/crates/api/src/routes/reference.rs` (`operation_id = "reference_termes"`),
sans session, termes actifs triés par `sort_order` puis `code`, libellés multilingues rendus entiers.
Semer le vocabulaire suffit à la servir.

Côté client : `api.reference.terms('negotiation_theme')`, déjà montée dans `useApi()`. **Aucune méthode
nouvelle, aucune route nouvelle.** Une seconde fabrique qui rendrait les mêmes termes serait un contrat
de trop.

**C'est la seule source des libellés.** Les routes ci-dessous n'en rendent aucun — voir § 2.

---

## 2. `GET /negotiation/me/themes` — ce que je suis

**Session requise.** Rend les thématiques suivies par la personne connectée.

| Élément | Valeur |
|---|---|
| `operation_id` | `negotiation_mes_thematiques` |
| Forme rendue | `MyThemes` — les **codes** suivis et, pour chacun, depuis quand |
| En-tête de réponse | `ETag` — voir le calcul ci-dessous |
| `If-None-Match` correspondant | `304`, sans corps |
| Sans thématique suivie | `200` avec une liste vide — **jamais un 404** |

### Le corps ne porte pas de libellés, et c'est ce qui rend l'empreinte sûre

L'empreinte se calcule sur **les codes suivis, triés** — jamais sur le corps rendu.

La raison est concrète : deux appareils de la même personne, l'un en français, l'autre en anglais,
auraient deux empreintes pour **un même état**, et l'empreinte cesserait de dire ce qu'elle doit dire.
Mais l'inverse est un piège symétrique : si le corps portait des libellés et que l'empreinte les
ignorait, un appareil qui change de langue recevrait un `304` et garderait ses libellés dans l'ancienne
langue.

Les deux pièges se ferment d'un seul geste : **le corps ne porte que des codes**. Le client a déjà le
vocabulaire par la route du § 1 — il le lit de toute façon pour afficher la liste —, et une seule source
des libellés vaut mieux que deux qui peuvent diverger.

*Conséquence côté écran* : le profil, qui affiche « Adaptation, Genre » en valeur de ligne, croise les
codes avec le vocabulaire gardé. Si le vocabulaire n'a jamais été lu — profil ouvert hors connexion sans
être passé par l'écran des thématiques —, la ligne dit **le nombre** (« 2 thématiques suivies ») et
jamais une liste de codes bruts.

### Le calcul

`kernel::crypto::token_hash` des codes triés par ordre alphabétique et joints, 16 octets en hexadécimal,
entre guillemets. Même mécanique qu'en 0b, autre entrée : **l'état, pas sa représentation**.

---

## 3. `PUT /negotiation/me/themes` — remplacer la liste

**Session requise. Remplacement en bloc, jamais un ajout ni un retrait unitaire.**

| Élément | Valeur |
|---|---|
| `operation_id` | `negotiation_suivre_des_thematiques` |
| Forme reçue → rendue | `ThemesPayload` → `MyThemes` |
| Corps reçu | La liste **entière** des codes suivis |
| En-tête accepté | `If-Match` — l'empreinte de l'état sur lequel le choix a été pris |
| Réponse | `200` avec le nouvel état, et sa nouvelle empreinte |

### `If-Match` : ce qui empêche un choix en retard d'écraser un choix récent

Le scénario que cela ferme, et qui n'a rien d'improbable en COP :

> Téléphone sans réseau à 10:00 — la personne choisit *adaptation* et *genre*, l'intention part en file.
> Tablette en ligne à 11:00 — elle choisit *finance*. Téléphone de retour à 12:00 — sans garde,
> l'intention de 10:00 part et **efface le choix de 11:00**, sans que personne ne le voie.

| Cas | Comportement |
|---|---|
| `If-Match` absent | Accepté. L'écran en ligne vient de lire l'état : il n'a rien à opposer |
| `If-Match` égal à l'empreinte courante | Accepté |
| `If-Match` différent, ou illisible | **`412`**, aucune écriture. Le corps porte le code stable et un message français |

**La comparaison porte sur les 32 caractères hexadécimaux** que l'API a émis, `W/` et suffixe de
relais retirés — Apache ajoute « -br » ou « -gzip » à l'`ETag` quand il compresse. Une seule fonction
compare, pour `If-Match` comme pour `If-None-Match` : `negotiation::domain::empreinte::correspond`.
Les réponses qui portent une empreinte sont `Cache-Control: private, no-cache`.

**Ce que fait l'application sur `412`** : elle **abandonne** l'intention — elle ne la rejoue pas, ne
tente pas de fusionner les deux listes —, relit l'état vrai, l'affiche, et le dit : « Vos thématiques
ont changé sur un autre appareil ». Fusionner serait pire que perdre : personne n'aurait choisi la liste
obtenue.

**Cette règle vaut pour toute la file.** Le parcours « Ma première COP » (étape 2) et les signalements
(3b) emploieront la même mécanique : une intention porte l'empreinte de l'état sur lequel elle a été
prise, et l'API refuse ce qui arrive trop tard.

### Comportement d'écriture

1. La transaction pose le contexte d'écriture (`app.actor_id`, `app.request_id`) avant la première
   écriture, par `Db::write(&ctx)` — sans quoi l'audit de la table serait anonyme.
2. L'empreinte reçue en `If-Match` est comparée **dans la transaction**, après verrouillage des suivis
   de la personne : la comparer avant ouvrirait la fenêtre qu'elle est censée fermer.
3. Les suivis vivants absents de la liste reçue sont **fermés** (`left_at = now()`), jamais supprimés.
4. Les codes reçus sans suivi vivant en ouvrent un.
5. Les codes reçus déjà suivis ne sont pas touchés — leur `followed_at` ne bouge pas, et l'audit ne
   s'encombre pas d'une écriture sans changement.

**Idempotence.** Rejouer le même corps donne le même état et n'écrit rien de plus. C'est ce qui permet à
une intention de repartir sans crainte quand le réseau revient ; `If-Match` est ce qui l'empêche de
repartir quand elle ne le devrait plus. Les deux sont nécessaires : l'idempotence protège du rejeu, pas
de l'ancienneté.

### Refus

| Cas | Réponse |
|---|---|
| Liste vide | `400`, code stable, message français nommant la règle : au moins une thématique |
| Code inconnu, ou d'un autre vocabulaire | `400`, et le message **nomme le code refusé** — patron de `programme/src/repo/themes.rs`, qui ne dit jamais « une thématique est inconnue » |
| Terme désactivé | Accepté s'il est déjà suivi, refusé s'il est nouveau : on ne retire pas à quelqu'un ce qu'il suivait, on ne laisse pas en choisir un qui n'est plus proposé |
| Doublons dans la liste reçue | Acceptés et réduits — le client n'a pas à s'en soucier |
| `If-Match` périmé | `412`, aucune écriture |
| Sans session | `401` |

**Codes d'erreur.** Le catalogue de `kernel/src/error.rs` **s'étend, il ne se double pas** : trois codes
nouveaux — liste vide, thématique inconnue, état périmé. Ils remontent seuls dans
`frontend/app/types/api.ts` par le schéma du catalogue.

---

## 4. Ce que les tests doivent prouver

Sur base réelle, dans le crate `negotiation` :

1. **Chemin nominal** — un `PUT` de deux codes ouvre deux suivis ; le `GET` les rend, avec son empreinte.
2. **Le `412` sur état changé** — lire l'empreinte, modifier l'état par une autre voie, rejouer le `PUT`
   avec l'ancienne empreinte : `412`, et **la base est inchangée**.
3. **Le rejeu identique** — deux `PUT` du même corps : même état, et **aucune ligne d'audit de plus**.
4. **Sans `If-Match`** — accepté, pour ne pas casser l'écran en ligne.
5. **Le vocabulaire gardé par la base** — un `INSERT` direct d'un terme d'`activity_theme` est refusé par
   `tg_theme_subscriptions_check_theme`, et l'API traduit l'erreur en message qui nomme le code.
6. **La fermeture, pas la suppression** — retirer une thématique pose `left_at` ; la ligne reste.
7. **L'auteur de l'écriture** — `platform.audit_log` porte un `actor_id` à chaque ligne.
8. **Le refus sans session** — `401`, y compris sur une URL forgée.

---

## 5. Ce que cette étape n'ajoute pas

- **Aucune route `/admin`** : rien ne s'administre ici ([research.md § R14](../research.md)).
- **Aucune route de profil** : le nom, le pays et la langue se lisent déjà par la session
  (`GET /auth/me`), et « Mon accès » par `GET /negotiation/me/access`, livrée en 0b.
- **Aucune route d'accord** : aucun consentement n'est offert à l'écran (écart 40).

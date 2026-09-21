# Contrat — la session dit d'où elle vient

> Modifie **deux routes existantes** d'`identity`, sans en changer le comportement pour le site.
> Le contrat réel est engendré par `make openapi` ; ce fichier dit ce qu'il doit porter.

## Ce qui s'ajoute au corps

`POST /api/auth/login` et `POST /api/auth/register` acceptent un objet **facultatif** :

```jsonc
{
  // … champs existants, inchangés
  "client": {
    "kind": "app",                    // "web" (défaut si l'objet est absent) | "app"
    "device_id": "9f2c…",             // opaque, engendré par le client, ≤ 128 caractères
    "label": "Android · Chrome",      // ≤ 80 caractères, montré à la personne
    "platform": "android"             // "android" | "ios" | "other"
  }
}
```

**Règles**

- L'objet absent vaut `{"kind": "web"}` : **le site n'est pas touché**, et aucun de ses appels ne
  change.
- Un `kind` inconnu est une erreur de validation qui désigne le champ ; il n'est jamais « corrigé »
  en silence.
- `device_id`, `label` et `platform` sont **déclaratifs**. Ils n'accordent aucun droit et ne sont
  jamais lus pour autoriser quoi que ce soit — au même titre que `user_agent`.
- `POST /api/auth/refresh` **ne prend pas** cet objet : la rotation recopie le client et l'appareil de
  la session qu'elle remplace. Sans cela, toute session de l'application redeviendrait « site » au
  premier renouvellement.

## Ce qui s'ajoute en lecture

`GET /api/auth/me` rend, pour la session courante :

```jsonc
{
  "session": { "client_kind": "app", "device_label": "Android · Chrome", "issued_at": "…" }
}
```

C'est ce qui permet au profil de dire de quel appareil on est connecté, sans requête de plus.

## La durée d'une session ouverte depuis l'application

**Le constat** : `AUTH_SESSION_TTL` vaut **12 h**, et `AUTH_SESSION_TTL_REMEMBERED` **30 j** — la
seconde ne s'applique que si la personne coche « se souvenir de moi ». **L'écran « 03b Connexion » de
la maquette n'a pas cette case.** Une négociatrice déconnectée toutes les douze heures, dans une salle
sans réseau pour se reconnecter, ne peut plus compter sur l'application — et c'est précisément là
qu'elle en a besoin.

**La règle** : une session ouverte avec `client.kind = "app"` prend **d'office une durée longue**,
sans case à cocher et sans que la personne ait à y penser.

| | Durée | Comportement |
|---|---|---|
| `web`, sans « se souvenir de moi » | 12 h | inchangé |
| `web`, avec « se souvenir de moi » | 30 j | inchangé |
| **`app`** | **90 jours, glissants** | repart de 90 jours **à chaque rotation** |

**Glissante, et c'est le point qui compte.** Une durée fixe de 90 jours posée à la connexion ferait
expirer le 14 novembre un compte ouvert le 15 octobre — au milieu de la COP, après des semaines de
préparation. Une durée qui repart à chaque renouvellement couvre l'avant-COP **et** la COP, et ne
s'éteint que pour qui n'a pas ouvert l'application depuis trois mois.

Un réglage porte la valeur — `AUTH_SESSION_TTL_APP`, par défaut `90d` —, jamais une constante écrite
dans le code.

### Ce que voit la personne quand la session a expiré

Hors connexion, l'application **ne se vide pas** :

- elle **garde ce qui a été lu** et l'affiche avec l'heure de sa lecture, comme toute donnée hors
  connexion (principe XI) ;
- elle ne réclame **rien** tant qu'il n'y a pas de réseau : une reconnexion sans réseau est
  impossible, et la demander serait une impasse ;
- au retour du réseau, et seulement là, elle dit que la session a expiré et propose de se reconnecter.
  Ce qui était lu reste lisible pendant ce temps.

**Ce qu'il ne faut surtout pas faire** : effacer l'écran ou renvoyer à la connexion dès que le jeton
est périmé. C'est le geste réflexe d'un client web, et il transforme une salle sans réseau en
application inutilisable.

## Déconnexion

`POST /api/auth/logout` est inchangé : il ferme **la session de cet appareil**, et elle seule
(FR-006). Les autres sessions de la personne restent ouvertes — c'est déjà le comportement, il est
simplement vérifié par un test de plus.

## À vérifier, pas à supposer

- Le cookie de rafraîchissement porte `Path=/api/auth` et `SameSite=Strict`. Sous préfixe `/v2`, le
  chemin doit suivre : étendre `identity/tests/cohabitation_sous_prefixe.rs`, ne pas en écrire un
  second.
- Aucune route ne lit d'en-tête `Authorization` : l'application installée reste sur les cookies, et
  le jeton attendra la coquille Capacitor (ADR-002).

---

## Le retour dans l'application depuis un courriel

Voir [courriels-retour-app.md](courriels-retour-app.md) : quand la demande vient de l'application, le
lien du courriel mène à Guide Négo, pas à l'écran du site.

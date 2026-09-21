# Contrat — le retour dans l'application depuis un courriel

## Le défaut à corriger

`identity/src/mail.rs` compose ses liens vers des **écrans du site** : `/verification-adresse`,
`/nouveau-mot-de-passe`, avec leurs chemins traduits. Une personne qui crée son compte **dans
l'application** ouvre son courriel, touche le lien, et se retrouve sur l'ePavillon — autre apparence,
autre logique, **et aucun chemin de retour**. Le parcours qu'elle avait commencé est perdu.

La maquette n'a pas d'écran pour ce moment. Il se compose avec les composants livrés en 0a.

## La règle

**C'est `identity` qui choisit le lien, d'après le client de la demande.** Le courriel appartient au
module qui l'envoie ; `negotiation` ne l'appelle jamais — ce serait la dépendance croisée que le
principe II interdit.

| Demande faite depuis | Lien du courriel |
|---|---|
| Le site (`client.kind = "web"`, ou absent) | `/verification-adresse?token=…`, `/nouveau-mot-de-passe?token=…` — **inchangé** |
| L'application (`client.kind = "app"`) | `/guide-nego/verification-adresse?token=…`, `/guide-nego/nouveau-mot-de-passe?token=…` |

Le client de la demande se retient avec le jeton — `identity.one_time_tokens` porte déjà un
`payload jsonb` pour cela. Il n'est **pas** relu de la session : au moment où le lien est ouvert, la
personne n'en a pas encore.

Les adresses de Guide Négo **ne sont pas localisées** (`defineI18nRoute(false)`, français quel que
soit le téléphone — R3 de l'étape 0a) : pas de préfixe `/en/`, contrairement aux écrans du site.

## Les deux téléphones ne se comportent pas pareil

**Android** — l'application installée capture le lien de son propre domaine et l'ouvre elle-même. La
page confirme, et enchaîne sur la suite du parcours : saisie du code, ou demande d'accès.

**iPhone** — l'application installée **ne partage pas le stockage de Safari**. Le lien s'ouvre dans
le navigateur, dans une session qui n'est pas celle de l'application. La page ne peut donc pas
« continuer le parcours » : elle **confirme et renvoie**.

```
        Adresse confirmée
        Retournez dans Guide Négo pour continuer.
```

Et l'application, **à son retour au premier plan**, relit l'état du compte et passe à la suite —
**sans rien demander à personne**. Une personne qui vient de confirmer son adresse ne doit pas avoir
à deviner qu'il faut toucher un bouton.

## Les écrans

| Écran | Ce qu'il porte |
|---|---|
| `/guide-nego/verification-adresse` | Les quatre états : jeton valide (confirmé), jeton expiré (avec « Renvoyer le courriel »), jeton déjà utilisé, jeton inconnu. Sur Android, enchaîne ; sur iPhone, renvoie à l'application |
| `/guide-nego/nouveau-mot-de-passe` | Saisie du nouveau mot de passe, mêmes exigences que le site, puis retour au parcours |
| `/guide-nego/compte` — écran d'attente de confirmation | **« J'ai confirmé mon adresse »** (relit l'état, ne fait rien d'autre) et **« Renvoyer le courriel »**, avec son délai d'attente |

Le retour au premier plan se détecte par `visibilitychange` — le même mécanisme que l'étape 0a emploie
déjà pour relire au retour du réseau. Aucune attente active, aucune interrogation répétée.

## Ce qui ne change pas

- **Les textes des courriels restent dans `identity/src/mail.rs`** : ce ne sont pas des traductions
  d'interface, et le fichier le dit en tête.
- Le site continue de recevoir ses liens d'aujourd'hui, au caractère près.
- La durée des jetons est inchangée : 24 h pour la vérification d'adresse, 1 h pour le mot de passe.

## À inscrire

Ces trois écrans ne sont pas dans `02-socle.html`. Ils s'inscrivent comme **écart** dans
[05-design.md](../../../docs/AppNego/05-design.md) § « Les écarts, tranchés », avec leur raison : la
maquette n'a pas prévu le retour d'un courriel, et sans ces écrans une personne qui crée son compte
depuis l'application sort de l'application pour n'y jamais revenir.

# Contrat — la copie gardée, format 2

Modifie [hors-connexion.md](../../011-guide-nego-documents/contracts/hors-connexion.md) de l'étape 1 pour ce qui touche le contenu d'une copie. Les caches, le magasin, les déclencheurs d'effacement et la file « au retour du réseau » ne changent pas.

## Ce qu'une copie contient

| Entrée | Clé (adresse absolue, `cleDeCopie`) | Réponse gardée |
|---|---|---|
| La lecture | `…/negotiation/documents/{id}/reading` | `200`, `application/json`, avec son `ETag` |
| Le PDF | `…/negotiation/documents/{id}/file` | `200`, `application/pdf`, corps entier — **jamais un `206`**, que `cache.put` refuse |

Plus aucune image de page. Rangée dans `gn-documents-publics` ou `gn-documents-reserves` selon l'accès du document, comme à l'étape 1.

## La fiche (`Copie`, magasin `copies`, IndexedDB `guide-nego` v3)

| Champ | Changement |
|---|---|
| `format` | **nouveau** : `2`. Constante `FORMAT_DE_COPIE` dans `utils/guide-nego/copies.ts` |
| `cles` | `[lecture, pdf]`, dans cet ordre |
| `octets` | taille de la lecture + taille du PDF |
| `pages_images` | **supprimé** |
| `mode` | **supprimé** : `large_text` vit dans la lecture |
| `id`, `version`, `reading_etag`, `reserve`, `gardee_a` | inchangés |

La base reste en **v3** : aucun magasin ne change de forme.

## Règles

1. **Entière ou absente.** Le téléchargement lit la lecture puis le PDF, en flux, et n'écrit rien avant d'avoir tout reçu ; la fiche se pose en dernier, sous le verrou `enSerie`. Une annulation, une coupure ou un manque de place ne laisse aucune entrée (FR-026).
2. **Progression.** Somme des `Content-Length` des deux réponses : « 1,2 Mo sur 3,2 Mo ».
3. **Format ancien.** `verifierLesCopies`, à l'ouverture de l'application, efface toute fiche dont `format` est absent ou différent de `FORMAT_DE_COPIE`, avec ses entrées ; la place est rendue et le document redevient « Non téléchargé » (FR-031). Aucune migration de copie, jamais : un changement de copie monte la constante.
4. **Lire une copie.** `lireLaCopie` vérifie l'intégrité — les deux entrées présentes —, puis rend la lecture et les octets du PDF ; le lecteur appelle `getDocument({ data })`. Une copie incomplète s'efface, comme à l'étape 1.
5. **Autre version.** Une empreinte de lecture différente à la relecture de la liste marque la copie « autre version », sans retéléchargement : la copie reste lisible telle quelle (étape 1, FR-020).
6. **Changement du choix « Texte agrandi ».** Il change l'empreinte de la lecture, pas le PDF. La relecture de la liste remplace la **seule entrée de lecture** de la copie, sans retélécharger le PDF — la fiche garde son format, ses octets se recalculent.
7. **Effacements.** Déconnexion, accès perdu, dépublication, « Tout retirer », « Libérer » : inchangés, et ils emportent désormais le PDF avec la lecture.

## Tests (sans navigateur, faux `caches` et IndexedDB)

- une copie de format 2 se garde, se lit, se retire ; `octets` = lecture + PDF ;
- une coupure pendant le PDF ne laisse ni lecture ni fiche ;
- une fiche sans `format` et une fiche de format 1 s'effacent à la vérification, entrées comprises ;
- le changement de choix ne remplace que la lecture ;
- l'effacement des réservés emporte le PDF.

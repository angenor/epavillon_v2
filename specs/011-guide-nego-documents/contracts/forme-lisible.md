# Contrat — la forme lisible

Ce que l'extraction produit, ce que la base garde, ce que l'API sert, ce que le téléphone lit. **Grammaire close** : aucun HTML, aucun style libre, comme `GnTexteLong` (`utils/guide-nego/texte-long.ts`). Un type inconnu du lecteur **s'ignore** au lieu de casser la page. Ce contrat peut s'étendre, jamais se redéfinir.

Les règles qui produisent ces blocs sont celles de [R7](../research.md), et l'essai les ajuste ([essai-extraction.md](../essai-extraction.md)).

## Le document

```text
DocumentReading {
  id, version,
  mode: "reflow" | "as_is",
  page_count,                  // pages du document, pas du fichier
  outline: OutlineEntry[],     // vide si aucun sommaire n'a été repéré
  pages: ReadingPage[]
}
```

## Le sommaire

```text
OutlineEntry {
  title: string,
  level: 1 | 2 | 3,
  page_index: integer,         // renvoie à ReadingPage.index
  children: OutlineEntry[]
}
```

Le nombre de sous-parties affiché sur un chapitre replié est `children.length`.

## La page

```text
ReadingPage {
  index: integer,              // 1..page_count, clé de la progression et des notes
  label: string,               // étiquette imprimée : « 59 »
  image?: string,              // adresse de l'image de la page, si gardée (R2)
  blocks: Block[]              // vide en mode as_is
}
```

## Les blocs

| `kind` | Champs | Rendu |
|---|---|---|
| `heading` | `level` (1 à 3), `spans` | Titre ; la taille ne suit pas celle du texte |
| `paragraph` | `spans` | Paragraphe recomposé |
| `list_item` | `spans`, `depth` (0 à 2), `marker` (« • », « 1. », « a) ») | Élément de liste |
| `note` | `spans`, `mark` (« 1 ») | Note de bas de page, rendue en fin de page |
| `origin` | `reason` : `"table"` ou `"figure"`, `caption?` : `spans`, `text?` : `spans` | « Voir la page d'origine », qui ouvre l'image de la page ; hors connexion, si l'image n'est pas gardée : « Page d'origine disponible avec le réseau ». `text` est le texte de la zone dans l'ordre du flux, **affiché replié** sous ce lien : il rend un tableau cherchable, et lisible à plat (ajout de l'essai, 23/09) |

## Les segments

```text
Span {
  text: string,
  italic?: true,
  bold?: true,
  term?: true                  // italique retenu comme terme anglais touchable (R7)
}
```

`term` implique `italic`. Toucher un segment `term` ouvre la feuille du lexique, titrée de `text`.

## Invariants

- Les `index` des pages sont contigus de 1 à `page_count`.
- Le `page_index` d'une entrée du sommaire existe.
- La concaténation des `spans[].text` d'une page, blocs `note` et `text` des blocs `origin` compris, **est** `document_pages.plain_text`. Rien ne se cherche qui ne s'affiche pas.
- Le texte n'est jamais modifié par une note de correction : les notes arrivent par une autre lecture ([api-documents.md](api-documents.md)) et se posent par-dessus.

## Tests

- **Côté Rust** : chaque règle de R7 sur une page d'essai construite à la main, et la forme produite validée contre ce contrat.
- **Côté client**, sans navigateur :
  - le rendu ignore un `kind` inconnu ;
  - la recherche sans accents trouve « progrès » par « progres » ;
  - le décompte « 5 passages dans 92 pages » ;
  - l'extrait cité d'une note retrouvé dans les segments, et la tête de page quand il est introuvable.

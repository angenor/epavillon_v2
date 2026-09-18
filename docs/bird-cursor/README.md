# bird-cursor — oiseau interactif qui suit le curseur

Un oiseau SVG animé qui poursuit le curseur, se pose sur vos composants et parle
dans une bulle au survol. Un seul fichier, aucune dépendance, ~30 Ko.

```
bird-cursor/
├── bird-cursor.js    ← tout est là : le dessin SVG + le moteur d'animation
├── bird-cursor.svg   ← l'illustration seule (référence / retouche graphique)
├── demo.html         ← page d'exemple, ouvrable directement dans un navigateur
└── README.md
```

---

## 1. Installation

Copiez `bird-cursor.js` dans vos assets, puis une seule ligne avant `</body>` :

```html
<script src="/assets/bird-cursor.js" defer></script>
```

C'est tout : l'oiseau s'injecte lui-même dans `<body>`, en overlay `position:fixed`
non cliquable (`pointer-events:none`), et n'interfère avec aucun de vos éléments.

Avec des réglages :

```html
<script src="/assets/bird-cursor.js" defer
        data-scale="0.42"
        data-near="230"
        data-standoff="150"
        data-idle="550"
        data-stiff="0.03"
        data-lag="0.035"></script>
```

### React / Vue / Svelte / Next

```jsx
useEffect(() => {
  const s = document.createElement('script');
  s.src = '/bird-cursor.js';
  document.body.appendChild(s);
  return () => { window.BirdCursor?.destroy(); s.remove(); };
}, []);
```

En Next.js : placez `bird-cursor.js` dans `/public` et utilisez
`<Script src="/bird-cursor.js" strategy="afterInteractive" />`.

---

## 2. Marquer vos composants

### `data-bird-perch` — les perchoirs

L'oiseau se pose sur le **bord supérieur** de ces éléments. Mettez-le sur ce qui
est visuellement « solide » : barre de nav, cartes, boutons, en-têtes de section,
footer.

```html
<header data-bird-perch>…</header>
<div class="card" data-bird-perch>…</div>
<button data-bird-perch>Commencer</button>
```

- Les éléments de moins de 44 px de large sont ignorés.
- Sans aucun perchoir sur la page, il se pose en bas de la fenêtre.
- Il reste collé à son perchoir pendant le scroll (la position est recalculée à
  chaque image).

### `data-bird-say` — les répliques

Au survol, l'oiseau vient se poser sur l'élément et le texte apparaît dans sa bulle.

```html
<button data-bird-perch data-bird-say="Gratuit pendant 14 jours.">Commencer</button>
<span data-bird-say="Vos 12 projets en cours, triés par échéance.">Projets</span>
```

- Fonctionne sur n'importe quel élément, même sans `data-bird-perch`.
- Une seule bulle à la fois ; elle disparaît en fondu à la sortie du survol.
- Gardez les textes courts (une à deux phrases, ~140 caractères max).

---

## 3. Options

| Attribut / clé   | Défaut | Rôle |
|------------------|--------|------|
| `data-scale`     | `0.42` | Taille de l'oiseau (0.42 ≈ 95 px de haut). |
| `data-standoff`  | `150`  | **Distance minimale gardée avec le curseur**, en px. Il ne se met jamais sous le pointeur, ni en vol ni posé. |
| `data-near`      | `230`  | En dessous de cette distance il ne décolle pas : il reste assis et suit du regard. |
| `data-idle`      | `550`  | Millisecondes d'immobilité du curseur avant l'atterrissage. |
| `data-stiff`     | `0.03` | Nervosité du vol. Plus haut = plus vif, plus bas = plus paresseux. |
| `data-damp`      | `0.7`  | Amortissement du vol. Réglé pour un arrêt net, sans rebond. |
| `data-lag`       | `0.035`| Temps de réaction : il vise une position **retardée** du curseur. Plus bas = plus contemplatif. |
| `data-gaze`      | `340`  | Portée du regard, en px. Au-delà, il regarde ailleurs. |
| `data-z-index`   | `9999` | Empilement de l'overlay (la bulle prend `z-index + 1`). |

### API JavaScript

```js
window.BirdCursor.mount({ scale: 0.5, standoff: 200, near: 260 }); // (re)monte
window.BirdCursor.destroy();                                       // retire tout
window.BirdCursor.defaults;                                        // les valeurs par défaut
```

`mount()` nettoie toujours l'instance précédente : pas de doublon possible.

### Accessibilité

`prefers-reduced-motion: reduce` désactive complètement l'animation (rien n'est
injecté). L'overlay est `pointer-events:none` et ne capte aucun clic ; la bulle
porte `role="status"`. Les textes de `data-bird-say` sont décoratifs : gardez
l'information essentielle dans votre page.

---

## 4. Comment ça marche (4 états)

| État | Déclencheur | Rendu |
|------|-------------|-------|
| `chase` | le curseur bouge et s'éloigne de plus de `near` | vol en ressort vers un point situé à `standoff` du curseur, ailes battantes, corps incliné, pattes rentrées |
| `land` | le curseur s'immobilise (`idle` ms) | approche amortie vers le perchoir le plus proche : il ralentit et se pose, sans dépasser la cible ni rebondir |
| `perch` | arrivée sur le perchoir | position **figée** sur le composant, ailes repliées, pattes posées, respiration, dodelinement |
| `say` | survol d'un `data-bird-say` | il rejoint l'élément survolé et la bulle s'affiche au-dessus de sa tête |

Détails utiles : le point d'appui est figé à l'atterrissage (il ne glisse donc
jamais horizontalement quand le curseur bouge) ; le retournement est interpolé
(pivot + petit saut + battement) ; la tête et la pupille suivent le curseur
jusqu'à −38°/+42° ; clignements aléatoires toutes les 2 à 6 s.

---

## 5. Modifier l'oiseau

Tout est dans `bird-cursor.js` :

- **`var MARKUP = "<svg …>"`** (ligne ~20) : le dessin, sous forme de chaîne.
  Groupes nommés : `#bird` (transform global), `#wing-front`, `#wing-back`,
  `#head`, `#pupil`, `#lid` (paupière), `#legs`, `#tail`.
  Pour retoucher le graphisme, éditez plutôt `bird-cursor.svg` dans Illustrator /
  Figma **en conservant les id**, puis recollez le balisage dans `MARKUP`
  (sérialisé en JSON : `JSON.stringify(markup)`).
- **`var DEFAULTS = { … }`** : les valeurs par défaut du tableau ci-dessus.
- **`function frame(now)`** : toute la boucle d'animation, commentée section par
  section (états, cible de vol, orientation, ailes, regard, clignement, rendu).
- **La bulle** est construite en DOM juste après les écouteurs souris : couleurs,
  police, rayon, ombre et queue sont dans les `cssText` — c'est là qu'on l'aligne
  sur votre charte.

Palette d'origine : `#EF3F1D` corps, `#A31332` / `#682335` ailes et contours,
`#FFFDF8` ventre et bulle, `#FFCD40` / `#F2AC30` bec et pattes, `#252538` œil.

### Pivots (coordonnées internes du SVG)

| Pièce | Pivot | Angle posé | Angle en vol |
|-------|-------|-----------|--------------|
| aile avant | `168,163` | `-54°` | `2°` ± amplitude |
| aile arrière | `183,167` | `1.28 ×` aile avant | idem |
| tête | `172,152` | `-38°` → `+42°` | idem |
| queue | `108,148` | dodelinement | selon la vitesse verticale |

---

## 6. Avec Claude Code

Déposez le dossier dans votre projet et donnez ce genre de consigne :

> Intègre `bird-cursor/bird-cursor.js` dans le site : copie-le dans `public/`,
> charge-le en `defer` dans le layout, puis ajoute `data-bird-perch` sur la
> navbar, les cartes et le footer, et un `data-bird-say` court et utile sur les
> 6 éléments les plus importants (une phrase, ton amical, dans la langue du site).
> Ne touche pas au moteur ; règle seulement `data-scale`, `data-standoff` et
> `data-near` pour que l'oiseau ne recouvre jamais du texte.

Autres consignes qui fonctionnent bien :

> Recolore l'oiseau et la bulle avec les tokens de notre design system
> (remplace les hex dans `MARKUP` et dans les `cssText` de la bulle).

> Fais que l'oiseau ne se pose que sur les composants visibles dans le viewport
> et jamais sur les éléments de formulaire.

> Ajoute un état « endormi » : après 30 s sans mouvement, ferme la paupière
> (`#lid`) et arrête le dodelinement.

Points de vigilance à rappeler à l'assistant : ne pas dupliquer l'appel à
`mount()` (utiliser `destroy()` au démontage du composant), garder les `id` du
SVG intacts, et conserver `pointer-events:none` sur l'overlay.

---

Licence : libre d'usage sur vos projets. Illustration d'origine fournie par le
client ; animation et moteur écrits pour ce projet.

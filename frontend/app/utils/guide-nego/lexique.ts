/**
 * Le lexique sur le téléphone : la forme normalisée d'un terme, la résolution d'un
 * texte vers son entrée, et le regroupement par lettre.
 *
 * `normaliserTerme` rend ce que rend `platform.normalize_label` : les deux côtés se
 * comparent sur les chaînes de `backend/crates/modules/negotiation/tests/fixtures/normalisation.json`.
 * L'égalité est vérifiée caractère par caractère sur les écritures latines, la
 * ponctuation, les exposants et les formes pleine chasse ; au-delà, les deux côtés
 * peuvent diverger.
 */
import type { GlossaryEntry } from '../../types/negotiation-savoir.ts'
import { plierLettre, replier } from './repli.ts'

// Ce que `unaccent` fait autrement que NFKD, relevé sur la base : lettres barrées,
// ligatures, et exposants, indices et fractions, que la base efface ou espace.
const COMME_LA_BASE: Record<string, string> = Object.fromEntries(
  Object.entries({
    ' ': 'ª²³¹ºẛ⁰ⁱ⁴⁵⁶⁷⁸⁹ⁿ₀₁₂₃₄₅₆₇₈₉ₐₑₒₓₕₖₗₘₙₚₛₜ₨℠™' +
      '\u212b\u00ad\u0363\u0364\u0365\u0366\u0367\u0368\u0369\u036a\u036b\u036c\u036d\u036e\u036f\u20d0\u20d1\u20d2\u20d3\u20d4\u20d5\u20d6\u20d7\u20d8\u20d9\u20da\u20db\u20dc\u20e1\u20e5\u20e6\u20e7\u20e8\u20e9\u20ea\u20eb\u20ec\u20ed\u20ee\u20ef\u20f0',
    ' c ': '©',
    ' r ': '®',
    ' p ': '℗',
    ' 1 4': '¼',
    ' 1 2': '½',
    ' 3 4': '¾',
    ' 1 7': '⅐',
    ' 1 9': '⅑',
    ' 1 10': '⅒',
    ' 1 3': '⅓',
    ' 2 3': '⅔',
    ' 1 5': '⅕',
    ' 2 5': '⅖',
    ' 3 5': '⅗',
    ' 4 5': '⅘',
    ' 1 6': '⅙',
    ' 5 6': '⅚',
    ' 1 8': '⅛',
    ' 3 8': '⅜',
    ' 5 8': '⅝',
    ' 7 8': '⅞',
    ' 1 ': '⅟',
    ' 0 3': '↉',
    ae: 'Ææ',
    oe: 'Œœɶ',
    ss: 'ßẞ',
    th: 'Þþ',
    a: 'Ⱥẚ',
    b: 'ƀƁƂƃɃɓʙ',
    c: 'ƇƈȻȼɕ',
    d: 'ÐðĐđƉƊƋƌȡɖɗ',
    e: 'ƐɆɇɛ',
    f: 'Ƒƒ',
    g: 'ƓǤǥɠɡɢʛ',
    h: 'Ħħɦɧʜ',
    i: 'ıƖƗɨɪ',
    j: 'ȷɈɉɟʝ',
    k: 'Ƙƙ',
    l: 'ĿŀŁłƚȴȽɫɬɭʟ',
    m: 'ɱ',
    n: 'ŊŋƝƞȵɲɳɴ',
    o: 'Øø',
    p: 'Ƥƥ℘',
    q: 'ĸʠ',
    r: 'Ɍɍɼɽɾʀ',
    s: 'ȿʂẜẝ',
    t: 'ŦŧƫƬƭƮȶȾʈ',
    u: 'Ʉʉ',
    v: 'ƲʋỼỽ',
    x: 'ℌ',
    y: 'ƳƴɎɏʏỾỿ',
    z: 'ƵƶȤȥɀʐʑ',
    hv: 'ƕ',
    oi: 'Ƣƣ',
    db: 'ȸ',
    qp: 'ȹ',
    dz: 'ʣʥ',
    ts: 'ʦ',
    ls: 'ʪ',
    lz: 'ʫ',
    ll: 'Ỻỻ',
    ce: '₠',
    cr: '₢',
    'fr ': '₣',
    'l ': '₤',
    pts: '₧',
    rs: '₹',
    tl: '₺',
    rx: '℞',
  }).flatMap(([valeur, lettres]) => [...lettres].map((lettre) => [lettre, valeur])),
)

/** Minuscules, sans accents ; tout ce qui n'est ni lettre ni chiffre devient une espace, réduite. */
export function normaliserTerme(texte: string): string {
  let plie = ''
  for (const lettre of texte) plie += COMME_LA_BASE[lettre] ?? plierLettre(lettre)
  return plie
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
}

/** Comme `platform.slugify`. */
export const slugDe = (texte: string): string => normaliserTerme(texte).replace(/ /g, '-').slice(0, 160)

/**
 * L'entrée que désigne un texte : le terme, puis le sigle, une variante, le slug —
 * dans l'ordre et avec le départage de `negotiation.glossary_resolve`. Jamais
 * d'approché : `null` plutôt qu'une mauvaise définition.
 */
export function resoudreLeTerme(texte: string, lexique: readonly GlossaryEntry[]): GlossaryEntry | null {
  const forme = normaliserTerme(texte)
  if (!forme) return null
  const slug = slugDe(texte)
  const rangs: ((e: GlossaryEntry) => boolean)[] = [
    (e) => normaliserTerme(e.term) === forme,
    (e) => e.acronym !== null && normaliserTerme(e.acronym) === forme,
    (e) => e.variants.some((v) => normaliserTerme(v) === forme),
    (e) => e.slug === slug,
  ]
  for (const correspond of rangs) {
    let trouvee: GlossaryEntry | null = null
    for (const e of lexique) if (correspond(e) && (!trouvee || e.id < trouvee.id)) trouvee = e
    if (trouvee) return trouvee
  }
  return null
}

/** La lettre d'un terme ; `#` pour un terme qui commence par un chiffre ou un signe. */
export function lettreDe(terme: string): string {
  const premiere = normaliserTerme(terme).charAt(0)
  return /[a-z]/.test(premiere) ? premiere.toUpperCase() : '#'
}

export interface GroupeDeLettre {
  lettre: string
  entrees: GlossaryEntry[]
}

/** Les entrées par lettre, de A à Z puis `#`, chacune dans l'ordre de son terme normalisé. */
export function grouperParLettre(lexique: readonly GlossaryEntry[]): GroupeDeLettre[] {
  const tries = [...lexique]
    .map((entree) => ({ entree, cle: normaliserTerme(entree.term) }))
    .sort((a, b) => (a.cle < b.cle ? -1 : a.cle > b.cle ? 1 : a.entree.id < b.entree.id ? -1 : 1))
  const groupes = new Map<string, GlossaryEntry[]>()
  for (const { entree } of tries) {
    const lettre = lettreDe(entree.term)
    const groupe = groupes.get(lettre)
    if (groupe) groupe.push(entree)
    else groupes.set(lettre, [entree])
  }
  return [...groupes.entries()]
    .sort(([a], [b]) => (a === '#' ? 1 : b === '#' ? -1 : a < b ? -1 : 1))
    .map(([lettre, entrees]) => ({ lettre, entrees }))
}

/** « GGA — global goal on adaptation » : le sigle devant, quand il existe. */
export const intituleDe = (entree: Pick<GlossaryEntry, 'term' | 'acronym'>): string =>
  entree.acronym ? `${entree.acronym} · ${entree.term}` : entree.term

/** La première phrase d'une définition : ce que montrent les résultats. */
export function premierePhrase(texte: string): string {
  const fin = /[.!?…](?=\s|$)/u.exec(texte)
  return fin ? texte.slice(0, fin.index + 1) : texte
}

export interface Morceau {
  texte: string
  marque: boolean
}

/** Le texte découpé autour de chaque apparition de la saisie, sans tenir compte des accents ni de la casse. */
export function morceauxSurlignes(texte: string, saisie: string): Morceau[] {
  const cherche = replier(saisie).replie.trim()
  if (!cherche) return [{ texte, marque: false }]
  const { replie, origine } = replier(texte)
  const morceaux: Morceau[] = []
  let depuis = 0
  let position = replie.indexOf(cherche)
  while (position !== -1) {
    const debut = origine[position] as number
    const fin = (origine[position + cherche.length - 1] as number) + 1
    if (debut > depuis) morceaux.push({ texte: texte.slice(depuis, debut), marque: false })
    morceaux.push({ texte: texte.slice(debut, fin), marque: true })
    depuis = fin
    position = replie.indexOf(cherche, position + cherche.length)
  }
  if (depuis < texte.length) morceaux.push({ texte: texte.slice(depuis), marque: false })
  return morceaux
}

export const DERNIERS_CONSULTES = 5

/** L'entrée ouverte passe en tête ; cinq au plus. */
export const derniersApres = (derniers: readonly string[], id: string): string[] =>
  [id, ...derniers.filter((d) => d !== id)].slice(0, DERNIERS_CONSULTES)

/** Une liste d'identifiants lue sur le téléphone : ce qui n'en est pas une vaut une liste vide. */
export function idsLus(brut: string | null): string[] {
  if (!brut) return []
  try {
    const lu: unknown = JSON.parse(brut)
    return Array.isArray(lu) ? lu.filter((v): v is string => typeof v === 'string') : []
  } catch {
    return []
  }
}

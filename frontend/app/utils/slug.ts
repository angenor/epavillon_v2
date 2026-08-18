/**
 * `platform.slugify()`, rejouée dans le navigateur.
 *
 * ── POURQUOI LA REJOUER PLUTÔT QUE D'ATTENDRE LA BASE ───────────────────────
 *
 * Le slug est une ADRESSE PUBLIQUE : `/evenements/cop31-belem-2027`. Il se saisit
 * au moment où l'on nomme l'édition, et l'écran doit le proposer pendant la
 * frappe — demander un aller-retour par caractère n'aurait aucun sens, et laisser
 * le champ vide obligerait à composer à la main une chaîne dont la forme est déjà
 * décidée par le modèle.
 *
 * ── LA DÉFINITION EST CELLE DE `000_bootstrap.sql`, PAS UNE APPROXIMATION ────
 *
 * `platform.slugify()` compose deux fonctions :
 *
 *     normalize_label()  minuscules, sans accents, toute ponctuation devient une
 *                        espace, espaces multiples réduits, extrémités rognées
 *     slugify()          les espaces deviennent des traits d'union, coupé à 160
 *
 * On suit le même ordre, à la lettre. L'écart le plus tentant serait de remplacer
 * la ponctuation par RIEN plutôt que par une espace : « Côte d'Ivoire » donnerait
 * alors `cotedivoire` au lieu de `cote-d-ivoire`, et deux systèmes qui composent
 * la même adresse ne tomberaient plus d'accord.
 *
 * Ce n'est PAS une validation : la base reste seule juge de l'unicité
 * (`ux_events_slug`), et l'écran rend son refus.
 */

/** `platform.normalize_label()` — forme canonique d'un libellé. */
export function normalizeLabelLike(value: string | null | undefined): string {
  return (value ?? '')
    .normalize('NFD')
    // Les diacritiques combinants, que `immutable_unaccent()` retire en base.
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    // Tout ce qui n'est ni lettre ASCII ni chiffre devient UNE espace — y compris
    // l'apostrophe et le tiret, exactement comme la classe `[^a-z0-9]+` du SQL.
    .replace(/[^a-z0-9]+/g, ' ')
    .trim()
}

/** `platform.slugify()` — le libellé canonique, en adresse. */
export function slugify(value: string | null | undefined): string {
  return normalizeLabelLike(value).replace(/ /g, '-').slice(0, 160)
}

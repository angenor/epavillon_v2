/**
 * Exigences et robustesse d'un mot de passe.
 *
 * LA RÈGLE, ARRÊTÉE PAR LE COMMANDITAIRE LE 17/08 : huit caractères au moins,
 * dont **une majuscule et une minuscule**. Le caractère spécial est FACULTATIF,
 * et le chiffre ne l'est même pas — ni l'un ni l'autre n'empêchent d'enregistrer.
 *
 * Ce qui suit sépare donc deux choses qu'on confond souvent :
 *
 *  · LES EXIGENCES (`missing`, `meetsRequirements`) — trois conditions, vérifiées
 *    et opposables. Elles seules empêchent l'envoi du formulaire, et l'écran les
 *    affiche en toutes lettres AVANT la saisie plutôt qu'après l'échec.
 *  · LA ROBUSTESSE (`score`, `level`, `advice`) — une estimation qui CONSEILLE
 *    et n'interdit jamais. Un mot de passe conforme mais faible s'enregistre ;
 *    l'indicateur dit seulement ce qui le rendrait meilleur. C'est ce qui permet
 *    de signaler `Azerty12` — conforme aux trois exigences, et pourtant dans la
 *    première liste que quiconque essaierait.
 *
 * CE FICHIER NE TRADUIT RIEN. Il rend des identifiants stables (`fair`,
 * `avoidSequence`, `uppercase`…) que l'écran résout dans ses propres fichiers
 * i18n : un utilitaire pur se teste sans monter Vue, et sa sortie ne change pas
 * de langue.
 *
 * CÔTÉ API (prompt B1), ces contrôles seront REFAITS — un indicateur de
 * navigateur n'est pas une sécurité. Ici, il guide la saisie ; là-bas, il décide.
 */

/** Longueur minimale exigée. */
export const MIN_PASSWORD_LENGTH = 8

/** Longueur au-delà de laquelle la longueur seule vaut déjà beaucoup. */
const LONG_ENOUGH = 16

/**
 * Les trois conditions opposables. Le caractère spécial n'en fait pas partie :
 * il est facultatif, et n'apparaît que dans le score et dans la liste affichée,
 * où il est marqué comme tel.
 */
export type PasswordRequirement = 'length' | 'uppercase' | 'lowercase'

/** Score de 0 (inutilisable) à 4 (solide). */
export type PasswordScore = 0 | 1 | 2 | 3 | 4

/**
 * Libellé de palier ; l'écran le traduit. `empty` = rien n'a encore été saisi,
 * `incomplete` = au moins une exigence n'est pas remplie — la liste des
 * conditions dit alors laquelle, ce qu'un simple « faible » ne ferait pas.
 */
export type PasswordStrengthLevel = 'empty' | 'incomplete' | 'weak' | 'fair' | 'good' | 'strong'

/** Conseil affiché sous l'indicateur. Un seul à la fois : le plus utile. */
export type PasswordAdvice =
  | 'lengthen'
  | 'avoidCommon'
  | 'avoidRepeat'
  | 'avoidSequence'
  | 'addVariety'
  | 'none'

export interface PasswordStrength {
  score: PasswordScore
  level: PasswordStrengthLevel
  advice: PasswordAdvice
  /** Exigences NON satisfaites, dans l'ordre où l'écran les affiche. */
  missing: PasswordRequirement[]
  /** Les trois exigences sont-elles remplies ? Seule condition d'envoi. */
  meetsRequirements: boolean
  /** Le caractère spécial, facultatif : renseigne l'indicateur, ne bloque rien. */
  hasSpecial: boolean
  /** Nombre de caractères saisis — l'écran l'affiche face au minimum. */
  length: number
}

/**
 * Mots et suites que l'on retrouve dans toutes les fuites publiques, plus ceux
 * que cette plateforme-ci inspire. Comparaison en minuscules, sur le mot de
 * passe entier : `epavillon2027` doit être signalé, il sera dans la première
 * liste que quiconque essaiera.
 */
const COMMON_FRAGMENTS = [
  'password',
  'motdepasse',
  'azerty',
  'qwerty',
  'epavillon',
  'ifdd',
  'francophonie',
  'bonjour',
  'soleil',
  'admin',
  'welcome',
  'bienvenue',
  '123456',
  'abcdef',
]

/** Suites de clavier et d'alphabet, dans les deux sens. */
const SEQUENCES = ['abcdefghijklmnopqrstuvwxyz', '0123456789', 'azertyuiop', 'qwertyuiop']

/** Trois signes identiques d'affilée, ou plus. */
function hasRepeatedRun(value: string): boolean {
  return /(.)\1{2,}/.test(value)
}

/** Quatre caractères consécutifs d'une suite connue, dans un sens ou dans l'autre. */
function hasSequence(value: string): boolean {
  const lower = value.toLowerCase()
  for (const sequence of SEQUENCES) {
    const reversed = [...sequence].reverse().join('')
    for (const source of [sequence, reversed]) {
      for (let i = 0; i + 4 <= source.length; i += 1) {
        if (lower.includes(source.slice(i, i + 4))) return true
      }
    }
  }
  return false
}

function containsCommonFragment(value: string): boolean {
  const lower = value.toLowerCase()
  return COMMON_FRAGMENTS.some((fragment) => lower.includes(fragment))
}

/** Nombre de familles de signes présentes : minuscules, majuscules, chiffres, autres. */
function characterVariety(value: string): number {
  return [/[a-zà-ÿ]/, /[A-ZÀ-Þ]/, /\d/, /[^\p{L}\d]/u].filter((pattern) =>
    pattern.test(value),
  ).length
}

/** Les trois exigences opposables, évaluées séparément du score. */
export function missingRequirements(password: string): PasswordRequirement[] {
  const missing: PasswordRequirement[] = []
  if (password.length < MIN_PASSWORD_LENGTH) missing.push('length')
  // `À-Þ` et `à-ÿ` : « Été » et « Ça » comptent, sans quoi la règle
  // désavantagerait le français qu'elle est censée servir.
  if (!/[A-ZÀ-Þ]/.test(password)) missing.push('uppercase')
  if (!/[a-zà-ÿ]/.test(password)) missing.push('lowercase')
  return missing
}

/** Un caractère spécial est-il présent ? Facultatif : il bonifie, il n'exige pas. */
const hasSpecialCharacter = (password: string): boolean => /[^\p{L}\d]/u.test(password)

/**
 * Évalue exigences, score et conseil.
 *
 * L'ORDRE DES PÉNALITÉS COMPTE : un mot de passe long ET contenant « azerty »
 * reste faible. La longueur ouvre le score, les motifs le referment — l'inverse
 * laisserait passer `Azertyazertyazerty`.
 */
export function evaluatePassword(password: string): PasswordStrength {
  const length = password.length
  const missing = missingRequirements(password)
  const hasSpecial = hasSpecialCharacter(password)

  if (length === 0) {
    return {
      score: 0,
      level: 'empty',
      advice: 'none',
      missing,
      meetsRequirements: false,
      hasSpecial,
      length,
    }
  }

  // Une exigence non remplie : le score n'a rien à dire de plus que la liste des
  // conditions, qui est précise. Annoncer « faible » à côté d'un « il manque une
  // majuscule » ferait deux messages pour un seul défaut.
  if (missing.length > 0) {
    return {
      score: 0,
      level: 'incomplete',
      advice: missing.includes('length') ? 'lengthen' : 'none',
      missing,
      meetsRequirements: false,
      hasSpecial,
      length,
    }
  }

  // La longueur donne l'essentiel : 8 signes valent 2, 12 valent 3, 16 valent 4.
  let score = 2
  if (length >= 12) score += 1
  if (length >= LONG_ENOUGH) score += 1

  const variety = characterVariety(password)
  // Les exigences garantissent déjà deux familles de signes ; la troisième —
  // chiffre ou caractère spécial — bonifie un mot de passe déjà d'une longueur
  // honnête. Elle ne rachète pas un mot de passe court.
  if (variety >= 3 && length >= 10) score += 1

  let advice: PasswordAdvice = 'none'

  if (containsCommonFragment(password)) {
    score -= 2
    advice = 'avoidCommon'
  } else if (hasSequence(password)) {
    score -= 2
    advice = 'avoidSequence'
  } else if (hasRepeatedRun(password)) {
    score -= 1
    advice = 'avoidRepeat'
  } else if (variety <= 2 && length < 12) {
    // Deux familles seulement — celles qu'exigent les règles — sur un mot de
    // passe court : un chiffre ou une ponctuation valent mieux qu'un conseil de
    // plus sur la longueur.
    advice = 'addVariety'
  } else if (score < 4) {
    advice = 'lengthen'
  }

  const bounded = Math.min(4, Math.max(1, score)) as PasswordScore
  const levels: Record<number, PasswordStrengthLevel> = {
    1: 'weak',
    2: 'fair',
    3: 'good',
    4: 'strong',
  }

  return {
    score: bounded,
    level: levels[bounded] ?? 'weak',
    advice,
    missing,
    // Les trois exigences sont remplies : le formulaire part, même sur un score
    // faible. L'indicateur conseille, il n'interdit pas.
    meetsRequirements: true,
    hasSpecial,
    length,
  }
}

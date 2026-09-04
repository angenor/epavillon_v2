/**
 * Les initiales d'un nom affiché — la bulle du menu du compte, celle du pied de
 * la navigation du back-office.
 *
 * PAS DE PHOTO SUR LA PLATEFORME : `identity.people` ne porte aucune colonne
 * d'avatar. La bulle rend donc une donnée réelle plutôt qu'une silhouette
 * générique répétée à l'identique pour tout le monde.
 *
 * `Array.from` et non `split('')` : une initiale accentuée ou hors du plan latin
 * ne doit pas être coupée en deux unités de code.
 */
export function initialsOf(name: string): string {
  const words = name.trim().split(/\s+/).filter(Boolean)
  const letters = [words[0], words.length > 1 ? words[words.length - 1] : undefined]
    .filter((word): word is string => Boolean(word))
    .map((word) => Array.from(word)[0] ?? '')
  return letters.join('').toLocaleUpperCase()
}

// Ajoute l'en-tête du projet au fichier engendré par openapi-typescript.
//
// Le générateur écrit son propre avertissement, mais il ne dit ni d'où vient le
// document, ni comment le refaire, ni pourquoi les corps y sont vides — trois
// questions que se pose forcément la première personne qui ouvre le fichier.
import { readFileSync, writeFileSync } from 'node:fs'

const cible = process.argv[2]
if (!cible) {
  console.error('usage : node prefix-api-types.mjs <fichier>')
  process.exit(1)
}

const EN_TETE = `/**
 * Contrat d'API — ENGENDRÉ, jamais écrit à la main.
 *
 *   make openapi
 *
 * Source : les annotations posées auprès des gestionnaires Rust, assemblées par
 * \`backend/crates/api/src/openapi.rs\`. Un chemin absent d'ici n'existe pas dans
 * l'API ; \`make check-api-contract\` refuse tout appel du site vers un chemin
 * qui n'y figure pas.
 *
 * CE QUE CE FICHIER PORTE : les chemins, les verbes, les paramètres, les codes
 * d'erreur et la forme du corps d'erreur (\`ApiError\`).
 *
 * CE QU'IL NE PORTE PAS : la forme des corps de requête et de réponse, qui
 * sortent en \`Record<string, never>\`. L'API les désigne par leur NOM
 * TypeScript, dans la description de chaque opération — \`EditionCallPayload\` →
 * \`CallSaveResult\` —, et leur source unique reste \`frontend/app/types/\`. C'est
 * une décision de l'API, pas un oubli : décrire deux fois la même forme, une
 * fois en Rust et une fois en TypeScript, produit deux vérités qui divergent au
 * premier ajustement. \`make check-api-contract\` vérifie que chaque
 * nom annoncé par l'API existe bien là-bas.
 *
 * Ce fichier est exclu du garde-fou des mille lignes de CLAUDE.md : il est
 * engendré, il ne se lit pas et ne se modifie pas.
 */
`

const contenu = readFileSync(cible, 'utf8')
if (contenu.startsWith('/**\n * Contrat d\'API')) process.exit(0)
writeFileSync(cible, EN_TETE + contenu.replace(/^\/\*\*[\s\S]*?\*\/\n/, ''))

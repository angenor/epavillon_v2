#!/usr/bin/env node
/**
 * Éprouve la liste de garde contre un serveur réel.
 *
 * POURQUOI. `install` est tout ou rien : une seule adresse qui répond autrement que
 * 200 et PLUS AUCUN téléphone ne garde l'application — sans un message, sans une
 * erreur, sans rien. L'empreinte des traductions est tirée d'un paquet par une
 * expression régulière, et la route `/_i18n/` passe en ligne derrière Apache et sous
 * `/v2/` : deux occasions de casser en silence.
 *
 * UNE REDIRECTION COMPTE POUR UNE PANNE. Une réponse redirigée, gardée puis servie à
 * une navigation, est refusée par le navigateur (« redirected response for a
 * navigation request »). Il faut donc un 200 franc, pas un 301 vers la même chose.
 *
 * Usage : node scripts/guide-nego-verifier-garde.mjs <adresse de guide-nego/>
 *   node scripts/guide-nego-verifier-garde.mjs http://localhost:3100/guide-nego/
 *   node scripts/guide-nego-verifier-garde.mjs https://ifdd.francophonie.org/v2/guide-nego/
 */

const PARALLELE = 8
const DELAI_MS = 20000

const base = process.argv[2]
if (!base) {
  console.error('Usage : node scripts/guide-nego-verifier-garde.mjs <adresse de guide-nego/>')
  process.exit(2)
}

const racine = base.endsWith('/') ? base : `${base}/`
const adresseDuWorker = new URL('sw.js', racine).href

async function lireLaListe() {
  const reponse = await fetch(adresseDuWorker, { redirect: 'manual' })
  if (reponse.status !== 200) {
    throw new Error(`${adresseDuWorker} répond ${reponse.status} — le service worker n'est pas servi là.`)
  }
  const source = await reponse.text()
  const trouve = source.match(/const LISTE = (\[[\s\S]*?\n\])/)
  if (!trouve) throw new Error(`${adresseDuWorker} ne porte pas de liste de garde lisible.`)
  return JSON.parse(trouve[1])
}

/** `manual` : on veut VOIR la redirection, pas la suivre. */
async function verifier(adresse) {
  const cible = new URL(adresse, adresseDuWorker).href
  try {
    const reponse = await fetch(cible, { redirect: 'manual', signal: AbortSignal.timeout(DELAI_MS) })
    if (reponse.status >= 300 && reponse.status < 400) {
      return { cible, souci: `redirigée (${reponse.status}) vers ${reponse.headers.get('location') ?? '?'}` }
    }
    if (reponse.status !== 200) return { cible, souci: `répond ${reponse.status}` }
    return null
  } catch (erreur) {
    return { cible, souci: erreur.name === 'TimeoutError' ? 'délai dépassé' : String(erreur.message ?? erreur) }
  }
}

let liste
try {
  liste = await lireLaListe()
} catch (erreur) {
  console.error(`Guide Négo — ${erreur.message ?? erreur}`)
  process.exit(1)
}

const soucis = []

for (let debut = 0; debut < liste.length; debut += PARALLELE) {
  const lot = await Promise.all(liste.slice(debut, debut + PARALLELE).map(verifier))
  soucis.push(...lot.filter(Boolean))
  process.stdout.write(`\r  ${Math.min(debut + PARALLELE, liste.length)} / ${liste.length} adresses…`)
}
process.stdout.write('\r')

if (soucis.length > 0) {
  console.error(`Guide Négo — ${soucis.length} adresse(s) de la garde ne répondent pas comme il faut :\n`)
  for (const { cible, souci } of soucis) console.error(`  ${cible}\n    → ${souci}`)
  console.error(
    "\nL'installation du service worker échouerait EN ENTIER : aucun téléphone ne garderait " +
      "l'application, et rien ne le dirait.",
  )
  process.exit(1)
}

console.info(`Guide Négo — ${liste.length} adresses de la garde servies en 200, sans redirection (${racine}).`)

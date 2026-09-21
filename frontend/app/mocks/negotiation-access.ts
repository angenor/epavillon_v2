/**
 * L'admission, sans API — l'état d'accès et les issues d'un code.
 *
 * **Le jeu sert deux choses seulement** : les tests, et le travail hors ligne.
 * L'application branchée lit le module `negotiation`, qui seul connaît l'état
 * réel du RBAC.
 *
 * LE MESSAGE EST ÉCRIT ICI COMME L'API L'ÉCRIT, et pour la même raison : c'est
 * elle qui compose, jamais l'écran. Un texte différent de part et d'autre
 * donnerait deux comportements pour un même refus, et le second ne recevrait
 * jamais les corrections du premier.
 *
 * Rien ici ne nomme ni ne suppose un genre (SC-006).
 */

import type { AccessStateView, RedeemResult } from '~/types/negotiation'

/** Les deux codes du semis de développement, et eux seuls. */
const CODE_DU_RESEAU = 'NEGO001'
const CODE_GENERAL = 'NEGO002'

const RESEAU = {
  code: 'women_negotiators',
  label: 'Réseau des négociatrices francophones',
}

/** Une visiteuse connectée qui n'a pas encore saisi de code. */
export const accesVisiteuse: AccessStateView = {
  admission_mode: 'code',
  state: 'visitor',
  granted: null,
  networks: [],
  request: null,
}

/**
 * Ce que le jeu retient entre deux appels : sans lui, un code accepté
 * n'ouvrirait rien et l'écran suivant redemanderait le code.
 */
let etatCourant: AccessStateView = { ...accesVisiteuse }

export function monAcces(): AccessStateView {
  return { ...etatCourant, networks: [...etatCourant.networks] }
}

export function reinitialiserLAcces(): void {
  etatCourant = { ...accesVisiteuse }
}

/** La même normalisation que la colonne engendrée de la base. */
function normaliser(saisi: string): string {
  return saisi.replace(/[^A-Za-z0-9]/g, '').toUpperCase()
}

export function saisirUnCode(code: string): RedeemResult {
  const normalise = normaliser(code)
  const reseau = normalise === CODE_DU_RESEAU
  const general = normalise === CODE_GENERAL

  if (!reseau && !general) {
    return {
      issue: 'unknown',
      message: "Ce code n'est pas reconnu. Vérifiez les huit caractères, tirets compris.",
      revoked_at: null,
      retry_after_seconds: null,
      granted: null,
      networks: [],
      request: null,
    }
  }

  const deja = etatCourant.state === 'granted'
  const rejoint = reseau && !etatCourant.networks.some((n) => n.code === RESEAU.code)

  etatCourant = {
    admission_mode: 'code',
    state: 'granted',
    granted: etatCourant.granted ?? {
      scope: reseau
        ? { type: 'negotiation_space', id: '0199a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b', name: 'COP31 — Climat' }
        : { type: 'global', id: null, name: null },
      granted_at: new Date().toISOString(),
      source_code_label: reseau ? 'Réseau des négociatrices — COP31' : 'Négociateurs francophones',
    },
    networks: rejoint ? [...etatCourant.networks, RESEAU] : [...etatCourant.networks],
    request: null,
  }

  if (deja) {
    return {
      issue: 'already_granted',
      message: rejoint
        ? `Vous avez déjà l'accès. L'appartenance au ${RESEAU.label} vient de s'y ajouter.`
        : "Vous avez déjà l'accès aux modules réservés.",
      revoked_at: null,
      retry_after_seconds: null,
      granted: etatCourant.granted,
      networks: [...etatCourant.networks],
      request: null,
    }
  }

  return {
    issue: 'accepted',
    message: reseau
      ? `Code reconnu. Bienvenue dans le ${RESEAU.label}.`
      : 'Code reconnu. Les modules réservés sont ouverts.',
    revoked_at: null,
    retry_after_seconds: null,
    granted: etatCourant.granted,
    networks: [...etatCourant.networks],
    request: null,
  }
}

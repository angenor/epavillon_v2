/**
 * LE BACK-OFFICE DE L'ADMISSION, sans API — codes, usages, demandes, mode.
 *
 * **Le jeu sert deux choses seulement** : les tests, et le travail hors ligne.
 * Branché, l'écran lit le crate `negotiation`, qui seul connaît l'état réel du
 * RBAC.
 *
 * L'ÉTAT D'UN CODE EST DÉRIVÉ ICI COMME LA VUE LE DÉRIVE — révoqué, terminé,
 * pas encore ouvert, épuisé, actif, et dans cet ordre. Le figer dans les
 * données ferait diverger la liste de ce que l'API refuse à l'application au
 * même instant.
 *
 * RÉVOQUER N'EST PAS RETIRER (ADR-006). Révoquer ferme la porte ; les accès
 * déjà accordés tiennent, et `granted_uses` dit combien. C'est la distinction
 * que ces données doivent rendre visible à qui essaie l'écran hors ligne.
 */

import type {
  AccessRequestQueue,
  AccessRequestRow,
  AdmissionSettings,
  CreateInvitationCodePayload,
  InvitationCodeDetail,
  InvitationCodeListScreen,
  InvitationCodeRow,
  InvitationCodeState,
  InvitationCodeUseRow,
  InvitationCodeUsesScreen,
  NetworkTermView,
  RevokeAllAccessResult,
  SpaceOption,
} from '~/types/admin-negotiation'
import type { AdmissionMode } from '~/types/negotiation'

const ESPACE_COP31 = '0199a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b'

const ESPACES: SpaceOption[] = [{ id: ESPACE_COP31, name: 'COP31 — Climat' }]

const RESEAU: NetworkTermView = {
  id: '0199a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a60',
  code: 'women_negotiators',
  label: 'Réseau des négociatrices francophones',
}

function ilYA(jours: number): string {
  return new Date(Date.now() - jours * 86_400_000).toISOString()
}

function dans(jours: number): string {
  return new Date(Date.now() + jours * 86_400_000).toISOString()
}

/** La même expression que `negotiation.v_invitation_codes`, dans le même ordre. */
function etat(code: InvitationCodeRow): InvitationCodeState {
  if (code.revoked_at) return 'revoked'
  if (code.valid_until && code.valid_until <= new Date().toISOString()) return 'expired'
  if (code.valid_from > new Date().toISOString()) return 'not_yet_valid'
  if (code.max_uses !== null && code.used_count >= code.max_uses) return 'exhausted'
  return 'active'
}

function code(partiel: Partial<InvitationCodeRow> & Pick<InvitationCodeRow, 'id' | 'code' | 'label'>): InvitationCodeRow {
  const complet: InvitationCodeRow = {
    state: 'active',
    scope: { type: 'negotiation_space', id: ESPACE_COP31, name: 'COP31 — Climat' },
    network: null,
    used_count: 0,
    max_uses: null,
    valid_from: ilYA(30),
    valid_until: null,
    revoked_at: null,
    revoked_reason: null,
    revoked_by_name: null,
    created_at: ilYA(30),
    created_by_name: 'Équipe IFDD',
    ...partiel,
  }
  return { ...complet, state: etat(complet) }
}

let codes: InvitationCodeRow[] = [
  code({
    id: '0199b001-0000-7000-8000-000000000001',
    code: 'NEGO-001',
    label: 'Réseau des négociatrices — COP31',
    network: RESEAU,
    used_count: 37,
    max_uses: 120,
    valid_until: dans(40),
  }),
  code({
    id: '0199b001-0000-7000-8000-000000000002',
    code: 'NEGO-002',
    label: 'Négociateurs francophones — accès général',
    scope: { type: 'global', id: null, name: null },
    used_count: 214,
  }),
  code({
    id: '0199b001-0000-7000-8000-000000000003',
    code: 'PREP-024',
    label: 'Atelier préparatoire de Dakar',
    used_count: 18,
    max_uses: 18,
  }),
  code({
    id: '0199b001-0000-7000-8000-000000000004',
    code: 'FUIT-009',
    label: 'Ancien code du groupe WhatsApp',
    used_count: 61,
    revoked_at: ilYA(3),
    revoked_reason: 'Code diffusé hors du groupe',
    revoked_by_name: 'Équipe IFDD',
  }),
  code({
    id: '0199b001-0000-7000-8000-000000000005',
    code: 'COP32-01',
    label: 'Pré-ouverture COP32',
    valid_from: dans(60),
    created_at: ilYA(2),
  }),
]

/** Les usages, par code. L'accès retiré garde sa date et son motif. */
const usages: Record<string, InvitationCodeUseRow[]> = {
  '0199b001-0000-7000-8000-000000000001': [
    {
      person_id: '0199b002-0000-7000-8000-000000000001',
      display_name: 'Awa Diallo',
      email: 'awa.diallo@example.org',
      used_at: ilYA(12),
      access_active: true,
      access_revoked_at: null,
      access_revoked_reason: null,
    },
    {
      person_id: '0199b002-0000-7000-8000-000000000002',
      display_name: 'Fatou Sow',
      email: 'fatou.sow@example.org',
      used_at: ilYA(9),
      access_active: true,
      access_revoked_at: null,
      access_revoked_reason: null,
    },
    {
      person_id: '0199b002-0000-7000-8000-000000000003',
      display_name: 'Mariam Traoré',
      email: 'mariam.traore@example.org',
      used_at: ilYA(6),
      access_active: false,
      access_revoked_at: ilYA(1),
      access_revoked_reason: 'Sortie du réseau',
    },
  ],
  '0199b001-0000-7000-8000-000000000004': [
    {
      person_id: '0199b002-0000-7000-8000-000000000004',
      display_name: 'Kofi Mensah',
      email: 'kofi.mensah@example.org',
      used_at: ilYA(20),
      access_active: true,
      access_revoked_at: null,
      access_revoked_reason: null,
    },
  ],
}

let demandes: AccessRequestRow[] = [
  {
    id: '0199b003-0000-7000-8000-000000000001',
    status: 'pending',
    person_id: '0199b002-0000-7000-8000-000000000005',
    display_name: 'Binta Ba',
    email: 'binta.ba@example.org',
    country: 'Sénégal',
    scope: { type: 'negotiation_space', id: ESPACE_COP31, name: 'COP31 — Climat' },
    invitation_code: 'NEGO-001',
    invitation_code_label: 'Réseau des négociatrices — COP31',
    message: null,
    submitted_at: ilYA(1),
    decided_at: null,
    decided_by_name: null,
    decision_reason: null,
  },
  {
    id: '0199b003-0000-7000-8000-000000000002',
    status: 'pending',
    person_id: '0199b002-0000-7000-8000-000000000006',
    display_name: 'Jean-Baptiste Koffi',
    email: 'jb.koffi@example.org',
    country: "Côte d'Ivoire",
    scope: { type: 'global', id: null, name: null },
    invitation_code: null,
    invitation_code_label: null,
    message: "Je participe à l'atelier préparatoire de Dakar.",
    submitted_at: ilYA(2),
    decided_at: null,
    decided_by_name: null,
    decision_reason: null,
  },
  {
    id: '0199b003-0000-7000-8000-000000000003',
    status: 'rejected',
    person_id: '0199b002-0000-7000-8000-000000000007',
    display_name: 'Test Test',
    email: 'test@example.org',
    country: null,
    scope: { type: 'global', id: null, name: null },
    invitation_code: null,
    invitation_code_label: null,
    message: null,
    submitted_at: ilYA(8),
    decided_at: ilYA(7),
    decided_by_name: 'Équipe IFDD',
    decision_reason: 'Compte non vérifié',
  },
]

let mode: AdmissionMode = 'code'

/** Les trois valeurs, avec ce que chacune produit — en faits, jamais en phrases. */
function options(): AdmissionSettings['options'] {
  return (['code', 'approval', 'code_and_approval'] as const).map((valeur) => ({
    mode: valeur,
    offers_code: valeur !== 'approval',
    code_opens: valeur === 'code',
    needs_approval: valeur !== 'code',
  }))
}

// ---------------------------------------------------------------------------
// Ce que les écrans lisent
// ---------------------------------------------------------------------------

export function codesDInvitation(filtres?: {
  etat?: string
  espace?: string
  q?: string
}): InvitationCodeListScreen {
  let lignes = codes.map((c) => ({ ...c, state: etat(c) }))

  if (filtres?.etat) lignes = lignes.filter((c) => c.state === filtres.etat)
  if (filtres?.espace === 'global') lignes = lignes.filter((c) => c.scope.type === 'global')
  else if (filtres?.espace) lignes = lignes.filter((c) => c.scope.id === filtres.espace)

  if (filtres?.q) {
    const cherche = filtres.q.toLowerCase()
    const normalise = filtres.q.replace(/[^A-Za-z0-9]/g, '').toUpperCase()
    lignes = lignes.filter(
      (c) =>
        c.label.toLowerCase().includes(cherche) ||
        c.code.replace(/[^A-Za-z0-9]/g, '').includes(normalise),
    )
  }

  return {
    rows: lignes,
    total: lignes.length,
    spaces: ESPACES,
    // Le compte des appartenances n'est pas la somme des usages : trois personnes
    // sont entrées par deux codes différents du même réseau.
    networks: [{ ...RESEAU, members_count: 14 }],
  }
}

export function codeDInvitation(codeId: string): InvitationCodeDetail | null {
  const ligne = codes.find((c) => c.id === codeId)
  if (!ligne) return null
  return {
    ...ligne,
    state: etat(ligne),
    granted_uses: (usages[codeId] ?? []).filter((u) => u.access_active).length,
  }
}

export function usagesDuCode(codeId: string): InvitationCodeUsesScreen {
  const lignes = usages[codeId] ?? []
  return {
    rows: [...lignes].sort((a, b) => Number(b.access_active) - Number(a.access_active)),
    total: lignes.length,
    granted_uses: lignes.filter((u) => u.access_active).length,
  }
}

/** Le code est engendré : huit caractères tirets compris, sans `0/O` ni `1/I/L`. */
export function creerUnCode(charge: CreateInvitationCodePayload): InvitationCodeRow {
  const alphabet = '23456789ABCDEFGHJKMNPQRSTUVWXYZ'
  const tire = Array.from({ length: 7 }, () =>
    alphabet[Math.floor(Math.random() * alphabet.length)],
  ).join('')

  const espace = charge.scope.type === 'negotiation_space' ? charge.scope.id : null
  const neuf = code({
    id: `0199b001-0000-7000-8000-${Date.now().toString(16).padStart(12, '0').slice(-12)}`,
    code: `${tire.slice(0, 4)}-${tire.slice(4)}`,
    label: charge.label,
    scope:
      espace === null
        ? { type: 'global', id: null, name: null }
        : { type: 'negotiation_space', id: espace, name: ESPACES.find((e) => e.id === espace)?.name ?? null },
    network: charge.grants_network === RESEAU.code ? RESEAU : null,
    max_uses: charge.max_uses ?? null,
    valid_from: charge.valid_from ?? new Date().toISOString(),
    valid_until: charge.valid_until ?? null,
    created_at: new Date().toISOString(),
  })

  codes = [neuf, ...codes]
  return neuf
}

/** Révoquer ne retire aucun accès : `granted_uses` ne bouge pas. */
export function revoquerUnCode(codeId: string, motif?: string | null): InvitationCodeRow | null {
  const ligne = codes.find((c) => c.id === codeId)
  if (!ligne) return null
  if (!ligne.revoked_at) {
    ligne.revoked_at = new Date().toISOString()
    ligne.revoked_reason = motif ?? null
    ligne.revoked_by_name = 'Équipe IFDD'
  }
  ligne.state = etat(ligne)
  return { ...ligne }
}

export function retirerUnAcces(codeId: string, personId: string, motif?: string | null): RevokeAllAccessResult {
  const ligne = (usages[codeId] ?? []).find((u) => u.person_id === personId)
  if (!ligne || !ligne.access_active) return { revoked: 0 }
  ligne.access_active = false
  ligne.access_revoked_at = new Date().toISOString()
  ligne.access_revoked_reason = motif ?? null
  return { revoked: 1 }
}

export function retirerTousLesAcces(codeId: string, motif?: string | null): RevokeAllAccessResult {
  const ouverts = (usages[codeId] ?? []).filter((u) => u.access_active)
  for (const ligne of ouverts) {
    ligne.access_active = false
    ligne.access_revoked_at = new Date().toISOString()
    ligne.access_revoked_reason = motif ?? null
  }
  return { revoked: ouverts.length }
}

export function fileDesDemandes(etatDemande?: string): AccessRequestQueue {
  const lignes = etatDemande ? demandes.filter((d) => d.status === etatDemande) : demandes
  return {
    rows: [...lignes].sort(
      (a, b) =>
        Number(b.status === 'pending') - Number(a.status === 'pending') ||
        a.submitted_at.localeCompare(b.submitted_at),
    ),
    total: lignes.length,
    pending: demandes.filter((d) => d.status === 'pending').length,
  }
}

export function trancherUneDemande(
  requestId: string,
  issue: 'approved' | 'rejected',
  motif?: string | null,
): void {
  demandes = demandes.map((d) =>
    d.id === requestId && d.status === 'pending'
      ? {
          ...d,
          status: issue,
          decided_at: new Date().toISOString(),
          decided_by_name: 'Équipe IFDD',
          decision_reason: motif ?? null,
        }
      : d,
  )
}

export function modeDAdmission(): AdmissionSettings {
  return { mode, options: options() }
}

export function changerLeModeDAdmission(valeur: AdmissionMode): AdmissionSettings {
  mode = valeur
  return { mode, options: options() }
}

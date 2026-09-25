/**
 * LE BACK-OFFICE DE L'ADMISSION — sa part de `useApi()`.
 *
 * Même motif qu'`api/admin-incidents.ts` : les pages appellent
 * `api.adminNegotiations.codes(…)`, n'importent aucun mock et n'appellent
 * jamais `$fetch`.
 *
 * ── PAS DE PÉRIMÈTRE D'ÉDITION ICI, ET C'EST LE POINT ───────────────────────
 *
 * Partout ailleurs, une liste du back-office passe par `assertEventInScope` :
 * un administrateur d'édition ne voit que la sienne. Guide Négo ne fonctionne
 * pas ainsi — **ce back-office exige la portée globale**, et rien d'autre
 * (FR-044, tranché le 21/09). Un espace de négociation n'est rattaché à aucune
 * édition : filtrer par édition n'aurait rien à filtrer, et laisserait croire à
 * une garde là où il n'y en aurait pas.
 *
 * L'API refuse donc en 403 avant toute lecture, et la page affiche
 * `UiForbiddenState` plutôt qu'une liste vide — « aucun code » et « ceci ne
 * vous regarde pas » ne se disent pas de la même façon.
 *
 * ── RÉVOQUER ET RETIRER SONT DEUX MÉTHODES ──────────────────────────────────
 *
 * Parce que ce sont deux décisions (ADR-006). `revoquerLeCode` ferme la porte
 * sans toucher aux accès accordés ; `retirerUnAcces` et `retirerTousLesAcces`
 * sortent des personnes sans invalider le code. Les fondre en une seule
 * méthode ferait de chaque fuite une exclusion collective.
 */

import type {
  AccessRequestQueue,
  AdmissionSettings,
  CreateInvitationCodePayload,
  InvitationCodeDetail,
  InvitationCodeListScreen,
  InvitationCodeRow,
  InvitationCodeUsesScreen,
  RevokeAllAccessResult,
} from '~/types/admin-negotiation'
import type { AdmissionMode } from '~/types/negotiation'
import type {
  AgendaItemAdmin,
  OfficialImportAdmin,
  UpdateOfficialImportPayload,
} from '~/types/negotiation-sessions'
import type { Uuid } from '~/types/shared'
import type { ApiTransport } from './proposal-review'

type Deps = Pick<ApiTransport, 'call' | 'callOrNull' | 'send'>

const exemplesDeLImport = () => import('~/mocks/admin-negotiation-import')

/** Les filtres d'URL de la liste, **nommés en français** comme l'API les lit. */
export interface FiltresDeCodes {
  etat?: string
  /** Un identifiant d'espace, ou le mot `global`. */
  espace?: string
  q?: string
}

export function createAdminNegotiationsApi({ call, callOrNull, send }: Deps) {
  return {
    /**
     * LA LISTE ET SES RÉFÉRENTIELS — en une réponse.
     *
     * `spaces` et `networks` viennent avec : le formulaire de création s'en
     * sert aussi, et deux appels distincts finiraient par proposer des portées
     * différentes d'un écran à l'autre.
     */
    codes: (filtres: FiltresDeCodes = {}): Promise<InvitationCodeListScreen> =>
      call('/admin/negotiation/invitation-codes', (m) => m.codesDInvitation(filtres), {
        etat: filtres.etat || undefined,
        espace: filtres.espace || undefined,
        q: filtres.q || undefined,
      }),

    /** UNE FICHE. `null` si le code n'existe pas — ou si la garde a refusé. */
    code: (codeId: Uuid): Promise<InvitationCodeDetail | null> =>
      callOrNull(`/admin/negotiation/invitation-codes/${codeId}`, (m) => m.codeDInvitation(codeId)),

    /** Qui est entré, quand, et si son accès tient encore. */
    usages: (codeId: Uuid): Promise<InvitationCodeUsesScreen> =>
      call(`/admin/negotiation/invitation-codes/${codeId}/uses`, (m) => m.usagesDuCode(codeId)),

    /**
     * CRÉER. **Le code n'est pas choisi** : l'API l'engendre et le rend, pour
     * qu'il soit diffusé aussitôt dans le groupe WhatsApp.
     */
    creerUnCode: (charge: CreateInvitationCodePayload): Promise<InvitationCodeRow> =>
      send('/admin/negotiation/invitation-codes', charge, (m) => m.creerUnCode(charge)),

    /** RÉVOQUER — **ne retire aucun accès déjà accordé** (ADR-006, FR-039). */
    revoquerLeCode: (codeId: Uuid, motif?: string | null): Promise<InvitationCodeRow> =>
      send(
        `/admin/negotiation/invitation-codes/${codeId}/revoke`,
        { reason: motif ?? null },
        (m) => m.revoquerUnCode(codeId, motif) as InvitationCodeRow,
      ),

    /** RETIRER L'ACCÈS d'une personne. Le code, lui, reste ce qu'il est. */
    retirerUnAcces: (codeId: Uuid, personId: Uuid, motif?: string | null): Promise<RevokeAllAccessResult> =>
      send(
        `/admin/negotiation/invitation-codes/${codeId}/uses/${personId}/revoke-access`,
        { reason: motif ?? null },
        (m) => m.retirerUnAcces(codeId, personId, motif),
      ),

    /** RETIRER TOUS LES ACCÈS d'un code — le geste d'un code compromis. */
    retirerTousLesAcces: (codeId: Uuid, motif?: string | null): Promise<RevokeAllAccessResult> =>
      send(
        `/admin/negotiation/invitation-codes/${codeId}/revoke-all-access`,
        { reason: motif ?? null },
        (m) => m.retirerTousLesAcces(codeId, motif),
      ),

    /** LA FILE des demandes. `pending` ne suit pas le filtre : c'est la pastille. */
    demandes: (etat?: string): Promise<AccessRequestQueue> =>
      call('/admin/negotiation/access-requests', (m) => m.fileDesDemandes(etat), {
        etat: etat || undefined,
      }),

    /** ADMETTRE : l'état, l'accès, le réseau et le courriel, en une transaction. */
    admettre: (requestId: Uuid, motif?: string | null): Promise<void> =>
      send(
        `/admin/negotiation/access-requests/${requestId}/approve`,
        { reason: motif ?? null },
        (m) => m.trancherUneDemande(requestId, 'approved', motif),
      ),

    /** REFUSER, motif facultatif — repris **tel quel** dans le courriel. */
    refuser: (requestId: Uuid, motif?: string | null): Promise<void> =>
      send(
        `/admin/negotiation/access-requests/${requestId}/reject`,
        { reason: motif ?? null },
        (m) => m.trancherUneDemande(requestId, 'rejected', motif),
      ),

    /** LE MODE D'ADMISSION, et ce que chaque valeur produit. */
    admission: (): Promise<AdmissionSettings> =>
      call('/admin/negotiation/admission', (m) => m.modeDAdmission()),

    /** LA BASCULE. **Prend effet à la tentative suivante**, sans mise en ligne. */
    changerLeMode: (mode: AdmissionMode): Promise<AdmissionSettings> =>
      send('/admin/negotiation/admission', { mode }, (m) => m.changerLeModeDAdmission(mode), 'PUT'),

    /** L'IMPORT DES SESSIONS OFFICIELLES d'une édition : réglage, santé, journal. */
    importOfficiel: (edition: string): Promise<OfficialImportAdmin> =>
      call('/admin/negotiation/import', async () => (await exemplesDeLImport()).etatDeLImport(edition), {
        edition,
      }),

    /** Le réglage entier. **Allumer pose la première lecture** ; éteindre coupe aussitôt. */
    reglerLImport: (edition: string, reglage: UpdateOfficialImportPayload): Promise<OfficialImportAdmin> =>
      send(
        `/admin/negotiation/import?edition=${encodeURIComponent(edition)}`,
        reglage,
        async () => (await exemplesDeLImport()).reglerLImport(edition, reglage),
        'PUT',
      ),

    /** « LIRE MAINTENANT » : une lecture de plus, sans replanifier la chaîne. */
    lireMaintenant: (edition: string): Promise<void> =>
      send(`/admin/negotiation/import/read?edition=${encodeURIComponent(edition)}`, {}, async () => {
        await exemplesDeLImport()
      }),

    /** L'ORDRE DU JOUR : les points sans thématique d'abord. */
    pointsDeLOrdreDuJour: (edition: string): Promise<AgendaItemAdmin[]> =>
      call('/admin/negotiation/agenda-items', async () => (await exemplesDeLImport()).points(edition), {
        edition,
      }),

    /** RATTACHER un point à une thématique, ou l'en détacher (`null`). */
    rattacherUnPoint: (pointId: Uuid, theme: string | null): Promise<AgendaItemAdmin> =>
      send(
        `/admin/negotiation/agenda-items/${pointId}`,
        { theme },
        async () => (await exemplesDeLImport()).rattacher(pointId, theme),
        'PUT',
      ),
  }
}

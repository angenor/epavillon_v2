import type { Mailing } from '~/types/admin-mailings'

/**
 * « Envoi parti », « Problème sur l'envoi » : la notification brève de fin
 * d'envoi, à la seule personne qui l'a lancé, à la fin ou au problème
 * (décisions du 31/10). Montée une fois par `layouts/admin.vue`.
 *
 * Le signal `person:<soi>` ne dit pas quel envoi (il ne porte aucune donnée) :
 * ce navigateur retient ceux qu'il a lancés et relit leur fiche. Un envoi fini
 * pendant que l'onglet dormait se retrouve au retour ou au prochain
 * chargement : en retard, pas perdu. L'envoi quitte la liste avant d'être
 * annoncé : un seul onglet l'annonce, une seule fois, page de l'envoi ouverte
 * ou non.
 */

const PREFIX = 'epavillon:mailings-launched:'
/** Au-delà, un envoi oublié ne s'annonce plus : la nouvelle serait périmée. */
const MAX_AGE_MS = 3 * 24 * 3600 * 1000

interface Launched {
  id: string
  at: number
}

function keyOf(personId: string): string {
  return PREFIX + personId
}

function read(personId: string): Launched[] {
  try {
    const raw = localStorage.getItem(keyOf(personId))
    const parsed: unknown = raw ? JSON.parse(raw) : []
    if (!Array.isArray(parsed)) return []
    const now = Date.now()
    return parsed.filter(
      (e): e is Launched => typeof e?.id === 'string' && typeof e?.at === 'number' && now - e.at < MAX_AGE_MS,
    )
  } catch {
    return []
  }
}

function write(personId: string, list: Launched[]): void {
  try {
    if (list.length) localStorage.setItem(keyOf(personId), JSON.stringify(list))
    else localStorage.removeItem(keyOf(personId))
  } catch {
    // Stockage refusé : la notification ne viendra pas, la liste le dit.
  }
}

/** Retire l'envoi ; faux s'il n'y était plus (un autre onglet l'a annoncé). */
function forget(personId: string, id: string): boolean {
  const list = read(personId)
  const kept = list.filter((e) => e.id !== id)
  if (kept.length === list.length) return false
  write(personId, kept)
  return true
}

/** À appeler après « Envoyer » et « Relancer », par qui a cliqué. */
export function trackLaunchedMailing(mailingId: string): void {
  if (!import.meta.client) return
  const personId = useAuthStore().person?.id
  if (!personId) return
  const list = read(personId).filter((e) => e.id !== mailingId)
  write(personId, [...list, { id: mailingId, at: Date.now() }])
}

export function useMailingNotices(): void {
  if (import.meta.server) return
  const auth = useAuthStore()
  const api = useApi()
  const toast = useToast()
  const { t, locale } = useI18n()
  const localePath = useLocalePath()
  const checking = new Set<string>()

  function announce(m: Mailing): void {
    const subject = m.subject.trim() || t('admin.mailing.common.noSubject')
    const link = { to: localePath(`/admin/courriels/${m.id}`), label: t('admin.mailing.common.notice.open') }
    if (m.status === 'failed') {
      toast.error(t('admin.mailing.common.notice.failed', { subject }), {
        message: m.failure_reason ?? t('admin.mailing.common.notice.failedUnknown'),
        link,
      })
      return
    }
    const sent = m.counts.sent
    const parts = [t('admin.mailing.common.notice.subject', { subject })]
    if (m.counts.failed) parts.push(t('admin.mailing.common.notice.someFailed', { count: m.counts.failed }, m.counts.failed))
    toast.success(
      t('admin.mailing.common.notice.sent', { count: new Intl.NumberFormat(locale.value).format(sent) }, sent),
      { message: parts.join(' · '), link },
    )
  }

  async function check(): Promise<void> {
    const personId = auth.person?.id
    if (!personId) return
    for (const { id } of read(personId)) {
      if (checking.has(id)) continue
      checking.add(id)
      try {
        const m = await api.adminMailings.byId(id)
        if (m?.status === 'queued') continue
        if (!forget(personId, id)) continue
        if (m && (m.status === 'sent' || m.status === 'failed')) announce(m)
      } catch {
        // API muette : le prochain signal, ou le prochain chargement, rattrape.
      } finally {
        checking.delete(id)
      }
    }
  }

  useRealtime(() => (auth.person ? [`person:${auth.person.id}`] : []), check)
  onMounted(() => void check())
  watch(
    () => auth.person?.id,
    () => void check(),
  )
}

import type { Intent } from '~/types/ui'

/**
 * Notifications brèves : « Envoi parti », « Problème d'envoi ». Montées par
 * `UiToastRegion` dans les layouts du back-office et du site.
 *
 * Une bonne nouvelle (succès, information) disparaît seule après 6 s ; un
 * problème (avertissement, échec) reste jusqu'à ce qu'on le ferme. L'état ne
 * tient que des objets simples (règle du payload de rendu serveur) ; les
 * minuteries vivent hors de l'état, dans le navigateur seulement.
 */

export type ToastIntent = Exclude<Intent, 'neutral'>

export interface ToastLink {
  to: string
  label: string
}

export interface Toast {
  id: number
  intent: ToastIntent
  title: string
  message?: string
  link?: ToastLink
  /** Millisecondes avant disparition ; nul : reste jusqu'à fermeture. */
  duration: number | null
}

export type ToastInput = Omit<Toast, 'id' | 'duration'> & { duration?: number | null }

export const TOAST_DURATION_MS = 6000
/** Au-delà, la plus ancienne part : une pile qui déborde ne se lit plus. */
const TOAST_MAX = 4

const timers = new Map<number, ReturnType<typeof setTimeout>>()
let nextId = 1

export function useToast() {
  const toasts = useState<Toast[]>('ui:toasts', () => [])

  function dismiss(id: number): void {
    const timer = timers.get(id)
    if (timer !== undefined) clearTimeout(timer)
    timers.delete(id)
    toasts.value = toasts.value.filter((toast) => toast.id !== id)
  }

  /** Relance le compte à rebours d'une notification qui s'efface seule. */
  function arm(id: number): void {
    if (!import.meta.client) return
    const toast = toasts.value.find((t) => t.id === id)
    if (!toast || toast.duration === null) return
    const previous = timers.get(id)
    if (previous !== undefined) clearTimeout(previous)
    timers.set(id, setTimeout(() => dismiss(id), toast.duration))
  }

  /** Survol ou focus : on ne retire pas un message pendant qu'on le lit. */
  function hold(id: number): void {
    const timer = timers.get(id)
    if (timer !== undefined) clearTimeout(timer)
    timers.delete(id)
  }

  function show(input: ToastInput): number {
    const id = nextId++
    const persists = input.intent === 'danger' || input.intent === 'warning'
    const toast: Toast = {
      ...input,
      id,
      duration: input.duration === undefined ? (persists ? null : TOAST_DURATION_MS) : input.duration,
    }
    const kept = toasts.value.slice(-(TOAST_MAX - 1))
    for (const dropped of toasts.value.filter((t) => !kept.includes(t))) hold(dropped.id)
    toasts.value = [...kept, toast]
    arm(id)
    return id
  }

  return {
    toasts: readonly(toasts),
    show,
    success: (title: string, rest: Omit<ToastInput, 'intent' | 'title'> = {}) => show({ ...rest, intent: 'success', title }),
    error: (title: string, rest: Omit<ToastInput, 'intent' | 'title'> = {}) => show({ ...rest, intent: 'danger', title }),
    info: (title: string, rest: Omit<ToastInput, 'intent' | 'title'> = {}) => show({ ...rest, intent: 'info', title }),
    dismiss,
    hold,
    arm,
  }
}

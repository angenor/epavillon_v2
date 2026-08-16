<script setup lang="ts">
import type { IncidentScope, IncidentSeverity } from '~/types/live'
import type { I18nText, Url } from '~/types/shared'
import type { Intent } from '~/types/ui'

/**
 * BANDEAU D'INCIDENT — motif transverse, publié depuis le back-office.
 *
 * Il rend une ligne de `live.active_incidents()` : la fonction remonte déjà la
 * hiérarchie (séance, journée, édition, organisation porteuse, plus le global)
 * et trie par gravité décroissante. Le front n'a donc AUCUN filtre de portée à
 * réimplémenter : il affiche ce qu'on lui donne, dans l'ordre reçu.
 *
 * QUATRE NIVEAUX DE GRAVITÉ, PAS TROIS. L'ENUM `live.incident_severity` en
 * déclare quatre — `info`, `warning`, `error`, `critical` — dans cet ordre,
 * volontairement croissant côté base pour que `ORDER BY severity DESC` remonte
 * le plus grave. Le modèle fait foi : les quatre sont rendus.
 *
 * `info` et `warning` sont refermables si l'incident le permet ; `error` et
 * `critical` ne le sont JAMAIS, quoi qu'en dise `is_dismissible`. Un incident
 * majeur qu'on peut chasser d'un clic est un incident qui ne sera pas lu — et le
 * bandeau est le seul canal dont dispose l'équipe pendant une COP.
 *
 * `role="alert"` à partir de `error` seulement : un bandeau d'information
 * présent au chargement ne doit pas interrompre la lecture.
 *
 * LE TEXTE VIENT DE LA BASE — `title` et `message` sont des `platform.i18n_text`,
 * résolus par `resolveI18nText()`. Ils ne passent PAS par les fichiers i18n :
 * c'est une donnée saisie par un administrateur, pas un libellé d'interface.
 * Seuls les libellés du composant lui-même (nom de la gravité, « masquer ») sont
 * traduits.
 */

interface Props {
  severity: IncidentSeverity
  /** Titre multilingue venu de la base. Facultatif — le message peut suffire. */
  title?: I18nText | null
  /** Message multilingue venu de la base. Obligatoire côté modèle. */
  message: I18nText
  /** Portée déclarée, affichée en repère (« toute la plateforme », « cette activité »). */
  scope?: IncidentScope
  actionUrl?: Url | null
  /** Libellé du lien d'action ; à défaut, un libellé générique. */
  actionLabel?: string
  /** Refermable — ignoré pour `error` et `critical`. */
  dismissible?: boolean
  /** Fin d'affichage annoncée, quand elle est connue. */
  displayUntil?: string | null
  /** Fuseau dans lequel afficher la date de fin. Requis avec `displayUntil`. */
  timezone?: string
  zoneLabel?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ dismiss: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

const isVisible = ref(true)

/** Gravité du modèle → intention d'interface. `error` et `critical` partagent le
 *  rouge ; c'est l'impossibilité de refermer et le trait renforcé qui les
 *  distinguent, pas une sixième couleur. */
const INTENTS: Record<IncidentSeverity, Intent> = {
  info: 'info',
  warning: 'warning',
  error: 'danger',
  critical: 'danger',
}

const ICONS: Record<IncidentSeverity, string> = {
  info: 'info',
  warning: 'warning',
  error: 'error',
  critical: 'ban',
}

const TONES: Record<IncidentSeverity, string> = {
  info: 'border-info-border bg-info-surface',
  warning: 'border-warning-border bg-warning-surface',
  error: 'border-danger-border bg-danger-surface',
  // Le niveau le plus grave se distingue par un trait de gauche épais, pas par
  // une couleur de plus : la palette n'a que quatre familles d'état.
  critical: 'border-danger-border bg-danger-surface border-l-4 border-l-danger-solid',
}

const ICON_TONES: Record<IncidentSeverity, string> = {
  info: 'text-info',
  warning: 'text-warning',
  error: 'text-danger',
  critical: 'text-danger',
}

/** Un incident grave ne se referme pas, quoi qu'en dise la base. */
const canDismiss = computed(
  () => Boolean(props.dismissible) && props.severity !== 'error' && props.severity !== 'critical',
)

const until = computed(() => {
  if (!props.displayUntil || !props.timezone) return null
  return t('incident-banner.until', { date: dateTime(props.displayUntil, props.timezone) })
})

function dismiss(): void {
  isVisible.value = false
  emit('dismiss')
}
</script>

<template>
  <div
    v-if="isVisible"
    :role="props.severity === 'info' ? 'note' : 'alert'"
    class="flex w-full gap-3 border-y px-4 py-3 sm:px-6"
    :class="TONES[props.severity]"
  >
    <UiIcon
      :name="ICONS[props.severity]"
      class="mt-0.5 shrink-0"
      :class="ICON_TONES[props.severity]"
      size="1.2rem"
    />

    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-center gap-2">
        <UiBadge :intent="INTENTS[props.severity]" size="sm" :solid="props.severity === 'critical'">
          {{ t(`incident-banner.severity.${props.severity}`) }}
        </UiBadge>
        <span v-if="props.scope" class="text-xs text-text-subtle">
          {{ t(`incident-banner.scope.${props.scope}`) }}
        </span>
      </div>

      <p v-if="props.title" class="mt-1.5 font-display text-base leading-snug text-text">
        {{ tr(props.title) }}
      </p>
      <p class="text-sm text-text-muted" :class="props.title ? 'mt-0.5' : 'mt-1.5'">
        {{ tr(props.message) }}
      </p>

      <div v-if="props.actionUrl || until" class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1">
        <UiButton
          v-if="props.actionUrl"
          variant="link"
          size="sm"
          :href="props.actionUrl"
          icon-trailing="external-link"
        >
          {{ props.actionLabel ?? t('incident-banner.action') }}
        </UiButton>
        <span v-if="until" class="text-xs text-text-subtle">{{ until }}</span>
      </div>
    </div>

    <button
      v-if="canDismiss"
      type="button"
      class="-mt-1 -mr-1 shrink-0 self-start rounded-md p-1.5 text-text-subtle transition-colors hover:bg-surface-hover hover:text-text"
      @click="dismiss"
    >
      <span class="sr-only">{{ t('incident-banner.dismiss') }}</span>
      <UiIcon name="close" size="1rem" />
    </button>
  </div>
</template>

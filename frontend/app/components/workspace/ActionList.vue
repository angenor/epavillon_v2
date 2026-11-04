<script setup lang="ts">
import type { WorkspaceAction } from '~/types/organization-workspace'
import type { TimeZoneName } from '~/types/shared'

/**
 * CE QUI ATTEND UNE ACTION DE L'ORGANISATION : la carte qui remonte sur le
 * bandeau (04/11), et celle qui justifie qu'on revienne sur la page.
 *
 * Rien que l'organisation ne puisse traiter : ce qu'attend le comité n'y figure
 * pas. Vide, elle reste lisible : « Rien ne vous attend » est une réponse, pas
 * une panne. Le nombre est dans la ligne (« 2 points à traiter »), et
 * l'échéance en gros chiffres condensés, colonne qu'on parcourt à la verticale.
 */

interface Props {
  actions: WorkspaceAction[]
  /** Fuseau d'affichage des échéances : celui de l'édition concernée. */
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { intlLocale } = useDateTime()
const localePath = useLocalePath()

const PRESENTATION = WORKSPACE_ACTION_PRESENTATION

const TONES: Record<'warning' | 'info', string> = {
  warning: 'border-l-warning bg-warning-surface/45',
  info: 'border-l-info bg-info-surface/45',
}

const ICON_TONES: Record<'warning' | 'info', string> = {
  warning: 'text-warning',
  info: 'text-info',
}

const detailOf = (action: WorkspaceAction): string => workspaceActionDetail(action, t)

/** « 8 nov. » : le jour et le mois suffisent à une échéance de la saison en cours. */
function shortDate(value: string): string {
  return new Intl.DateTimeFormat(intlLocale.value, { day: 'numeric', month: 'short', timeZone: props.timezone }).format(new Date(value))
}
</script>

<template>
  <WorkspacePanel
    heading-id="workspace-actions-title"
    :title="t('organization.workspace.actions.title')"
    :meta="t('organization.workspace.actions.count', props.actions.length)"
  >
    <div v-if="props.actions.length === 0" class="flex items-start gap-3 rounded-md bg-surface-sunken p-4">
      <UiIcon name="check-circle" class="mt-0.5 shrink-0 text-success" size="1.25rem" />
      <div>
        <p class="font-bold text-text">{{ t('organization.workspace.actions.empty.title') }}</p>
        <p class="mt-0.5 max-w-(--measure) text-sm text-text-muted">
          {{ t('organization.workspace.actions.empty.description') }}
        </p>
      </div>
    </div>

    <ul v-else class="flex flex-col gap-2">
      <li v-for="(action, index) in props.actions" :key="`${action.kind}-${action.proposal_id ?? index}`">
        <NuxtLink
          :to="localePath(action.target)"
          class="grid min-h-(--target-min) grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-4 gap-y-1 rounded-md border border-l-4 border-border p-3 text-text no-underline transition-colors duration-(--duration-fast) hover:border-border-strong hover:text-text sm:grid-cols-[auto_minmax(0,1fr)_auto_auto] sm:px-4 sm:py-3.5"
          :class="TONES[PRESENTATION[action.kind].tone]"
        >
          <UiIcon
            :name="PRESENTATION[action.kind].icon"
            class="mt-0.75 self-start"
            :class="ICON_TONES[PRESENTATION[action.kind].tone]"
            size="1.15rem"
          />

          <span class="min-w-0">
            <span class="flex flex-wrap items-baseline gap-x-2.5 gap-y-0.5">
              <b class="font-bold text-text">{{ t(`organization.workspace.actions.kind.${action.kind}.label`) }}</b>
              <span v-if="action.reference_code" class="text-[0.78rem] text-text-subtle tabular-nums">{{ action.reference_code }}</span>
            </span>
            <span class="mt-0.5 block text-[0.92rem] text-text-secondary sm:truncate">{{ action.subject }}</span>
            <span
              class="mt-0.5 block text-[0.85rem]"
              :class="action.kind === 'changes_requested' ? 'font-bold text-warning' : 'text-text-muted'"
            >
              {{ detailOf(action) }}
            </span>
          </span>

          <!-- L'échéance : la colonne qu'on parcourt pour savoir par quoi commencer. -->
          <span
            v-if="action.due_at"
            class="col-start-2 row-start-2 flex items-baseline gap-1.5 font-condensed leading-none sm:col-start-3 sm:row-start-1 sm:block sm:min-w-21 sm:text-right"
          >
            <small class="block font-sans text-[0.72rem] text-text-muted">{{ t('organization.workspace.actions.before') }}</small>
            <b class="text-[1.2rem] font-bold text-heading tabular-nums sm:text-[1.55rem]">{{ shortDate(action.due_at) }}</b>
          </span>
          <span v-else class="hidden sm:block" aria-hidden="true" />

          <UiIcon name="chevron-right" class="col-start-3 row-start-1 text-text-subtle sm:col-start-4" size="1.125rem" />
        </NuxtLink>
      </li>
    </ul>
  </WorkspacePanel>
</template>

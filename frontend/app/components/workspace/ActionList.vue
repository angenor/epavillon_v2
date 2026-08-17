<script setup lang="ts">
import type { WorkspaceAction } from '~/types/organization-workspace'
import type { TimeZoneName } from '~/types/shared'

/**
 * CE QUI ATTEND UNE ACTION DE L'ORGANISATION — le bloc le plus haut de l'écran,
 * et celui qui justifie qu'on y revienne.
 *
 * DEUX RÈGLES, ET ELLES SE TIENNENT :
 *
 *  1. RIEN QUE L'ORGANISATION NE PUISSE TRAITER. Ce qu'attend le comité — une
 *     revue en retard, une décision — n'a rien à faire ici. Une liste où l'on
 *     trouve ce qu'on ne peut pas faire cesse d'être lue, et c'est alors la
 *     ligne qui comptait qu'on rate.
 *  2. VIDE, ELLE RESTE LISIBLE. « Rien ne vous attend » est une réponse, pas une
 *     page cassée : c'est la même exigence que pour le tableau de bord du
 *     back-office, où un système qui va bien ne doit pas ressembler à une panne.
 *
 * LE NOMBRE EST DANS LA LIGNE, PAS DANS UNE PASTILLE À PART. « 3 points à
 * traiter » se lit d'un trait ; un « 3 » posé à côté d'un libellé oblige à
 * deviner ce qu'il compte.
 */

interface Props {
  actions: WorkspaceAction[]
  /** Fuseau d'affichage des échéances — celui de l'édition concernée. */
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { date } = useDateTime()
const localePath = useLocalePath()

/**
 * Chaque nature porte son icône et sa gravité. Le CODE vient du contrat
 * (`WorkspaceActionKind`), la couleur du rôle : jaune pour ce qui demande
 * attention, cyan pour ce qui informe. Rien en rouge — aucune de ces lignes
 * n'est un échec, ce sont des choses à faire.
 */
const PRESENTATION: Record<WorkspaceAction['kind'], { icon: string; tone: 'warning' | 'info' }> = {
  changes_requested: { icon: 'warning', tone: 'warning' },
  draft_before_deadline: { icon: 'edit', tone: 'warning' },
  coorganization_to_confirm: { icon: 'building', tone: 'info' },
  membership_request: { icon: 'users', tone: 'info' },
  session_report_missing: { icon: 'document', tone: 'info' },
}

const TONES: Record<'warning' | 'info', string> = {
  warning: 'border-l-warning bg-warning-surface/40',
  info: 'border-l-info bg-info-surface/40',
}

const ICON_TONES: Record<'warning' | 'info', string> = {
  warning: 'text-warning',
  info: 'text-info',
}

/** Une clé d'action peut avoir un détail au pluriel : seul le premier en a un. */
function detailOf(action: WorkspaceAction): string {
  const key = `organization.workspace.actions.kind.${action.kind}.detail`
  return action.kind === 'changes_requested' ? t(key, action.count) : t(key)
}
</script>

<template>
  <section aria-labelledby="workspace-actions-title">
    <div class="mb-4 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
      <h2 id="workspace-actions-title" class="text-xl font-semibold">
        {{ t('organization.workspace.actions.title') }}
      </h2>
      <p class="text-sm text-text-subtle">
        {{ t('organization.workspace.actions.count', props.actions.length) }}
      </p>
    </div>

    <!-- L'état vide n'est pas un état d'erreur : ni bordure rouge, ni glyphe
         d'alerte. Un encart calme qui dit que tout est à jour. -->
    <UiCard v-if="props.actions.length === 0" sunken>
      <div class="flex items-start gap-3">
        <UiIcon name="check" class="mt-0.5 shrink-0 text-success" size="1.25rem" />
        <div>
          <p class="font-semibold text-text">{{ t('organization.workspace.actions.empty.title') }}</p>
          <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
            {{ t('organization.workspace.actions.empty.description') }}
          </p>
        </div>
      </div>
    </UiCard>

    <ul v-else class="flex flex-col gap-2">
      <li v-for="(action, index) in props.actions" :key="`${action.kind}-${action.proposal_id ?? index}`">
        <NuxtLink
          :to="localePath(action.target)"
          class="flex min-h-(--target-min) items-start gap-3 rounded-md border border-border border-l-(length:--border-thick) px-4 py-3 no-underline transition-colors duration-(--duration-fast) hover:border-border-strong"
          :class="TONES[PRESENTATION[action.kind].tone]"
        >
          <UiIcon
            :name="PRESENTATION[action.kind].icon"
            class="mt-0.5 shrink-0"
            :class="ICON_TONES[PRESENTATION[action.kind].tone]"
            size="1.125rem"
          />

          <span class="min-w-0 flex-1">
            <span class="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
              <span class="font-semibold text-text">
                {{ t(`organization.workspace.actions.kind.${action.kind}.label`) }}
              </span>
              <span v-if="action.reference_code" class="font-mono text-xs text-text-subtle">
                {{ action.reference_code }}
              </span>
            </span>
            <span class="mt-0.5 block truncate text-sm text-text-secondary">{{ action.subject }}</span>
            <span class="mt-0.5 block text-sm text-text-muted">{{ detailOf(action) }}</span>
          </span>

          <!-- L'échéance à droite, en chiffres tabulaires : c'est la colonne
               qu'on parcourt à la verticale pour savoir par quoi commencer. -->
          <span
            v-if="action.due_at"
            class="hidden shrink-0 self-center text-sm tabular-nums text-text-secondary sm:block"
          >
            {{ t('organization.workspace.actions.dueOn', { date: date(action.due_at, props.timezone) }) }}
          </span>

          <UiIcon name="chevron-right" class="mt-0.5 shrink-0 text-text-subtle" size="1.125rem" />
        </NuxtLink>
      </li>
    </ul>
  </section>
</template>

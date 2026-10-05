<script setup lang="ts">
import type { WorkspaceAction } from '~/types/organization-workspace'
import type { TimeZoneName } from '~/types/shared'

/**
 * La première action en attente, en tête du menu du compte : c'est elle qui dit
 * à qui a déposé un dossier qu'on attend sa réponse. Les suivantes se comptent et
 * renvoient à l'espace, où `WorkspaceActionList` les donne toutes.
 */

interface Props {
  actions: WorkspaceAction[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { date } = useDateTime()
const localePath = useLocalePath()

// L'ordre de `WORKSPACE_ACTION_PRESENTATION` est celui de l'urgence : des corrections
// demandées passent avant un compte rendu attendu.
const PRIORITY = Object.keys(WORKSPACE_ACTION_PRESENTATION)
const first = computed(
  () => [...props.actions].sort((a, b) => PRIORITY.indexOf(a.kind) - PRIORITY.indexOf(b.kind))[0] ?? null,
)
const presentation = computed(() => (first.value ? WORKSPACE_ACTION_PRESENTATION[first.value.kind] : null))

const TONES = {
  warning: { caption: 'text-warning', card: 'border-warning-border bg-warning-surface', ink: 'text-warning' },
  info: { caption: 'text-info', card: 'border-info-border bg-info-surface', ink: 'text-info' },
} as const

const tone = computed(() => TONES[presentation.value?.tone ?? 'info'])
</script>

<template>
  <div v-if="first && presentation" class="px-4.5 pt-4 pb-1.5">
    <p class="text-[11.5px] font-bold uppercase" :class="tone.caption" :style="{ letterSpacing: 'var(--tracking-caps)' }">
      {{ t('nav.account.pending') }}
    </p>
    <NuxtLink
      :to="localePath(first.target)"
      role="menuitem"
      class="mt-2.5 flex gap-3 rounded-lg border px-3.5 py-3 no-underline transition-colors duration-(--duration-fast) hover:border-border-strong"
      :class="tone.card"
    >
      <UiIcon :name="presentation.icon" size="1.125rem" :stroke-width="1.8" class="mt-px shrink-0" :class="tone.ink" />
      <span class="min-w-0 flex-1">
        <span class="block text-sm font-bold" :class="tone.ink">
          {{ t(`organization.workspace.actions.kind.${first.kind}.label`) }}
          <template v-if="first.kind === 'changes_requested'"> · {{ t('nav.account.points', first.count) }}</template>
        </span>
        <span class="mt-0.5 line-clamp-2 block text-[13px] leading-snug text-text-secondary">
          {{ first.subject }}<template v-if="first.due_at"> — {{ t('nav.account.dueOn', { date: date(first.due_at, props.timezone) }) }}</template>
        </span>
        <span class="mt-2 inline-flex items-center gap-1.5 text-[13.5px] font-bold" :class="tone.ink">
          {{ t(`organization.workspace.actions.kind.${first.kind}.cta`) }}
          <UiIcon name="arrow-right" size="0.9rem" :stroke-width="2" />
        </span>
      </span>
    </NuxtLink>
    <NuxtLink
      v-if="props.actions.length > 1"
      :to="localePath('/mon-organisation')"
      role="menuitem"
      class="mt-0.5 flex min-h-(--target-min) items-center text-[13px] font-semibold text-text-secondary no-underline hover:text-text"
    >
      {{ t('nav.account.morePending', props.actions.length - 1) }}
    </NuxtLink>
  </div>
</template>

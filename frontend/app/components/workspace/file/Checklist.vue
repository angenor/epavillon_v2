<script setup lang="ts">
import type { ProposalFileChecklistItem } from '~/utils/proposal-file'

/**
 * « VOTRE DOSSIER » EN UN COUP D'ŒIL, dans la marge (29/10).
 *
 * Grand écran seulement : sur téléphone, l'encadré des manques en tête de page
 * dit déjà l'essentiel, et une seconde liste repousserait le dossier.
 */

interface Props {
  items: ProposalFileChecklistItem[]
}

const props = defineProps<Props>()

const { t } = useI18n()

const done = computed(() => props.items.filter((item) => item.done).length)
const ratio = computed(() => (props.items.length === 0 ? 0 : Math.round((done.value / props.items.length) * 100)))
</script>

<template>
  <aside
    :aria-label="t('organization.workspace.proposal.checklist.title')"
    class="hidden rounded-lg border border-border bg-surface-raised p-5 lg:sticky lg:top-24 lg:block"
  >
    <p class="font-semibold text-heading">{{ t('organization.workspace.proposal.checklist.title') }}</p>
    <div class="mt-3 h-2 overflow-hidden rounded-full bg-surface-sunken" aria-hidden="true">
      <div class="h-full rounded-full bg-success" :style="{ width: `${ratio}%` }" />
    </div>
    <p class="mt-1.5 text-sm text-text-muted">
      {{ t('organization.workspace.proposal.checklist.progress', { done, total: props.items.length }) }}
    </p>
    <ul class="mt-3 flex flex-col">
      <li v-for="item in props.items" :key="item.key">
        <a
          :href="`#${item.anchor}`"
          class="flex min-h-10 items-center gap-2.5 text-sm text-text no-underline hover:text-accent"
        >
          <span
            aria-hidden="true"
            class="grid size-5 shrink-0 place-items-center rounded-full text-xs font-bold"
            :class="
              item.done
                ? 'bg-success-surface text-success'
                : item.danger
                  ? 'bg-danger text-danger-contrast'
                  : 'border border-warning-border bg-warning-surface text-warning'
            "
          >
            <UiIcon v-if="item.done" name="check" size="0.75rem" />
            <template v-else>!</template>
          </span>
          <span>{{ t(`organization.workspace.proposal.checklist.items.${item.key}`) }}</span>
          <span class="sr-only">
            {{ t(item.done ? 'organization.workspace.proposal.checklist.done' : 'organization.workspace.proposal.checklist.todo') }}
          </span>
        </a>
      </li>
    </ul>
  </aside>
</template>

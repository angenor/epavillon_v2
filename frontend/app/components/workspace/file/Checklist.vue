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
  /** Seule dans la colonne, elle remonte sur le bandeau : ombre plutôt que filet. */
  raised?: boolean
}

const props = withDefaults(defineProps<Props>(), { raised: false })

const { t } = useI18n()

const done = computed(() => props.items.filter((item) => item.done).length)
const ratio = computed(() => (props.items.length === 0 ? 0 : Math.round((done.value / props.items.length) * 100)))
</script>

<template>
  <nav
    :aria-label="t('organization.workspace.proposal.checklist.title')"
    class="hidden rounded-lg bg-surface-raised px-5.5 py-5 lg:sticky lg:top-[calc(var(--nav-height)+1.5rem)] lg:block"
    :class="props.raised ? 'shadow-lg' : 'border border-border'"
  >
    <p class="heading-condensed text-[1.35rem] text-heading">{{ t('organization.workspace.proposal.checklist.title') }}</p>
    <div class="mt-3 h-1.5 overflow-hidden rounded-full bg-surface-sunken" aria-hidden="true">
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
                  ? 'attention-blink bg-danger text-danger-contrast'
                  : 'attention-blink border border-warning-border bg-warning-surface text-warning'
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
  </nav>
</template>

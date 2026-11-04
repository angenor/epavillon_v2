<script setup lang="ts">
/**
 * L'étiquette posée sur un filet qui ouvre une section de l'espace organisation
 * (signature de la direction condensée), sans couleur de journée : bleu nuit,
 * ou vert pour ce qui suit une activité retenue.
 */

interface Props {
  label: string
  headingId?: string
  icon?: string
  count?: number | null
  meta?: string
  tone?: 'plain' | 'success'
  /** Niveau du titre : `h2` pour une section, `p` pour un intertitre de recherche. */
  as?: 'h2' | 'h3' | 'p'
}

const props = withDefaults(defineProps<Props>(), { headingId: undefined, icon: '', count: null, meta: '', tone: 'plain', as: 'h2' })

const toneClass = computed(() =>
  props.tone === 'success'
    ? '[--rule-on:var(--color-success-contrast)] [--rule-solid:var(--color-success-solid)]'
    : '[--rule-on:var(--color-text-on-inverse)] [--rule-solid:var(--color-surface-inverse)] dark:[--rule-solid:var(--color-surface-inverse-raised)]',
)
</script>

<template>
  <div class="rule-line" :class="toneClass">
    <component :is="props.as" :id="props.headingId" class="rule-label">
      <UiIcon v-if="props.icon" :name="props.icon" size="1.1rem" :stroke-width="2" />
      {{ props.label }}
      <span v-if="props.count !== null" class="font-sans text-sm font-bold tracking-normal opacity-85">{{ props.count }}</span>
    </component>
    <span v-if="props.meta" class="pb-1.5 text-sm text-text-secondary">{{ props.meta }}</span>
  </div>
</template>

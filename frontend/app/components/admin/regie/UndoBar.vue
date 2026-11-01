<script setup lang="ts">
/**
 * Le bandeau « Défaire » qui suit chaque geste. Une action, une barre de temps :
 * le toast commun n'a ni l'une ni l'autre, et lui en donner ferait un second
 * composant sous le même nom.
 */
interface Props {
  title: string
  detail: string | null
  /** Part du temps restant, de 1 à 0. */
  remaining: number
  actionLabel: string
  busy: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ undo: []; close: [] }>()
const { t } = useI18n()
</script>

<template>
  <div
    role="status"
    class="fixed bottom-5 left-1/2 z-40 flex w-[min(40rem,calc(100%-1.5rem))] -translate-x-1/2 items-center gap-3.5 overflow-hidden rounded-lg bg-surface-inverse py-3 pr-3 pl-4 text-text-on-inverse shadow-lg"
  >
    <p class="m-0 min-w-0 flex-1 text-[0.92rem]">
      {{ props.title }}
      <small v-if="props.detail" class="block text-[0.8rem] text-text-on-inverse-muted">{{ props.detail }}</small>
    </p>
    <UiButton size="sm" variant="on-inverse" :loading="props.busy" @click="emit('undo')">{{ props.actionLabel }}</UiButton>
    <UiButton size="sm" variant="inverse" icon="close" icon-only :label="t('common.actions.close')" @click="emit('close')" />
    <span
      class="absolute bottom-0 left-0 h-0.75 bg-accent-on-inverse transition-[width] duration-1000 ease-linear"
      :style="{ width: `${Math.round(props.remaining * 100)}%` }"
      aria-hidden="true"
    />
  </div>
</template>

<script setup lang="ts">
import type { Toast, ToastIntent } from '~/composables/useToast'

/**
 * Une notification brève, posée par `UiToastRegion`. Même vocabulaire que
 * `UiAlert` (icône de forme distincte, ton porté par le conteneur, titre en
 * couleur de texte), sur une surface flottante.
 *
 * Un échec s'annonce aussitôt (`role="alert"`) ; le reste attend la fin de la
 * phrase en cours (`role="status"`).
 */

const props = defineProps<{ toast: Toast }>()
const emit = defineEmits<{ dismiss: []; hold: []; release: [] }>()

const { t } = useI18n()
const localePath = useLocalePath()

const ICONS: Record<ToastIntent, string> = {
  info: 'info',
  success: 'check-circle',
  warning: 'warning',
  danger: 'error',
}

const TONES: Record<ToastIntent, string> = {
  info: 'border-info-border bg-info-surface text-info',
  success: 'border-success-border bg-success-surface text-success',
  warning: 'border-warning-border bg-warning-surface text-warning',
  danger: 'border-danger-border bg-danger-surface text-danger',
}
</script>

<template>
  <div
    :role="props.toast.intent === 'danger' ? 'alert' : 'status'"
    class="pointer-events-auto flex w-full items-start gap-3 rounded-lg border py-2 ps-4 pe-1 shadow-lg"
    :class="TONES[props.toast.intent]"
    @mouseenter="emit('hold')"
    @mouseleave="emit('release')"
    @focusin="emit('hold')"
    @focusout="emit('release')"
  >
    <UiIcon :name="ICONS[props.toast.intent]" class="mt-2.5 shrink-0" size="1.25rem" />

    <div class="min-w-0 flex-1 py-2">
      <p class="font-display text-sm leading-snug font-semibold text-text">{{ props.toast.title }}</p>
      <p v-if="props.toast.message" class="mt-1 text-sm text-text-secondary">{{ props.toast.message }}</p>
      <NuxtLink
        v-if="props.toast.link"
        :to="localePath(props.toast.link.to)"
        class="mt-1 inline-flex min-h-(--target-min) items-center text-sm font-semibold text-inherit underline underline-offset-2"
        @click="emit('dismiss')"
      >
        {{ props.toast.link.label }}
      </NuxtLink>
    </div>

    <button
      type="button"
      class="ui-toast-close grid size-(--target-min) shrink-0 cursor-pointer place-items-center rounded-md text-inherit transition-colors duration-(--duration-fast)"
      @click="emit('dismiss')"
    >
      <span class="sr-only">{{ t('common.actions.close') }}</span>
      <UiIcon name="close" size="1rem" />
    </button>
  </div>
</template>

<style scoped>
.ui-toast-close:hover {
  background: color-mix(in srgb, currentColor 12%, transparent);
}
</style>

<script setup lang="ts">
/**
 * La pile des notifications brèves, une par layout. En haut à droite, sous la
 * barre : le bas à droite porte déjà les fenêtres flottantes (`UiFloatingPanel`).
 * Sur téléphone, pleine largeur moins les marges.
 *
 * « Moins d'animations » : `main.css` ramène les transitions à rien.
 */

// Bandeau institutionnel compris : en haut de page, il est encore visible.
const props = withDefaults(defineProps<{ top?: string }>(), {
  top: 'calc(var(--topbar-height) + var(--nav-height) + var(--space-4))',
})

const { t } = useI18n()
const { toasts, dismiss, hold, arm } = useToast()
</script>

<template>
  <section
    :aria-label="t('toast.region')"
    class="pointer-events-none fixed inset-x-4 z-50 sm:start-auto sm:end-6 sm:w-96"
    :style="{ top: props.top }"
  >
    <TransitionGroup
      tag="ol"
      class="flex flex-col gap-2"
      enter-active-class="transition duration-(--duration-base) ease-out"
      enter-from-class="translate-x-4 opacity-0"
      leave-active-class="transition duration-(--duration-fast) ease-in"
      leave-to-class="opacity-0"
    >
      <li v-for="toast in toasts" :key="toast.id">
        <UiToast
          :toast="toast"
          @dismiss="dismiss(toast.id)"
          @hold="hold(toast.id)"
          @release="arm(toast.id)"
        />
      </li>
    </TransitionGroup>
  </section>
</template>

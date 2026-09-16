<script setup lang="ts">
/**
 * Bouton flottant qui ouvre une petite fenêtre, sans masquer la page.
 *
 * À l'inverse de `UiDrawer` et `UiModal`, la fenêtre n'est PAS modale : on y
 * travaille en lisant la page derrière (noter un dossier en le parcourant). Pas
 * de voile, pas de piège du focus ; Échap ferme et rend le focus au bouton.
 */

interface Props {
  open: boolean
  title: string
  /** Nom accessible du bouton de lancement. */
  launcherLabel: string
  launcherIcon?: string
  /** Texte court affiché à la place de l'icône — une note, un compte. */
  launcherValue?: string | null
  width?: string
}

const props = withDefaults(defineProps<Props>(), {
  launcherIcon: 'plus',
  launcherValue: null,
  width: '26rem',
})
const emit = defineEmits<{ 'update:open': [value: boolean] }>()

const { t } = useI18n()
const titleId = useId()
const launcher = ref<HTMLButtonElement | null>(null)
const heading = ref<HTMLHeadingElement | null>(null)

function setOpen(value: boolean): void {
  emit('update:open', value)
}

watch(
  () => props.open,
  async (open) => {
    await nextTick()
    if (open) heading.value?.focus()
    else launcher.value?.focus()
  },
)
</script>

<template>
  <Teleport to="body">
    <button
      v-if="!props.open"
      ref="launcher"
      type="button"
      class="fixed end-4 bottom-4 z-30 flex size-14 cursor-pointer items-center justify-center rounded-full bg-accent-solid text-lg font-semibold text-accent-contrast tabular-nums shadow-lg transition-colors hover:bg-accent-solid-hover sm:end-6 sm:bottom-6"
      :aria-label="props.launcherLabel"
      :title="props.launcherLabel"
      aria-haspopup="dialog"
      :aria-expanded="false"
      @click="setOpen(true)"
    >
      <span v-if="props.launcherValue">{{ props.launcherValue }}</span>
      <UiIcon v-else :name="props.launcherIcon" size="1.5rem" />
    </button>

    <Transition
      enter-active-class="transition duration-(--duration-base) ease-out"
      enter-from-class="translate-y-2 opacity-0"
      leave-active-class="transition duration-(--duration-base) ease-in"
      leave-to-class="translate-y-2 opacity-0"
    >
      <section
        v-if="props.open"
        role="dialog"
        aria-modal="false"
        :aria-labelledby="titleId"
        class="fixed inset-x-0 bottom-0 z-30 flex max-h-[85dvh] flex-col rounded-t-lg border border-border bg-surface-overlay text-text shadow-lg sm:inset-x-auto sm:end-6 sm:bottom-6 sm:max-h-[calc(100dvh-10rem)] sm:w-(--floating-width) sm:rounded-lg"
        :style="{ '--floating-width': props.width }"
        @keydown.esc.stop="setOpen(false)"
      >
        <header class="flex items-center justify-between gap-3 border-b border-border-subtle py-2 ps-5 pe-2">
          <h2 :id="titleId" ref="heading" tabindex="-1" class="text-base font-semibold outline-none">
            {{ props.title }}
          </h2>
          <UiButton
            variant="ghost"
            icon="close"
            icon-only
            :label="t('common.actions.close')"
            @click="setOpen(false)"
          />
        </header>

        <div class="min-h-0 flex-1 overflow-y-auto">
          <slot />
        </div>
      </section>
    </Transition>
  </Teleport>
</template>

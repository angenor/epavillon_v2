<script setup lang="ts">
/**
 * Bouton flottant qui ouvre une petite fenêtre, sans masquer la page.
 *
 * À l'inverse de `UiDrawer` et `UiModal`, la fenêtre n'est PAS modale : on y
 * travaille en lisant la page derrière (noter un dossier en le parcourant). Pas
 * de voile, pas de piège du focus ; Échap ferme et rend le focus au bouton.
 *
 * Plusieurs fenêtres sur un écran : les boutons s'empilent en colonne (`stack`),
 * restent visibles, et la fenêtre s'ouvre à leur gauche. L'écran n'en ouvre
 * qu'une à la fois, sinon elles se recouvrent.
 */

interface Props {
  open: boolean
  title: string
  /** Nom accessible du bouton de lancement. */
  launcherLabel: string
  launcherIcon?: string
  /** Texte court affiché à la place de l'icône : une note, un compte. */
  launcherValue?: string | null
  /** Pastille chiffrée sur le bouton (messages non lus) ; zéro la masque. */
  launcherBadge?: number
  /** Rang du bouton dans la colonne, en partant du bas. */
  stack?: number
  width?: string
  /** Hauteur fixe sur grand écran ; sinon la fenêtre suit son contenu. */
  height?: string | null
  /** Bouton d'agrandissement dans l'en-tête. */
  expandable?: boolean
  /** Plein écran sous 640 px, au lieu d'un volet bas. */
  fullscreenOnMobile?: boolean
  /** Faux : le contenu gère lui-même son défilement (une liste et un pied fixe). */
  scrollBody?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  launcherIcon: 'plus',
  launcherValue: null,
  launcherBadge: 0,
  stack: 0,
  width: '26rem',
  height: null,
  expandable: false,
  fullscreenOnMobile: false,
  scrollBody: true,
})
const emit = defineEmits<{ 'update:open': [value: boolean] }>()

const { t } = useI18n()
const titleId = useId()
const launcher = ref<HTMLButtonElement | null>(null)
const heading = ref<HTMLHeadingElement | null>(null)
const windowEl = ref<HTMLElement | null>(null)
const expanded = ref(false)

function setOpen(value: boolean): void {
  emit('update:open', value)
}

watch(
  () => props.open,
  async (open) => {
    // Le focus ne revient au bouton que s'il était dans la fenêtre : quand une
    // autre fenêtre s'ouvre à sa place, c'est elle qui le garde.
    const hadFocus = !open && Boolean(windowEl.value?.contains(document.activeElement))
    await nextTick()
    if (open) heading.value?.focus()
    else if (hadFocus) launcher.value?.focus()
  },
)

const windowStyle = computed(() => ({
  '--floating-width': expanded.value ? 'min(52rem, calc(100vw - 8rem))' : props.width,
  '--floating-height': expanded.value ? 'calc(100dvh - 3rem)' : (props.height ?? 'auto'),
}))
</script>

<template>
  <Teleport to="body">
    <button
      ref="launcher"
      type="button"
      class="fixed end-4 bottom-[calc(1rem+var(--floating-stack)*4.25rem)] z-30 flex size-14 cursor-pointer items-center justify-center rounded-full text-lg font-semibold tabular-nums shadow-lg transition-colors sm:end-6 sm:bottom-[calc(1.5rem+var(--floating-stack)*4.25rem)]"
      :class="props.open
        ? 'border-2 border-accent bg-surface-overlay text-accent'
        : 'bg-accent-solid text-accent-contrast hover:bg-accent-solid-hover'"
      :style="{ '--floating-stack': props.stack }"
      :aria-label="props.launcherLabel"
      :title="props.launcherLabel"
      aria-haspopup="dialog"
      :aria-expanded="props.open"
      @click="setOpen(!props.open)"
    >
      <span v-if="props.launcherValue">{{ props.launcherValue }}</span>
      <UiIcon v-else :name="props.launcherIcon" size="1.5rem" />
      <UiCounter
        v-if="props.launcherBadge > 0"
        class="absolute -end-1 -top-1 ring-2 ring-surface"
        :value="props.launcherBadge"
        tone="accent"
        aria-hidden="true"
      />
    </button>

    <Transition
      enter-active-class="transition duration-(--duration-base) ease-out"
      enter-from-class="translate-y-2 opacity-0"
      leave-active-class="transition duration-(--duration-base) ease-in"
      leave-to-class="translate-y-2 opacity-0"
    >
      <section
        v-if="props.open"
        ref="windowEl"
        role="dialog"
        aria-modal="false"
        :aria-labelledby="titleId"
        class="fixed z-40 flex flex-col border border-border bg-surface-overlay text-text shadow-lg sm:inset-x-auto sm:end-24 sm:bottom-6 sm:h-(--floating-height) sm:max-h-[calc(100dvh-3rem)] sm:w-(--floating-width) sm:rounded-lg"
        :class="props.fullscreenOnMobile
          ? 'inset-0 max-sm:h-dvh sm:top-auto'
          : 'inset-x-0 bottom-0 max-h-[85dvh] rounded-t-lg'"
        :style="windowStyle"
        @keydown.esc.stop="setOpen(false)"
      >
        <header class="flex items-center justify-between gap-2 border-b border-border-subtle py-2 ps-5 pe-2">
          <h2 :id="titleId" ref="heading" tabindex="-1" class="min-w-0 truncate text-base font-semibold outline-none">
            {{ props.title }}
          </h2>
          <div class="flex shrink-0 items-center">
            <UiButton
              v-if="props.expandable"
              class="max-sm:hidden"
              variant="ghost"
              :icon="expanded ? 'minimize' : 'maximize'"
              icon-only
              :label="expanded ? t('common.actions.reduce') : t('common.actions.enlarge')"
              :aria-pressed="expanded"
              @click="expanded = !expanded"
            />
            <UiButton
              variant="ghost"
              icon="close"
              icon-only
              :label="t('common.actions.close')"
              @click="setOpen(false)"
            />
          </div>
        </header>

        <div class="min-h-0 flex-1" :class="props.scrollBody ? 'overflow-y-auto' : 'flex flex-col'">
          <slot />
        </div>
      </section>
    </Transition>
  </Teleport>
</template>

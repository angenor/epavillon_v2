<script setup lang="ts">
import type { Size } from '~/types/ui'

/**
 * Boîte de dialogue modale — confirmer, saisir peu, décider.
 *
 * `<dialog>` NATIF, et pour trois raisons qui sont autant de pièges évités : la
 * couche supérieure du navigateur (aucun conflit de `z-index` avec un en-tête
 * collant), le PIÉGEAGE DU FOCUS gratuit (`showModal()`), et la fermeture par
 * Échap sans écouteur de clavier. Une modale reconstruite en `<div>` demande de
 * réécrire les trois, et la troisième manque toujours.
 *
 * ELLE NE CONTIENT JAMAIS DE FORMULAIRE LONG — règle du guide. Le dialogue sert
 * la décision irréversible et la saisie de deux champs ; au-delà, c'est le
 * panneau latéral (`UiDrawer`) ou une page dédiée qu'il faut. Un formulaire long
 * dans une modale ne se sauvegarde pas en brouillon, ne s'envoie pas par
 * courriel, ne survit pas à un rafraîchissement, et pousse la moitié de ses
 * champs sous le pli sur un écran de portable.
 *
 * FERMETURE PAR LE FOND : proposée par défaut, désactivable. Un formulaire à
 * moitié rempli ne doit pas disparaître sur un clic maladroit — c'est le cas
 * `dismissible: false`, où seuls le bouton et Échap ferment.
 *
 * TITRE OBLIGATOIRE, relié par `aria-labelledby` : une boîte de dialogue sans
 * nom est annoncée « dialogue » et rien d'autre.
 *
 * DÉFILEMENT : le corps défile, l'en-tête et le pied restent. Sur écran étroit,
 * la modale occupe presque tout l'écran — une boîte centrée de 400 px sur un
 * téléphone de 375 px n'a pas de sens.
 *
 * RAYON PLAFONNÉ À 8 px (`--radius-lg`) : « coins légèrement arrondis, 6 à 8 px »
 * est une décision de direction artistique, et une surface flottante n'y échappe
 * pas — c'est même là qu'un rayon plus large ferait basculer l'ensemble du côté
 * du tableau de bord générique que le projet écarte.
 */

interface Props {
  /** Ouverture contrôlée par l'appelant. */
  open: boolean
  title: string
  /** Précision sous le titre. */
  description?: string
  size?: Size | 'xl'
  /** Fermeture par clic sur le fond et par la croix. */
  dismissible?: boolean
  /** Masque la croix de l'en-tête — dialogue à décision obligatoire. */
  hideCloseButton?: boolean
}

const props = withDefaults(defineProps<Props>(), { size: 'md', dismissible: true })
const emit = defineEmits<{ 'update:open': [value: boolean]; close: [] }>()

const { t } = useI18n()
const dialog = ref<HTMLDialogElement | null>(null)
const titleId = useId()
const descriptionId = useId()

const WIDTHS: Record<Size | 'xl', string> = {
  sm: 'sm:max-w-md',
  md: 'sm:max-w-lg',
  lg: 'sm:max-w-2xl',
  xl: 'sm:max-w-4xl',
}

function close(): void {
  emit('update:open', false)
  emit('close')
}

/** Clic sur le fond : le `<dialog>` lui-même reçoit l'événement, pas son contenu. */
function onBackdropClick(event: MouseEvent): void {
  if (!props.dismissible) return
  if (event.target === dialog.value) close()
}

watch(
  () => props.open,
  (isOpen) => {
    const element = dialog.value
    if (!element) return
    if (isOpen && !element.open) {
      element.showModal()
      // Le corps de page ne défile pas derrière la modale.
      document.documentElement.style.overflow = 'hidden'
    } else if (!isOpen && element.open) {
      element.close()
      document.documentElement.style.overflow = ''
    }
  },
  { flush: 'post' },
)

onBeforeUnmount(() => {
  document.documentElement.style.overflow = ''
})
</script>

<template>
  <dialog
    ref="dialog"
    :aria-labelledby="titleId"
    :aria-describedby="props.description ? descriptionId : undefined"
    class="ui-modal m-auto w-[calc(100vw-2rem)] rounded-lg border border-border bg-surface-overlay p-0 text-text shadow-lg backdrop:bg-black/50"
    :class="WIDTHS[props.size]"
    @click="onBackdropClick"
    @cancel.prevent="props.dismissible ? close() : undefined"
  >
    <div class="flex max-h-[85vh] flex-col">
      <header class="flex items-start gap-4 border-b border-border-subtle px-5 py-4">
        <div class="min-w-0 flex-1">
          <h2 :id="titleId" class="text-xl">{{ props.title }}</h2>
          <p v-if="props.description" :id="descriptionId" class="mt-1 text-sm text-text-muted">
            {{ props.description }}
          </p>
        </div>
        <button
          v-if="props.dismissible && !props.hideCloseButton"
          type="button"
          class="-mt-1 -mr-1 shrink-0 rounded-md p-2 text-text-subtle transition-colors hover:bg-surface-hover hover:text-text"
          @click="close"
        >
          <span class="sr-only">{{ t('common.actions.close') }}</span>
          <UiIcon name="close" size="1.1rem" />
        </button>
      </header>

      <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        <slot />
      </div>

      <footer
        v-if="$slots.footer"
        class="flex flex-wrap items-center justify-end gap-2 border-t border-border-subtle bg-surface-sunken px-5 py-3"
      >
        <slot name="footer" />
      </footer>
    </div>
  </dialog>
</template>

<style scoped>
/* `display: none` du `<dialog>` fermé empêche toute transition d'entrée : on
   anime donc l'ouverture seule, ce qui suffit — la fermeture doit être immédiate.
   `prefers-reduced-motion` la neutralise (voir `main.css`). */
.ui-modal[open] {
  animation: ui-modal-in 160ms ease-out;
}

@keyframes ui-modal-in {
  from {
    opacity: 0;
    transform: translateY(0.5rem);
  }
}
</style>

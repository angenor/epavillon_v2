<script setup lang="ts">
/**
 * Panneau latéral — le détail sans quitter la liste.
 *
 * QUAND LE PRÉFÉRER À UNE MODALE : quand ce qui est affiché COMMENTE la liste
 * qu'on est en train de parcourir — l'aperçu d'une proposition depuis le tableau
 * du back-office, les filtres avancés, l'historique d'une activité. Une modale
 * interrompt ; un tiroir accompagne. Pour une décision qui doit être prise avant
 * toute autre chose, c'est `UiModal` qu'il faut.
 *
 * C'EST DONC ICI QUE VA LA SAISIE UN PEU LONGUE, jamais dans un dialogue : la
 * règle du guide — « un dialogue ne contient jamais de formulaire long » — a
 * pour contrepartie que le panneau, lui, l'accepte, parce qu'il est haut, qu'il
 * défile, et qu'il laisse la liste visible pour comparer. Passé une dizaine de
 * champs, c'est encore une page dédiée qu'il faut : elle a une adresse, donc un
 * brouillon rechargeable.
 *
 * Même socle technique que la modale : `<dialog>` natif, donc couche supérieure,
 * piégeage du focus et fermeture par Échap sans une ligne de clavier.
 *
 * SUR ÉCRAN ÉTROIT, le tiroir occupe toute la largeur : un panneau de 420 px sur
 * un téléphone de 375 px laisserait une bande de contenu inutilisable derrière.
 */

interface Props {
  open: boolean
  title: string
  description?: string
  /** Côté d'apparition. À droite par défaut : c'est le sens de lecture du détail. */
  side?: 'right' | 'left'
  /** Largeur CSS sur écran large. */
  width?: string
  dismissible?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  side: 'right',
  width: '28rem',
  dismissible: true,
})
const emit = defineEmits<{ 'update:open': [value: boolean]; close: [] }>()

const { t } = useI18n()
const dialog = ref<HTMLDialogElement | null>(null)
const titleId = useId()
const descriptionId = useId()

function close(): void {
  emit('update:open', false)
  emit('close')
}

function onBackdropClick(event: MouseEvent): void {
  if (props.dismissible && event.target === dialog.value) close()
}

function syncOpenState(): void {
  const element = dialog.value
  if (!element) return
  if (props.open && !element.open) {
    element.showModal()
    document.documentElement.style.overflow = 'hidden'
  } else if (!props.open && element.open) {
    element.close()
    document.documentElement.style.overflow = ''
  }
}

watch(() => props.open, syncOpenState, { flush: 'post' })

/**
 * AU MONTAGE AUSSI — le même défaut que `UiModal` corrigeait au prompt A2, et
 * que le tiroir traînait encore. Un tiroir monté DÉJÀ ouvert — `v-if="cible"`
 * sur le composant, `open` posé dans le même geste que la cible — ne s'affichait
 * jamais : le watcher n'avait rien vu changer. Le motif est naturel dès qu'un
 * panneau porte sur un élément choisi dans une LISTE, et c'est exactement ce qui
 * s'est produit au prompt A12 : le panneau d'attribution de rôle s'ouvrait depuis
 * la fiche d'une personne, où il est monté fermé, et restait invisible depuis la
 * liste, où il est monté ouvert.
 */
onMounted(syncOpenState)

onBeforeUnmount(() => {
  document.documentElement.style.overflow = ''
})
</script>

<template>
  <dialog
    ref="dialog"
    :aria-labelledby="titleId"
    :aria-describedby="props.description ? descriptionId : undefined"
    class="ui-drawer m-0 h-dvh max-h-dvh w-full max-w-full border-border bg-surface-overlay p-0 text-text shadow-lg backdrop:bg-scrim/60"
    :class="props.side === 'right' ? 'ml-auto sm:border-l' : 'mr-auto sm:border-r'"
    :style="{
      '--drawer-width': props.width,
      '--drawer-from': props.side === 'right' ? '1.5rem' : '-1.5rem',
    }"
    @click="onBackdropClick"
    @cancel.prevent="props.dismissible ? close() : undefined"
  >
    <div class="flex h-full flex-col">
      <header class="flex items-start gap-4 border-b border-border-subtle px-5 py-4">
        <div class="min-w-0 flex-1">
          <h2 :id="titleId" class="text-xl">{{ props.title }}</h2>
          <p v-if="props.description" :id="descriptionId" class="mt-1 text-sm text-text-muted">
            {{ props.description }}
          </p>
        </div>
        <button
          v-if="props.dismissible"
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
@media (min-width: 640px) {
  .ui-drawer {
    width: var(--drawer-width);
  }
}

.ui-drawer[open] {
  animation: ui-drawer-in 180ms ease-out;
}

@keyframes ui-drawer-in {
  from {
    opacity: 0.6;
    transform: translateX(var(--drawer-from, 1.5rem));
  }
}
</style>

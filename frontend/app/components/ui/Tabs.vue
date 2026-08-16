<script setup lang="ts">
import type { TabItem } from '~/types/ui'

/**
 * Onglets — plusieurs vues d'un même objet : le dossier, ses évaluations, ses
 * échanges, son historique.
 *
 * CE QU'ILS NE SONT PAS : une navigation entre pages. Si chaque volet mérite son
 * adresse — et c'est le cas dès qu'on veut pouvoir l'envoyer par courriel —,
 * ce sont des liens qu'il faut, pas des onglets. `to` permet de rendre chaque
 * onglet comme un lien tout en gardant l'habillage.
 *
 * CLAVIER (motif ARIA « tabs », activation manuelle) : flèches pour déplacer le
 * focus, Entrée ou Espace pour activer. L'activation automatique au simple
 * déplacement du focus est écartée : elle déclencherait un chargement à chaque
 * pression de flèche, sur des volets qui interrogent l'API.
 *
 * Un seul onglet est dans le parcours de tabulation (`tabindex="0"`) : c'est
 * l'onglet actif. Les flèches font le reste — c'est la règle du motif, et c'est
 * ce qui évite de traverser douze onglets pour atteindre le contenu.
 *
 * DÉBORDEMENT : la barre défile horizontalement DANS son cadre sur écran étroit.
 * Le corps de page, lui, ne défile jamais horizontalement.
 */

interface Props {
  items: TabItem[]
  /** Onglet actif — sa `value`. */
  modelValue: string
  /** Nom de la barre d'onglets, annoncé par les lecteurs d'écran. */
  label: string
  /** Rendre les onglets comme des liens : fonction qui donne l'URL d'un onglet. */
  to?: (item: TabItem) => string
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const tabRefs = ref<HTMLElement[]>([])

const enabledIndexes = computed(() =>
  props.items.map((item, index) => (item.disabled ? -1 : index)).filter((index) => index >= 0),
)

function select(item: TabItem): void {
  if (item.disabled) return
  emit('update:modelValue', item.value)
}

/** Déplace le FOCUS sans activer : l'activation reste manuelle. */
function move(currentIndex: number, step: 1 | -1 | 'first' | 'last'): void {
  const indexes = enabledIndexes.value
  if (indexes.length === 0) return
  let next: number
  if (step === 'first') next = indexes[0] as number
  else if (step === 'last') next = indexes.at(-1) as number
  else {
    const position = indexes.indexOf(currentIndex)
    next = indexes[(position + step + indexes.length) % indexes.length] as number
  }
  tabRefs.value[next]?.focus()
}
</script>

<template>
  <div
    role="tablist"
    :aria-label="props.label"
    class="flex gap-1 overflow-x-auto border-b border-border"
  >
    <component
      :is="props.to ? resolveComponent('NuxtLink') : 'button'"
      v-for="(item, index) in props.items"
      :key="item.value"
      :ref="(element: unknown) => { if (element) tabRefs[index] = (element as { $el?: HTMLElement }).$el ?? element as HTMLElement }"
      :to="props.to ? props.to(item) : undefined"
      :type="props.to ? undefined : 'button'"
      role="tab"
      :aria-selected="item.value === props.modelValue"
      :aria-disabled="item.disabled ? 'true' : undefined"
      :tabindex="item.value === props.modelValue ? 0 : -1"
      class="-mb-px inline-flex shrink-0 items-center gap-2 border-b-2 px-3.5 py-2.5 text-sm whitespace-nowrap no-underline transition-colors"
      :class="[
        item.value === props.modelValue
          ? 'border-accent font-semibold text-accent'
          : 'border-transparent text-text-muted hover:border-border-strong hover:text-text',
        item.disabled ? 'cursor-not-allowed text-text-subtle hover:border-transparent hover:text-text-subtle' : '',
      ]"
      @click="select(item)"
      @keydown.right.prevent="move(index, 1)"
      @keydown.left.prevent="move(index, -1)"
      @keydown.home.prevent="move(index, 'first')"
      @keydown.end.prevent="move(index, 'last')"
    >
      {{ item.label }}
      <!-- Le compteur est une information, pas une décoration : il garde le
           traitement neutre des pastilles et ne prend jamais l'accent. -->
      <span
        v-if="item.count !== undefined"
        class="rounded-full bg-surface-sunken px-1.5 py-0.5 font-mono text-xs tabular-nums text-text-muted"
      >
        {{ item.count }}
      </span>
    </component>
  </div>
</template>

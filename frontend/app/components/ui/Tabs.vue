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
 * L'ONGLET DOIT ÊTRE RELIÉ À SON PANNEAU, sans quoi le motif ARIA reste à
 * moitié écrit : un lecteur d'écran annonce « onglet 2 sur 5 » sans pouvoir
 * dire ce qu'il commande. Deux façons de le faire, selon où vit le panneau :
 *   · le contenu passé au créneau par défaut est rendu ICI, dans un
 *     `role="tabpanel"` déjà relié — c'est le cas courant, rien à câbler ;
 *   · un panneau rendu ailleurs dans la page se déclare par `panelId`, et
 *     reprend l'identifiant d'onglet exposé par `tabId()` pour son
 *     `aria-labelledby`.
 * Sans panneau ni `panelId`, aucun `aria-controls` n'est émis : pointer un
 * identifiant absent du document est pire que ne rien pointer du tout.
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
  /** Identifiant du panneau commandé, quand il est rendu hors de ce composant. */
  panelId?: (item: TabItem) => string
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const slots = useSlots()

const tabRefs = ref<HTMLElement[]>([])

// Préfixe stable entre serveur et client : deux barres d'onglets sur une même
// page ne doivent pas produire deux fois le même identifiant.
const uid = useId()
const tabId = (value: string): string => `${uid}-tab-${value}`
const ownPanelId = (value: string): string => `${uid}-panel-${value}`

/** Le panneau interne n'existe que si l'appelant a rempli le créneau. */
const hasOwnPanel = computed(() => Boolean(slots.default))

function controls(item: TabItem): string | undefined {
  if (props.panelId) return props.panelId(item)
  return hasOwnPanel.value ? ownPanelId(item.value) : undefined
}

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

// Un panneau rendu ailleurs a besoin de l'identifiant de son onglet pour son
// `aria-labelledby` : c'est la seule chose que l'extérieur ne peut pas deviner.
defineExpose({ tabId })
</script>

<template>
  <div>
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
        :id="tabId(item.value)"
        :to="props.to ? props.to(item) : undefined"
        :type="props.to ? undefined : 'button'"
        role="tab"
        :aria-selected="item.value === props.modelValue"
        :aria-controls="controls(item)"
        :aria-disabled="item.disabled ? 'true' : undefined"
        :tabindex="item.value === props.modelValue ? 0 : -1"
        class="-mb-px inline-flex min-h-(--target-min) shrink-0 items-center gap-2 border-b-(length:--border-thick) px-4 text-sm font-semibold whitespace-nowrap no-underline transition-colors duration-(--duration-fast)"
        :class="[
          item.value === props.modelValue
            ? 'border-accent text-accent'
            : 'border-transparent text-text-secondary hover:border-border-strong hover:text-text',
          // L'onglet indisponible s'estompe et refuse le curseur : la couleur
          // seule ne dit pas qu'un volet n'est pas encore ouvert.
          item.disabled
            ? 'cursor-not-allowed opacity-45 hover:border-transparent hover:text-text-secondary'
            : '',
        ]"
        @click="select(item)"
        @keydown.right.prevent="move(index, 1)"
        @keydown.left.prevent="move(index, -1)"
        @keydown.home.prevent="move(index, 'first')"
        @keydown.end.prevent="move(index, 'last')"
      >
        {{ item.label }}
        <!-- Pastille autonome, jamais un simple nombre collé au libellé : c'est
             elle qui rend le volume lisible d'un coup d'œil dans une barre de
             cinq onglets. Dimensions du guide (22 px), dérivées du pas de 4 px. -->
        <UiCounter v-if="item.count !== undefined" :value="item.count" tone="accent" />
      </component>
    </div>

    <!-- `tabindex="0"` sur le panneau : sans lui, un volet qui ne contient que
         du texte n'est pas atteignable au clavier depuis son onglet. -->
    <div
      v-if="hasOwnPanel"
      :id="ownPanelId(props.modelValue)"
      role="tabpanel"
      :aria-labelledby="tabId(props.modelValue)"
      tabindex="0"
      class="px-1 py-5"
    >
      <slot :value="props.modelValue" />
    </div>
  </div>
</template>

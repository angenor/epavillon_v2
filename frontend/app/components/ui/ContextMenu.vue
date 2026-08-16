<script setup lang="ts">
import type { MenuItem } from '~/types/ui'

/**
 * Menu d'actions — le « … » de fin de ligne d'un tableau, les actions
 * secondaires d'un en-tête.
 *
 * CE N'EST PAS UN MENU DE NAVIGATION. Il ne contient que des ACTIONS sur l'objet
 * courant : modifier, dupliquer, retirer. Une navigation doit rester un lien,
 * pour être ouverte dans un onglet — c'est le rôle de `UiNavBar` et `UiSideNav`.
 *
 * CLAVIER, entièrement implémenté parce qu'aucun élément natif ne le fournit :
 * Entrée ou Espace ouvre, flèches parcourent, Début et Fin vont aux extrémités,
 * Échap referme et REND LE FOCUS au déclencheur — sans ce dernier point, la
 * tabulation repart du haut de la page et l'utilisateur se perd.
 *
 * MENU FERMÉ = MENU ABSENT DU DOM. Les entrées ne sont ni focalisables ni
 * lisibles quand il est refermé, sans avoir à jouer sur `tabindex`.
 *
 * ENTRÉES DE 44 px : c'est un menu qu'on vise au doigt depuis une tablette, au
 * fond d'une salle de négociation, et 32 px y produisent une erreur sur deux.
 *
 * SURTITRE DE GROUPE (`groupLabel`) plutôt qu'un second niveau de menu : « douze
 * actions » se parcourt mal, « trois groupes de quatre » se parcourt bien, et
 * un sous-menu déroulant coûterait un second piège au focus pour rien.
 *
 * RACCOURCI CLAVIER (`shortcut`) poussé à droite, en `<kbd>` : il n'est pas
 * décoratif, c'est ainsi qu'un utilisateur quotidien apprend à se passer du
 * menu. Le composant l'AFFICHE ; c'est à l'écran de le brancher — un raccourci
 * capté ici serait actif alors même que le menu est fermé.
 *
 * ACTION DESTRUCTRICE : rendue en rouge — LIBELLÉ ET ICÔNE — et séparée du
 * reste par un trait. Teinter le seul libellé laissait un pictogramme gris
 * devant un texte rouge, ce qui affaiblissait exactement le signal qui doit
 * arrêter la main.
 */

interface Props {
  items: MenuItem[]
  /** Nom accessible du déclencheur — « Actions sur COP31-00147 », pas « Actions ». */
  label: string
  /** Alignement du panneau sous le déclencheur. */
  align?: 'start' | 'end'
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), { align: 'end' })
const emit = defineEmits<{ select: [value: string] }>()

const isOpen = ref(false)
const root = ref<HTMLElement | null>(null)
const trigger = ref<HTMLButtonElement | null>(null)
const itemRefs = ref<HTMLButtonElement[]>([])
const activeIndex = ref(-1)

/** Entrées réellement atteignables au clavier — les désactivées sont sautées. */
const enabledIndexes = computed(() =>
  props.items.map((item, index) => (item.disabled ? -1 : index)).filter((index) => index >= 0),
)

function open(startAt: 'first' | 'last' = 'first'): void {
  if (props.disabled) return
  isOpen.value = true
  const indexes = enabledIndexes.value
  activeIndex.value = startAt === 'first' ? (indexes[0] ?? -1) : (indexes.at(-1) ?? -1)
  nextTick(() => itemRefs.value[activeIndex.value]?.focus())
}

function close(returnFocus = true): void {
  isOpen.value = false
  activeIndex.value = -1
  if (returnFocus) nextTick(() => trigger.value?.focus())
}

function move(step: 1 | -1): void {
  const indexes = enabledIndexes.value
  if (indexes.length === 0) return
  const position = indexes.indexOf(activeIndex.value)
  const next = indexes[(position + step + indexes.length) % indexes.length] ?? indexes[0]
  activeIndex.value = next as number
  itemRefs.value[activeIndex.value]?.focus()
}

function select(item: MenuItem): void {
  if (item.disabled) return
  emit('select', item.value)
  close()
}

/** Clic hors du menu : fermeture SANS rendre le focus — l'utilisateur est ailleurs. */
function onDocumentPointerDown(event: PointerEvent): void {
  if (!isOpen.value) return
  if (root.value && !root.value.contains(event.target as Node)) close(false)
}

onMounted(() => document.addEventListener('pointerdown', onDocumentPointerDown))
onBeforeUnmount(() => document.removeEventListener('pointerdown', onDocumentPointerDown))
</script>

<template>
  <div ref="root" class="relative inline-block">
    <button
      ref="trigger"
      type="button"
      class="inline-flex size-8 items-center justify-center rounded-md border border-transparent text-text-muted transition-colors duration-(--duration-fast) hover:bg-surface-hover hover:text-text disabled:cursor-not-allowed disabled:text-text-subtle"
      :class="isOpen ? 'bg-surface-hover text-text' : ''"
      :disabled="props.disabled"
      :aria-expanded="isOpen"
      aria-haspopup="menu"
      :aria-label="props.label"
      :title="props.label"
      @click="isOpen ? close() : open()"
      @keydown.down.prevent="open('first')"
      @keydown.up.prevent="open('last')"
    >
      <UiIcon name="more-horizontal" size="1.15rem" />
    </button>

    <div
      v-if="isOpen"
      role="menu"
      :aria-label="props.label"
      class="absolute z-40 mt-1 min-w-65 rounded-lg border border-border bg-surface-overlay p-2 shadow-md"
      :class="props.align === 'end' ? 'right-0' : 'left-0'"
      @keydown.down.prevent="move(1)"
      @keydown.up.prevent="move(-1)"
      @keydown.home.prevent="((activeIndex = enabledIndexes[0] ?? -1), itemRefs[activeIndex]?.focus())"
      @keydown.end.prevent="((activeIndex = enabledIndexes.at(-1) ?? -1), itemRefs[activeIndex]?.focus())"
      @keydown.esc.prevent="close()"
      @keydown.tab="close(false)"
    >
      <template v-for="(item, index) in props.items" :key="item.value">
        <hr v-if="item.separatorBefore" class="my-2 border-separator">
        <!-- `role="presentation"` : le surtitre n'est pas une entrée de menu et
             ne doit pas être compté comme telle par un lecteur d'écran. -->
        <p
          v-if="item.groupLabel"
          role="presentation"
          class="px-3 py-2 text-xs font-semibold tracking-caps text-text-secondary uppercase"
        >
          {{ item.groupLabel }}
        </p>
        <button
          :ref="(element) => { if (element) itemRefs[index] = element as HTMLButtonElement }"
          type="button"
          role="menuitem"
          class="flex min-h-(--target-min) w-full items-center gap-3 rounded-md px-3 text-start text-sm transition-colors duration-(--duration-fast) disabled:cursor-not-allowed disabled:opacity-45"
          :class="
            item.destructive
              ? 'text-danger hover:bg-danger-surface'
              : 'text-text hover:bg-surface-hover'
          "
          :disabled="item.disabled"
          :tabindex="index === activeIndex ? 0 : -1"
          @click="select(item)"
        >
          <!-- L'icône d'une action ordinaire reste en second rang ; celle d'une
               action destructrice hérite du rouge du libellé. -->
          <UiIcon
            v-if="item.icon"
            :name="item.icon"
            size="1rem"
            class="shrink-0"
            :class="item.destructive ? '' : 'text-text-secondary'"
          />
          {{ item.label }}
          <kbd v-if="item.shortcut" class="ms-auto font-mono text-xs font-normal text-text-muted">
            {{ item.shortcut }}
          </kbd>
        </button>
      </template>
    </div>
  </div>
</template>

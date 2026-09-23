<script setup lang="ts">
import type { OutlineEntry, ReadingPage } from '~/types/negotiation-documents'
import { sectionDeLaPage } from '~/utils/guide-nego/forme-lisible'

/**
 * Le sommaire du document — maquette 04 · 04. Les chapitres sont repliés, sauf celui
 * qu'on lit, déplié jusqu'à la section en cours (FR-040).
 */
const props = defineProps<{
  sommaire: OutlineEntry[]
  pages: ReadingPage[]
  pageEnCours: number
}>()

defineEmits<{ aller: [pageIndex: number] }>()

const enCours = computed(() => sectionDeLaPage(props.sommaire, props.pageEnCours, 3))

/** Le chemin jusqu'à une entrée : ses ancêtres, puis elle. */
function chemin(entrees: OutlineEntry[], cible: OutlineEntry): OutlineEntry[] {
  for (const entree of entrees) {
    if (entree === cible) return [entree]
    const dessous = chemin(entree.children ?? [], cible)
    if (dessous.length) return [entree, ...dessous]
  }
  return []
}

const depliees = ref(new Set<OutlineEntry>(enCours.value ? chemin(props.sommaire, enCours.value) : []))

function basculer(entree: OutlineEntry): void {
  const suivantes = new Set(depliees.value)
  if (suivantes.has(entree)) suivantes.delete(entree)
  else suivantes.add(entree)
  depliees.value = suivantes
}

const etiquettes = computed(() => new Map(props.pages.map((p) => [p.index, p.label])))
const etiquetteDe = (index: number): string => etiquettes.value.get(index) ?? String(index)
</script>

<template>
  <ul class="gn-sommaire-liste" role="list">
    <GnLigneSommaire
      v-for="(entree, rang) in sommaire"
      :key="rang"
      :entree="entree"
      :etiquette-de="etiquetteDe"
      :en-cours="enCours"
      :depliees="depliees"
      @aller="$emit('aller', $event)"
      @basculer="basculer"
    />
  </ul>
</template>

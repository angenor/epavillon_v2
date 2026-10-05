<script setup lang="ts">
/**
 * La rangée d'onglets sous l'en-tête. Elle filtre une liste EN PLACE : rien ne change
 * de panneau, donc ce sont des boutons à deux états et non des `role="tab"`, qui
 * promettraient à un lecteur d'écran un panneau par onglet — panneau qui n'existe pas.
 *
 * Comme la barre basse, la rangée seule défile quand la place manque : aucun libellé
 * n'est tronqué ni renvoyé à la ligne, et l'onglet actif est ramené dans la vue.
 */
export interface OngletDeFiltre {
  valeur: string
  libelle: string
}

defineProps<{ onglets: OngletDeFiltre[]; libelle: string }>()
const actif = defineModel<string>({ required: true })

const rangee = useTemplateRef<HTMLElement>('rangee')

function amenerDansLaVue() {
  rangee.value
    ?.querySelector('[aria-pressed="true"]')
    ?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
}

onMounted(amenerDansLaVue)
watch(actif, () => nextTick(amenerDansLaVue))
</script>

<template>
  <div ref="rangee" class="gn-onglets-filtre" role="group" :aria-label="libelle">
    <button
      v-for="onglet in onglets"
      :key="onglet.valeur"
      type="button"
      class="gn-onglets-filtre__onglet"
      :class="{ 'gn-onglets-filtre__onglet--actif': onglet.valeur === actif }"
      :aria-pressed="onglet.valeur === actif"
      @click="actif = onglet.valeur"
    >
      {{ onglet.libelle }}
    </button>
  </div>
</template>

<style>
/* Les pilules de jour de la maquette : la rangée déborde à droite et défile seule. */
[data-app="guide-nego"] .gn-onglets-filtre {
  display: flex;
  gap: 6px;
  margin-inline-end: calc(-1 * var(--gn-marge-ecran));
  padding-inline-end: var(--gn-marge-ecran);
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

[data-app="guide-nego"] .gn-onglets-filtre::-webkit-scrollbar {
  display: none;
}

[data-app="guide-nego"] .gn-onglets-filtre__onglet {
  flex: none;
  height: var(--gn-pilule-jour);
  padding-inline: var(--gn-espace-16);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-pilule);
  background: transparent;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: 1;
  font-weight: var(--gn-graisse-demi-gras);
  white-space: nowrap;
}

[data-app="guide-nego"] .gn-onglets-filtre__onglet:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-onglets-filtre__onglet--actif {
  background: var(--gn-accent);
  border-color: var(--gn-accent);
  color: var(--gn-accent-inv);
  font-weight: var(--gn-graisse-extra-gras);
}
</style>

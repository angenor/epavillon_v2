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
/* Le filet 3 px au-dessus vient de l'en-tête (`.gn-entete`) : la rangée n'en repose pas un second. */
[data-app="guide-nego"] .gn-onglets-filtre {
  display: flex;
  gap: var(--gn-espace-16);
  /* La rangée seule défile : la page, jamais. */
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

[data-app="guide-nego"] .gn-onglets-filtre::-webkit-scrollbar {
  display: none;
}

[data-app="guide-nego"] .gn-onglets-filtre__onglet {
  flex: none;
  min-height: var(--gn-onglet-filtre-hauteur);
  display: flex;
  align-items: center;
  border: none;
  /* Le filet de l'actif est posé dès le repos, en transparent : sinon la rangée sautait de 3 px. */
  border-bottom: var(--gn-filet-3) solid transparent;
  background: none;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  white-space: nowrap;
}

[data-app="guide-nego"] .gn-onglets-filtre__onglet:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-onglets-filtre__onglet:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-onglets-filtre__onglet--actif {
  border-bottom-color: var(--gn-accent);
  color: var(--gn-titre);
  font-weight: var(--gn-graisse-gras);
}
</style>

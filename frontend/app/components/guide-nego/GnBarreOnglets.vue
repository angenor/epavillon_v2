<script setup lang="ts">
import { onglets as listerOnglets, ongletActif, type Onglet } from '~/utils/guide-nego/onglets'

/**
 * La barre basse de la maquette Nuit : onglets de 80 × 56, répartis. Quand la place
 * manque (cinq onglets, police agrandie, écran étroit), la barre seule défile, et rien
 * n'est tronqué.
 *
 * L'onglet allumé se déduit de l'adresse. `forcerActif` le désigne à la main : hors
 * d'un écran de l'application — la planche de design — aucune adresse ne l'allume, et
 * une barre sans onglet actif ne montre pas ce qu'elle est censée montrer.
 */
const props = withDefaults(
  defineProps<{
    echangesOuverts?: boolean
    compteurs?: Record<string, number>
    forcerActif?: string
  }>(),
  { echangesOuverts: false, compteurs: () => ({}), forcerActif: undefined },
)

const { t } = useI18n()
const route = useRoute()

const onglets = computed<Onglet[]>(() => listerOnglets(props.echangesOuverts))
const actif = computed(() =>
  props.forcerActif
    ? (onglets.value.find((onglet) => onglet.cle === props.forcerActif) ?? null)
    : ongletActif(route.path, onglets.value),
)

const barre = useTemplateRef<HTMLElement>('barre')

/** L'onglet actif doit être dans la vue même quand la barre défile. */
function amenerDansLaVue() {
  barre.value?.querySelector('[aria-current="page"]')?.scrollIntoView({ block: 'nearest', inline: 'nearest' })
}

let observateur: ResizeObserver | undefined

onMounted(() => {
  amenerDansLaVue()
  // La place peut changer sans que l'onglet change : police agrandie en cours de
  // session, rotation de l'écran. Sans cela l'onglet actif sortait de la vue et n'y
  // revenait qu'à la navigation suivante.
  observateur = new ResizeObserver(amenerDansLaVue)
  observateur.observe(barre.value!)
})

onBeforeUnmount(() => observateur?.disconnect())

watch(actif, () => nextTick(amenerDansLaVue))
</script>

<template>
  <nav ref="barre" class="gn-onglets" :aria-label="t('gn-barre-onglets.libelle')">
    <NuxtLink
      v-for="onglet in onglets"
      :key="onglet.cle"
      :to="onglet.route"
      class="gn-onglets__onglet"
      :class="{ 'gn-onglets__onglet--actif': actif?.cle === onglet.cle }"
      :aria-current="actif?.cle === onglet.cle ? 'page' : undefined"
    >
      <span class="gn-onglets__marque">
        <GnPicto :nom="onglet.picto" :taille="24" />
        <span v-if="compteurs[onglet.cle]" class="gn-onglets__compteur">
          {{ compteurs[onglet.cle] }}
          <span class="gn-hors-ecran">{{ t('gn-barre-onglets.non-lus') }}</span>
        </span>
      </span>
      <span class="gn-onglets__libelle">{{ t(`gn-barre-onglets.${onglet.cle}`) }}</span>
    </NuxtLink>
  </nav>
</template>

<style>
[data-app="guide-nego"] .gn-onglets {
  position: fixed;
  bottom: 0;
  /* Sur un écran large, la barre reste dans la colonne de l'application. */
  left: 50%;
  transform: translateX(-50%);
  width: min(100%, var(--gn-colonne-largeur));
  z-index: 5;
  display: flex;
  justify-content: space-around;
  /* 88 de la maquette, zone de geste comprise ; plus haut seulement si la zone sûre l'exige. */
  height: max(var(--gn-barre-onglets), calc(var(--gn-espace-8) + var(--gn-onglet-hauteur) + env(safe-area-inset-bottom)));
  padding-top: var(--gn-espace-8);
  background: var(--gn-barre-fond);
  border-top: var(--gn-filet-1) solid var(--gn-filet-doux);
  /* La barre seule défile : la page, jamais. */
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

[data-app="guide-nego"] .gn-onglets::-webkit-scrollbar {
  display: none;
}

[data-app="guide-nego"] .gn-onglets__onglet {
  flex: 0 1 var(--gn-onglet-largeur);
  /* 80 comme la maquette, même quand « Négociations » y déborde d'un pixel ou deux. */
  min-width: 64px;
  height: var(--gn-onglet-hauteur);
  padding-inline: var(--gn-espace-4);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-4);
  border-radius: var(--gn-rayon-14);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-12);
  /* L'interligne normal de la maquette : avec 1,2, le pictogramme descend d'un pixel. */
  line-height: normal;
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration: none;
  white-space: nowrap;
}

[data-app="guide-nego"] .gn-onglets__onglet:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-onglets__onglet:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-anneau));
}

[data-app="guide-nego"] .gn-onglets__onglet--actif {
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-onglets__marque {
  position: relative;
  display: flex;
}

[data-app="guide-nego"] .gn-onglets__compteur {
  position: absolute;
  top: -6px;
  right: -12px;
  min-width: 18px;
  height: 18px;
  padding-inline: var(--gn-espace-4);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-danger-aplat);
  color: var(--gn-danger-texte);
  font-size: var(--gn-taille-11);
  font-weight: var(--gn-graisse-extra-gras);
}
</style>

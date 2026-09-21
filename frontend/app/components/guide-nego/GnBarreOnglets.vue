<script setup lang="ts">
import { onglets as listerOnglets, ongletActif, type Onglet } from '~/utils/guide-nego/onglets'

/**
 * La barre basse, à LARGEUR DE LIBELLÉ : « Négociations » et « Francophonie » ne
 * tiennent pas dans un cinquième d'écran. À 360 px, cinq onglets tiennent exactement ;
 * quand la place manque — police agrandie, écran plus étroit — la barre seule défile,
 * et rien n'est tronqué.
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
        <GnPicto :nom="onglet.picto" :taille="26" />
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
  width: min(100%, 480px);
  z-index: 5;
  display: flex;
  height: calc(var(--gn-barre-onglets) + env(safe-area-inset-bottom));
  padding-bottom: env(safe-area-inset-bottom);
  background: var(--gn-fond);
  border-top: var(--gn-filet-2) solid var(--gn-filet-fort);
  /* La barre seule défile : la page, jamais. */
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

[data-app="guide-nego"] .gn-onglets::-webkit-scrollbar {
  display: none;
}

[data-app="guide-nego"] .gn-onglets__onglet {
  flex: 1 0 auto;
  min-width: var(--gn-onglet-min);
  padding-inline: var(--gn-onglet-air);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 3px;
  margin-top: calc(-1 * var(--gn-filet-2));
  border-top: var(--gn-filet-3) solid transparent;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
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
  border-top-color: var(--gn-accent);
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-onglets__marque {
  position: relative;
  display: flex;
}

[data-app="guide-nego"] .gn-onglets__compteur {
  position: absolute;
  top: -6px;
  right: -10px;
  min-width: var(--gn-compteur);
  height: var(--gn-compteur);
  padding-inline: 5px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
  font-size: var(--gn-taille-13);
  font-weight: var(--gn-graisse-gras);
}
</style>

<script setup lang="ts">
/**
 * Section 5, septième lot : le lexique (étape 2) — la ligne de terme sous ses trois
 * usages, le rail A–Z, et la pilule qui mène à un terme lié. Les spécimens sont ceux
 * de la maquette 06 ; leurs textes viennent d'ici, comme ils viendraient du savoir.
 */
const { t } = useI18n()
const k = (cle: string) => t(`gn-planche-composants-lexique.${cle}`)

const VERS = '/guide-nego/lexique/liste'

const bracketed = computed(() => ({
  term: 'bracketed text',
  acronym: null,
  translation: k('bracketed-traduction'),
  definition: k('bracketed-definition'),
}))
const gga = computed(() => ({
  term: 'global goal on adaptation',
  acronym: 'GGA',
  translation: k('gga-traduction'),
  definition: k('gga-definition'),
}))

const courante = ref<string | null>('C')
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection :titre="k('ligne')" :propos="k('ligne-propos')">
      <span class="gn-planche-composants__legende">{{ k('ligne-liste') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLigneTerme :entree="bracketed" :vers="VERS" />
        <GnLigneTerme :entree="gga" :vers="VERS" />
      </div>
      <span class="gn-planche-composants__legende">{{ k('ligne-resultat') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLigneTerme :entree="bracketed" :vers="VERS" extrait surligne="brack" />
      </div>
      <span class="gn-planche-composants__legende">{{ k('ligne-favori') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLigneTerme :entree="gga" :vers="VERS" extrait favori />
      </div>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('rail')" :propos="k('rail-propos')">
      <div class="gn-planche-composants__vitrine gn-planche-rail">
        <GnRailAlphabet :presentes="['A', 'B', 'C', 'G', 'H', 'I', 'L', 'N', 'P']" :courante="courante" @choisir="courante = $event" />
        <p class="gn-planche-note">{{ t('gn-planche-composants-lexique.rail-choisie', { lettre: courante }) }}</p>
      </div>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('lie')" :propos="k('lie-propos')">
      <div class="gn-planche-composants__vitrine gn-planche-composants__rangee">
        <GnPilule :vers="VERS"><i lang="en">agreed language</i></GnPilule>
        <GnPilule :vers="VERS"><i lang="en">landing zone</i></GnPilule>
      </div>
    </GnPlancheSection>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-planche-rail {
  flex-direction: row;
  align-items: flex-start;
  justify-content: space-between;
}
</style>

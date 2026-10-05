<script setup lang="ts">
import type { GlossaryEntry } from '~/types/negotiation-savoir'
import { intituleDe, morceauxSurlignes, premierePhrase } from '~/utils/guide-nego/lexique'

/**
 * Une entrée du lexique en ligne-carte (« Derniers consultés », maquette 03) : le terme
 * anglais à gauche, sa traduction à droite, et dans les résultats la première phrase de
 * la définition dessous. Toute la carte est la cible.
 * La saisie se marque dans le terme et la traduction.
 */
const props = withDefaults(
  defineProps<{
    entree: Pick<GlossaryEntry, 'term' | 'acronym' | 'translation' | 'definition'>
    vers: string
    extrait?: boolean
    surligne?: string
    favori?: boolean
  }>(),
  { extrait: false, surligne: '', favori: false },
)

const { t } = useI18n()

const terme = computed(() => morceauxSurlignes(intituleDe(props.entree), props.surligne))
const traduction = computed(() => morceauxSurlignes(props.entree.translation, props.surligne))
</script>

<template>
  <NuxtLink :to="vers" class="gn-ligne-terme">
    <GnPicto v-if="favori" nom="star" :taille="20" class="gn-ligne-terme__favori" />
    <span v-if="favori" class="gn-hors-ecran">{{ t('gn-ligne-terme.favori') }}</span>
    <span class="gn-ligne-terme__corps">
      <span class="gn-ligne-terme__terme" lang="en">
        <template v-for="(m, i) in terme" :key="i"><mark v-if="m.marque">{{ m.texte }}</mark><template v-else>{{ m.texte }}</template></template>
      </span>
      <span class="gn-ligne-terme__traduction">
        <template v-for="(m, i) in traduction" :key="i"><mark v-if="m.marque">{{ m.texte }}</mark><template v-else>{{ m.texte }}</template></template>
      </span>
      <span v-if="extrait" class="gn-ligne-terme__extrait">{{ premierePhrase(entree.definition) }}</span>
    </span>
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-ligne-terme {
  min-height: var(--gn-ligne-reglage);
  padding: var(--gn-espace-8) var(--gn-espace-16);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-radius: var(--gn-rayon-16);
  background: var(--gn-fond-2);
  color: var(--gn-texte);
  text-decoration: none;
}

/* Les lignes-cartes se suivent à 6 px, qu'elles soient sœurs ou chacune dans son `li`. */
[data-app="guide-nego"] .gn-ligne-terme + .gn-ligne-terme,
[data-app="guide-nego"] li + li > .gn-ligne-terme {
  margin-top: 6px;
}

[data-app="guide-nego"] .gn-ligne-terme:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-ligne-terme__corps {
  flex: 1;
  min-width: 0;
  display: grid;
  /* Deux longs intitulés reviennent chacun à la ligne dans sa colonne, la traduction gardant la droite. */
  grid-template-columns: minmax(0, 1fr) fit-content(60%);
  align-items: baseline;
  column-gap: var(--gn-espace-12);
  row-gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-ligne-terme__terme {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-ligne-terme__traduction {
  text-align: end;
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-ligne-terme__extrait {
  grid-column: 1 / -1;
  padding-top: var(--gn-espace-4);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

/* Le mot trouvé : souligné à l'accent, sans aplat. */
[data-app="guide-nego"] .gn-ligne-terme mark {
  background: none;
  color: inherit;
  font-weight: var(--gn-graisse-extra-gras);
  text-decoration: underline;
  text-decoration-color: var(--gn-accent);
  text-decoration-thickness: 2px;
  text-underline-offset: 3px;
}

[data-app="guide-nego"] .gn-ligne-terme__favori {
  color: var(--gn-accent);
}
</style>

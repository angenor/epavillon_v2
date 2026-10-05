<script setup lang="ts">
import type { FaqEntry } from '~/types/negotiation-savoir'
import { morceauxSurlignes } from '~/utils/guide-nego/lexique'

/** Une question de la FAQ en ligne, sur le dessin de la ligne de document : toute la ligne est la cible. */
const props = withDefaults(
  defineProps<{
    entree: Pick<FaqEntry, 'question' | 'verified_on' | 'status'>
    vers: string
    surligne?: string
  }>(),
  { surligne: '' },
)

const question = computed(() => morceauxSurlignes(props.entree.question, props.surligne))
</script>

<template>
  <NuxtLink :to="vers" class="gn-ligne-question">
    <span class="gn-ligne-question__corps">
      <span class="gn-ligne-question__question">
        <template v-for="(m, i) in question" :key="i"><mark v-if="m.marque">{{ m.texte }}</mark><template v-else>{{ m.texte }}</template></template>
      </span>
      <GnVerifieLe v-if="entree.verified_on" :jour="entree.verified_on" :a-revoir="entree.status === 'to_review'" />
    </span>
    <GnPicto nom="chevron" :taille="20" class="gn-ligne-question__chevron" />
  </NuxtLink>
</template>

<style>
/* Ligne de liste de la maquette 01 : titre 16/700, filet doux dessous. */
[data-app="guide-nego"] .gn-ligne-question {
  min-height: var(--gn-ligne-reglage);
  padding-block: var(--gn-ligne-air);
  display: flex;
  align-items: center;
  gap: 14px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet-doux);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-ligne-question:active {
  background: var(--gn-presse);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-ligne-question:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-ligne-question__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-ligne-question__question {
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

/* Le mot trouvé : souligné à l'accent, sans aplat, comme dans le lexique. */
[data-app="guide-nego"] .gn-ligne-question mark {
  background: none;
  color: inherit;
  font-weight: var(--gn-graisse-extra-gras);
  text-decoration: underline;
  text-decoration-color: var(--gn-accent);
  text-decoration-thickness: 2px;
  text-underline-offset: 3px;
}

[data-app="guide-nego"] .gn-ligne-question__chevron {
  color: var(--gn-picto-secondaire);
}
</style>

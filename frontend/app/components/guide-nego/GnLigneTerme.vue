<script setup lang="ts">
import type { GlossaryEntry } from '~/types/negotiation-savoir'
import { intituleDe, morceauxSurlignes, premierePhrase } from '~/utils/guide-nego/lexique'

/**
 * Une entrée du lexique en ligne : le terme anglais en italique, sa traduction, et
 * dans les résultats la première phrase de la définition. Toute la ligne est la cible.
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
    <GnPicto v-if="favori" nom="star" :taille="24" class="gn-ligne-terme__favori" />
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
    <GnPicto nom="chevron" :taille="24" class="gn-ligne-terme__chevron" />
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-ligne-terme {
  min-height: var(--gn-cible);
  padding-block: var(--gn-ligne-air);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-ligne-terme:active {
  background: var(--gn-presse);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-ligne-terme:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-ligne-terme__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-ligne-terme__terme {
  color: var(--gn-titre);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  font-style: italic;
}

[data-app="guide-nego"] .gn-ligne-terme__traduction {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-ligne-terme__extrait {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

/* Le mot trouvé : souligné et gras, sans aplat — la couleur ne décore pas. */
[data-app="guide-nego"] .gn-ligne-terme mark {
  background: none;
  color: inherit;
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-decoration-thickness: 2px;
  text-underline-offset: 3px;
}

[data-app="guide-nego"] .gn-ligne-terme__favori {
  color: var(--gn-attention);
}

[data-app="guide-nego"] .gn-ligne-terme__chevron {
  color: var(--gn-picto-secondaire);
}
</style>

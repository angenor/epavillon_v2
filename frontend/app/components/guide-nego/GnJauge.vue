<script setup lang="ts">
import { partOccupee, tailleLisible, type Place } from '~/utils/guide-nego/place'

/**
 * La place occupée : un trait de 6 px, accent sur gris pâle, **et toujours le nombre
 * écrit** — « 7 Mo utilisés · 2,1 Go libres ». Le trait est décoratif : une barre ne
 * se lit ni à voix haute ni en fort contraste, le texte oui.
 */
const props = defineProps<{ place: Place }>()

const { t, locale } = useI18n()

const part = computed(() => partOccupee(props.place))
const texte = computed(() => {
  const utilise = tailleLisible(props.place.utilise, locale.value)
  return props.place.libre === null
    ? t('gn-jauge.utilise', { utilise })
    : t('gn-jauge.utilise-libre', { utilise, libre: tailleLisible(props.place.libre, locale.value) })
})
</script>

<template>
  <div class="gn-jauge">
    <span v-if="part !== null" class="gn-jauge__piste" aria-hidden="true">
      <span class="gn-jauge__part" :style="{ inlineSize: `${part * 100}%` }" />
    </span>
    <span class="gn-jauge__texte">{{ texte }}</span>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-jauge {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-jauge__piste {
  display: block;
  block-size: var(--gn-jauge);
  background: var(--gn-jauge-fond);
}

[data-app="guide-nego"] .gn-jauge__part {
  display: block;
  block-size: 100%;
  /* Un octet occupé se voit : la part ne tombe jamais à un trait invisible. */
  min-inline-size: 4px;
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-jauge__texte {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}
</style>

<script setup lang="ts">
/**
 * Une avancée — téléchargement, lecture : 6 px `accent` sur `gris-pale`. Sans total
 * connu, la barre ne s'affiche pas : elle mentirait sur la part. `GnJauge`, elle, dit
 * une place et porte son propre texte.
 *
 * `decoratif` quand le texte voisin dit déjà la part ; sinon `libelle` nomme la barre.
 */
const props = withDefaults(
  defineProps<{
    part: number | null
    libelle?: string
    decoratif?: boolean
  }>(),
  { libelle: undefined, decoratif: false },
)

const bornee = computed(() => (props.part === null ? null : Math.min(1, Math.max(0, props.part))))
const pourcent = computed(() => (bornee.value === null ? 0 : Math.round(bornee.value * 100)))
</script>

<template>
  <span
    v-if="bornee !== null"
    class="gn-progression"
    :role="decoratif ? undefined : 'progressbar'"
    :aria-hidden="decoratif ? 'true' : undefined"
    :aria-label="decoratif ? undefined : libelle"
    :aria-valuemin="decoratif ? undefined : 0"
    :aria-valuemax="decoratif ? undefined : 100"
    :aria-valuenow="decoratif ? undefined : pourcent"
  >
    <span class="gn-progression__part" :style="{ inlineSize: `${bornee * 100}%` }" />
  </span>
</template>

<style>
[data-app="guide-nego"] .gn-progression {
  display: block;
  block-size: var(--gn-jauge);
  background: var(--gn-jauge-fond);
}

[data-app="guide-nego"] .gn-progression__part {
  display: block;
  block-size: 100%;
  background: var(--gn-accent);
}
</style>

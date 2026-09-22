<script setup lang="ts">
import { blocsDe } from '~/utils/guide-nego/texte-long'

/**
 * Un texte long — politique de confidentialité, conditions d'utilisation — dans le
 * dessin de Guide Négo. La grammaire est close (`utils/guide-nego/texte-long.ts`) :
 * **rien n'est injecté en HTML**, tout passe par l'interpolation.
 */
const props = defineProps<{ markdown: string }>()

const blocs = computed(() => blocsDe(props.markdown))
</script>

<template>
  <div class="gn-texte-long">
    <template v-for="(bloc, index) in blocs" :key="index">
      <h2 v-if="bloc.type === 'titre2'" class="gn-texte-long__titre2">
        <GnTexteLongSegments :segments="bloc.segments" />
      </h2>
      <h3 v-else-if="bloc.type === 'titre3'" class="gn-texte-long__titre3">
        <GnTexteLongSegments :segments="bloc.segments" />
      </h3>
      <p v-else-if="bloc.type === 'paragraphe'" class="gn-texte-long__paragraphe">
        <GnTexteLongSegments :segments="bloc.segments" />
      </p>
      <component
        :is="bloc.type === 'liste' ? 'ul' : 'ol'"
        v-else-if="bloc.type === 'liste' || bloc.type === 'liste-numerotee'"
        class="gn-texte-long__liste"
        :class="{ 'gn-texte-long__liste--numerotee': bloc.type === 'liste-numerotee' }"
      >
        <li v-for="(element, rang) in bloc.elements" :key="rang">
          <GnTexteLongSegments :segments="element" />
        </li>
      </component>
    </template>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-texte-long {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-texte-long__titre2 {
  padding-top: var(--gn-espace-12);
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-texte-long__titre3 {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-texte-long__liste {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-inline-start: var(--gn-espace-24);
  list-style: disc;
}

[data-app="guide-nego"] .gn-texte-long__liste--numerotee {
  list-style: decimal;
}

[data-app="guide-nego"] .gn-texte-long a {
  color: var(--gn-accent);
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>

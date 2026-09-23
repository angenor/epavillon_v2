<script setup lang="ts">
import type { SegmentSurligne } from '~/utils/guide-nego/lecteur'

/**
 * Une suite de segments de la forme lisible — jamais de HTML. Un terme anglais est un
 * seul bouton, même quand une occurrence cherchée le coupe : il ouvre sa feuille,
 * titrée du terme entier. L'occurrence courante est surlignée pleine (04 · 06).
 */
const props = defineProps<{ segments: SegmentSurligne[] }>()
defineEmits<{ terme: [texte: string] }>()

interface Groupe {
  source: number
  terme: boolean
  morceaux: SegmentSurligne[]
}

const groupes = computed<Groupe[]>(() => {
  const liste: Groupe[] = []
  for (const s of props.segments) {
    const dernier = liste.at(-1)
    if (s.term && dernier?.terme && dernier.source === s.source) dernier.morceaux.push(s)
    else liste.push({ source: s.source, terme: !!s.term, morceaux: [s] })
  }
  return liste
})

const texteDu = (g: Groupe) => g.morceaux.map((m) => m.text).join('')

const classes = (s: SegmentSurligne) => ({
  'gn-segments-lus__italique': s.italic,
  'gn-segments-lus__gras': s.bold,
  'gn-segments-lus__courant': s.surlignage === 'courant',
  'gn-segments-lus__autre': s.surlignage === 'autre',
})
</script>

<template>
  <template v-for="(g, i) in groupes" :key="i">
    <!-- Un bouton ne coule pas dans la phrase : il en couperait la ligne. Le rôle suffit. -->
    <span
      v-if="g.terme"
      role="button"
      tabindex="0"
      aria-haspopup="dialog"
      class="gn-segments-lus__terme"
      @click.stop="$emit('terme', texteDu(g))"
      @keydown.enter.prevent="$emit('terme', texteDu(g))"
      @keydown.space.prevent="$emit('terme', texteDu(g))"
    >
      <template v-for="(m, j) in g.morceaux" :key="j">
        <mark
          v-if="m.surlignage"
          :class="classes(m)"
          :data-occurrence-courante="m.surlignage === 'courant' || undefined"
          :tabindex="m.surlignage === 'courant' ? -1 : undefined"
        >{{ m.text }}</mark>
        <template v-else>{{ m.text }}</template>
      </template>
    </span>
    <template v-for="(m, j) in g.morceaux" v-else :key="j">
      <mark
        v-if="m.surlignage"
        :class="classes(m)"
        :data-occurrence-courante="m.surlignage === 'courant' || undefined"
        :tabindex="m.surlignage === 'courant' ? -1 : undefined"
      >{{ m.text }}</mark>
      <span v-else-if="m.italic || m.bold" :class="classes(m)">{{ m.text }}</span>
      <template v-else>{{ m.text }}</template>
    </template>
  </template>
</template>

<style>
[data-app="guide-nego"] .gn-segments-lus__italique {
  font-style: italic;
}

[data-app="guide-nego"] .gn-segments-lus__gras {
  font-weight: var(--gn-graisse-gras);
}

/* Le terme anglais : italique souligné (04 · 01). */
[data-app="guide-nego"] .gn-segments-lus__terme {
  font-style: italic;
  text-decoration: underline;
  text-decoration-thickness: var(--gn-filet-2);
  text-decoration-color: var(--gn-accent);
  text-underline-offset: var(--gn-espace-4);
  cursor: pointer;
}

/* Le passage atteint se pose sous la ligne qui dit la page en cours, pas au-dessus. */
[data-app="guide-nego"] [data-occurrence-courante] {
  scroll-margin-top: 30vh;
}

[data-app="guide-nego"] .gn-segments-lus__courant {
  background: var(--gn-titre);
  color: var(--gn-sur-titre);
}

[data-app="guide-nego"] .gn-segments-lus__autre {
  background: var(--gn-fond-2);
  color: var(--gn-texte);
  font-weight: var(--gn-graisse-gras);
}
</style>

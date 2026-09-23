<script setup lang="ts">
import type { OutlineEntry } from '~/types/negotiation-documents'

/**
 * Une entrée du sommaire et ses descendants, à poser dans un `ul.gn-sommaire-liste`.
 * Seul un chapitre — une entrée de premier rang — se replie : ses sous-parties restent
 * dépliées sous lui, comme dans la maquette (3.6 montre toujours 3.6.1 à 3.6.3).
 */
const props = withDefaults(
  defineProps<{
    entree: OutlineEntry
    etiquetteDe: (pageIndex: number) => string
    enCours: OutlineEntry | null
    depliees: ReadonlySet<OutlineEntry>
    /** Rang dans l'arbre, posé par la récursion : il donne le retrait, pas `level`, qui peut sauter un niveau. */
    profondeur?: number
  }>(),
  { profondeur: 0 },
)

const emit = defineEmits<{ aller: [pageIndex: number]; basculer: [entree: OutlineEntry] }>()

const { t } = useI18n()

const chapitre = computed(() => props.profondeur === 0)
const repliable = computed(() => chapitre.value && props.entree.children.length > 0)
const ouverte = computed(() => !repliable.value || props.depliees.has(props.entree))
const courante = computed(() => props.enCours === props.entree)
const nombre = computed(() => props.entree.children.length)

const libelleChevron = computed(() =>
  ouverte.value
    ? t('gn-ligne-sommaire.replier', { titre: props.entree.title })
    : t('gn-ligne-sommaire.deplier', { titre: props.entree.title, parties: t('gn-ligne-sommaire.parties', { count: nombre.value }, nombre.value) }),
)
</script>

<template>
  <li class="gn-sommaire">
    <div
      class="gn-sommaire__ligne"
      :class="{ 'gn-sommaire__ligne--chapitre': chapitre, 'gn-sommaire__ligne--en-cours': courante }"
      :style="{ '--gn-sommaire-retrait': profondeur }"
    >
      <button
        type="button"
        class="gn-sommaire__aller"
        :aria-current="courante ? 'true' : undefined"
        @click="emit('aller', entree.page_index)"
      >
        <span class="gn-sommaire__titre">{{ entree.title }}</span>
        <span v-if="repliable && !ouverte" class="gn-sommaire__compte" aria-hidden="true">{{ nombre }}</span>
        <span class="gn-sommaire__page">
          <span class="gn-hors-ecran">{{ t('gn-ligne-sommaire.page') }}</span>
          {{ etiquetteDe(entree.page_index) }}
        </span>
      </button>
      <button
        v-if="repliable"
        type="button"
        class="gn-sommaire__chevron"
        :aria-expanded="ouverte"
        :aria-label="libelleChevron"
        @click="emit('basculer', entree)"
      >
        <GnPicto :nom="ouverte ? 'chev-up' : 'chev-down'" />
      </button>
      <span v-else class="gn-sommaire__place" aria-hidden="true" />
    </div>
    <ul v-if="entree.children.length > 0 && ouverte" class="gn-sommaire-liste" role="list">
      <GnLigneSommaire
        v-for="(enfant, rang) in entree.children"
        :key="rang"
        :entree="enfant"
        :etiquette-de="etiquetteDe"
        :en-cours="enCours"
        :depliees="depliees"
        :profondeur="profondeur + 1"
        @aller="emit('aller', $event)"
        @basculer="emit('basculer', $event)"
      />
    </ul>
  </li>
</template>

<style>
[data-app="guide-nego"] .gn-sommaire-liste {
  margin: 0;
  padding: 0;
  list-style: none;
}

[data-app="guide-nego"] .gn-sommaire__ligne {
  --gn-sommaire-hauteur: var(--gn-cible);
  display: flex;
  align-items: stretch;
  /* Le chevron centré dans sa cible de 48 tombe à 16 px du bord, comme la marge d'écran. */
  padding-inline-end: calc(var(--gn-marge-ecran) - (var(--gn-cible) - var(--gn-picto-taille)) / 2);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-sommaire__ligne--chapitre {
  --gn-sommaire-hauteur: var(--gn-ligne-reglage);
}

[data-app="guide-nego"] .gn-sommaire__ligne--en-cours {
  border-inline-start: var(--gn-filet-3) solid var(--gn-filet-fort);
}

[data-app="guide-nego"] .gn-sommaire__aller {
  flex: 1;
  min-width: 0;
  min-height: var(--gn-sommaire-hauteur);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  padding-block: var(--gn-espace-4);
  padding-inline: calc(var(--gn-marge-ecran) + var(--gn-sommaire-retrait) * var(--gn-espace-16)) 0;
  border: 0;
  background: none;
  color: var(--gn-texte);
  font: inherit;
  text-align: start;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-sommaire__ligne--en-cours .gn-sommaire__aller {
  padding-inline-start: calc(var(--gn-marge-ecran) + var(--gn-sommaire-retrait) * var(--gn-espace-16) - var(--gn-filet-3));
}

[data-app="guide-nego"] .gn-sommaire__aller:active,
[data-app="guide-nego"] .gn-sommaire__chevron:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-sommaire__aller:focus-visible,
[data-app="guide-nego"] .gn-sommaire__chevron:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-anneau));
}

[data-app="guide-nego"] .gn-sommaire__titre {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
  font-size: var(--gn-taille-15);
  line-height: 1.3;
  font-weight: var(--gn-graisse-regulier);
}

[data-app="guide-nego"] .gn-sommaire__ligne--chapitre .gn-sommaire__titre {
  font-size: var(--gn-taille-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-sommaire__ligne--en-cours .gn-sommaire__titre {
  color: var(--gn-titre);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-sommaire__compte {
  flex: none;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-sommaire__page {
  flex: none;
  min-width: 28px;
  text-align: end;
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-sommaire__chevron,
[data-app="guide-nego"] .gn-sommaire__place {
  flex: none;
  width: var(--gn-cible);
}

[data-app="guide-nego"] .gn-sommaire__chevron {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 0;
  background: none;
  color: var(--gn-picto-secondaire);
  cursor: pointer;
}
</style>

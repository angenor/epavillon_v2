<script setup lang="ts">
import type { PisteJusquALaCop } from '~/utils/guide-nego/journee'

/**
 * « Jusqu'à la COP » (Nuit 05) : la piste d'aujourd'hui à la clôture, la COP à l'accent,
 * et les jalons lus dans les données. Aucun jalon écrit à la main : sans donnée, il ne
 * paraît pas.
 */
const props = defineProps<{
  piste: PisteJusquALaCop
  ouverture: string
  cloture: string
  jalons: { cle: string; titre: string; quand: string; ouverture?: boolean }[]
}>()

const { t } = useI18n()

const pct = (fraction: number) => `${(fraction * 100).toFixed(3)}%`
// Trop près d'un bord, la date d'ouverture recouvrirait « Aujourd'hui » ou la clôture.
const ouvertureLisible = computed(() => props.piste.ouverture > 0.22 && props.piste.ouverture < 0.8)
</script>

<template>
  <section class="gn-jusqua" :aria-label="t('gn-journee-jusqua.titre')">
    <h2 class="gn-jusqua__titre">{{ t('gn-journee-jusqua.titre') }}</h2>
    <div class="gn-jusqua__frise" aria-hidden="true">
      <span class="gn-jusqua__piste">
        <span class="gn-jusqua__cop" :style="{ left: pct(piste.ouverture) }" />
        <span class="gn-jusqua__aujourdhui" />
        <span v-for="j in piste.jalons" :key="j.cle" class="gn-jusqua__jalon" :style="{ left: pct(j.position) }" />
      </span>
      <span class="gn-jusqua__dates">
        <span class="gn-jusqua__date gn-jusqua__date--debut">{{ t('gn-journee-jusqua.aujourdhui') }}</span>
        <span v-if="ouvertureLisible" class="gn-jusqua__date" :style="{ left: pct(piste.ouverture) }">{{ ouverture }}</span>
        <span class="gn-jusqua__date gn-jusqua__date--fin">{{ cloture }}</span>
      </span>
    </div>
    <ul role="list" class="gn-jusqua__liste">
      <li v-for="j in jalons" :key="j.cle" class="gn-jusqua__element">
        <span class="gn-jusqua__puce" :class="{ 'gn-jusqua__puce--ouverture': j.ouverture }" />
        <span class="gn-jusqua__corps">
          <span class="gn-jusqua__nom">{{ j.titre }}</span>
          <span class="gn-jusqua__quand">{{ j.quand }}</span>
        </span>
      </li>
    </ul>
  </section>
</template>

<style>
[data-app="guide-nego"] .gn-jusqua {
  padding: 18px var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
  background: var(--gn-fond-2);
  border-radius: var(--gn-rayon-24);
}

[data-app="guide-nego"] .gn-jusqua__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-jusqua__frise {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

[data-app="guide-nego"] .gn-jusqua__piste {
  position: relative;
  display: block;
  height: 12px;
  border-radius: var(--gn-rayon-6);
  background: var(--gn-bloc-releve);
}

[data-app="guide-nego"] .gn-jusqua__cop {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  border-radius: var(--gn-rayon-6);
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-jusqua__aujourdhui {
  position: absolute;
  left: 0;
  top: -5px;
  bottom: -5px;
  width: 2px;
  border-radius: 1px;
  background: var(--gn-texte);
}

[data-app="guide-nego"] .gn-jusqua__jalon {
  position: absolute;
  top: 2px;
  width: 8px;
  height: 8px;
  margin-left: -4px;
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-jusqua__dates {
  position: relative;
  display: block;
  height: 14px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-11);
  line-height: 14px;
}

[data-app="guide-nego"] .gn-jusqua__date {
  position: absolute;
  white-space: nowrap;
}

[data-app="guide-nego"] .gn-jusqua__date--debut {
  left: 0;
}

[data-app="guide-nego"] .gn-jusqua__date--fin {
  right: 0;
}

[data-app="guide-nego"] .gn-jusqua__liste {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-jusqua__element {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-jusqua__puce {
  flex: none;
  width: 8px;
  height: 8px;
  margin-top: 6px;
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-jusqua__puce--ouverture {
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-jusqua__corps {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

[data-app="guide-nego"] .gn-jusqua__nom {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-jusqua__quand {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}
</style>

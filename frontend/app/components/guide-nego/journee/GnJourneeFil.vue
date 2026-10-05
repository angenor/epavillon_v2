<script setup lang="ts">
import type { FilDuJour } from '~/utils/guide-nego/journee'

/**
 * « Le fil du jour » (ADR-023) : les trois agendas sur une même ligne horaire, chacun
 * sur sa piste, jamais confondus. Ma prochaine session porte l'accent ; le trait
 * vertical dit maintenant.
 */
const props = defineProps<{
  titre: string
  fil: FilDuJour
  heure: string
  pistes: { libelle: string; resume: string }[]
}>()

const { t } = useI18n()

const pct = (fraction: number) => `${(fraction * 100).toFixed(3)}%`
const graduation = (h: number) => t('gn-journee-fil.graduation', { heure: String(Math.round(h)).padStart(2, '0') })
const rangees = computed(() => props.pistes.map((p, i) => ({ ...p, creneaux: props.fil.pistes[i] ?? [] })))
</script>

<template>
  <section class="gn-fil" :aria-label="t('gn-journee-fil.libelle')">
    <div class="gn-fil__tete">
      <h2 class="gn-fil__titre">{{ titre }}</h2>
      <span class="gn-fil__heure">{{ heure }}</span>
    </div>
    <div class="gn-fil__pistes">
      <div v-for="r in rangees" :key="r.libelle" class="gn-fil__rangee" role="img" :aria-label="r.resume">
        <span class="gn-fil__nom" aria-hidden="true">{{ r.libelle }}</span>
        <span class="gn-fil__piste">
          <span
            v-for="(c, i) in r.creneaux"
            :key="i"
            class="gn-fil__creneau"
            :class="{ 'gn-fil__creneau--marque': c.marque }"
            :style="{ left: pct(c.gauche), width: c.largeur === null ? undefined : pct(c.largeur) }"
          />
        </span>
      </div>
      <span v-if="fil.maintenant !== null" class="gn-fil__calque" aria-hidden="true">
        <span class="gn-fil__maintenant" :style="{ left: pct(fil.maintenant) }" />
      </span>
    </div>
    <div class="gn-fil__graduations" aria-hidden="true">
      <span v-for="h in fil.graduations" :key="h">{{ graduation(h) }}</span>
    </div>
  </section>
</template>

<style>
[data-app="guide-nego"] .gn-fil {
  --gn-fil-nom: 80px;
  padding: 18px var(--gn-espace-16) 14px;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  background: var(--gn-fond-2);
  border-radius: var(--gn-rayon-24);
}

[data-app="guide-nego"] .gn-fil__tete {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

[data-app="guide-nego"] .gn-fil__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-fil__heure {
  color: var(--gn-accent);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-gras);
  font-variant-numeric: tabular-nums;
}

[data-app="guide-nego"] .gn-fil__pistes {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

[data-app="guide-nego"] .gn-fil__rangee {
  display: flex;
  align-items: center;
  gap: 10px;
}

[data-app="guide-nego"] .gn-fil__nom {
  flex: none;
  width: 70px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-12);
  line-height: var(--gn-interligne-12);
}

[data-app="guide-nego"] .gn-fil__piste {
  position: relative;
  flex: 1;
  height: 12px;
  overflow: hidden;
  border-radius: var(--gn-rayon-6);
  background: var(--gn-bloc-releve);
}

[data-app="guide-nego"] .gn-fil__creneau {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 12px;
  min-width: 12px;
  border-radius: var(--gn-rayon-6);
  background: var(--gn-neutre-marque);
}

[data-app="guide-nego"] .gn-fil__creneau--marque {
  background: var(--gn-accent);
}

/* Le calque couvre les pistes seules, après les noms : le trait s'y place en fraction de la plage. */
[data-app="guide-nego"] .gn-fil__calque {
  position: absolute;
  inset: 0 0 0 var(--gn-fil-nom);
  pointer-events: none;
}

[data-app="guide-nego"] .gn-fil__maintenant {
  position: absolute;
  top: -6px;
  bottom: -6px;
  width: 2px;
  margin-left: -1px;
  border-radius: 1px;
  background: var(--gn-texte);
}

[data-app="guide-nego"] .gn-fil__graduations {
  margin-left: var(--gn-fil-nom);
  display: flex;
  justify-content: space-between;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-11);
  line-height: var(--gn-interligne-11);
  font-variant-numeric: tabular-nums;
}
</style>

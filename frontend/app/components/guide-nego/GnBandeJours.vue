<script setup lang="ts">
/**
 * La bande des jours (07 · 4 septies) : une case par jour qui a des sessions.
 * Les clés `AAAA-MM-JJ` sont déjà des jours de la COP ; on les lit en UTC, à midi,
 * pour que le fuseau du téléphone ne décale jamais le nom du jour.
 */
const props = defineProps<{
  jours: readonly string[]
  /** Le jour courant, dans le fuseau de la COP. */
  aujourdhui: string
}>()
const choisi = defineModel<string>({ required: true })

const { t, locale } = useI18n()

const midi = (jour: string) => {
  const [a, m, j] = jour.split('-').map(Number) as [number, number, number]
  return new Date(Date.UTC(a, m - 1, j, 12))
}

const cases = computed(() => {
  const court = new Intl.DateTimeFormat(locale.value, { weekday: 'short', timeZone: 'UTC' })
  const long = new Intl.DateTimeFormat(locale.value, { weekday: 'long', day: 'numeric', month: 'long', timeZone: 'UTC' })
  return props.jours.map((jour) => {
    const date = midi(jour)
    return {
      jour,
      semaine: court.format(date),
      numero: date.getUTCDate(),
      complet: long.format(date),
      passe: jour < props.aujourdhui,
      courant: jour === props.aujourdhui,
    }
  })
})

const rangee = useTemplateRef<HTMLElement>('rangee')

function amenerDansLaVue() {
  rangee.value?.querySelector('[aria-pressed="true"]')?.scrollIntoView({ block: 'nearest', inline: 'center' })
}

onMounted(amenerDansLaVue)
watch(choisi, () => nextTick(amenerDansLaVue))
</script>

<template>
  <div ref="rangee" class="gn-bande-jours" role="group" :aria-label="t('gn-bande-jours.libelle')">
    <button
      v-for="c in cases"
      :key="c.jour"
      type="button"
      class="gn-bande-jours__jour"
      :class="{
        'gn-bande-jours__jour--choisi': c.jour === choisi,
        'gn-bande-jours__jour--courant': c.courant,
        'gn-bande-jours__jour--passe': c.passe,
      }"
      :aria-pressed="c.jour === choisi"
      :aria-label="c.courant ? t('gn-bande-jours.aujourdhui', { jour: c.complet }) : c.complet"
      @click="choisi = c.jour"
    >
      <span class="gn-bande-jours__semaine">{{ c.semaine }}</span>
      <span class="gn-bande-jours__numero">{{ c.numero }}</span>
    </button>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-bande-jours {
  display: flex;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

[data-app="guide-nego"] .gn-bande-jours::-webkit-scrollbar {
  display: none;
}

[data-app="guide-nego"] .gn-bande-jours__jour {
  flex: 1 0 var(--gn-bande-jour-min);
  min-width: var(--gn-bande-jour-min);
  /* Peu de jours : ils restent des cases, sans s'étirer sur toute la largeur. */
  max-width: calc(var(--gn-bande-jour-min) * 1.5);
  min-height: var(--gn-bande-jour-hauteur);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border: none;
  border-bottom: var(--gn-filet-3) solid transparent;
  background: none;
  color: var(--gn-texte);
  cursor: pointer;
}

[data-app="guide-nego"] .gn-bande-jours__jour:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-bande-jours__jour:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-bande-jours__semaine {
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-bande-jours__numero {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-bande-jours__jour--passe {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-bande-jours__jour--courant {
  color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-bande-jours__jour--choisi {
  color: var(--gn-titre);
  border-bottom-color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-bande-jours__jour--choisi .gn-bande-jours__numero,
[data-app="guide-nego"] .gn-bande-jours__jour--choisi .gn-bande-jours__semaine {
  font-weight: var(--gn-graisse-gras);
}
</style>

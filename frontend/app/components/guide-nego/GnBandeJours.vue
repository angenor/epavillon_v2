<script setup lang="ts">
/**
 * La bande des jours (Nuit 02) : une pilule par jour qui a des sessions, « Mar. 10 ».
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

const majuscule = (texte: string) => texte.charAt(0).toLocaleUpperCase(locale.value) + texte.slice(1)

const cases = computed(() => {
  const court = new Intl.DateTimeFormat(locale.value, { weekday: 'short', timeZone: 'UTC' })
  const long = new Intl.DateTimeFormat(locale.value, { weekday: 'long', day: 'numeric', month: 'long', timeZone: 'UTC' })
  return props.jours.map((jour) => {
    const date = midi(jour)
    return {
      jour,
      court: majuscule(`${court.format(date)} ${date.getUTCDate()}`),
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
      {{ c.court }}
    </button>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-bande-jours {
  display: flex;
  gap: 6px;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

[data-app="guide-nego"] .gn-bande-jours::-webkit-scrollbar {
  display: none;
}

[data-app="guide-nego"] .gn-bande-jours__jour {
  flex: none;
  height: var(--gn-pilule-jour);
  padding: 0 var(--gn-espace-16);
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-22);
  background: transparent;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  font-weight: var(--gn-graisse-demi-gras);
  white-space: nowrap;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-bande-jours__jour:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-bande-jours__jour--courant {
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-bande-jours__jour--choisi,
[data-app="guide-nego"] .gn-bande-jours__jour--choisi:active {
  border-color: var(--gn-accent);
  background: var(--gn-accent);
  color: var(--gn-accent-inv);
  font-weight: var(--gn-graisse-extra-gras);
}
</style>

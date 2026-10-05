<script setup lang="ts">
import type { EditionGardee } from '~/utils/guide-nego/edition'
import { pisteJusquALaCop } from '~/utils/guide-nego/journee'

/**
 * L'accueil avant l'ouverture (Nuit 05), sous le compte à rebours : « Jusqu'à la COP »
 * et « Se préparer ». Les jalons sont les réunions de la Francophonie d'avant
 * l'ouverture, puis l'ouverture elle-même : rien n'est écrit à la main.
 */
const props = defineProps<{
  edition: EditionGardee & { debut: string; fin: string }
  maintenant: Date
  fuseau: string
}>()

const { t, locale } = useI18n()
const { dayLong } = useDateTime()
const { tr } = useI18nText()
const session = useGnSession()
const reunions = useGnReunions()
const parcours = useGnParcours()

const court = (iso: string) =>
  new Intl.DateTimeFormat(locale.value, { day: 'numeric', month: 'short', timeZone: props.fuseau }).format(new Date(iso))
const majuscule = (texte: string) => texte.charAt(0).toLocaleUpperCase(locale.value) + texte.slice(1)

const avant = computed(() =>
  reunions.reunions.value.filter(
    (r) => r.status !== 'cancelled' && new Date(r.start_at).getTime() > props.maintenant.getTime() && r.start_at < props.edition.debut,
  ),
)

const piste = computed(() =>
  pisteJusquALaCop(
    props.maintenant,
    props.edition.debut,
    props.edition.fin,
    avant.value.map((r) => ({ cle: r.id, date: r.start_at })),
  ),
)

const jalons = computed(() => {
  const places = new Set(piste.value.jalons.map((j) => j.cle))
  return [
    ...avant.value
      .filter((r) => places.has(r.id))
      .sort((a, b) => a.start_at.localeCompare(b.start_at))
      .map((r) => ({ cle: r.id, titre: tr(r.title), quand: majuscule(dayLong(r.start_at, props.fuseau)) })),
    {
      cle: 'ouverture',
      titre: t('gn-journee-avant.ouverture', { edition: props.edition.libelle }),
      quand: majuscule(dayLong(props.edition.debut, props.fuseau)),
      ouverture: true,
    },
  ]
})

const avancement = computed(() => parcours.etat.value)
const largeur = computed(() =>
  avancement.value.total > 0 ? `${Math.round((avancement.value.faites / avancement.value.total) * 100)}%` : '0%',
)

onMounted(() => {
  void reunions.rafraichir()
  void parcours.assurer()
})
</script>

<template>
  <GnJourneeJusquALaCop :piste="piste" :ouverture="court(edition.debut)" :cloture="court(edition.fin)" :jalons="jalons" />

  <section class="gn-avant">
    <h2 class="gn-avant__titre">{{ t('gn-journee-avant.se-preparer') }}</h2>
    <ul role="list">
      <li>
        <GnJourneeLigne vers="/guide-nego/ressources/parcours" picto="flag" :titre="t('gn-journee-avant.parcours.titre')">
          <span v-if="avancement.total > 0" class="gn-avant__avancement">
            <span class="gn-avant__jauge"><span class="gn-avant__rempli" :style="{ width: largeur }" /></span>
            <span class="gn-avant__compte">
              {{ t('gn-journee-avant.parcours.avancement', { faites: avancement.faites, total: avancement.total }) }}
            </span>
          </span>
        </GnJourneeLigne>
      </li>
      <li v-if="session.connectee.value">
        <GnJourneeLigne
          vers="/guide-nego/thematiques"
          picto="filter"
          :titre="t('gn-journee-avant.thematiques.titre')"
          :precision="t('gn-journee-avant.thematiques.precision')"
        />
      </li>
      <li>
        <GnJourneeLigne
          vers="/guide-nego/ressources/documents"
          picto="download"
          :titre="t('gn-journee-avant.documents.titre')"
          :precision="t('gn-journee-avant.documents.precision')"
        />
      </li>
    </ul>
  </section>
</template>

<style>
[data-app="guide-nego"] .gn-avant {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-avant__titre {
  margin-bottom: var(--gn-espace-4);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-18);
  line-height: var(--gn-interligne-18);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-avant__avancement {
  padding-top: 4px;
  display: flex;
  align-items: center;
  gap: 10px;
}

[data-app="guide-nego"] .gn-avant__jauge {
  position: relative;
  flex: 1;
  height: 4px;
  border-radius: 2px;
  background: var(--gn-bloc-releve);
}

[data-app="guide-nego"] .gn-avant__rempli {
  position: absolute;
  inset: 0 auto 0 0;
  border-radius: 2px;
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-avant__compte {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-variant-numeric: tabular-nums;
}
</style>

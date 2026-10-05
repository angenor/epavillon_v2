<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { EtatActivite, MarqueActivite } from '~/utils/guide-nego/pavillon'
import type { NoeudDeFrise } from '~/components/guide-nego/journee/GnFrise.vue'

/**
 * Une activité du Pavillon dans la frise (Nuit 02) : l'état de l'activité, puis
 * l'inscription ou la rediffusion. Le lien couvre la carte par son pseudo-élément ; la
 * rediffusion passe au-dessus, comme l'étiquette Pavillon d'une réunion. Le surtitre
 * « Pavillon de la Francophonie » la distingue des deux autres agendas.
 */
const props = withDefaults(
  defineProps<{
    activite: PublicScheduleRow
    etat: EtatActivite
    marque: MarqueActivite | null
    fuseau: string
    /** Le nom du lieu pour « heure d'Antalya » ; à défaut, celui du fuseau. */
    ville?: string | null
    /** Où elle se tient, pour une ligne lue hors de la section Pavillon (« Ma journée »). */
    lieu?: string | null
    vers: string
    /** Hors du jour choisi (la recherche globale), la ligne dit son jour au-dessus de l'heure. */
    jour?: boolean
  }>(),
  { ville: null, lieu: null, jour: false },
)

const { t } = useI18n()
const { intlLocale, time, timeRange, dayLong } = useDateTime()
const { tr } = useI18nText()

const jourCourt = computed(() =>
  new Intl.DateTimeFormat(intlLocale.value, { weekday: 'long', day: 'numeric', month: 'long', timeZone: props.fuseau }).format(
    new Date(props.activite.starts_at),
  ),
)
const surtitre = computed(() =>
  props.jour ? t('gn-ligne-activite.origine-et-jour', { jour: jourCourt.value }) : t('gn-ligne-activite.origine'),
)
const fin = computed(() => time(props.activite.ends_at, props.fuseau))
const noeud = computed<NoeudDeFrise>(() => {
  if (props.etat === 'annulee') return 'danger'
  if (props.etat === 'en-cours') return 'attention'
  if (props.etat === 'terminee' || props.etat === 'reportee') return 'eteint'
  if (props.marque?.nom === 'inscrite') return 'mienne'
  return 'neutre'
})
const debut = computed(() => time(props.activite.starts_at, props.fuseau))
const entendu = computed(() =>
  t('gn-ligne-activite.entendu', {
    jour: dayLong(props.activite.starts_at, props.fuseau),
    heures: timeRange(props.activite.starts_at, props.activite.ends_at, props.fuseau, props.ville ?? undefined),
  }),
)

const rediffusion = computed(() => (props.marque?.nom === 'rediffusion' ? props.marque : null))
const libelleRediffusion = computed(() =>
  rediffusion.value?.minutes
    ? t('gn-ligne-activite.rediffusion', { minutes: rediffusion.value.minutes })
    : t('gn-marque-etat.rediffusion'),
)
const attente = computed(() => {
  const m = props.marque
  if (m?.nom !== 'liste-attente') return undefined
  return m.position ? t('gn-ligne-activite.attente-position', { position: m.position }) : undefined
})
</script>

<template>
  <GnFrise
    :heure="debut"
    :noeud="noeud"
    :bordee="marque?.nom === 'inscrite' && etat !== 'annulee'"
    :eteinte="etat === 'terminee' || etat === 'annulee'"
  >
    <span class="gn-frise__surtitre">{{ surtitre }}</span>
    <NuxtLink :to="vers" class="gn-frise__lien gn-frise__titre" :class="{ 'gn-frise__titre--barre': etat === 'annulee' }">
      <span class="gn-hors-ecran">{{ entendu }}</span>
      {{ tr(activite.title) }}
    </NuxtLink>
    <span class="gn-frise__meta">
      <GnMarqueEtat v-if="etat === 'en-cours'" etat="en-cours" aplat />
      {{ lieu ? t('gn-ligne-activite.lieu-et-fin', { lieu, fin }) : t('gn-ligne-activite.fin', { fin }) }}
    </span>
    <GnMarqueEtat v-if="etat !== 'en-cours' && etat !== 'prevue'" :etat="etat" />
    <a v-if="rediffusion" :href="rediffusion.url" target="_blank" rel="noopener">
      <GnMarqueEtat etat="rediffusion" :libelle="libelleRediffusion" />
    </a>
    <GnMarqueEtat v-else-if="marque?.nom === 'inscrite'" etat="inscrite" />
    <GnMarqueEtat v-else-if="marque?.nom === 'liste-attente'" etat="liste-attente" :libelle="attente" />
    <GnMarqueEtat v-else-if="marque?.nom === 'complet'" etat="complet" />
    <span v-else-if="marque?.nom === 'sans-inscription'" class="gn-frise__meta">
      {{ t('gn-ligne-activite.sans-inscription') }}
    </span>
  </GnFrise>
</template>

<script setup lang="ts">
import type { FrancophoneMeeting } from '~/types/negotiation-meetings'
import type { EtatReunion } from '~/utils/guide-nego/reunions'
import { cheminDeLActivite } from '~/utils/guide-nego/pavillon'
import type { NoeudDeFrise } from '~/components/guide-nego/journee/GnFrise.vue'

/**
 * Une réunion de la Francophonie dans la frise (Nuit 02) : une seule marque d'état.
 *
 * Le lien couvre la carte par son pseudo-élément, et l'étiquette Pavillon passe
 * au-dessus : un lien dans un lien n'est pas du HTML, et un lecteur d'écran s'y perd.
 * Elle ouvre l'activité liée si l'édition gardée la connaît, sinon la section Pavillon.
 */

const props = withDefaults(
  defineProps<{
    reunion: FrancophoneMeeting
    etat: EtatReunion
    fuseau: string
    /** Le nom du lieu pour « heure d'Antalya » ; à défaut, celui du fuseau. */
    ville?: string | null
    vers: string
    /** Hors d'une liste du jour, le jour se redit au-dessus du titre. */
    jour?: boolean
  }>(),
  { ville: null, jour: true },
)

const { t } = useI18n()
const { intlLocale, time, timeRange, dayLong } = useDateTime()
const { tr } = useI18nText()
const pavillon = useGnPavillon()

const jourCourt = computed(() =>
  new Intl.DateTimeFormat(intlLocale.value, { weekday: 'long', day: 'numeric', month: 'long', timeZone: props.fuseau }).format(
    new Date(props.reunion.start_at),
  ),
)
const surtitre = computed(() =>
  props.jour ? t('gn-ligne-reunion.origine-et-jour', { jour: jourCourt.value }) : t('gn-ligne-reunion.origine'),
)
const noeud = computed<NoeudDeFrise>(() => {
  if (props.etat === 'annulee') return 'danger'
  if (props.etat === 'terminee') return 'eteint'
  if (props.etat === 'inscrite') return 'mienne'
  if (props.etat === 'liste-attente') return 'attention'
  return 'neutre'
})
const debut = computed(() => time(props.reunion.start_at, props.fuseau))
const fin = computed(() => time(props.reunion.end_at, props.fuseau))
const entendu = computed(() =>
  t('gn-ligne-reunion.entendu', {
    jour: dayLong(props.reunion.start_at, props.fuseau),
    heures: timeRange(props.reunion.start_at, props.reunion.end_at, props.fuseau, props.ville ?? undefined),
  }),
)

const lieu = computed(() => {
  const r = props.reunion
  if (r.format === 'online') return t('gn-ligne-reunion.en-ligne')
  if (!r.venue) return r.format === 'hybrid' ? t('gn-ligne-reunion.en-ligne') : t('gn-ligne-reunion.lieu-inconnu')
  return r.format === 'hybrid' ? t('gn-ligne-reunion.et-en-ligne', { lieu: r.venue }) : r.venue
})

const versPavillon = computed(() => {
  const slug = pavillon.slugDeLActiviteLiee(props.reunion.pavilion_session_id)
  return slug ? cheminDeLActivite(slug) : undefined
})

const accesLimite = computed(() => {
  const r = props.reunion
  if (r.open_access) return null
  const public_ = r.access_audience ? tr(r.access_audience) : ''
  return public_ ? t('gn-ligne-reunion.acces-limite', { public: public_ }) : t('gn-marque-etat.acces-limite')
})
</script>

<template>
  <GnFrise :heure="debut" :noeud="noeud" :bordee="etat === 'inscrite'" :eteinte="etat === 'annulee' || etat === 'terminee'">
    <span class="gn-frise__surtitre">{{ surtitre }}</span>
    <NuxtLink :to="vers" class="gn-frise__lien gn-frise__titre" :class="{ 'gn-frise__titre--barre': etat === 'annulee' }">
      <span class="gn-hors-ecran">{{ entendu }}</span>
      {{ tr(reunion.title) }}
    </NuxtLink>
    <span class="gn-frise__meta">{{ t('gn-ligne-reunion.lieu-et-fin', { lieu, fin }) }}</span>
    <GnMarqueEtat v-if="accesLimite" etat="acces-limite" :libelle="accesLimite" />
    <GnMarqueEtat :etat="etat" />
    <GnEtiquettePavillon v-if="reunion.pavilion_session_id" :vers="versPavillon" />
  </GnFrise>
</template>

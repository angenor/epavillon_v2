<script setup lang="ts">
import type { NetworkMeeting, OfficialSession } from '~/types/negotiation-sessions'
import type { NoeudDeFrise } from '~/components/guide-nego/journee/GnFrise.vue'
import { titreDeSession, type EtatAffiche } from '~/utils/guide-nego/sessions'

/**
 * Une ligne de la frise des sessions (Nuit 02) : l'heure, l'axe et son nœud coloré par
 * l'état, la carte. Une session de mon agenda est bordée d'accent. Une réunion non
 * annoncée garde sa couleur : elle ne vient pas de la source officielle.
 */
const props = withDefaults(
  defineProps<{
    session?: OfficialSession | null
    reunion?: NetworkMeeting | null
    etat?: EtatAffiche
    fuseau: string
    vers: string
    mienne?: boolean
    monGroupe?: boolean
    /** « HH:MM » de la validation du dernier signalement affiché. */
    signale?: string | null
    /** Mon agenda : les sessions suivies qui la chevauchent, signalées sans être empêchées. */
    chevauche?: OfficialSession[]
    rappel?: boolean
  }>(),
  { session: null, reunion: null, etat: 'prevue', mienne: false, monGroupe: false, signale: null, chevauche: () => [], rappel: false },
)

const { t, locale } = useI18n()
const { time } = useDateTime()

const debut = computed(() => props.session?.start_at ?? props.reunion?.start_at ?? null)
const heure = computed(() => (debut.value ? time(debut.value, props.fuseau) : t('gn-frise-session.sans-heure')))
const titre = computed(() => (props.session ? titreDeSession(props.session, locale.value) : (props.reunion?.title ?? '')))
const traduit = computed(() => locale.value === 'fr' && Boolean(props.session?.title_fr))
const genre = computed(() => (props.reunion ? 'reseau' : props.etat))
const marquee = computed(() => props.mienne && genre.value !== 'annulee' && genre.value !== 'reseau')
const eteinte = computed(() => genre.value === 'annulee' || genre.value === 'terminee')

const venue = computed(() => props.session?.venue ?? props.reunion?.venue ?? null)
const jusqua = computed(() => (props.session?.end_at ? t('gn-frise-session.jusqua', { heure: time(props.session.end_at, props.fuseau) }) : null))

/** Ce qui suit le titre : où et jusqu'à quand, ou ce qui a changé. */
const precision = computed(() => {
  const s = props.session
  if (genre.value === 'reseau') return t('gn-frise-session.non-annoncee')
  if (!s) return ''
  if (genre.value === 'annulee') return t(`gn-etat-session.annulee-sans-heure.${s.cancelled?.reason ?? 'source'}`)
  if (genre.value === 'deplacee' && s.previous) {
    const avant = s.previous
    const heureChange = avant.start_at !== s.start_at
    const salleChange = avant.venue !== s.venue
    const de = [heureChange ? time(avant.start_at, props.fuseau) : null, salleChange ? avant.venue : null].filter(Boolean).join(', ')
    const a = [heureChange ? time(s.start_at, props.fuseau) : null, salleChange ? s.venue : null].filter(Boolean).join(', ')
    if (de && a) return t('gn-frise-session.deplacee', { de, a })
  }
  const morceaux = [
    venue.value,
    genre.value === 'terminee' ? t('gn-frise-session.terminee') : jusqua.value,
    props.mienne ? t('gn-frise-session.dans-mon-agenda') : null,
    props.monGroupe ? t('gn-frise-session.mon-groupe') : null,
  ]
  return morceaux.filter(Boolean).join(' · ')
})

const chevauchements = computed(() =>
  props.chevauche.map((autre) => ({
    id: autre.id,
    texte: t('gn-frise-session.chevauche', {
      heures: autre.end_at ? `${time(autre.start_at, props.fuseau)}-${time(autre.end_at, props.fuseau)}` : time(autre.start_at, props.fuseau),
      titre: titreDeSession(autre, locale.value),
    }),
  })),
)

const noeud = computed<NoeudDeFrise>(() => {
  if (marquee.value) return 'mienne'
  if (genre.value === 'en-cours') return 'attention'
  if (genre.value === 'annulee') return 'danger'
  if (genre.value === 'reseau') return 'reseau'
  return 'neutre'
})

const ton = computed(() => {
  if (genre.value === 'annulee') return 'danger'
  if (genre.value === 'deplacee' && props.session?.previous) return 'attention'
  if (genre.value === 'reseau') return 'reseau'
  return null
})
</script>

<template>
  <GnFrise :heure="heure" :noeud="noeud" :bordee="marquee" :eteinte="eteinte">
    <NuxtLink :to="vers" class="gn-frise__lien gn-frise__titre" :class="{ 'gn-frise__titre--barre': genre === 'annulee' }">
      {{ titre }}
    </NuxtLink>
    <span class="gn-frise__meta">
      <span v-if="genre === 'en-cours'" class="gn-frise__pilule">{{ t('gn-frise-session.en-cours') }}</span>
      <span :class="ton ? `gn-frise__ton--${ton}` : undefined">{{ precision }}</span>
      <span v-if="traduit" class="gn-frise-session__traduit" :title="t('gn-frise-session.traduction')">
        <GnPicto nom="translate" :taille="16" />
        <span class="gn-hors-ecran">{{ t('gn-frise-session.traduction') }}</span>
      </span>
    </span>
    <span v-for="c in chevauchements" :key="c.id" class="gn-frise__meta gn-frise__ton--attention">{{ c.texte }}</span>
    <span v-if="rappel" class="gn-frise__meta">
      <GnPicto nom="bell" :taille="16" />
      {{ t('gn-frise-session.rappel') }}
    </span>
    <span v-if="signale" class="gn-frise__meta gn-frise__ton--attention">{{ t('gn-frise-session.signale', { heure: signale }) }}</span>
  </GnFrise>
</template>

<style>
[data-app="guide-nego"] .gn-frise-session__traduit {
  display: inline-flex;
  color: var(--gn-texte-2);
}
</style>

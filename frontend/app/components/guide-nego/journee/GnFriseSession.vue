<script setup lang="ts">
import type { NetworkMeeting, OfficialSession } from '~/types/negotiation-sessions'
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
  }>(),
  { session: null, reunion: null, etat: 'prevue', mienne: false, monGroupe: false, signale: null },
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

const ton = computed(() => {
  if (genre.value === 'annulee') return 'danger'
  if (genre.value === 'deplacee' && props.session?.previous) return 'attention'
  if (genre.value === 'reseau') return 'reseau'
  return null
})
</script>

<template>
  <NuxtLink :to="vers" class="gn-frise" :class="`gn-frise--${genre}`">
    <span class="gn-frise__heure" :class="{ 'gn-frise__heure--eteinte': eteinte }">{{ heure }}</span>
    <span class="gn-frise__axe" aria-hidden="true">
      <span class="gn-frise__trait gn-frise__trait--haut" />
      <span class="gn-frise__noeud" :class="{ 'gn-frise__noeud--mienne': marquee }" />
      <span class="gn-frise__trait" />
    </span>
    <span class="gn-frise__carte" :class="{ 'gn-frise__carte--mienne': marquee }">
      <span class="gn-frise__titre" :class="{ 'gn-frise__titre--barre': genre === 'annulee' }">{{ titre }}</span>
      <span class="gn-frise__meta">
        <span v-if="genre === 'en-cours'" class="gn-frise__en-cours">{{ t('gn-frise-session.en-cours') }}</span>
        <span :class="ton ? `gn-frise__precision--${ton}` : undefined">{{ precision }}</span>
        <span v-if="traduit" class="gn-frise__traduit" :title="t('gn-frise-session.traduction')">
          <GnPicto nom="translate" :taille="16" />
          <span class="gn-frise__cache">{{ t('gn-frise-session.traduction') }}</span>
        </span>
      </span>
      <span v-if="signale" class="gn-frise__signale">{{ t('gn-frise-session.signale', { heure: signale }) }}</span>
    </span>
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-frise {
  display: flex;
  gap: 14px;
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-frise__heure {
  flex: none;
  width: 46px;
  padding-top: var(--gn-espace-16);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__heure--eteinte {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-frise__axe {
  flex: none;
  width: 12px;
  display: flex;
  flex-direction: column;
  align-items: center;
}

[data-app="guide-nego"] .gn-frise__trait {
  flex: 1;
  width: 2px;
  background: var(--gn-filet);
}

[data-app="guide-nego"] .gn-frise__trait--haut {
  flex: none;
  height: 20px;
}

[data-app="guide-nego"] .gn-frise__noeud {
  flex: none;
  width: 12px;
  height: 12px;
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-neutre-marque);
}

[data-app="guide-nego"] .gn-frise--en-cours .gn-frise__noeud {
  background: var(--gn-attention);
}

[data-app="guide-nego"] .gn-frise--annulee .gn-frise__noeud {
  background: var(--gn-danger);
}

[data-app="guide-nego"] .gn-frise--reseau .gn-frise__noeud {
  background: var(--gn-reseau);
}

[data-app="guide-nego"] .gn-frise__noeud--mienne,
[data-app="guide-nego"] .gn-frise--en-cours .gn-frise__noeud--mienne {
  background: transparent;
  border: 3px solid var(--gn-accent);
}

[data-app="guide-nego"] .gn-frise__carte {
  flex: 1;
  min-width: 0;
  margin: 6px 0 10px;
  padding: 14px var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  background: var(--gn-fond-2);
  border: var(--gn-filet-1) solid transparent;
  border-radius: var(--gn-rayon-20);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-frise__carte--mienne {
  border-color: var(--gn-accent);
}

[data-app="guide-nego"] .gn-frise:active .gn-frise__carte {
  background: var(--gn-bloc-releve);
}

[data-app="guide-nego"] .gn-frise__titre {
  font-size: var(--gn-taille-16);
  line-height: 1.3;
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__titre--barre {
  color: var(--gn-texte-2);
  text-decoration: line-through;
}

[data-app="guide-nego"] .gn-frise__meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 10px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-frise__en-cours {
  padding: 3px 9px;
  border-radius: var(--gn-rayon-pilule);
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
  font-weight: var(--gn-graisse-extra-gras);
}

[data-app="guide-nego"] .gn-frise__precision--attention {
  color: var(--gn-attention);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__precision--danger {
  color: var(--gn-danger);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__precision--reseau {
  color: var(--gn-reseau);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-frise__traduit {
  display: inline-flex;
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-frise__cache {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

[data-app="guide-nego"] .gn-frise__signale {
  color: var(--gn-attention);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-gras);
}
</style>

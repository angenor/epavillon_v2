<script setup lang="ts">
import type { OfficialSession } from '~/types/negotiation-sessions'
import type { EtatAffiche } from '~/utils/guide-nego/sessions'
import { dayKeyInZone } from '~/utils/datetime'

/**
 * L'état d'une session officielle : la marque d'état, et ce que l'état porte —
 * l'ancienne heure et l'ancienne salle d'une déplacée, l'heure du constat d'une annulée
 * et, quand ce n'est pas la source qui l'a dit, pourquoi.
 */
const props = defineProps<{
  session: OfficialSession
  etat: EtatAffiche
  /** Le fuseau de la COP : les heures y sont lues. */
  fuseau: string
}>()

const { t } = useI18n()
const { time, date } = useDateTime()

const libelle = computed<string | undefined>(() => {
  const s = props.session
  if (props.etat === 'annulee') {
    const heure = s.cancelled ? time(s.cancelled.at, props.fuseau) : ''
    const motif = s.cancelled?.reason ?? 'source'
    return heure
      ? t(`gn-etat-session.annulee.${motif}`, { heure })
      : t(`gn-etat-session.annulee-sans-heure.${motif}`)
  }
  if (props.etat === 'deplacee' && s.previous) {
    const avant = s.previous
    const heureChangee = avant.start_at !== s.start_at
    const salleChangee = !!avant.venue && avant.venue !== s.venue
    const heure = time(avant.start_at, props.fuseau)
    // Changée de jour, l'heure seule tromperait : « était le 17 novembre 2025 à 09:00 ».
    if (heureChangee && dayKeyInZone(avant.start_at, props.fuseau) !== dayKeyInZone(s.start_at, props.fuseau)) {
      const jour = date(avant.start_at, props.fuseau)
      return salleChangee
        ? t('gn-etat-session.deplacee.jour-salle', { jour, heure, salle: avant.venue })
        : t('gn-etat-session.deplacee.jour', { jour, heure })
    }
    if (heureChangee && salleChangee) return t('gn-etat-session.deplacee.heure-salle', { heure, salle: avant.venue })
    if (heureChangee) return t('gn-etat-session.deplacee.heure', { heure })
    if (salleChangee) return t('gn-etat-session.deplacee.salle', { salle: avant.venue })
  }
  return undefined
})
</script>

<template>
  <GnMarqueEtat :etat="etat" :libelle="libelle" />
</template>

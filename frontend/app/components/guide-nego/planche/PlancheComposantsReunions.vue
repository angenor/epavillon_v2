<script setup lang="ts">
import type { FrancophoneMeeting } from '~/types/negotiation-meetings'
import type { EtatReunion } from '~/utils/guide-nego/reunions'

/**
 * Section 5, huitième lot : les réunions de la Francophonie (étape 4) — ligne de réunion,
 * étiquette Pavillon. Les spécimens sont les trois rendez-vous de la maquette 10 · 1a ;
 * l'état est posé, pas calculé.
 */
const { t } = useI18n()
const k = (cle: string) => t(`gn-planche-composants-reunions.${cle}`)

const FUSEAU = 'Europe/Istanbul'
const VILLE = 'Antalya'
const VERS = '/guide-nego/francophonie'

const le = (jour: string, heure: string) => `2026-11-${jour}T${heure}:00+03:00`

function specimen(id: string, champs: Partial<FrancophoneMeeting>): FrancophoneMeeting {
  return {
    id,
    type: { code: 'negotiators_consultation', label: { fr: k('nature') } },
    title: { fr: '' },
    description: null,
    start_at: le('17', '18:30'),
    end_at: le('17', '20:00'),
    format: 'onsite',
    venue: null,
    has_video: false,
    organizer: 'IFDD',
    open_access: true,
    access_audience: null,
    requires_registration: true,
    capacity: null,
    registered_count: 0,
    waitlist_enabled: false,
    registration_opens_at: null,
    registration_closes_at: null,
    status: 'scheduled',
    cancellation_reason: null,
    pavilion_session_id: null,
    ...champs,
  }
}

const lignes = computed<{ reunion: FrancophoneMeeting; etat: EtatReunion }[]>(() => [
  {
    reunion: specimen('atelier', {
      title: { fr: k('atelier') },
      start_at: le('08', '09:00'),
      end_at: le('08', '17:00'),
      venue: k('hotel'),
    }),
    etat: 'terminee',
  },
  {
    reunion: specimen('concertation', {
      title: { fr: k('concertation') },
      venue: k('pavillon'),
      format: 'hybrid',
      pavilion_session_id: 'activite',
    }),
    etat: 'inscrite',
  },
  {
    reunion: specimen('ministerielle', {
      title: { fr: k('ministerielle') },
      start_at: le('18', '12:30'),
      end_at: le('18', '14:00'),
      venue: k('salle'),
      open_access: false,
      access_audience: { fr: k('public') },
    }),
    etat: 'prevue',
  },
  {
    reunion: specimen('en-ligne', { title: { fr: k('en-ligne') }, format: 'online', start_at: le('19', '08:00'), end_at: le('19', '09:00') }),
    etat: 'liste-attente',
  },
  {
    reunion: specimen('complete', { title: { fr: k('complete') }, start_at: le('19', '15:00'), end_at: le('19', '16:00'), venue: k('salle') }),
    etat: 'complet',
  },
  {
    reunion: specimen('annulee', { title: { fr: k('annulee') }, start_at: le('20', '10:00'), end_at: le('20', '11:00'), venue: k('salle'), status: 'cancelled' }),
    etat: 'annulee',
  },
])
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection :titre="k('titre')" :propos="k('propos')">
      <span class="gn-planche-composants__legende">{{ k('lignes') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLigneReunion
          v-for="l in lignes"
          :key="l.reunion.id"
          :reunion="l.reunion"
          :etat="l.etat"
          :fuseau="FUSEAU"
          :ville="VILLE"
          :vers="VERS"
        />
      </div>

      <span class="gn-planche-composants__legende">{{ k('journee') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLigneReunion :reunion="lignes[1]!.reunion" etat="inscrite" :fuseau="FUSEAU" :ville="VILLE" :jour="false" :vers="VERS" />
      </div>

      <span class="gn-planche-composants__legende">{{ k('etiquette') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnEtiquettePavillon />
      </div>
    </GnPlancheSection>
  </div>
</template>

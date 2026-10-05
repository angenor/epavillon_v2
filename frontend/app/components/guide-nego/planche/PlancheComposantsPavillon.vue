<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { EtatActivite, MarqueActivite } from '~/utils/guide-nego/pavillon'

/**
 * Section 5, neuvième lot : le Pavillon de la Francophonie (étape 5) — bloc de lieu et
 * ligne d'activité. Les spécimens reprennent la maquette 10 · 1c ; état et marque sont posés.
 */
const { t } = useI18n()
const k = (cle: string) => t(`gn-planche-composants-pavillon.${cle}`)

const FUSEAU = 'Europe/Istanbul'
const VILLE = 'Antalya'
const VERS = '/guide-nego/francophonie?section=pavillon'

function specimen(id: string, heure: string, titre: string): PublicScheduleRow {
  return {
    id,
    event_id: 'planche',
    event_day_id: null,
    proposal_id: null,
    slug: id,
    title: { fr: titre },
    summary: null,
    starts_at: `2026-11-12T${heure}:00+03:00`,
    ends_at: `2026-11-12T${heure}:00+03:00`,
    timezone: FUSEAU,
    format: 'in_person',
    status: 'scheduled',
    room_id: null,
    room_name: null,
    organization_id: null,
    organization_name: null,
    organization_acronym: null,
    organization_country_code: null,
    organization_country: null,
    is_streamed: false,
    broadcast_channel_id: null,
    capacity: null,
    tracks: [],
    cover: null,
    temporal_state: 'upcoming',
    registered_count: 0,
    theme_codes: [],
    themes: [],
  }
}

const lignes = computed<{ activite: PublicScheduleRow; etat: EtatActivite; marque: MarqueActivite | null }[]>(() => [
  {
    activite: specimen('sahel', '09:30', k('sahel')),
    etat: 'terminee',
    marque: { nom: 'rediffusion', url: 'https://example.org/rediffusion', minutes: 52 },
  },
  { activite: specimen('adaptation', '11:00', k('adaptation')), etat: 'en-cours', marque: { nom: 'inscrite' } },
  { activite: specimen('femmes', '14:00', k('femmes')), etat: 'prevue', marque: { nom: 'inscrite' } },
  { activite: specimen('cdn', '16:00', k('cdn')), etat: 'prevue', marque: { nom: 'liste-attente', position: 3 } },
  { activite: specimen('complet', '17:00', k('complet')), etat: 'prevue', marque: { nom: 'complet' } },
  { activite: specimen('libre', '18:00', k('libre')), etat: 'prevue', marque: { nom: 'sans-inscription' } },
  { activite: specimen('reportee', '18:30', k('reportee')), etat: 'reportee', marque: null },
  { activite: specimen('annulee', '19:00', k('annulee')), etat: 'annulee', marque: null },
])
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection :titre="k('titre')" :propos="k('propos')">
      <span class="gn-planche-composants__legende">{{ k('lieu') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnBlocLieu :nom="k('stand')" :adresse="k('adresse')" plan="https://example.org/plan" />
        <GnBlocLieu :nom="k('stand')" />
      </div>

      <span class="gn-planche-composants__legende">{{ k('lignes') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLigneActivite
          v-for="l in lignes"
          :key="l.activite.id"
          v-bind="l"
          :fuseau="FUSEAU"
          :ville="VILLE"
          :vers="VERS"
        />
      </div>

      <span class="gn-planche-composants__legende">{{ k('journee') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLigneActivite
          v-bind="lignes[2]!"
          :lieu="k('journee-lieu')"
          :fuseau="FUSEAU"
          :ville="VILLE"
          :vers="VERS"
        />
      </div>
    </GnPlancheSection>
  </div>
</template>

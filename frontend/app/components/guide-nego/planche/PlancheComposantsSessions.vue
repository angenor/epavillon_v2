<script setup lang="ts">
import type { OfficialSession } from '~/types/negotiation-sessions'
import type { EtatAffiche } from '~/utils/guide-nego/sessions'

/**
 * Section 5, septième lot : les sessions de négociation (étape 3a) — bande des jours,
 * ligne de session et ses états, lecture impossible. Les spécimens sont les six
 * sessions de la maquette 07 ; l'état est posé, pas calculé, pour que la planche ne
 * dépende pas de l'heure qu'il est.
 */
const { t } = useI18n()
const k = (cle: string) => t(`gn-planche-composants-sessions.${cle}`)

const FUSEAU = 'Europe/Istanbul'
const VILLE = 'Antalya'
const VERS = '/guide-nego/negociations'

const jours = ['2026-11-08', '2026-11-09', '2026-11-10', '2026-11-11', '2026-11-12', '2026-11-13', '2026-11-14']
const jour = ref('2026-11-12')

const le12 = (heure: string) => `2026-11-12T${heure}:00+03:00`

function specimen(id: string, champs: Partial<OfficialSession>): OfficialSession {
  return {
    id,
    title_en: '',
    title_fr: null,
    start_at: le12('10:00'),
    end_at: le12('12:00'),
    venue: null,
    previous: null,
    type: null,
    group: null,
    theme: null,
    agenda_item: null,
    open_access: true,
    status: 'scheduled',
    cancelled: null,
    source_url: null,
    read_at: le12('11:35'),
    ...champs,
  }
}

const type = (code: string) => ({ code, label: { fr: k(`type-${code}`) }, term_en: code })

const lignes = computed<{ session: OfficialSession; etat: EtatAffiche; thematique?: string; monGroupe?: boolean }[]>(() => [
  {
    session: specimen('africain', {
      title_en: k('africain'),
      start_at: le12('08:00'),
      end_at: le12('09:30'),
      venue: k('salle-12'),
      open_access: false,
      type: type('group_coordination'),
      group: { code: 'african_group', label: { fr: k('groupe-africain') } },
    }),
    etat: 'terminee',
    monGroupe: true,
  },
  {
    session: specimen('adaptation', {
      title_en: 'Informal consultations on the global goal on adaptation',
      title_fr: k('adaptation'),
      venue: k('salle-7'),
      type: type('informal_consultations'),
      theme: 'adaptation',
    }),
    etat: 'en-cours',
    thematique: k('thematique-adaptation'),
  },
  {
    session: specimen('genre', {
      title_en: 'Contact group on the gender action plan',
      title_fr: k('genre'),
      start_at: le12('15:00'),
      end_at: le12('16:30'),
      venue: k('salle-9'),
      type: type('contact_group'),
      theme: 'gender',
      previous: { start_at: le12('11:30'), end_at: le12('13:00'), venue: k('salle-3'), changed_at: le12('10:02') },
    }),
    etat: 'deplacee',
    thematique: k('thematique-genre'),
  },
  {
    session: specimen('transition', {
      title_en: 'Informal consultations on the just transition',
      title_fr: k('transition'),
      start_at: le12('15:00'),
      end_at: le12('16:30'),
      venue: k('salle-4'),
      type: type('informal_consultations'),
      status: 'cancelled',
      cancelled: { at: le12('09:30'), reason: 'source' },
    }),
    etat: 'annulee',
  },
  {
    session: specimen('pleniere', {
      title_en: 'Closing plenary of the SBSTA',
      start_at: le12('17:00'),
      end_at: null,
      venue: k('pleniere-salle'),
      open_access: null,
      type: type('plenary'),
    }),
    etat: 'prevue',
  },
])

const retiree = computed(() =>
  specimen('retiree', { status: 'cancelled', cancelled: { at: le12('07:10'), reason: 'removed' } }),
)
const reportee = computed(() =>
  specimen('reportee', { status: 'cancelled', cancelled: { at: le12('08:45'), reason: 'postponed' } }),
)
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection :titre="k('titre')" :propos="k('propos')">
      <span class="gn-planche-composants__legende">{{ k('bande') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnBandeJours v-model="jour" :jours="jours" aujourdhui="2026-11-12" />
      </div>

      <span class="gn-planche-composants__legende">{{ k('lignes') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLigneSession
          v-for="l in lignes"
          :key="l.session.id"
          :session="l.session"
          :etat="l.etat"
          :fuseau="FUSEAU"
          :ville="VILLE"
          :thematique="l.thematique ?? null"
          :mon-groupe="l.monGroupe ?? false"
          :vers="VERS"
        />
      </div>

      <span class="gn-planche-composants__legende">{{ k('etats') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnEtatSession v-for="l in lignes" :key="l.session.id" :session="l.session" :etat="l.etat" :fuseau="FUSEAU" />
        <GnEtatSession :session="retiree" etat="annulee" :fuseau="FUSEAU" />
        <GnEtatSession :session="reportee" etat="annulee" :fuseau="FUSEAU" />
      </div>

      <span class="gn-planche-composants__legende">{{ k('coupure') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnLectureImpossible raison="unreachable" :depuis="le12('06:40')" programme="https://unfccc.int" />
      </div>
      <div class="gn-planche-composants__vitrine">
        <GnLectureImpossible raison="disabled" programme="https://unfccc.int" />
      </div>
    </GnPlancheSection>
  </div>
</template>

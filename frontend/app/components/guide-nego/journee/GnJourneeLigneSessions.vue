<script setup lang="ts">
import { ligneSessionsDuJour } from '~/utils/guide-nego/reunions'
import { etatAffiche } from '~/utils/guide-nego/sessions'

/**
 * La ligne « Sessions de négociation » de « Ma journée » (FR-021) : les sessions du jour
 * de « Mon agenda », sinon de mes thématiques. La page lit les sessions et l'agenda ;
 * cette ligne ne fait que les croiser, depuis la garde quand le réseau manque.
 */
const props = defineProps<{ maintenant: Date }>()

const { t } = useI18n()
const session = useGnSession()
const lecture = useGnSessions()
const agenda = useGnAgenda()
const thematiques = useGnThematiques()

const fuseau = computed(() => lecture.fuseau.value ?? 'UTC')
const origine = computed(() => t('gn-journee-lignes.sessions.origine'))

const ligne = computed(() =>
  ligneSessionsDuJour(agenda.agenda.value, lecture.sessions.value, thematiques.mesCodes.value, props.maintenant, fuseau.value),
)

const lignes = computed(() =>
  ligne.value.sessions.map((s) => ({
    session: s,
    etat: etatAffiche(s, props.maintenant, fuseau.value),
    thematique: s.theme ? thematiques.nomDe(s.theme) : null,
    vers: `/guide-nego/negociations/${s.id}${ligne.value.source === 'agenda' ? '?depuis=agenda' : ''}`,
  })),
)

const pret = computed(() => session.pret.value && (!session.connectee.value || lecture.etat.value.pret))
const texteVide = computed(() =>
  session.connectee.value && lecture.etat.value.valeur === null
    ? t('gn-journee-lignes.jamais-lue')
    : t('gn-journee-lignes.sessions.vide'),
)
</script>

<template>
  <div v-if="lignes.length">
    <GnLigneSession
      v-for="l in lignes"
      :key="l.session.id"
      :session="l.session"
      :etat="l.etat"
      :fuseau="fuseau"
      :ville="lecture.ville.value"
      :thematique="l.thematique"
      :origine="origine"
      forme="agenda"
      :vers="l.vers"
    />
  </div>
  <GnJourneeLigneVide v-else-if="pret" picto="nego" :origine="origine" :texte="texteVide" vers="/guide-nego/negociations" />
</template>

<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import { cheminDeLActivite, etatDeLActivite, lignePavillonDuJour } from '~/utils/guide-nego/pavillon'

/**
 * La ligne « Pavillon de la Francophonie » de « Ma journée » (FR-009) : les activités du
 * jour, sinon « Rien aujourd'hui. Prochaine : … ». Jamais filtrée par les thématiques
 * suivies (FR-003) ; lecture publique, gardée hors connexion.
 */
const props = defineProps<{ maintenant: Date }>()

const { t } = useI18n()
const { timeWithZone, dayLong } = useDateTime()
const { tr } = useI18nText()
const session = useGnSession()
const edition = useGnEdition()
const lecture = useGnPavillon()
const inscriptions = useGnInscriptionsPavillon()

const fuseau = computed(() => lecture.fuseau.value ?? 'UTC')
const ville = computed(() => edition.edition.value?.city ?? null)
const origine = computed(() => t('gn-journee-ligne-pavillon.origine'))
const ligne = computed(() => lignePavillonDuJour(lecture.activites.value, props.maintenant, fuseau.value))

function lieuDe(a: PublicScheduleRow): string | null {
  const stand = lecture.lieu.value ? tr(lecture.lieu.value.name) : ''
  const salle = a.room_name ? tr(a.room_name) : ''
  if (stand && salle) return t('gn-journee-ligne-pavillon.lieu', { stand, salle })
  return stand || salle || null
}

const lignes = computed(() =>
  ligne.value.activites.map((a) => ({
    activite: a,
    etat: etatDeLActivite(a, props.maintenant),
    marque: inscriptions.marque(a.id, props.maintenant),
    lieu: lieuDe(a),
    vers: cheminDeLActivite(a.slug),
  })),
)

const vide = computed(() => {
  const prochaine = ligne.value.prochaine
  if (prochaine) {
    return {
      texte: t('gn-journee-ligne-pavillon.prochaine', {
        titre: tr(prochaine.title),
        jour: dayLong(prochaine.starts_at, fuseau.value),
        heure: timeWithZone(prochaine.starts_at, fuseau.value, ville.value ?? undefined),
      }),
      vers: cheminDeLActivite(prochaine.slug),
    }
  }
  const jamaisLue = lecture.etat.value.valeur === null && !lecture.sansEdition.value
  return {
    texte: t(jamaisLue ? 'gn-journee-lignes.jamais-lue' : 'gn-journee-ligne-pavillon.vide'),
    vers: '/guide-nego/francophonie?section=pavillon',
  }
})

onMounted(async () => {
  void lecture.rafraichir()
  await session.assurer()
  inscriptions.assurer()
})
</script>

<template>
  <div v-if="lignes.length">
    <GnLigneActivite
      v-for="l in lignes"
      :key="l.activite.id"
      v-bind="l"
      :fuseau="fuseau"
      :ville="ville"
    />
  </div>
  <GnJourneeLigneVide v-else-if="lecture.pret.value" picto="flag" :origine="origine" :texte="vide.texte" :vers="vide.vers" />
</template>

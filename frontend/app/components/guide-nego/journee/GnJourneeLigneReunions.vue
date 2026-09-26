<script setup lang="ts">
import { ligneReunionsDuJour } from '~/utils/guide-nego/reunions'

/**
 * La ligne « Réunions de la Francophonie » de « Ma journée » (FR-021) : les réunions du
 * jour ; sinon « Rien aujourd'hui. Prochaine : … ». Lecture publique, gardée hors connexion.
 */
const props = defineProps<{ maintenant: Date }>()

const { t } = useI18n()
const { timeWithZone, dayLong } = useDateTime()
const { tr } = useI18nText()
const session = useGnSession()
const lecture = useGnReunions()
const inscriptions = useGnInscriptionsReunions()

const fuseau = computed(() => lecture.fuseau.value ?? 'UTC')
const origine = computed(() => t('gn-journee-lignes.reunions.origine'))
const ligne = computed(() => ligneReunionsDuJour(lecture.reunions.value, props.maintenant, fuseau.value))

const vide = computed(() => {
  const prochaine = ligne.value.prochaine
  if (prochaine) {
    return {
      texte: t('gn-journee-lignes.reunions.prochaine', {
        titre: tr(prochaine.title),
        jour: dayLong(prochaine.start_at, fuseau.value),
        heure: timeWithZone(prochaine.start_at, fuseau.value, lecture.ville.value ?? undefined),
      }),
      vers: `/guide-nego/francophonie/reunions/${prochaine.id}`,
    }
  }
  const jamaisLue = lecture.etat.value.valeur === null && !lecture.sansEdition.value
  return {
    texte: t(jamaisLue ? 'gn-journee-lignes.jamais-lue' : 'gn-journee-lignes.reunions.vide'),
    vers: '/guide-nego/francophonie',
  }
})

onMounted(async () => {
  void lecture.rafraichir()
  await session.assurer()
  inscriptions.assurer()
})
</script>

<template>
  <div v-if="ligne.reunions.length">
    <GnLigneReunion
      v-for="r in ligne.reunions"
      :key="r.id"
      :reunion="r"
      :etat="inscriptions.etat(r.id, maintenant) ?? 'prevue'"
      :fuseau="fuseau"
      :ville="lecture.ville.value"
      :jour="false"
      :vers="`/guide-nego/francophonie/reunions/${r.id}`"
    />
  </div>
  <GnJourneeLigneVide v-else-if="lecture.pret.value" picto="franco" :origine="origine" :texte="vide.texte" :vers="vide.vers" />
</template>

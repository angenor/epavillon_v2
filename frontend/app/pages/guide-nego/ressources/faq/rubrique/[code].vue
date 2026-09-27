<script setup lang="ts">
/** Une rubrique de la FAQ : ses questions, lues dans le savoir gardé. Public. */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const savoir = useGnSavoir()
const connexion = useGnConnexion()

const FAQ = '/guide-nego/ressources/faq'

onMounted(() => void savoir.assurer())

const code = computed(() => String(route.params.code ?? ''))
const rubrique = computed(() => savoir.rubriques.value.find((r) => r.code === code.value) ?? null)
const questions = computed(() => savoir.faq.value.filter((e) => e.section_code === code.value))
const etat = computed(() => savoir.etat.value)
const chargement = computed(() => !etat.value.pret)
const jamaisLu = computed(() => !chargement.value && !etat.value.valeur)

const sousTitre = computed(() =>
  rubrique.value ? t('guide-nego.faq-rubrique.compte', { count: questions.value.length }, questions.value.length) : undefined,
)

useHead({ title: computed(() => rubrique.value?.label ?? t('guide-nego.faq-rubrique.titre')) })
</script>

<template>
  <GnEcran :titre="rubrique?.label ?? t('guide-nego.faq-rubrique.titre')" :sous-titre="sousTitre" :retour="FAQ">
    <template #connexion>
      <GnLigneConnexion :en-ligne="connexion.etat.value.enLigne" :lu-a="etat.luA" />
    </template>

    <GnChargement v-if="chargement" forme="squelette" :lignes="4" :libelle="t('guide-nego.faq-rubrique.chargement')" />

    <GnEtatErreur
      v-else-if="jamaisLu"
      :titre="t('guide-nego.faq.erreur.titre')"
      :texte="t('guide-nego.faq.erreur.texte')"
    />

    <GnEtatVide
      v-else-if="!rubrique || !questions.length"
      picto="quiz"
      :titre="t('guide-nego.faq-rubrique.vide.titre')"
      :texte="t('guide-nego.faq-rubrique.vide.texte')"
      :sortie="t('guide-nego.faq-rubrique.vide.sortie')"
      :sortie-vers="FAQ"
    />

    <ul v-else role="list" class="gn-faq-rubrique">
      <li v-for="e in questions" :key="e.id">
        <GnLigneQuestion :entree="e" :vers="`${FAQ}/${e.id}`" />
      </li>
    </ul>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-faq-rubrique {
  padding-top: var(--gn-espace-8);
}
</style>

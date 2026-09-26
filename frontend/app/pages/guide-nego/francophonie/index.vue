<script setup lang="ts">
/**
 * Écran 10 — l'onglet Francophonie : deux sections, jamais une liste mêlée (ADR-008).
 * Le titre dit le nom complet de la section, le segment actif est plein. La section
 * Pavillon est un seul composant, que l'étape 5 réécrit sans toucher cette page.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

type Section = 'reunions' | 'pavillon'

const { t } = useI18n()
const { dayLong, zoneLabel } = useDateTime()
const route = useRoute()
const router = useRouter()
const connexion = useGnConnexion()
const edition = useGnEdition()
const reunions = useGnReunions()
const pavillon = useGnPavillon()
const jourDuPavillon = useState<string | null>('gn-pavillon-jour-affiche', () => null)

const k = (cle: string, params: Record<string, unknown> = {}) => t(`guide-nego.francophonie.${cle}`, params)

const section = computed<Section>(() => (route.query.section === 'pavillon' ? 'pavillon' : 'reunions'))
const segments = computed(() => [
  { valeur: 'reunions', libelle: k('sections.reunions') },
  { valeur: 'pavillon', libelle: k('sections.pavillon') },
])
const choix = computed<string>({
  get: () => section.value,
  set: (valeur) => void router.replace({ query: { ...route.query, section: valeur === 'pavillon' ? 'pavillon' : 'reunions' } }),
})

const titre = computed(() => k(section.value === 'pavillon' ? 'titre-pavillon' : 'titre-reunions'))

const sousTitre = computed(() => {
  const e = edition.edition.value
  if (section.value === 'pavillon') {
    const fuseau = pavillon.fuseau.value ?? reunions.fuseau.value ?? e?.timezone
    if (!fuseau) return e?.libelle
    const affiche = jourDuPavillon.value
    const jour = affiche ? dayLong(`${affiche}T12:00:00Z`, 'UTC') : dayLong(new Date(), fuseau)
    const zone = zoneLabel(fuseau, reunions.ville.value ?? e?.city ?? undefined)
    return k('sous-titre-pavillon', { jour: jour.charAt(0).toLocaleUpperCase() + jour.slice(1), zone })
  }
  if (!e) return undefined
  const n = reunions.reunions.value.length
  return t('guide-nego.francophonie.sous-titre-reunions', { edition: e.libelle, count: n }, n)
})

const luA = computed(() => (section.value === 'reunions' ? reunions.luA.value : pavillon.luA.value))

useHead({ title: titre })
</script>

<template>
  <GnEcran :titre="titre" :sous-titre="sousTitre" :ce-qui-se-lit="k('hors-connexion')" cloche>
    <template #connexion>
      <GnLigneConnexion :en-ligne="connexion.etat.value.enLigne" :lu-a="luA" />
    </template>

    <GnSegmente v-model="choix" :segments="segments" :libelle="k('sections.libelle')" class="gn-francophonie__sections" />

    <GnSectionPavillon v-if="section === 'pavillon'" />
    <GnSectionReunions v-else />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-francophonie__sections {
  margin-top: var(--gn-espace-12);
}
</style>

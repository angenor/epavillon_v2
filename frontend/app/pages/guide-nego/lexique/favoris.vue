<script setup lang="ts">
/**
 * « Mes termes favoris » — maquette 06, écran 05. Se lit sans réseau ; sans compte,
 * les favoris sont ceux du téléphone. Public : l'état « accès refusé » est sans objet.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const savoir = useGnSavoir()
const session = useGnSession()
const { favoris, assurer: assurerLesFavoris } = useGnFavorisLexique()

const favorisLus = ref(false)
const retour = ref('/guide-nego/lexique')

onMounted(async () => {
  const avant: unknown = window.history.state?.back
  if (typeof avant === 'string') retour.value = avant
  void savoir.assurer()
  await assurerLesFavoris().catch(() => undefined)
  favorisLus.value = true
})

const etat = computed(() => savoir.etat.value)
const chargement = computed(() => !etat.value.pret || !favorisLus.value)
const jamaisLu = computed(() => !chargement.value && !etat.value.valeur)
// Dans l'ordre alphabétique du lexique ; un favori dépublié ne s'affiche pas.
const termes = computed(() => savoir.lexique.value.filter((e) => favoris.value.has(e.id)))

const sousTitre = computed(() =>
  chargement.value || jamaisLu.value
    ? undefined
    : t('guide-nego.lexique-favoris.sous-titre', { count: termes.value.length }, termes.value.length),
)

useHead({ title: t('guide-nego.lexique-favoris.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.lexique-favoris.titre')" :sous-titre="sousTitre" :retour="retour" lexique-ouvert>
    <GnChargement v-if="chargement" forme="squelette" :lignes="3" :libelle="t('guide-nego.lexique-favoris.chargement')" />

    <GnEtatErreur
      v-else-if="jamaisLu"
      :titre="t('guide-nego.lexique-favoris.erreur.titre')"
      :texte="t('guide-nego.lexique-favoris.erreur.texte')"
    />

    <GnEtatVide
      v-else-if="!termes.length"
      picto="star"
      :titre="t('guide-nego.lexique-favoris.vide.titre')"
      :texte="t('guide-nego.lexique-favoris.vide.texte')"
      :sortie="t('guide-nego.lexique-favoris.vide.sortie')"
      sortie-vers="/guide-nego/lexique/liste"
    />

    <template v-else>
      <ul role="list">
        <li v-for="e in termes" :key="e.id">
          <GnLigneTerme :entree="e" :vers="`/guide-nego/lexique/${e.slug}`" extrait favori />
        </li>
      </ul>
      <p class="gn-termes-favoris__note">{{ t('guide-nego.lexique-favoris.rappel') }}</p>
    </template>

    <p v-if="!chargement && !session.connectee.value" class="gn-termes-favoris__note">
      {{ t('guide-nego.lexique-favoris.sans-compte') }}
    </p>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-termes-favoris__note {
  padding-block: var(--gn-espace-12);
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}
</style>

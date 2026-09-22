<script setup lang="ts">
import type { LegalTextKey } from '~/types/platform'

/**
 * Un texte qui engage, en entier, dans le dessin de Guide Négo — avec sa version,
 * et son heure de lecture quand le réseau manque (FR-033).
 *
 * Quatre états : chargement ; **vide**, c'est-à-dire en préparation par l'IFDD ;
 * erreur, jamais reçu sur cet appareil ; l'accès refusé est sans objet — ces
 * textes se lisent sans compte, comme au site.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const route = useRoute()
const connexion = useGnConnexion()
const { momentLisible } = useGnMomentLecture()

const CLES: readonly LegalTextKey[] = ['privacy', 'terms']
const demandee = String(route.params.cle)
const cle = CLES.find((c) => c === demandee) ?? null

const lecture = cle ? useGnTexte(cle) : null
onMounted(() => void lecture?.rafraichir())

const texte = computed(() => lecture?.texte.value ?? null)
const titre = computed(() => (cle ? t(`guide-nego.textes.${cle}`) : t('guide-nego.textes.inconnu.titre')))

const sousTitre = computed(() => {
  const lu = texte.value
  if (!lu) return undefined
  if (lu.status === 'published' && lu.effective_date) {
    const date = new Intl.DateTimeFormat(locale.value, { dateStyle: 'long', timeZone: 'UTC' }).format(
      new Date(`${lu.effective_date}T00:00:00Z`),
    )
    return t('guide-nego.textes.en-vigueur', { date, version: lu.version })
  }
  return t('guide-nego.textes.version', { version: lu.version })
})

const luHorsConnexion = computed(() => {
  if (connexion.etat.value.enLigne || !lecture) return null
  const moment = momentLisible(lecture.luA.value)
  return moment ? t('guide-nego.textes.lu', { moment }) : null
})

useHead({ title: titre.value })
</script>

<template>
  <GnEcran
    :titre="titre"
    :sous-titre="sousTitre"
    retour="/guide-nego/ressources/a-propos"
    :onglets="false"
  >
    <GnEtatVide
      v-if="!cle"
      picto="doc"
      :titre="t('guide-nego.textes.inconnu.titre')"
      :texte="t('guide-nego.textes.inconnu.texte')"
      :sortie="t('guide-nego.textes.inconnu.retour')"
      sortie-vers="/guide-nego/ressources/a-propos"
    />

    <GnChargement
      v-else-if="!lecture?.pret.value"
      forme="squelette"
      :lignes="8"
      :libelle="t('guide-nego.textes.chargement')"
    />

    <GnEtatErreur
      v-else-if="!texte"
      :titre="t('guide-nego.textes.erreur.titre')"
      :texte="t('guide-nego.textes.erreur.texte')"
      :sortie="t('guide-nego.textes.erreur.reessayer')"
      @sortie="lecture?.rafraichir()"
    />

    <!-- En attente du texte de l'IFDD : rien n'est écrit à sa place. -->
    <GnEtatVide
      v-else-if="texte.status === 'pending' || !texte.body"
      picto="doc"
      :titre="t('guide-nego.textes.en-preparation.titre')"
      :texte="t('guide-nego.textes.en-preparation.texte')"
    />

    <template v-else>
      <p v-if="luHorsConnexion" class="gn-texte__lu">{{ luHorsConnexion }}</p>
      <GnTexteLong :markdown="texte.body" class="gn-texte__corps" />
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-texte__lu {
  padding-top: var(--gn-espace-12);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-texte__corps {
  padding-block: var(--gn-espace-16);
}
</style>

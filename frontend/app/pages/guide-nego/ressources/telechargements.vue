<script setup lang="ts">
/**
 * « Mes téléchargements » — ce qui est gardé sur ce téléphone, et la place que
 * Guide Négo y occupe.
 *
 * À cette étape aucun document n'est gardé : ils arrivent avec les documents de
 * négociation. L'écran montre malgré tout la place réelle — la coquille et les
 * données lues comptent déjà (FR-023).
 *
 * **Rien ici ne demande de compte** : ce qui est téléchargé vit sur le téléphone, pas
 * sur le compte. L'état « accès refusé » est donc sans objet, et c'est voulu.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { place, mesuree, mesurer, liberer } = useGnPlace()

const confirmation = ref(false)
const liberation = ref(false)
const liberee = ref(false)

onMounted(() => void mesurer())

async function libererLaPlace(): Promise<void> {
  liberation.value = true
  try {
    await liberer()
    liberee.value = true
  } finally {
    liberation.value = false
  }
}

useHead({ title: t('guide-nego.telechargements.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.telechargements.titre')"
    :sous-titre="t('guide-nego.telechargements.sous-titre')"
    retour="/guide-nego/ressources/reglages"
    :onglets="false"
  >
    <GnEtatVide
      picto="download"
      :titre="t('guide-nego.telechargements.vide.titre')"
      :texte="t('guide-nego.telechargements.vide.texte')"
    />

    <GnEnteteGroupe :titre="t('guide-nego.telechargements.place.titre')" />
    <div class="gn-telechargements__place">
      <GnChargement v-if="!mesuree" forme="squelette" :lignes="2" :libelle="t('guide-nego.telechargements.place.mesure')" />

      <!-- Le navigateur ne mesure pas : on le dit, jamais un zéro faux (R8). -->
      <GnLigneInformation v-else-if="!place" :texte="t('guide-nego.telechargements.place.impossible')" />

      <template v-else>
        <GnJauge :place="place" />
        <p class="gn-telechargements__aide">{{ t('guide-nego.telechargements.place.aide') }}</p>
        <GnBouton variante="secondaire" :chargement="liberation" @clic="confirmation = true">
          {{ t('guide-nego.telechargements.liberer.bouton') }}
        </GnBouton>
      </template>
    </div>

    <GnConfirmation
      v-model="confirmation"
      :question="t('guide-nego.telechargements.liberer.question')"
      :phrase="t('guide-nego.telechargements.liberer.phrase')"
      :action="t('guide-nego.telechargements.liberer.action')"
      :retour="t('guide-nego.telechargements.liberer.retour')"
      @confirmer="libererLaPlace"
    />

    <GnMessageEphemere
      v-if="liberee"
      :texte="t('guide-nego.telechargements.liberer.faite')"
      @fini="liberee = false"
    />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-telechargements__place {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-telechargements__aide {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
  max-width: var(--gn-mesure-lecture);
}
</style>

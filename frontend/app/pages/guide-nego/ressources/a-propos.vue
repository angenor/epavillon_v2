<script setup lang="ts">
import type { LegalTextKey } from '~/types/platform'
import { editionDuGuide, type EditionGardee } from '~/utils/guide-nego/edition'

/**
 * Écran 12 — « À propos ». **Il n'affirme que ce qui est vrai aujourd'hui** :
 * l'éditeur, l'origine des sessions sans parler d'un accord qui n'est pas encore
 * obtenu, ce qui reste sur le téléphone et ce qui suit le compte — sans nommer
 * d'hébergeur, sans restitutions, qui n'existent pas.
 *
 * **Aucun interrupteur d'accord** (écart 40) : aucun n'aurait d'effet. Les textes
 * qui engagent viennent de l'API, comme au site ; tant que l'IFDD ne les a pas
 * fournis, la ligne le dit. L'écran se lit sans compte (FR-037).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const api = useApi()
const config = useRuntimeConfig()

const edition = useGnLecture<EditionGardee | null>('edition', async () =>
  editionDuGuide(await api.events.publicList()),
)
const textes: Record<LegalTextKey, ReturnType<typeof useGnTexte>> = {
  privacy: useGnTexte('privacy'),
  terms: useGnTexte('terms'),
}

onMounted(() => {
  void edition.rafraichir()
  void textes.privacy.rafraichir()
  void textes.terms.rafraichir()
})

const sousTitre = computed(() => {
  const libelle = edition.etat.value.valeur?.libelle
  return libelle
    ? t('guide-nego.a-propos.sous-titre', { edition: libelle })
    : t('guide-nego.a-propos.sous-titre-sans-edition')
})

/** L'identifiant de la construction servie : c'est lui que la garde porte. */
const version = computed(() => String(config.app.buildId ?? '').slice(0, 8))

function valeurDu(cle: LegalTextKey): string {
  const { texte, pret, lu } = textes[cle]
  if (!pret.value) return t('guide-nego.a-propos.textes.chargement')
  if (!lu.value || !texte.value) return t('guide-nego.a-propos.textes.illisible')
  if (texte.value.status === 'pending') return t('guide-nego.a-propos.textes.en-preparation')
  return t('guide-nego.a-propos.textes.version', { version: texte.value.version })
}

useHead({ title: t('guide-nego.a-propos.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.a-propos.titre')"
    :sous-titre="sousTitre"
    retour="/guide-nego/ressources/reglages"
    :onglets="false"
  >
    <div class="gn-a-propos__presentation">
      <p v-if="version" class="gn-a-propos__version">{{ t('guide-nego.a-propos.version', { version }) }}</p>
      <p>{{ t('guide-nego.a-propos.editeur') }}</p>
      <p class="gn-a-propos__source">{{ t('guide-nego.a-propos.source') }}</p>
    </div>

    <GnEnteteGroupe :titre="t('guide-nego.a-propos.confidentialite.titre')" />
    <p class="gn-a-propos__paragraphe">{{ t('guide-nego.a-propos.confidentialite.texte') }}</p>

    <GnEnteteGroupe :titre="t('guide-nego.a-propos.textes.titre')" />
    <GnLigneReglage
      :libelle="t('guide-nego.textes.privacy')"
      :valeur="valeurDu('privacy')"
      picto="shield"
      vers="/guide-nego/ressources/textes/privacy"
    />
    <GnLigneReglage
      :libelle="t('guide-nego.textes.terms')"
      :valeur="valeurDu('terms')"
      picto="doc"
      vers="/guide-nego/ressources/textes/terms"
      derniere
    />

    <GnEnteteGroupe :titre="t('guide-nego.a-propos.licences.titre')" />
    <GnLigneReglage
      :libelle="t('guide-nego.a-propos.licences.police')"
      :valeur="t('guide-nego.a-propos.licences.police-licence')"
    />
    <GnLigneReglage
      :libelle="t('guide-nego.a-propos.licences.bibliotheques')"
      :valeur="t('guide-nego.a-propos.licences.bibliotheques-licence')"
      derniere
    />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-a-propos__presentation {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-16);
  max-width: var(--gn-mesure-lecture);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-a-propos__version,
[data-app="guide-nego"] .gn-a-propos__source {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-a-propos__paragraphe {
  padding-top: var(--gn-espace-12);
  max-width: var(--gn-mesure-lecture);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}
</style>

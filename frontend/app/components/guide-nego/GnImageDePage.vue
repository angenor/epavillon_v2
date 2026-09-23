<script setup lang="ts">
/**
 * L'image d'une page d'origine, dans ses quatre états. Tant qu'elle se charge, sa place
 * est réservée — au format d'une page A4 en mode « tel quel » : sans cela les pages
 * grandiraient une à une, et la page de reprise glisserait hors de l'écran.
 */
withDefaults(
  defineProps<{
    etat: 'aucune' | 'attente' | 'chargee' | 'hors-connexion' | 'echec'
    adresse: string | null
    /** Le texte de remplacement de l'image. */
    libelle: string
    /** Ce qui s'affiche sans réseau. */
    attente: string
    /** Une page entière (« tel quel ») : la place réservée est celle d'une page. */
    pleine?: boolean
  }>(),
  { pleine: false },
)

defineEmits<{ echec: []; reessayer: [] }>()

const { t } = useI18n()
</script>

<template>
  <img
    v-if="etat === 'chargee' && adresse"
    :src="adresse"
    :alt="libelle"
    class="gn-image-page"
    @error="$emit('echec')"
  />
  <p v-else-if="etat === 'hors-connexion'" class="gn-image-page__ligne">
    <GnPicto nom="wifi-off" :taille="20" />
    {{ attente }}
  </p>
  <p v-else-if="etat === 'echec'" class="gn-image-page__ligne gn-image-page__ligne--echec">
    <GnPicto nom="warn" :taille="20" />
    <span class="gn-image-page__texte">{{ t('gn-image-de-page.echec') }}</span>
    <button type="button" class="gn-image-page__reessayer" @click.stop="$emit('reessayer')">
      {{ t('gn-image-de-page.reessayer') }}
    </button>
  </p>
  <div
    v-else
    class="gn-image-page__place"
    :class="{ 'gn-image-page__place--pleine': pleine }"
    role="img"
    :aria-label="t('gn-image-de-page.chargement')"
  />
</template>

<style>
[data-app="guide-nego"] .gn-image-page {
  display: block;
  inline-size: 100%;
  block-size: auto;
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-image-page__place {
  inline-size: 100%;
  min-block-size: var(--gn-cible);
  background: var(--gn-squelette);
}

[data-app="guide-nego"] .gn-image-page__place--pleine {
  aspect-ratio: 210 / 297;
}

[data-app="guide-nego"] .gn-image-page__ligne {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  min-height: var(--gn-cible);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

/* Écart 20 : le triangle rouge dit l'erreur. */
[data-app="guide-nego"] .gn-image-page__ligne--echec {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-image-page__texte {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-image-page__reessayer {
  flex: none;
  min-height: var(--gn-cible);
  padding-inline: var(--gn-espace-8);
  border: none;
  background: none;
  color: var(--gn-accent);
  font: inherit;
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: var(--gn-espace-4);
  cursor: pointer;
}
</style>

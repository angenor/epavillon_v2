<script setup lang="ts">
const props = defineProps<{
  /** Titre du document à jour. */
  titre: string
  publieLe: string
  pages: number | null
  /** Le document à jour est déjà sur le téléphone. */
  telecharge: boolean
  /** Le document à jour est réservé et la personne n'a pas l'accès. */
  reserveSansAcces?: boolean
  vers: string
}>()

const { t } = useI18n()
const { date } = useDateTime()

// Un fait de publication, pas un créneau : il se lit dans le fuseau du téléphone, sans heure.
const fuseau = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'

const ligne = computed(() => {
  if (props.reserveSansAcces) return t('gn-bandeau-remplace.reserve')
  const publie = date(props.publieLe, fuseau)
  return [
    publie ? t('gn-bandeau-remplace.publie-le', { date: publie }) : null,
    props.pages === null ? null : t('gn-bandeau-remplace.pages', { count: props.pages }, props.pages),
    props.telecharge ? t('gn-bandeau-remplace.telecharge') : null,
  ]
    .filter((m): m is string => !!m)
    .join(' · ')
})
</script>

<template>
  <NuxtLink :to="vers" class="gn-bandeau-remplace">
    <GnPicto nom="refresh" :taille="20" class="gn-bandeau-remplace__picto" />
    <span class="gn-bandeau-remplace__corps">
      <span class="gn-bandeau-remplace__mot">{{ t('gn-bandeau-remplace.remplace-par') }}</span>
      <span class="gn-bandeau-remplace__titre">{{ titre }}</span>
      <span v-if="ligne" class="gn-bandeau-remplace__ligne">{{ ligne }}</span>
    </span>
    <GnPicto nom="chevron" :taille="24" class="gn-bandeau-remplace__chevron" />
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-bandeau-remplace {
  min-height: var(--gn-cible);
  padding: var(--gn-espace-12) var(--gn-marge-ecran);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  background: var(--gn-attention-fond);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-bandeau-remplace:active {
  filter: brightness(0.9);
}

[data-app="guide-nego"] .gn-bandeau-remplace:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-bandeau-remplace__picto {
  /* Centré sur la première ligne, celle du mot « Remplacé par ». */
  margin-block-start: calc((1em * var(--gn-interligne-15) - var(--gn-picto-marque)) / 2);
  font-size: var(--gn-taille-15);
  color: var(--gn-attention);
}

[data-app="guide-nego"] .gn-bandeau-remplace__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-bandeau-remplace__mot {
  color: var(--gn-attention);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-bandeau-remplace__titre {
  color: var(--gn-accent);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration: underline;
  text-underline-offset: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-bandeau-remplace__ligne {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-bandeau-remplace__chevron {
  color: var(--gn-picto-secondaire);
}
</style>

<script setup lang="ts">
/**
 * Un groupe de la recherche globale (09 Recherche) : son nom en capitales et son
 * compte, ses lignes, et au besoin le lien vers la suite dans l'écran du contenu.
 * Les lignes sont celles de chaque contenu — terme, question, document, session.
 */
withDefaults(
  defineProps<{
    titre: string
    /** « 2 entrées », « 1 réponse », « 1 aujourd'hui » : le compte se dit avec son nom. */
    compte: string
    /** Les résultats au-delà de ceux montrés, dans l'écran qui les porte tous. */
    suite?: { libelle: string; vers: string } | null
  }>(),
  { suite: null },
)
</script>

<template>
  <section class="gn-groupe-resultats">
    <GnEnteteGroupe :titre="titre" :compteur="compte" />
    <ul role="list" class="gn-groupe-resultats__liste">
      <slot />
    </ul>
    <NuxtLink v-if="suite" :to="suite.vers" class="gn-groupe-resultats__suite">
      {{ suite.libelle }}
      <GnPicto nom="chevron" :taille="20" />
    </NuxtLink>
  </section>
</template>

<style>
[data-app="guide-nego"] .gn-groupe-resultats .gn-groupe__compteur {
  font-weight: var(--gn-graisse-regulier);
}

[data-app="guide-nego"] .gn-groupe-resultats__liste > li:last-child > * {
  border-bottom: none;
}

/* La saisie marquée, dans toutes les lignes du groupe : gras souligné, comme au lexique. */
[data-app="guide-nego"] .gn-groupe-resultats mark {
  background: none;
  color: inherit;
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-decoration-thickness: 2px;
  text-underline-offset: 3px;
}

[data-app="guide-nego"] .gn-groupe-resultats__suite {
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-accent);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-groupe-resultats__suite:active {
  background: var(--gn-presse);
}
</style>

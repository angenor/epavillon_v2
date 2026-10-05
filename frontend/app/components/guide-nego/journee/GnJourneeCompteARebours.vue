<script setup lang="ts">
/**
 * Le compte à rebours de l'accueil (ADR-023) : minutes avant la prochaine session,
 * ou jours avant l'ouverture. Le chiffre porte l'accent ; ce qu'il compte se lit dessous.
 */
withDefaults(
  defineProps<{
    libelle: string
    valeur: string
    titre?: string
    quand?: string
    vers?: string
  }>(),
  { titre: undefined, quand: undefined, vers: undefined },
)
</script>

<template>
  <section class="gn-compte-a-rebours">
    <p class="gn-compte-a-rebours__libelle">{{ libelle }}</p>
    <p class="gn-compte-a-rebours__valeur">{{ valeur }}</p>
    <NuxtLink v-if="titre && vers" :to="vers" class="gn-compte-a-rebours__cible">
      <span class="gn-compte-a-rebours__titre">{{ titre }}</span>
      <span v-if="quand" class="gn-compte-a-rebours__quand">{{ quand }}</span>
    </NuxtLink>
    <p v-else-if="quand" class="gn-compte-a-rebours__detail">{{ quand }}</p>
  </section>
</template>

<style>
[data-app="guide-nego"] .gn-compte-a-rebours {
  padding-top: 6px;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-compte-a-rebours__libelle {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-compte-a-rebours__valeur {
  color: var(--gn-accent);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-76);
  line-height: var(--gn-interligne-76);
  font-weight: var(--gn-graisse-extra-gras);
  letter-spacing: var(--gn-approche-76);
  font-variant-numeric: tabular-nums;
}

[data-app="guide-nego"] .gn-compte-a-rebours__cible {
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-compte-a-rebours__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-20);
  line-height: 1.25;
  font-weight: var(--gn-graisse-demi-gras);
  text-wrap: pretty;
}

[data-app="guide-nego"] .gn-compte-a-rebours__quand,
[data-app="guide-nego"] .gn-compte-a-rebours__detail {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-compte-a-rebours__detail {
  padding-top: 6px;
}
</style>

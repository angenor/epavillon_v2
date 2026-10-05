<script setup lang="ts">
import type { AvatarDEntete } from '~/components/guide-nego/GnEntete.vue'

/**
 * L'en-tête de l'accueil (Nuit 01, 04, 05) : l'avatar et le jour, ou le nom de
 * l'application et « Se connecter » sans compte. Le titre de l'écran reste pour les
 * lecteurs d'écran : la maquette ne l'affiche pas.
 */
withDefaults(
  defineProps<{
    titre: string
    jour: string
    contexte: string
    avatar?: AvatarDEntete
    cloche?: boolean
    nonLues?: number
  }>(),
  { avatar: undefined, cloche: false, nonLues: 0 },
)

const { t } = useI18n()
</script>

<template>
  <header class="gn-journee-entete">
    <h1 class="gn-journee-entete__titre">{{ titre }}</h1>
    <div v-if="avatar" class="gn-journee-entete__qui">
      <GnAvatar :prenom="avatar.prenom" :nom="avatar.nom" :image="avatar.image" vers="/guide-nego/ressources/reglages" />
      <span class="gn-journee-entete__textes">
        <span class="gn-journee-entete__jour">{{ jour }}</span>
        <span class="gn-journee-entete__contexte">{{ contexte }}</span>
      </span>
    </div>
    <span v-else class="gn-journee-entete__textes">
      <span class="gn-journee-entete__appli">{{ t('gn-journee-entete.appli') }}</span>
      <span class="gn-journee-entete__contexte">{{ t('gn-journee-entete.jour-et-contexte', { jour, contexte }) }}</span>
    </span>
    <div class="gn-journee-entete__actions">
      <template v-if="avatar">
        <GnLoupe />
        <GnCloche v-if="cloche" :non-lues="nonLues" />
      </template>
      <NuxtLink v-else to="/guide-nego/connexion" class="gn-journee-entete__connexion">
        {{ t('gn-journee-entete.se-connecter') }}
      </NuxtLink>
    </div>
  </header>
</template>

<style>
[data-app="guide-nego"] .gn-journee-entete {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-journee-entete__titre {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

[data-app="guide-nego"] .gn-journee-entete__qui {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-journee-entete__textes {
  min-width: 0;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-journee-entete__jour {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-journee-entete__appli {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-18);
  line-height: var(--gn-interligne-18);
  font-weight: var(--gn-graisse-extra-gras);
  letter-spacing: -0.01em;
}

[data-app="guide-nego"] .gn-journee-entete__contexte {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-journee-entete__actions {
  flex: none;
  display: flex;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-journee-entete__connexion {
  height: var(--gn-bouton-rond);
  padding-inline: var(--gn-espace-16);
  display: flex;
  align-items: center;
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-22);
  color: var(--gn-texte);
  font-size: var(--gn-taille-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-journee-entete__connexion:active {
  background: var(--gn-presse);
}
</style>

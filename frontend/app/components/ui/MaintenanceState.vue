<script setup lang="ts">
/**
 * ESPACE EN COURS DE MAINTENANCE — l'état qu'affiche un module présent dans le
 * modèle de données mais fermé dans le jalon en cours.
 *
 * CE N'EST NI UNE ERREUR NI UN ÉTAT VIDE, et les confondre trompe le visiteur :
 * une erreur invite à réessayer, un état vide laisse croire qu'il n'y a rien à
 * voir. Ici, il y a bien quelque chose, et ce quelque chose n'est pas encore
 * ouvert. D'où une pastille d'attention — jaune, la couleur de ce qui demande
 * de patienter — et une phrase qui dit ce que l'espace portera. La pastille
 * porte déjà l'état : le répéter en sous-titre n'ajoute rien et occupe la place
 * de la seule information utile, à savoir ce qu'on trouvera ici.
 *
 * L'AFFICHAGE RESTE COMMANDÉ PAR `platform.feature_flags` : c'est à la page ou
 * au layout de décider si l'espace est ouvert. Ce composant ne rend que le cas
 * fermé.
 */

interface Props {
  /** Titre de l'espace concerné — « Négociations », « Communauté ». */
  title: string
  /** Ce que l'espace portera, en une phrase. Sans elle, le texte générique. */
  description?: string
}

const props = defineProps<Props>()

const { t } = useI18n()
</script>

<template>
  <section class="mx-auto flex max-w-(--measure) flex-col items-center py-12 text-center sm:py-16">
    <UiBadge intent="warning" solid :label="t('nav.maintenance.badge')" />

    <h1 class="mt-4 font-display text-3xl text-text">{{ props.title }}</h1>

    <p class="mt-3 text-base text-text-secondary">
      {{ props.description ?? t('nav.maintenance.description') }}
    </p>
  </section>
</template>

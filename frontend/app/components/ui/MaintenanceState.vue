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
 * L'AFFICHAGE EST COMMANDÉ PAR `platform.feature_flags`, ET PAR LE ROUTAGE.
 * Depuis A14, ce n'est plus à la page de décider : le middleware global
 * `feature-flag` sert `pages/maintenance/[module].vue` quand le drapeau
 * `<module>.enabled` est éteint. Ce composant ne rend que le cas fermé, et son
 * seul appelant normal est cette page-là.
 *
 * LE CRÉNEAU `actions` PORTE LE RENVOI VERS CE QUI EST OUVERT. Annoncer une
 * fermeture sans dire où aller laisse le visiteur sur une impasse polie ; c'est
 * un créneau et non des propriétés, parce que les destinations dépendent de
 * l'écran et qu'aucune liste figée ici ne vieillirait bien.
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
    <UiBadge intent="warning" solid :label="t('maintenance-state.badge')" />

    <h1 class="mt-4 font-display text-3xl text-text">{{ props.title }}</h1>

    <p class="mt-3 text-base text-text-secondary">
      {{ props.description ?? t('maintenance-state.description') }}
    </p>

    <div v-if="$slots.actions" class="mt-8 w-full">
      <slot name="actions" />
    </div>
  </section>
</template>

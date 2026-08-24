<script setup lang="ts">
/**
 * Enveloppe des cinq écrans d'authentification : elle tient les états communs.
 *
 * POURQUOI UN COMPOSANT PLUTÔT QUE CINQ FOIS LE MÊME `v-if`. Les quatre états
 * obligatoires — chargement, vide, erreur, accès refusé — sont ce qu'on oublie
 * en dernier et ce qui se voit en premier. Écrits cinq fois, ils divergent :
 * une page traite l'erreur, la voisine l'oublie, et rien ne le signale. Ici,
 * trois d'entre eux sont tenus une seule fois pour les cinq écrans :
 *
 *  · CHARGEMENT — la session est en cours de résolution. Au rendu serveur, elle
 *    l'est toujours : sans ce cas, chaque page enverrait son formulaire avant de
 *    savoir si quelqu'un est déjà connecté, puis le remplacerait.
 *  · ERREUR — la session n'a pas pu être chargée. Le bouton de reprise rejoue
 *    l'appel ; il n'est proposé que parce qu'il a une chance d'aboutir.
 *  · ACCÈS REFUSÉ — une personne déjà connectée ouvre un écran de connexion.
 *
 * Le quatrième — l'état VIDE — n'a pas de forme commune : il ne se présente que
 * sur les écrans à jeton (aucun lien suivi) et sur l'inscription (référentiel des
 * pays introuvable). Chacun le rend dans son propre contexte, avec ses mots.
 *
 * `allowAuthenticated` sert l'écran de vérification d'adresse : suivre le lien
 * reçu par courriel alors qu'on est connecté est un cas parfaitement ordinaire,
 * et le refuser obligerait à se déconnecter pour valider sa propre adresse.
 */

interface Props {
  /** Laisser passer une personne connectée ? Faux partout sauf vérification. */
  allowAuthenticated?: boolean
  /** Destination du bouton « Continuer » de l'état « déjà connecté ». */
  continueTo?: string
}

const props = withDefaults(defineProps<Props>(), { allowAuthenticated: false, continueTo: '/' })

const { t } = useI18n()
const auth = useAuthStore()

const showSignedIn = computed(
  () => !props.allowAuthenticated && auth.isResolved && auth.isAuthenticated,
)
</script>

<template>
  <div>
    <UiLoadingState
      v-if="!auth.isResolved && auth.loadError === null"
      variant="text"
      :lines="4"
      :label="t('common.states.loading.label')"
    />

    <UiErrorState
      v-else-if="auth.loadError !== null"
      :title="t('common.states.error.title')"
      :description="t('common.states.error.description')"
      :detail="loadFailureMessage(auth.loadError, t)"
      :retry-label="t('common.actions.retry')"
      @retry="auth.retryLoad()"
    />

    <AuthSignedInNotice
      v-else-if="showSignedIn && auth.person"
      :person="auth.person"
      :continue-to="props.continueTo"
    />

    <slot v-else />
  </div>
</template>

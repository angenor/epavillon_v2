<script setup lang="ts">
import type { Person } from '~/types/identity'

/**
 * Écran présenté à une personne DÉJÀ CONNECTÉE qui ouvre une page
 * d'authentification.
 *
 * C'EST L'ÉTAT « ACCÈS REFUSÉ » DE CES CINQ ÉCRANS, sous la seule forme qui ait
 * un sens ici. `UiForbiddenState` parle de droits manquants — « votre compte ne
 * dispose pas des droits nécessaires » — ce qui serait faux : rien ne manque, la
 * page ne s'adresse simplement pas à cette personne-là.
 *
 * POURQUOI NE PAS REDIRIGER. Un renvoi automatique vers l'accueil laisse croire
 * que le clic a raté. Surtout, le cas le plus fréquent n'est pas l'erreur : c'est
 * quelqu'un qui veut se connecter AVEC UN AUTRE COMPTE — un chargé de programme
 * qui bascule sur le compte d'une organisation qu'il accompagne. La déconnexion
 * est donc offerte ici, à l'endroit exact où le besoin apparaît.
 *
 * LE COMPTE EN COURS EST NOMMÉ. Sans cela, « vous êtes déjà connecté » ne dit
 * pas sous quelle identité, et l'on se déconnecte pour vérifier.
 */

interface Props {
  person: Person
  /** Où mène « Continuer ». L'accueil par défaut. */
  continueTo?: string
}

const props = withDefaults(defineProps<Props>(), { continueTo: '/' })

const { t } = useI18n()
const localePath = useLocalePath()
const auth = useAuthStore()

const isSigningOut = ref(false)

async function signOut(): Promise<void> {
  isSigningOut.value = true
  try {
    await auth.signOut()
  } finally {
    isSigningOut.value = false
  }
}
</script>

<template>
  <AuthCard
    icon="check-circle"
    icon-intent="success"
    :title="t('auth-form.signedIn.title')"
    :description="t('auth-form.signedIn.description')"
  >
    <dl class="rounded-md border border-border bg-surface-sunken px-4 py-3 text-sm">
      <dt class="text-text-subtle">{{ t('auth-form.signedIn.account') }}</dt>
      <dd class="mt-0.5 font-bold text-text">{{ props.person.display_name }}</dd>
      <dd class="mt-0.5 break-words text-text-muted">{{ props.person.primary_email }}</dd>
    </dl>

    <div class="mt-5 grid gap-2">
      <UiButton
        :to="localePath(props.continueTo)"
        variant="primary"
        block
        icon-trailing="arrow-right"
        :label="t('auth-form.signedIn.continue')"
      />
      <UiButton
        variant="secondary"
        block
        :loading="isSigningOut"
        :label="t('auth-form.signedIn.signOut')"
        @click="signOut"
      />
    </div>
  </AuthCard>
</template>

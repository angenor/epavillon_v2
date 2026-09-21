<script setup lang="ts">
/**
 * Le retour depuis le courriel de réinitialisation. **Mêmes exigences que le
 * site** — huit caractères, une majuscule, une minuscule — et elles ne sont pas
 * recopiées : `utils/password-strength.ts` les porte, l'API les refait.
 *
 * Le jeton est contrôlé **avant** d'afficher le formulaire, pour ne pas faire
 * composer un mot de passe en vain ; il est revérifié à l'envoi, et c'est là
 * seulement qu'il compte.
 */
import { MIN_PASSWORD_LENGTH, missingRequirements } from '~/utils/password-strength'

definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const api = useApi()
const route = useRoute()

type Issue = 'attente' | 'formulaire' | 'refuse' | 'fait' | 'panne'

const issue = ref<Issue>('attente')
const motDePasse = ref('')
const faute = ref<string | null>(null)
const envoi = ref(false)

const jeton = computed(() => {
  const valeur = route.query.token
  return typeof valeur === 'string' ? valeur : null
})

onMounted(async () => {
  if (!jeton.value) {
    issue.value = 'refuse'
    return
  }
  try {
    const controle = await api.auth.checkPasswordResetToken(jeton.value)
    issue.value = controle.status === 'valid' ? 'formulaire' : 'refuse'
  } catch {
    issue.value = 'panne'
  }
})

async function envoyer(): Promise<void> {
  faute.value = null
  if (!jeton.value || envoi.value) return

  if (missingRequirements(motDePasse.value).length > 0) {
    faute.value = t('guide-nego.nouveau-mot-de-passe.faute', { min: MIN_PASSWORD_LENGTH })
    return
  }

  envoi.value = true
  try {
    const reponse = await api.auth.resetPassword(jeton.value, motDePasse.value)
    issue.value = reponse.status === 'reset' ? 'fait' : 'refuse'
  } catch (erreur) {
    faute.value =
      erreur instanceof Error && erreur.message ? erreur.message : t('api.unreachable.network')
  } finally {
    envoi.value = false
  }
}

useHead({ title: t('guide-nego.nouveau-mot-de-passe.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.nouveau-mot-de-passe.titre')" :onglets="false">
    <GnChargement
      v-if="issue === 'attente'"
      forme="arc"
      :libelle="t('guide-nego.nouveau-mot-de-passe.attente')"
    />

    <form v-else-if="issue === 'formulaire'" novalidate @submit.prevent="envoyer">
      <GnChamp
        v-model="motDePasse"
        type="password"
        :libelle="t('guide-nego.nouveau-mot-de-passe.champ')"
        :aide="t('guide-nego.nouveau-mot-de-passe.aide')"
        :erreur="faute ?? undefined"
        autocomplete="new-password"
      />
      <div class="gn-nouveau-mdp__sorties">
        <GnBouton type="submit" variante="principal" :chargement="envoi">
          {{ t('guide-nego.nouveau-mot-de-passe.enregistrer') }}
        </GnBouton>
      </div>
    </form>

    <GnEtatVide
      v-else-if="issue === 'fait'"
      picto="check"
      :titre="t('guide-nego.nouveau-mot-de-passe.fait.titre')"
      :texte="t('guide-nego.nouveau-mot-de-passe.fait.texte')"
      :sortie="t('guide-nego.nouveau-mot-de-passe.vers-connexion')"
      sortie-vers="/guide-nego/connexion"
    />

    <GnEtatErreur
      v-else-if="issue === 'refuse'"
      :titre="t('guide-nego.nouveau-mot-de-passe.refuse.titre')"
      :texte="t('guide-nego.nouveau-mot-de-passe.refuse.texte')"
      :sortie="t('guide-nego.nouveau-mot-de-passe.redemander')"
      sortie-vers="/guide-nego/mot-de-passe-oublie"
    />

    <GnEtatErreur
      v-else
      :titre="t('guide-nego.nouveau-mot-de-passe.panne')"
      :texte="t('api.unreachable.network')"
      :sortie="t('guide-nego.nouveau-mot-de-passe.vers-connexion')"
      sortie-vers="/guide-nego/connexion"
    />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-nouveau-mdp__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}
</style>

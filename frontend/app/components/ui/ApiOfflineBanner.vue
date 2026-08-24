<script setup lang="ts">
/**
 * BANDEAU DE MODE DÉGRADÉ — la plateforme ne répond plus.
 *
 * IL N'Y EN A QU'UN, POUR TOUTE L'APPLICATION. Une page qui charge six blocs
 * afficherait sinon six messages d'erreur identiques, et la personne en
 * conclurait que six choses sont cassées. Ici, un seul message dit la vraie
 * cause, et chaque bloc garde son état d'erreur pour ce qui le concerne.
 *
 * IL DIT AUSSI CE QUE ÇA CHANGE : les données affichées peuvent dater, et un
 * enregistrement n'aboutira pas. Sans cette phrase, quelqu'un continue de
 * remplir un formulaire de vingt champs qui ne partira nulle part.
 *
 * IL DISPARAÎT TOUT SEUL. Le bouton force un essai, mais le premier appel qui
 * aboutit — une navigation, un chargement — suffit à le retirer.
 *
 * À NE PAS CONFONDRE AVEC `UiIncidentBanner`, qui porte un message publié par
 * l'équipe depuis le back-office. Celui-ci n'est publié par personne : il est
 * constaté.
 */
const { reachable, reason } = useApiStatus()
const { client, isConfigured } = useApi()
const { t } = useI18n()

const enCours = ref(false)

/**
 * Une sonde, pas un rechargement de page. `/ready` ne divulgue rien et ne
 * demande aucune session : c'est la route faite pour ça. Elle passe par le
 * client brut — l'envelopper reviendrait à signaler une panne pour une sonde
 * qui échoue, ce qui est son travail normal.
 */
async function reessayer(): Promise<void> {
  if (enCours.value) return
  enCours.value = true
  try {
    await client('/ready')
    await reloadNuxtApp({ persistState: true })
  } catch {
    // Toujours injoignable : le bandeau reste, il n'y a rien à ajouter.
  } finally {
    enCours.value = false
  }
}
</script>

<template>
  <div
    v-if="isConfigured && !reachable"
    role="alert"
    class="w-full border-b border-warning-border bg-warning-surface text-warning"
  >
    <div class="mx-auto flex w-full max-w-[1280px] flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:gap-4 sm:px-6">
      <UiIcon name="warning" class="hidden shrink-0 sm:block" aria-hidden="true" />

      <div class="min-w-0 flex-1">
        <p class="font-medium">{{ t('api.offline.title') }}</p>
        <p class="text-sm text-text-secondary">
          {{ reason ? t(`api.unreachable.${reason}`) : t('api.offline.description') }}
        </p>
      </div>

      <button
        type="button"
        class="min-h-[var(--target-min)] shrink-0 cursor-pointer rounded-md border border-current px-4 text-sm font-medium disabled:cursor-progress disabled:opacity-70"
        :disabled="enCours"
        @click="reessayer"
      >
        {{ enCours ? t('api.offline.checking') : t('api.offline.retry') }}
      </button>
    </div>
  </div>
</template>

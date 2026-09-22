<script setup lang="ts">
import type { EffectivePermission } from '~/types/identity'
import type { AdmissionMode } from '~/types/negotiation'
import type { SelectOption } from '~/types/ui'

/**
 * LE MODE D'ADMISSION DE GUIDE NÉGO.
 *
 * TROIS VALEURS, ET CE QUE CHACUNE PRODUIT POUR LA PERSONNE QUI ENTRE (FR-042).
 * L'API rend des FAITS — le champ de code est-il proposé, un code juste
 * ouvre-t-il, faut-il trancher — et l'écran compose ses phrases par ses
 * fichiers de traduction, comme tout écran du site. Rendre du texte français
 * depuis l'API donnerait deux catalogues pour un même écran.
 *
 * LA BASCULE PREND EFFET À LA TENTATIVE SUIVANTE, sans mise en ligne : la
 * valeur est relue à chaque saisie de code, sans cache. C'est SC-002, et c'est
 * ce que la phrase de confirmation promet.
 *
 * CHANGER DE MODE N'ANNULE AUCUNE DEMANDE EN ATTENTE (FR-029). L'écran le dit
 * **avant** la bascule quand il y en a : une personne qui attend une réponse
 * doit l'obtenir, quel que soit le réglage du lendemain.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationAdmission' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()

useHead(() => ({ title: t('admin.negociations.admission.title') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canManage = computed(() => hasPermission(granted.value, 'negotiation.space.manage'))

const { data: settings, status, error, refresh } = await useAsyncData(
  'admin-negotiation-admission',
  () => api.adminNegotiations.admission(),
  { lazy: true },
)

/** La file sert l'avertissement de FR-029 : on ne bascule pas sans savoir qui attend. */
const { data: queue } = await useAsyncData(
  'admin-negotiation-admission-queue',
  () => api.adminNegotiations.demandes('pending'),
  { lazy: true },
)

const choisi = ref<AdmissionMode | null>(null)
const submitting = ref(false)
const writeError = ref<string | null>(null)
const saved = ref(false)

/** Ce qui est en base commande l'écran tant que personne n'a choisi autre chose. */
const courant = computed<AdmissionMode | null>(() => choisi.value ?? settings.value?.mode ?? null)
const modifie = computed(() => Boolean(settings.value && courant.value !== settings.value.mode))

const options = computed(() => settings.value?.options ?? [])
const choixCourant = computed(() => options.value.find((o) => o.mode === courant.value) ?? null)

/** Les trois modes, avec ce que chacun produit en second rang. */
const choix = computed<SelectOption[]>(() =>
  options.value.map((option) => ({
    value: option.mode,
    label: t(`admin.negociations.admission.modes.${option.mode}.label`),
    description: t(`admin.negociations.admission.modes.${option.mode}.description`),
  })),
)

function selectionner(mode: AdmissionMode): void {
  choisi.value = mode
  saved.value = false
  writeError.value = null
}

async function enregistrer(): Promise<void> {
  if (!courant.value || submitting.value) return
  submitting.value = true
  writeError.value = null
  try {
    settings.value = await api.adminNegotiations.changerLeMode(courant.value)
    choisi.value = null
    saved.value = true
  } catch (erreur) {
    // Le message vient de l'API et s'affiche tel quel.
    writeError.value = erreur instanceof Error ? erreur.message : t('api.unreachable.network')
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-3xl">
    <UiForbiddenState
      v-if="!canManage && permissionStatus !== 'pending'"
      :required-scope="t('admin.negociations.admission.forbidden.scope')"
      :description="t('admin.negociations.admission.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.negociations.admission.title') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.negociations.admission.subtitle') }}
        </p>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiLoadingState v-else-if="status === 'pending' && !settings" class="mt-8" />

      <template v-else-if="settings">
        <UiAlert v-if="writeError" class="mt-6" intent="danger" live :message="writeError" />
        <UiAlert
          v-else-if="saved"
          class="mt-6"
          intent="success"
          live
          dismissible
          :message="t('admin.negociations.admission.saved')"
        />

        <UiAlert
          v-if="queue && queue.pending > 0"
          class="mt-6"
          intent="warning"
          compact
          :message="t('admin.negociations.admission.pendingWarning', { count: queue.pending })"
        />

        <UiRadio
          class="mt-8"
          name="admission-mode"
          :label="t('admin.negociations.admission.choose')"
          :options="choix"
          :model-value="courant ?? ''"
          @update:model-value="(value: string) => selectionner(value as AdmissionMode)"
        />

        <section v-if="choixCourant" class="mt-8">
          <h2 class="text-lg font-semibold">
            {{ t('admin.negociations.admission.effects.title') }}
          </h2>
          <ul class="mt-3 space-y-2 text-text-muted">
            <li class="flex items-start gap-2">
              <UiIcon :name="choixCourant.offers_code ? 'check' : 'minus'" size="1.15rem" />
              <span>
                {{
                  choixCourant.offers_code
                    ? t('admin.negociations.admission.effects.offersCode')
                    : t('admin.negociations.admission.effects.noCode')
                }}
              </span>
            </li>
            <li class="flex items-start gap-2">
              <UiIcon :name="choixCourant.code_opens ? 'check' : 'clock'" size="1.15rem" />
              <span>
                {{
                  choixCourant.code_opens
                    ? t('admin.negociations.admission.effects.codeOpens')
                    : t('admin.negociations.admission.effects.needsApproval')
                }}
              </span>
            </li>
          </ul>
        </section>

        <div class="mt-8 flex flex-wrap gap-3">
          <UiButton :disabled="!modifie" :loading="submitting" @click="enregistrer">
            {{ t('admin.negociations.admission.save') }}
          </UiButton>
          <UiButton variant="ghost" icon="inbox" to="/admin/negociations/demandes">
            {{ t('admin.negociations.admission.queueLink') }}
          </UiButton>
        </div>
      </template>
    </template>
  </div>
</template>

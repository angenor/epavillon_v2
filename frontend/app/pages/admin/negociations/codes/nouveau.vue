<script setup lang="ts">
import type {
  CreateInvitationCodePayload,
  InvitationCodeRow,
  InvitationCodeScopePayload,
} from '~/types/admin-negotiation'
import type { EffectivePermission } from '~/types/identity'

/**
 * CRÉER UN CODE D'INVITATION.
 *
 * LE CODE NE SE CHOISIT PAS. L'API l'engendre — huit caractères, tirets
 * compris, sans `0/O` ni `1/I/L`, parce qu'il se recopie à l'œil depuis
 * WhatsApp. Offrir un champ « code souhaité » produirait des codes devinables
 * (« COP31 », « IFDD2026 ») sur une porte que rien d'autre ne protège.
 *
 * LA PORTÉE N'A QUE DEUX VALEURS, et ce ne sont pas celles de tout le
 * back-office : `global` ou `negotiation_space`, les deux `allowed_scopes` du
 * rôle `negotiator`. Une portée d'édition n'existe pas ici — aucun espace de
 * négociation n'est rattaché à une COP.
 *
 * LE CODE EST MONTRÉ APRÈS CRÉATION, et il reste lisible dans la liste
 * (FR-037) : l'écran de réussite sert à le diffuser tout de suite, pas à le
 * révéler une seule fois.
 *
 * AUCUNE VALIDATION NE DOUBLE LA BASE. `ck_invitation_codes_uses`,
 * `_period` et `_scope` refusent un quota nul, une période inversée et une
 * portée incohérente ; l'écran pose le message de l'API sous le champ qu'elle
 * nomme.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationCodes', to: '/admin/negociations/codes' },
    { labelKey: 'admin.negociations.codes.form.title' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.negociations.codes.form.title') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

/** Sur la portée globale, et elle seule — comme l'API le teste. */
const canManage = computed(() => hasPermission(granted.value, 'negotiation.space.manage'))

const { data: screen } = await useAsyncData('admin-negotiation-codes-referentiels', () =>
  api.adminNegotiations.codes(),
)

const label = ref('')
const scopeKind = ref<'global' | 'negotiation_space'>('negotiation_space')
const spaceId = ref('')
const network = ref('')
const maxUses = ref('')
const validFrom = ref('')
const validUntil = ref('')

const submitting = ref(false)
const formError = ref<string | null>(null)
const errorField = ref<string | null>(null)
const created = ref<InvitationCodeRow | null>(null)
const copied = ref(false)

const spaces = computed(() => screen.value?.spaces ?? [])

const spaceOptions = computed(() => [
  { value: '', label: t('admin.negociations.codes.form.space.placeholder') },
  ...spaces.value.map((espace) => ({ value: espace.id, label: espace.name })),
])

const networkOptions = computed(() => [
  { value: '', label: t('admin.negociations.codes.form.network.none') },
  ...(screen.value?.networks ?? []).map((reseau) => ({ value: reseau.code, label: reseau.label })),
])

const scopeOptions = computed(() => [
  {
    value: 'negotiation_space',
    label: t('admin.negociations.codes.form.scope.space'),
  },
  { value: 'global', label: t('admin.negociations.codes.form.scope.global') },
])

/** Le seul espace disponible se choisit tout seul : un écran ne fait pas choisir sans choix. */
watchEffect(() => {
  if (!spaceId.value && spaces.value.length === 1) spaceId.value = spaces.value[0]!.id
})

/** Une date de formulaire (`AAAA-MM-JJ`) devient un instant ; vide reste nul. */
function instant(jour: string): string | null {
  return jour ? new Date(`${jour}T00:00:00`).toISOString() : null
}

async function submit(): Promise<void> {
  if (submitting.value) return
  formError.value = null
  errorField.value = null

  if (!label.value.trim()) {
    formError.value = t('admin.negociations.codes.form.error.label')
    errorField.value = 'label'
    return
  }
  if (scopeKind.value === 'negotiation_space' && !spaceId.value) {
    formError.value = t('admin.negociations.codes.form.error.space')
    errorField.value = 'space'
    return
  }

  const scope: InvitationCodeScopePayload =
    scopeKind.value === 'global' ? { type: 'global' } : { type: 'negotiation_space', id: spaceId.value }

  const charge: CreateInvitationCodePayload = {
    label: label.value.trim(),
    scope,
    grants_network: network.value || null,
    max_uses: maxUses.value ? Number(maxUses.value) : null,
    valid_from: instant(validFrom.value),
    valid_until: instant(validUntil.value),
  }

  submitting.value = true
  try {
    created.value = await api.adminNegotiations.creerUnCode(charge)
  } catch (erreur) {
    // Le message vient de l'API et s'affiche tel quel : son catalogue est déjà
    // français, et en écrire un second donnerait deux textes pour un refus.
    formError.value = erreur instanceof Error ? erreur.message : t('api.unreachable.network')
    errorField.value = erreur instanceof ApiRequestError ? (erreur.field ?? null) : null
  } finally {
    submitting.value = false
  }
}

async function copier(): Promise<void> {
  if (!created.value) return
  try {
    await navigator.clipboard.writeText(created.value.code)
    copied.value = true
  } catch {
    // Un presse-papiers refusé n'est pas une panne : le code reste à l'écran,
    // et il se recopie à la main — c'est de toute façon ce qui lui arrivera.
    copied.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-4xl">
    <UiForbiddenState
      v-if="!canManage && permissionStatus !== 'pending'"
      :required-scope="t('admin.negociations.codes.list.forbidden.scope')"
      :description="t('admin.negociations.codes.list.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else-if="created">
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.negociations.codes.form.created.title') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.negociations.codes.form.created.description') }}
        </p>
      </header>

      <UiCard class="mt-6 text-center">
        <p class="font-mono text-4xl font-semibold tracking-[0.2em]">{{ created.code }}</p>
        <p class="mt-2 text-text-muted">{{ created.label }}</p>
        <div class="mt-5 flex flex-wrap justify-center gap-3">
          <UiButton icon="copy" variant="secondary" @click="copier">
            {{
              copied
                ? t('admin.negociations.codes.form.created.copied')
                : t('admin.negociations.codes.form.created.copy')
            }}
          </UiButton>
          <UiButton :to="localePath(`/admin/negociations/codes/${created.id}`)">
            {{ t('admin.negociations.codes.form.created.detail') }}
          </UiButton>
          <UiButton variant="ghost" :to="localePath('/admin/negociations/codes')">
            {{ t('admin.negociations.codes.form.created.back') }}
          </UiButton>
        </div>
      </UiCard>
    </template>

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.negociations.codes.form.title') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.negociations.codes.form.subtitle') }}
        </p>
      </header>

      <UiAlert v-if="formError" class="mt-6" intent="danger" live :message="formError" />

      <form class="mt-6 space-y-5" novalidate @submit.prevent="submit">
        <UiFormField
          :label="t('admin.negociations.codes.form.label.label')"
          :hint="t('admin.negociations.codes.form.label.hint')"
          :error="errorField === 'label' ? formError ?? undefined : undefined"
          required
        >
          <template #default="{ control }">
            <UiInput
              v-bind="control"
              v-model="label"
              :placeholder="t('admin.negociations.codes.form.label.placeholder')"
              :maxlength="120"
            />
          </template>
        </UiFormField>

        <UiFormField
          :label="t('admin.negociations.codes.form.scope.label')"
          :hint="t('admin.negociations.codes.form.scope.hint')"
          required
        >
          <template #default="{ control }">
            <UiSelect
              v-bind="control"
              :model-value="scopeKind"
              :options="scopeOptions"
              @update:model-value="(value: string) => (scopeKind = value as typeof scopeKind)"
            />
          </template>
        </UiFormField>

        <UiFormField
          v-if="scopeKind === 'negotiation_space'"
          :label="t('admin.negociations.codes.form.space.label')"
          :error="errorField === 'space' ? formError ?? undefined : undefined"
          required
        >
          <template #default="{ control }">
            <UiSelect v-bind="control" v-model="spaceId" :options="spaceOptions" />
          </template>
        </UiFormField>

        <UiFormField
          :label="t('admin.negociations.codes.form.network.label')"
          :hint="t('admin.negociations.codes.form.network.hint')"
        >
          <template #default="{ control }">
            <UiSelect v-bind="control" v-model="network" :options="networkOptions" />
          </template>
        </UiFormField>

        <UiFormField
          :label="t('admin.negociations.codes.form.maxUses.label')"
          :hint="t('admin.negociations.codes.form.maxUses.hint')"
          :error="errorField === 'max_uses' ? formError ?? undefined : undefined"
        >
          <template #default="{ control }">
            <UiInput
              v-bind="control"
              v-model="maxUses"
              type="number"
              :min="1"
              :placeholder="t('admin.negociations.codes.form.maxUses.placeholder')"
            />
          </template>
        </UiFormField>

        <div class="grid gap-5 sm:grid-cols-2">
          <UiFormField
            :label="t('admin.negociations.codes.form.validFrom.label')"
            :hint="t('admin.negociations.codes.form.validFrom.hint')"
          >
            <template #default="{ control }">
              <UiDatePicker v-bind="control" v-model="validFrom" />
            </template>
          </UiFormField>

          <UiFormField
            :label="t('admin.negociations.codes.form.validUntil.label')"
            :hint="t('admin.negociations.codes.form.validUntil.hint')"
            :error="errorField === 'valid_until' ? formError ?? undefined : undefined"
          >
            <template #default="{ control }">
              <UiDatePicker v-bind="control" v-model="validUntil" :min="validFrom || undefined" />
            </template>
          </UiFormField>
        </div>

        <div class="flex flex-wrap gap-3 pt-2">
          <UiButton type="submit" :loading="submitting">
            {{ t('admin.negociations.codes.form.submit') }}
          </UiButton>
          <UiButton variant="ghost" :to="localePath('/admin/negociations/codes')">
            {{ t('admin.negociations.codes.form.cancel') }}
          </UiButton>
        </div>
      </form>
    </template>
  </div>
</template>

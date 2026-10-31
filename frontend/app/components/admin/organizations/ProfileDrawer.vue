<script setup lang="ts">
import type { OrganizationDetail, OrganizationRef } from '~/types/admin-organizations'
import type { AttachedMedia, AttachableRoleRule } from '~/types/media'
import type { SelectOption } from '~/types/ui'
import type { AssetId, OrganizationId } from '~/types/shared'

/**
 * MODIFIER LA FICHE D'UNE ORGANISATION (31/10) : logo, nom, sigle, type, pays,
 * site. Ouvert depuis la fiche du back-office et depuis la fiche d'un dossier :
 * il charge lui-même ce qu'il montre et rend la fiche recomposée par `saved`.
 *
 * La fiche est partagée par tous ses dossiers et ses membres, et le tiroir le
 * dit avant toute saisie. L'ancien nom reste trouvable (règle n° 1) : la base
 * en fait un ancien nom.
 */

interface Props {
  organizationId: OrganizationId
}

const props = defineProps<Props>()
const open = defineModel<boolean>('open', { required: true })
const emit = defineEmits<{ saved: [detail: OrganizationDetail] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const localePath = useLocalePath()

const detail = ref<OrganizationDetail | null>(null)
const countries = ref<SelectOption[]>([])
const types = ref<SelectOption[]>([])
const logoRule = ref<AttachableRoleRule | null>(null)
const logo = ref<AttachedMedia | null>(null)
const loadError = ref(false)
const busy = ref(false)
const error = ref<string | null>(null)
const conflict = ref<OrganizationRef | null>(null)

const form = reactive({ legal_name: '', acronym: '', organization_type_code: '', country_id: '', website: '' })

const owner = computed(() => ({ schema: 'org', table: 'organizations', id: props.organizationId }))

async function loadLogo(): Promise<void> {
  logo.value = (await api.media.attached(owner.value, 'logo'))[0] ?? null
}

async function load(): Promise<void> {
  loadError.value = false
  error.value = null
  conflict.value = null
  try {
    const [fiche, countryList, typeList, rules] = await Promise.all([
      api.adminOrganizations.detail(props.organizationId),
      api.reference.countries(),
      api.reference.terms('organization_type'),
      api.media.roles('org', 'organizations'),
      loadLogo(),
    ])
    if (!fiche) {
      loadError.value = true
      return
    }
    detail.value = fiche
    countries.value = countryList
      .filter((country) => country.is_active || country.id === fiche.country_id)
      .map((country) => ({ value: country.id, label: tr(country.name) }))
      .sort((a, b) => a.label.localeCompare(b.label, locale.value))
    types.value = typeList.map((term) => ({ value: term.code, label: tr(term.label) }))
    logoRule.value = rules.find((rule) => rule.role === 'logo') ?? null
    Object.assign(form, {
      legal_name: fiche.legal_name,
      acronym: fiche.acronym ?? '',
      organization_type_code: fiche.organization_type_code,
      country_id: fiche.country_id ?? '',
      website: fiche.website ?? '',
    })
  } catch {
    loadError.value = true
  }
}

watch(open, (isOpen) => {
  if (isOpen) load()
}, { immediate: true })

const countryOptions = computed<SelectOption[]>(() => [{ value: '', label: t('admin-organization-profile.countryNone') }, ...countries.value])

const shared = computed(() => {
  const card = detail.value?.scorecard
  if (!card) return ''
  return t('admin-organization-profile.shared', {
    proposals: t('admin-organization-profile.proposals', card.propositions_deposees),
    members: t('admin-organization-profile.members', card.membres_actifs),
  })
})

async function attachLogo(assetId: AssetId | null): Promise<void> {
  if (!assetId) return
  error.value = null
  try {
    await api.media.attach({
      owner_schema: 'org',
      owner_table: 'organizations',
      owner_id: props.organizationId,
      assignments: [{ role: 'logo', asset_id: assetId }],
    })
    await loadLogo()
  } catch (thrown) {
    error.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

async function save(): Promise<void> {
  busy.value = true
  error.value = null
  conflict.value = null
  try {
    const result = await api.adminOrganizations.setProfile(
      {
        organization_id: props.organizationId,
        legal_name: form.legal_name.trim(),
        acronym: form.acronym.trim() || null,
        organization_type_code: form.organization_type_code,
        country_id: form.country_id || null,
        website: form.website.trim() || null,
      },
      auth.person?.id ?? null,
    )
    if (result.status === 'name_taken') {
      conflict.value = result.conflict_with
      return
    }
    if (result.status !== 'saved') return
    emit('saved', result.detail)
    open.value = false
  } catch (thrown) {
    error.value = apiErrorMessage(thrown, (key) => t(key))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <UiDrawer v-model:open="open" :title="t('admin-organization-profile.title')" width="32rem">
    <UiErrorState v-if="loadError" :title="t('admin-organization-profile.loadError')" @retry="load" />
    <UiLoadingState v-else-if="!detail" />

    <form v-else class="flex flex-col gap-5" @submit.prevent="save">
      <UiAlert v-if="shared" :message="shared" />

      <div class="flex items-center gap-4">
        <span
          class="grid size-20 shrink-0 place-items-center overflow-hidden rounded-md border border-border bg-white p-2"
        >
          <img v-if="logo" :src="logo.url" :alt="logo.alt_text ? tr(logo.alt_text) : ''" class="max-h-full max-w-full object-contain" />
          <span v-else class="text-center text-xs text-text-subtle">{{ t('admin-organization-profile.logoNone') }}</span>
        </span>
        <MediaImageField
          role="logo"
          :asset-id="null"
          :label="t('admin-organization-profile.logo')"
          :button-label="t(logo ? 'admin-organization-profile.logoReplace' : 'admin-organization-profile.logoAdd')"
          button-icon="upload"
          button-variant="secondary"
          :rule="logoRule"
          :owner="owner"
          :alt-fallback="form.legal_name"
          @update:asset-id="attachLogo"
        />
      </div>

      <UiInput id="organization-profile-name" v-model="form.legal_name" :label="t('admin-organization-profile.name')" required />
      <div class="grid gap-4 sm:grid-cols-2">
        <UiInput id="organization-profile-acronym" v-model="form.acronym" :label="t('admin-organization-profile.acronym')" />
        <UiSelect
          id="organization-profile-country"
          v-model="form.country_id"
          :label="t('admin-organization-profile.country')"
          :options="countryOptions"
          hide-optional
        />
        <UiSelect
          id="organization-profile-type"
          v-model="form.organization_type_code"
          :label="t('admin-organization-profile.type')"
          :options="types"
          required
        />
        <UiInput
          id="organization-profile-website"
          v-model="form.website"
          type="url"
          :label="t('admin-organization-profile.website')"
          :hint="t('admin-organization-profile.websiteHint')"
        />
      </div>
      <p class="text-sm text-text-muted">{{ t('admin-organization-profile.formerNames') }}</p>

      <UiAlert v-if="conflict" intent="warning" :message="t('admin-organization-profile.nameTaken', { name: conflict.legal_name })" live>
        <UiButton variant="link" :to="localePath(`/admin/organisations/${conflict.organization_id}`)">
          {{ t('admin-organization-profile.openOther') }}
        </UiButton>
      </UiAlert>
      <UiAlert v-if="error" intent="danger" :message="error" live />
    </form>

    <template #footer>
      <UiButton variant="secondary" @click="open = false">{{ t('admin-organization-profile.cancel') }}</UiButton>
      <UiButton variant="primary" :loading="busy" :disabled="!detail" @click="save">
        {{ t('admin-organization-profile.save') }}
      </UiButton>
    </template>
  </UiDrawer>
</template>

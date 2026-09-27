<script setup lang="ts">
import type { AdminFrancophoneMeeting, PavilionActivityOption } from '~/types/negotiation-meetings'
import type { TimeZoneName } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/** L'activité du Pavillon qui prolonge la réunion. Le choix s'enregistre aussitôt. */

const props = defineProps<{
  meeting: AdminFrancophoneMeeting
  timezone: TimeZoneName
  zoneLabel: string
  readonly?: boolean
}>()
const emit = defineEmits<{ updated: [meeting: AdminFrancophoneMeeting] }>()

const { t, locale } = useI18n()
const api = useApi()
const { dateTime } = useDateTime()

const { data: activites, status, error, refresh } = await useAsyncData<PavilionActivityOption[]>(
  `admin-negotiation-pavilion-activities-${props.meeting.edition}`,
  () => api.adminNegotiations.activitesDuPavillon(props.meeting.edition),
  { default: () => [], lazy: true },
)

const AUCUNE = 'none'
const options = computed<SelectOption[]>(() => [
  { value: AUCUNE, label: t('admin.negociations.reunions.pavilion.none') },
  ...activites.value.map((a) => ({
    value: a.id,
    label: resolveI18nText(a.title, locale.value),
    description: t('admin.negociations.reunions.pavilion.when', { date: dateTime(a.starts_at, props.timezone), zone: props.zoneLabel }),
  })),
])

const enCours = ref(false)
const erreur = ref<string | null>(null)
const enregistre = ref(false)

async function choisir(valeur: string): Promise<void> {
  const cible = valeur === AUCUNE ? null : valeur
  if (cible === props.meeting.pavilion_session_id || enCours.value) return
  enCours.value = true
  erreur.value = null
  enregistre.value = false
  try {
    emit('updated', await api.adminNegotiations.lierAuPavillon(props.meeting.id, cible))
    enregistre.value = true
  } catch (e) {
    erreur.value = apiErrorMessage(e, t)
  } finally {
    enCours.value = false
  }
}
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
    <h2 class="text-lg font-semibold">{{ t('admin.negociations.reunions.pavilion.title') }}</h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">{{ t('admin.negociations.reunions.pavilion.hint') }}</p>

    <UiErrorState v-if="error" class="mt-4" compact :retry-label="t('common.actions.retry')" @retry="refresh()" />
    <UiSkeletonLoader v-else-if="status === 'pending'" class="mt-4" height="2.75rem" />
    <template v-else>
      <UiCombobox
        class="mt-4"
        :label="t('admin.negociations.reunions.pavilion.label')"
        :placeholder="t('admin.negociations.reunions.pavilion.placeholder')"
        :options="options"
        :model-value="props.meeting.pavilion_session_id ?? AUCUNE"
        :disabled="enCours"
        :readonly="props.readonly"
        :error="erreur ?? undefined"
        hide-optional
        block
        @update:model-value="choisir"
      />
      <p v-if="!activites.length" class="mt-2 text-sm text-text-muted">
        {{ t('admin.negociations.reunions.pavilion.empty') }}
      </p>
      <p v-if="enregistre" class="mt-2 text-sm text-success" role="status">
        {{ t('admin.negociations.reunions.pavilion.saved') }}
      </p>
    </template>
  </section>
</template>

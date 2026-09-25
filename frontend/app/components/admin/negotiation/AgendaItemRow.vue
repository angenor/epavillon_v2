<script setup lang="ts">
import type { AgendaItemAdmin } from '~/types/negotiation-sessions'
import type { SelectOption } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * Un point de l'ordre du jour et sa thématique. Le choix s'enregistre aussitôt :
 * ses sessions en héritent à la lecture, rien n'est recopié sur elles.
 */

const props = defineProps<{
  point: AgendaItemAdmin
  /** Les thématiques, libellés résolus depuis le vocabulaire. */
  themes: SelectOption[]
  timezone: TimeZoneName
}>()
const emit = defineEmits<{ updated: [point: AgendaItemAdmin] }>()

const { t } = useI18n()
const api = useApi()
const { date } = useDateTime()

const options = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.negociations.ordre-du-jour.row.none') },
  ...props.themes,
])
const enCours = ref(false)
const erreur = ref<string | null>(null)
const enregistre = ref(false)

async function choisir(valeur: string): Promise<void> {
  const theme = valeur || null
  if (theme === props.point.theme || enCours.value) return
  enCours.value = true
  erreur.value = null
  enregistre.value = false
  try {
    emit('updated', await api.adminNegotiations.rattacherUnPoint(props.point.id, theme))
    enregistre.value = true
  } catch (e) {
    erreur.value = apiErrorMessage(e, t)
  } finally {
    enCours.value = false
  }
}
</script>

<template>
  <li class="grid gap-3 border-b border-border py-4 sm:grid-cols-[minmax(0,1fr)_16rem] sm:items-start">
    <div class="min-w-0">
      <p class="font-semibold">{{ point.code }}</p>
      <p class="mt-0.5 text-text-muted">{{ point.title }}</p>
      <p class="mt-1 text-sm text-text-muted">
        {{ t('admin.negociations.ordre-du-jour.row.sessions', point.session_count) }}
        <template v-if="point.theme_set_at">
          · {{ t('admin.negociations.ordre-du-jour.row.setAt', { date: date(point.theme_set_at, timezone) }) }}
        </template>
      </p>
    </div>
    <div>
      <UiSelect
        :label="t('admin.negociations.ordre-du-jour.row.theme', { code: point.code })"
        hide-label
        hide-optional
        block
        :options="options"
        :model-value="point.theme ?? ''"
        :disabled="enCours"
        :error="erreur ?? undefined"
        @update:model-value="choisir"
      />
      <p v-if="enregistre" class="mt-1 text-xs text-success" role="status">
        {{ t('admin.negociations.ordre-du-jour.row.saved') }}
      </p>
    </div>
  </li>
</template>

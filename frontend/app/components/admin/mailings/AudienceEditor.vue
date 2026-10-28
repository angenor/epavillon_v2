<script setup lang="ts">
import type { MailingCriterion, MailingCriterionKind } from '~/types/admin-mailings'
import type { ShowcaseSessionOption } from '~/types/admin-showcase'
import type { SelectOption } from '~/types/ui'

/**
 * ÉTAPE 1, LES DESTINATAIRES : des critères nommés qui s'additionnent, plus des
 * adresses libres. Une famille cochée sans filtre prend son défaut (tous les
 * dossiers déposés, les inscrits qui ont une place, tous les rôles de
 * l'édition) ; le décompte exact se lit à l'étape de vérification.
 */
interface Props {
  audience: MailingCriterion[]
  extraText: string
  sessions: ShowcaseSessionOption[]
  extraError?: string
  disabled?: boolean
  /** Faux dans la fenêtre de rédaction : les adresses libres vont dans le champ « À ». */
  showExtra?: boolean
}

const props = withDefaults(defineProps<Props>(), { extraError: undefined, showExtra: true })
const emit = defineEmits<{
  'update:audience': [value: MailingCriterion[]]
  'update:extraText': [value: string]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

function criterionOf(kind: MailingCriterionKind): MailingCriterion | undefined {
  return props.audience.find((c) => c.kind === kind)
}

function toggleKind(kind: MailingCriterionKind, on: boolean): void {
  const rest = props.audience.filter((c) => c.kind !== kind)
  emit('update:audience', on ? [...rest, emptyCriterion(kind)] : rest)
}

function patch(kind: MailingCriterionKind, change: Partial<MailingCriterion>): void {
  emit(
    'update:audience',
    props.audience.map((c) => (c.kind === kind ? { ...c, ...change } : c)),
  )
}

type ListKey = 'proposal_statuses' | 'registration_statuses' | 'role_codes'

function toggleValue(kind: MailingCriterionKind, key: ListKey, value: string, on: boolean): void {
  const current = criterionOf(kind)?.[key] ?? []
  patch(kind, { [key]: on ? [...current, value] : current.filter((v) => v !== value) })
}

const sessionOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.mailing.compose.audience.allSessions') },
  ...props.sessions.map((s) => ({
    value: s.id,
    label: tr(s.title),
    description: t('admin.mailing.common.atZone', {
      date: dateTime(s.starts_at, s.timezone),
      zone: t(zoneFormulaKey('zoneOf', timeZoneCityLabel(s.timezone)), { zone: timeZoneCityLabel(s.timezone) }),
    }),
  })),
])

const FILTERS: Record<MailingCriterionKind, { key: ListKey; values: readonly string[]; labelKey: string } | null> = {
  submitters: { key: 'proposal_statuses', values: MAILING_PROPOSAL_STATUSES, labelKey: 'proposalStatus' },
  speakers: { key: 'proposal_statuses', values: MAILING_PROPOSAL_STATUSES, labelKey: 'proposalStatus' },
  registrants: { key: 'registration_statuses', values: MAILING_REGISTRATION_STATUSES, labelKey: 'registrationStatus' },
  roles: { key: 'role_codes', values: MAILING_ROLE_CODES, labelKey: 'role' },
}
</script>

<template>
  <section class="space-y-6" :aria-label="t('admin.mailing.compose.steps.audience')">
    <p v-if="props.showExtra" class="max-w-(--measure) text-text-secondary">
      {{ t('admin.mailing.compose.audience.intro') }}
    </p>

    <ul class="space-y-3">
      <li
        v-for="kind in MAILING_KINDS"
        :key="kind"
        class="rounded-lg border bg-surface-raised p-4"
        :class="criterionOf(kind) ? 'border-accent' : 'border-border'"
      >
        <UiCheckbox
          :model-value="Boolean(criterionOf(kind))"
          :label="t(`admin.mailing.common.kind.${kind}`)"
          :hint="t(`admin.mailing.compose.audience.kindHint.${kind}`)"
          :disabled="props.disabled"
          @update:model-value="toggleKind(kind, $event)"
        />

        <div v-if="criterionOf(kind)" class="mt-4 space-y-4 border-t border-border pt-4 sm:pl-8">
          <fieldset v-if="FILTERS[kind]">
            <legend class="text-sm font-semibold text-text">
              {{ t(`admin.mailing.compose.audience.filter.${kind}`) }}
            </legend>
            <p class="mt-0.5 text-xs text-text-subtle">
              {{ t(`admin.mailing.compose.audience.filterDefault.${kind}`) }}
            </p>
            <div class="mt-2 flex flex-wrap gap-x-5 gap-y-1">
              <UiCheckbox
                v-for="value in FILTERS[kind]!.values"
                :key="value"
                :model-value="(criterionOf(kind)?.[FILTERS[kind]!.key] ?? []).includes(value)"
                :label="t(`admin.mailing.common.${FILTERS[kind]!.labelKey}.${value}`)"
                :disabled="props.disabled"
                @update:model-value="toggleValue(kind, FILTERS[kind]!.key, value, $event)"
              />
            </div>
          </fieldset>

          <UiSelect
            v-if="kind !== 'roles' && props.sessions.length"
            :model-value="criterionOf(kind)?.session_id ?? ''"
            :options="sessionOptions"
            :label="t('admin.mailing.compose.audience.session')"
            :hint="t('admin.mailing.compose.audience.sessionHint')"
            :disabled="props.disabled"
            block
            @update:model-value="patch(kind, { session_id: $event || null })"
          />
        </div>
      </li>
    </ul>

    <UiTextarea
      v-if="props.showExtra"
      :model-value="props.extraText"
      :label="t('admin.mailing.compose.audience.extra')"
      :hint="t('admin.mailing.compose.audience.extraHint')"
      :error="props.extraError"
      :placeholder="t('admin.mailing.compose.audience.extraPlaceholder')"
      :disabled="props.disabled"
      :rows="3"
      auto-grow
      block
      @update:model-value="emit('update:extraText', $event)"
    />
  </section>
</template>

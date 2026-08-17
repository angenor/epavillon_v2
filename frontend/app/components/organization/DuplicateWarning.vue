<script setup lang="ts">
import type { SimilarOrganization } from '~/types/org'
import type { CreateOrganizationPayload } from '~/types/organization-join'

/**
 * L'écran intermédiaire : côte à côte, ce qu'on s'apprête à créer et ce qui
 * existe déjà.
 *
 * IL NE BLOQUE RIEN, ET C'EST TOUT SON INTÉRÊT. « Créer quand même » est
 * toujours accessible, en second rang. Un refus pur produirait deux effets
 * connus : la personne recommence avec une orthographe différente jusqu'à passer
 * — et le doublon créé est alors PIRE, puisqu'il ne se ressemble plus —, ou elle
 * abandonne son dossier. Le seul levier honnête est de rendre l'erreur VISIBLE :
 * on montre les deux fiches attribut par attribut, et l'on demande de trancher
 * en connaissance de cause.
 *
 * LA COMPARAISON EST LIGNE À LIGNE, PAS DEUX RÉSUMÉS. Deux cartes côte à côte
 * obligent l'œil à faire l'aller-retour ; un tableau met le sigle sous le sigle
 * et la ville sous la ville, et la ressemblance saute alors aux yeux — c'est le
 * seul moment où la personne peut encore la voir.
 *
 * Ce que la maquette de référence impose ici : l'avertissement est JAUNE
 * (quelque chose demande attention, rien n'est encore fautif), et non rouge — le
 * rouge est réservé à l'échec et à la suppression, or créer une organisation
 * n'en est pas un.
 */

interface Props {
  draft: CreateOrganizationPayload
  /** Correspondances fortes, la meilleure en tête. */
  matches: SimilarOrganization[]
  /** Libellés résolus par l'appelant — la base porte ces valeurs, pas l'i18n. */
  draftCountryName?: string | null
  draftTypeLabel?: string | null
  countryNameOf: (countryId: string | null) => string | null
  typeLabelOf: (code: string) => string | null
  busyJoinId?: string | null
  creating?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  join: [match: SimilarOrganization]
  createAnyway: []
  back: []
}>()

const { t } = useI18n()

const best = computed(() => props.matches[0] ?? null)
const others = computed(() => props.matches.slice(1))

/** Les quatre attributs qui départagent deux fiches, dans l'ordre où on les lit. */
const rows = computed(() => {
  const match = best.value
  if (!match) return []
  return [
    {
      key: 'name',
      label: t('organization.join.create.fields.legalName'),
      draft: props.draft.legal_name,
      existing: match.legal_name,
    },
    {
      key: 'acronym',
      label: t('organization.join.create.fields.acronym'),
      draft: props.draft.acronym,
      existing: match.acronym,
    },
    {
      key: 'type',
      label: t('organization.join.create.fields.type'),
      draft: props.draftTypeLabel ?? null,
      existing: props.typeLabelOf(match.organization_type_code),
    },
    {
      key: 'place',
      label: t('organization.join.create.fields.place'),
      draft: [props.draft.city, props.draftCountryName].filter(Boolean).join(' · ') || null,
      existing: [match.city, props.countryNameOf(match.country_id)].filter(Boolean).join(' · ') || null,
    },
  ]
})

/** Deux valeurs identiques sont mises en évidence : c'est la preuve, pas le décor. */
function isSame(a: string | null, b: string | null): boolean {
  if (!a || !b) return false
  return a.localeCompare(b, undefined, { sensitivity: 'base' }) === 0
}
</script>

<template>
  <section v-if="best" class="grid gap-5">
    <UiAlert
      intent="warning"
      live
      :title="t('organization.join.duplicate.title')"
      :message="t('organization.join.duplicate.description')"
    />

    <UiCard flush>
      <div class="overflow-x-auto">
        <table class="w-full min-w-[520px] border-collapse text-sm">
          <caption class="sr-only">{{ t('organization.join.duplicate.tableCaption') }}</caption>
          <thead>
            <tr class="border-b border-border">
              <th scope="col" class="w-32 px-4 py-3 text-start text-xs text-text-subtle uppercase">
                <span class="sr-only">{{ t('organization.join.duplicate.attribute') }}</span>
              </th>
              <th scope="col" class="px-4 py-3 text-start font-display text-sm text-text">
                {{ t('organization.join.duplicate.yours') }}
              </th>
              <th scope="col" class="px-4 py-3 text-start font-display text-sm text-text">
                <span class="flex flex-wrap items-center gap-2">
                  {{ t('organization.join.duplicate.existing') }}
                  <UiBadge
                    v-if="best.verified_at"
                    size="sm"
                    intent="success"
                    icon="check"
                    :label="t('organization.join.results.verified')"
                  />
                </span>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in rows" :key="row.key" class="border-b border-border-subtle last:border-0">
              <th scope="row" class="px-4 py-3 text-start align-top text-xs font-normal text-text-muted">
                {{ row.label }}
              </th>
              <td class="px-4 py-3 align-top text-text">
                {{ row.draft ?? t('common.labels.unknown') }}
              </td>
              <td
                class="px-4 py-3 align-top"
                :class="isSame(row.draft, row.existing) ? 'bg-warning-surface font-bold text-text' : 'text-text'"
              >
                {{ row.existing ?? t('common.labels.unknown') }}
                <span v-if="isSame(row.draft, row.existing)" class="sr-only">
                  — {{ t('organization.join.duplicate.identical') }}
                </span>
              </td>
            </tr>
            <tr>
              <th scope="row" class="px-4 py-3 text-start align-top text-xs font-normal text-text-muted">
                {{ t('organization.join.duplicate.members') }}
              </th>
              <td class="px-4 py-3 align-top text-text-muted">—</td>
              <td class="px-4 py-3 align-top text-text">
                {{ t('organization.join.results.members', best.member_count) }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </UiCard>

    <div class="flex flex-col gap-3 sm:flex-row-reverse sm:items-center sm:justify-start">
      <UiButton
        variant="primary"
        size="lg"
        :loading="props.busyJoinId === best.organization_id"
        :label="t('organization.join.duplicate.joinExisting')"
        @click="emit('join', best)"
      />
      <UiButton
        variant="secondary"
        :loading="props.creating"
        :label="t('organization.join.duplicate.createAnyway')"
        @click="emit('createAnyway')"
      />
      <UiButton
        variant="ghost"
        icon="chevron-left"
        :label="t('organization.join.duplicate.back')"
        @click="emit('back')"
      />
    </div>

    <!-- Les autres correspondances fortes ne disparaissent pas parce qu'une
         meilleure existe : deux fiches proches peuvent l'être pour des raisons
         différentes, et c'est parfois la seconde qui est la bonne. -->
    <div v-if="others.length > 0" class="grid gap-3">
      <h3 class="font-display text-sm text-text-secondary">
        {{ t('organization.join.duplicate.others') }}
      </h3>
      <OrganizationMatchCard
        v-for="match in others"
        :key="match.organization_id"
        :match="match"
        :country-name="props.countryNameOf(match.country_id)"
        :type-label="props.typeLabelOf(match.organization_type_code)"
        :busy="props.busyJoinId === match.organization_id"
        @join="emit('join', $event)"
      />
    </div>
  </section>
</template>

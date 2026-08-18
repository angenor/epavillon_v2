<script setup lang="ts">
import type { OrganizationDomainRow } from '~/types/admin-organizations'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES DOMAINES DE COURRIEL, ET LEUR VÉRIFICATION.
 *
 * DEUX RÔLES DANS UN SEUL TABLEAU, et ils ne se confondent pas. Un domaine
 * vérifié est d'abord un SIGNAL DE DÉDOUBLONNAGE — deux fiches qui le partagent
 * sont la même maison, quels que soient les libellés saisis. Il est ensuite un
 * MÉCANISME DE RATTACHEMENT : `auto_join` fait entrer les nouveaux inscrits sans
 * intervention humaine.
 *
 * `ck_domain_autojoin_requires_verification` LIE LES DEUX COLONNES : pas de
 * rattachement automatique sans vérification. Retirer la vérification retire donc
 * le rattachement — la base refuserait l'écriture inverse, et l'écran ne propose
 * pas ce que la base refuse.
 *
 * LA COLONNE « PARTAGÉ » EST LA PLUS IMPORTANTE DE CE TABLEAU. Elle nomme les
 * autres fiches déclarant le même domaine, et c'est là que le doublon se voit à
 * l'œil nu — avant même que le worker ait rempli la file. La vérification manuelle
 * y bute d'ailleurs : `ux_organization_domains_verified` n'autorise qu'une seule
 * organisation par domaine vérifié.
 */

interface Props {
  domains: OrganizationDomainRow[]
  timezone: TimeZoneName
  canManage: boolean
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  verify: [domain: OrganizationDomainRow, verified: boolean]
  autoJoin: [domain: OrganizationDomainRow, autoJoin: boolean]
}>()

const { t } = useI18n()
const { date } = useDateTime()
const localePath = useLocalePath()

const columns = computed<TableColumn[]>(() => [
  { key: 'domain', label: t('admin.organization.detail.domains.columns.domain') },
  { key: 'verified', label: t('admin.organization.detail.domains.columns.verified') },
  { key: 'autoJoin', label: t('admin.organization.detail.domains.columns.autoJoin') },
  {
    key: 'shared',
    label: t('admin.organization.detail.domains.columns.shared'),
    hideBelow: 'lg',
  },
])
</script>

<template>
  <section>
    <h2 class="text-lg font-semibold text-text">
      {{ t('admin.organization.detail.domains.title') }}
    </h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('admin.organization.detail.domains.description') }}
    </p>

    <UiTable
      class="mt-4"
      :columns="columns"
      :rows="props.domains"
      row-key="id"
      row-label-key="domain"
      :caption="t('admin.organization.detail.domains.caption')"
      visually-hidden-caption
      :hoverable="false"
    >
      <template #cell-domain="{ row }">
        <span class="font-mono text-sm text-text">{{ row.domain }}</span>
      </template>

      <template #cell-verified="{ row }">
        <div class="flex flex-wrap items-center gap-2">
          <UiBadge
            :intent="row.verified_at ? 'success' : 'neutral'"
            size="sm"
            :label="
              row.verified_at
                ? t('admin.organization.detail.domains.verified', {
                    date: date(row.verified_at, props.timezone),
                  })
                : t('admin.organization.detail.domains.unverified')
            "
          />
          <span v-if="row.verification_method" class="text-xs text-text-subtle">
            {{ t('admin.organization.detail.domains.method.' + row.verification_method) }}
          </span>
          <UiButton
            v-if="props.canManage"
            variant="link"
            size="sm"
            :disabled="props.busy"
            @click="emit('verify', row, row.verified_at === null)"
          >
            {{
              t(
                row.verified_at
                  ? 'admin.organization.detail.domains.unverify'
                  : 'admin.organization.detail.domains.verify',
              )
            }}
          </UiButton>
        </div>
      </template>

      <template #cell-autoJoin="{ row }">
        <!-- Désactivé sans vérification : c'est la contrainte
             `ck_domain_autojoin_requires_verification` qui parle, pas l'écran. -->
        <UiSwitch
          :model-value="row.auto_join"
          :label="
            t(
              row.auto_join
                ? 'admin.organization.detail.domains.autoJoinOn'
                : 'admin.organization.detail.domains.autoJoinOff',
            )
          "
          :disabled="!props.canManage || props.busy || row.verified_at === null"
          :hint="row.verified_at === null ? t('admin.organization.detail.domains.autoJoinHint') : undefined"
          @update:model-value="(value: boolean) => emit('autoJoin', row, value)"
        />
      </template>

      <template #cell-shared="{ row }">
        <ul v-if="row.shared_with.length > 0" class="flex flex-col gap-1">
          <li v-for="other in row.shared_with" :key="other.organization_id">
            <NuxtLink
              :to="localePath(`/admin/organisations/${other.organization_id}`)"
              class="text-sm text-warning"
            >
              {{ t('admin.organization.detail.domains.sharedWith', { name: other.legal_name }) }}
            </NuxtLink>
          </li>
        </ul>
        <span v-else class="text-sm text-text-subtle">
          {{ t('admin.organization.detail.domains.notShared') }}
        </span>
      </template>

      <template #empty>
        <UiEmptyState compact :title="t('admin.organization.detail.domains.empty')" />
      </template>
    </UiTable>
  </section>
</template>

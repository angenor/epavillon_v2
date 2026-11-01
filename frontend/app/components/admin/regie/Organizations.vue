<script setup lang="ts">
import type { RegieOrganization } from '~/types/admin-regie'

interface Props {
  organizations: RegieOrganization[]
  isJoint?: boolean
  /** Précision après les noms (« 3 intervenants présents »). */
  detail?: string | null
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { isJoint: false, detail: null, size: 'sm' })
const { t, locale } = useI18n()

const label = computed(() => regieOrganizationsLabel(props.organizations, locale.value))
</script>

<template>
  <p v-if="props.organizations.length" class="flex min-w-0 items-center gap-2 text-sm">
    <span class="flex shrink-0 items-center">
      <OrganizationLogoMark
        v-for="(organization, index) in props.organizations"
        :key="organization.id"
        :name="organization.name"
        :acronym="organization.acronym"
        :logo="organization.logo"
        :size="props.size"
        :class="index > 0 ? '-ml-2' : ''"
      />
    </span>
    <span class="min-w-0">
      <b class="font-semibold">{{ label }}</b>
      <span v-if="props.isJoint" class="ml-1.5 text-text-muted">{{ t('admin.regie.slot.joint') }}</span>
      <span v-if="props.detail" class="ml-1.5 text-text-muted">{{ props.detail }}</span>
    </span>
  </p>
</template>

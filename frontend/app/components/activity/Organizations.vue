<script setup lang="ts">
import type { SessionOrganization } from '~/types/programme/session'

/** Les organisations de l'activité, porteuse d'abord : logo, nom complet et rôle. */

interface Props {
  organizations: SessionOrganization[]
}

const props = defineProps<Props>()

const { t } = useI18n()

const ROLE_ORDER = ['lead', 'co_organizer', 'partner', 'sponsor']

const organizations = computed(() =>
  [...props.organizations]
    .filter((organization) => Boolean(organization.name))
    .sort((a, b) => ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role) || a.sort_order - b.sort_order)
    .map((organization) => ({
      id: organization.organization_id,
      name: organization.name ?? '',
      acronym: organization.acronym ?? null,
      logo: organization.logo ?? null,
      role: t(`activity.organizations.role.${organization.role}`),
    })),
)
</script>

<template>
  <section v-if="organizations.length" aria-labelledby="organisations-titre">
    <h2 id="organisations-titre" class="heading-condensed text-[clamp(1.8rem,3.4vw,2.3rem)] text-heading">
      {{ t('activity.organizations.title') }}
    </h2>
    <ul class="mt-4.5 grid grid-cols-1 gap-3 sm:grid-cols-2">
      <li
        v-for="organization in organizations"
        :key="organization.id"
        class="flex min-w-0 items-center gap-3 rounded-lg border border-border-subtle bg-surface-raised p-3"
      >
        <OrganizationLogoMark :name="organization.name" :acronym="organization.acronym" :logo="organization.logo" />
        <p class="min-w-0">
          <b class="block text-[0.92rem] leading-snug font-bold text-text">{{ organization.name }}</b>
          <span class="text-[0.78rem] text-text-muted">{{ organization.role }}</span>
        </p>
      </li>
    </ul>
  </section>
</template>

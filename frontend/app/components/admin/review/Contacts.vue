<script setup lang="ts">
import type { ProposalContactPerson } from '~/types/admin-review'

/** Le déposant et la personne de contact du dossier, avec l'adresse à copier. */

interface Props {
  submitter: ProposalContactPerson | null
  contact: ProposalContactPerson | null
  canChangeSubmitter: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ changeSubmitter: [] }>()

const { t } = useI18n()

const rows = computed(() =>
  [
    props.submitter ? { person: props.submitter, role: 'submitter' as const } : null,
    props.contact ? { person: props.contact, role: 'contact' as const } : null,
  ].filter((row) => row !== null),
)

function detail(person: ProposalContactPerson, role: 'submitter' | 'contact'): string {
  return [t(`admin.proposal.dashboard.contacts.${role}`), person.job_title, person.organization_name]
    .filter(Boolean)
    .join(' · ')
}
</script>

<template>
  <section aria-labelledby="dashboard-contacts-title">
    <h2 id="dashboard-contacts-title" class="flex flex-wrap items-baseline gap-x-3 border-b border-border pb-2 text-lg font-semibold">
      {{ t('admin.proposal.dashboard.contacts.title') }}
      <small class="text-sm font-normal text-text-muted">{{ t('admin.proposal.dashboard.contacts.subtitle') }}</small>
    </h2>

    <p v-if="rows.length === 0" class="mt-3 text-sm text-text-muted">{{ t('admin.proposal.dashboard.contacts.none') }}</p>

    <ul v-else class="mt-1">
      <li
        v-for="row in rows"
        :key="row.role"
        class="flex flex-wrap items-center gap-x-4 gap-y-2 border-b border-border-subtle py-3"
      >
        <AdminReviewPersonAvatar :name="row.person.display_name" :photo="row.person.photo" />
        <div class="min-w-0 flex-[1_1_14rem]">
          <p class="font-semibold text-text">{{ row.person.display_name }}</p>
          <p class="text-sm text-text-muted">{{ detail(row.person, row.role) }}</p>
        </div>
        <AdminReviewEmailAddress :email="row.person.email" />
        <UiButton
          v-if="row.role === 'submitter' && props.canChangeSubmitter"
          variant="ghost"
          @click="emit('changeSubmitter')"
        >
          {{ t('admin.proposal.dashboard.contacts.change') }}
        </UiButton>
      </li>
    </ul>
  </section>
</template>

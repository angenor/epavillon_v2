<script setup lang="ts">
import type { PublicSessionSpeaker, SessionOrganization } from '~/types/programme/session'

/** Intervenants et organisations associées — ce que la base publie, sans rien de plus. */

interface Props {
  speakers: PublicSessionSpeaker[]
  organizations: SessionOrganization[]
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

const ROLE_ORDER = ['moderator', 'keynote', 'facilitator', 'speaker', 'panelist', 'interpreter']

const speakers = computed(() =>
  [...props.speakers]
    .sort((a, b) => ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role) || a.sort_order - b.sort_order)
    .map((speaker) => ({
      id: speaker.id,
      name: speaker.display_name,
      role: speaker.role,
      lead: speaker.role === 'moderator' || speaker.role === 'keynote',
      job: speaker.job_title_snapshot ?? '',
      organization: speaker.organization_snapshot ?? '',
      bio: speaker.bio ? tr(speaker.bio) : '',
      avatar: speaker.avatar,
      initials: initialsOf(speaker.display_name),
    })),
)

const ORG_ORDER = ['lead', 'co_organizer', 'partner', 'sponsor']

const organizations = computed(() =>
  [...props.organizations]
    .sort((a, b) => ORG_ORDER.indexOf(a.role) - ORG_ORDER.indexOf(b.role) || a.sort_order - b.sort_order)
    .map((organization) => ({
      id: organization.organization_id,
      role: organization.role,
      acronym: organization.acronym ?? '',
      name: organization.name ?? '',
      country: organization.country ? tr(organization.country) : '',
    })),
)
</script>

<template>
  <section v-if="speakers.length" aria-labelledby="intervenants-titre">
    <h2 id="intervenants-titre" class="mb-5 border-b-4 border-poster-ink pb-2.5 font-poster text-[2.375rem] leading-none font-black uppercase font-stretch-[68%]">
      {{ t('activity.people.speakers') }}
    </h2>
    <ul class="grid grid-cols-2 gap-x-5 gap-y-9 pt-2 sm:grid-cols-3 xl:grid-cols-4">
      <li v-for="speaker in speakers" :key="speaker.id" class="flex flex-col items-center text-center">
        <UiImage
          v-if="speaker.avatar"
          :image="speaker.avatar"
          ratio="1 / 1"
          rounded="rounded-full"
          class="size-24 shrink-0 overflow-hidden rounded-full border-2 border-poster-ink"
          sizes="96px"
        />
        <span
          v-else
          class="flex size-24 shrink-0 items-center justify-center rounded-full border-2 border-poster-ink bg-poster-paper-sunken font-poster text-[1.75rem] font-black"
          aria-hidden="true"
        >
          {{ speaker.initials }}
        </span>
        <span
          class="mt-3 rounded-sm px-1.5 py-0.5 font-poster-mono text-[0.625rem] font-semibold tracking-[0.06em] uppercase"
          :class="speaker.lead ? 'bg-poster-ink text-poster-on-ink-accent' : 'text-poster-ink-muted'"
        >
          {{ t(`activity.people.role.${speaker.role}`) }}
        </span>
        <span class="mt-1 text-[1.0625rem] leading-snug font-bold">{{ speaker.name }}</span>
        <span v-if="speaker.job" class="mt-1 text-sm leading-snug text-poster-ink-muted italic">{{ speaker.job }}</span>
        <span v-if="speaker.organization" class="text-sm leading-snug text-poster-ink-muted">{{ speaker.organization }}</span>
        <details v-if="speaker.bio" class="mt-1.5 text-sm">
          <summary class="cursor-pointer py-1 font-semibold underline underline-offset-4">{{ t('activity.people.bio') }}</summary>
          <p class="mt-1 text-left whitespace-pre-line text-poster-ink-muted">{{ speaker.bio }}</p>
        </details>
      </li>
    </ul>
  </section>

  <section v-if="organizations.length > 1" aria-labelledby="organisations-titre">
    <h2 id="organisations-titre" class="mb-5 border-b-4 border-poster-ink pb-2.5 font-poster text-[2.375rem] leading-none font-black uppercase font-stretch-[68%]">
      {{ t('activity.people.organizations') }}
    </h2>
    <ul class="flex flex-wrap gap-3.5">
      <li v-for="organization in organizations" :key="`${organization.role}-${organization.id}`" class="flex w-52 flex-col gap-1.5">
        <span class="font-poster-mono text-[0.6875rem] font-semibold tracking-[0.08em] text-poster-ink-muted uppercase">
          {{ t(`activity.people.orgRole.${organization.role}`) }}
        </span>
        <span
          class="flex min-h-21 flex-col gap-1 rounded-md border-2 border-poster-ink px-3.5 py-3 shadow-poster-sm"
          :class="organization.role === 'lead' ? 'bg-poster-today' : 'bg-poster-paper-raised'"
        >
          <span v-if="organization.acronym" class="font-poster text-[1.375rem] leading-none font-black font-stretch-[75%]">{{ organization.acronym }}</span>
          <span class="text-xs leading-snug">{{ organization.name }}</span>
          <span v-if="organization.country" class="text-xs text-poster-ink-muted">{{ organization.country }}</span>
        </span>
      </li>
    </ul>
  </section>
</template>

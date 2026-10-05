<script setup lang="ts">
import type { AttachedImage } from '~/types/media'
import type { PublicSessionSpeaker, SessionOrganization } from '~/types/programme/session'

/** Intervenants et organisations associées — ce que la base publie, sans rien de plus. */

interface Props {
  speakers: PublicSessionSpeaker[]
  organizations: SessionOrganization[]
  leadLogo: AttachedImage | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

function firstWord(text: string): { lead: string; rest: string } {
  const index = text.indexOf(' ')
  return index > 0 ? { lead: text.slice(0, index), rest: text.slice(index) } : { lead: text, rest: '' }
}

const speakersTitle = computed(() => firstWord(t('activity.people.speakers')))
const organizationsTitle = computed(() => firstWord(t('activity.people.organizedBy')))

const ROLE_ORDER = ['moderator', 'keynote', 'facilitator', 'speaker', 'panelist', 'interpreter']

const speakers = computed(() =>
  [...props.speakers]
    .sort((a, b) => ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role) || a.sort_order - b.sort_order)
    .map((speaker) => ({
      id: speaker.id,
      name: speaker.display_name,
      role: speaker.role,
      affiliation: [speaker.job_title_snapshot, speaker.organization_snapshot].filter(Boolean).join(' · '),
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

const lead = computed(() => organizations.value.find((organization) => organization.role === 'lead') ?? null)
const others = computed(() => organizations.value.filter((organization) => organization !== lead.value))

const logo = computed(() =>
  props.leadLogo
    ? { src: props.leadLogo.sources.thumb?.url ?? props.leadLogo.url, alt: tr(props.leadLogo.alt_text) }
    : null,
)
</script>

<template>
  <section v-if="speakers.length" aria-labelledby="intervenants-titre" class="font-sans">
    <h2 id="intervenants-titre" class="border-b border-text pb-3 font-sans text-[26px] leading-tight font-light text-text">
      <b class="font-bold">{{ speakersTitle.lead }}</b>{{ speakersTitle.rest }}
    </h2>
    <ul>
      <li
        v-for="speaker in speakers"
        :key="speaker.id"
        class="grid grid-cols-[48px_minmax(0,1fr)] gap-x-4 gap-y-1 border-b border-border-subtle py-4.5 sm:grid-cols-[64px_minmax(0,1fr)_auto] sm:items-center sm:gap-x-5"
      >
        <span class="row-span-2 self-start sm:row-span-1 sm:self-center">
          <UiImage
            v-if="speaker.avatar"
            :image="speaker.avatar"
            ratio="1 / 1"
            rounded="rounded-full"
            class="size-12 sm:size-16"
            sizes="64px"
          />
          <span
            v-else
            class="flex size-12 items-center justify-center rounded-full bg-surface-inverse text-base font-light text-text-on-inverse sm:size-16 sm:text-xl"
            aria-hidden="true"
          >
            {{ speaker.initials }}
          </span>
        </span>

        <span
          class="col-start-2 row-start-1 text-xs font-bold text-accent uppercase sm:col-start-3 sm:text-right"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ t(`activity.people.role.${speaker.role}`) }}
        </span>

        <div class="col-start-2 row-start-2 min-w-0 sm:row-start-1">
          <p class="text-lg leading-snug font-bold text-text">{{ speaker.name }}</p>
          <p v-if="speaker.affiliation" class="mt-0.5 text-sm text-text-muted">{{ speaker.affiliation }}</p>
          <details v-if="speaker.bio" class="group mt-1 text-sm">
            <summary class="inline-flex min-h-11 cursor-pointer list-none items-center gap-1 font-bold text-accent [&::-webkit-details-marker]:hidden">
              {{ t('activity.people.bio') }}
              <UiIcon name="chevron-down" size="1rem" class="transition-transform group-open:rotate-180" />
            </summary>
            <p class="max-w-prose pb-1 leading-relaxed whitespace-pre-line text-text-muted">{{ speaker.bio }}</p>
          </details>
        </div>
      </li>
    </ul>
  </section>

  <section v-if="organizations.length" aria-labelledby="organisations-titre" class="font-sans">
    <h2 id="organisations-titre" class="border-b border-text pb-3 font-sans text-[26px] leading-tight font-light text-text">
      <b class="font-bold">{{ organizationsTitle.lead }}</b>{{ organizationsTitle.rest }}
    </h2>

    <div v-if="lead" class="flex flex-col gap-4 border-b border-border-subtle py-5 sm:flex-row sm:items-center sm:gap-6">
      <span class="flex h-24 w-[200px] shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border-subtle bg-surface-inverse-selected p-3">
        <img v-if="logo" :src="logo.src" :alt="logo.alt" class="size-full object-contain" loading="lazy" decoding="async">
        <span v-else class="text-[22px] font-bold text-text-on-inverse-selected">{{ lead.acronym || initialsOf(lead.name) }}</span>
      </span>
      <div class="min-w-0">
        <p class="text-xs font-bold text-accent uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ t('activity.people.orgRole.lead') }}
        </p>
        <p class="mt-1 text-xl leading-snug font-bold text-text">{{ lead.name }}</p>
        <p v-if="lead.country" class="mt-0.5 text-sm text-text-muted">{{ lead.country }}</p>
      </div>
    </div>

    <ul v-if="others.length" class="grid sm:grid-cols-2 sm:gap-x-8">
      <li
        v-for="organization in others"
        :key="`${organization.role}-${organization.id}`"
        class="flex items-center gap-4 border-b border-border-subtle py-4"
      >
        <span class="flex h-14 w-[120px] shrink-0 items-center justify-center rounded-md border border-border-subtle bg-surface-inverse-selected px-2">
          <span class="truncate text-base font-bold text-text-on-inverse-selected">
            {{ organization.acronym || initialsOf(organization.name) }}
          </span>
        </span>
        <div class="min-w-0">
          <p class="text-[11px] font-bold text-accent uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
            {{ t(`activity.people.orgRole.${organization.role}`) }}
          </p>
          <p class="mt-0.5 text-[15px] leading-snug font-bold text-text">{{ organization.name }}</p>
          <p v-if="organization.country" class="text-[13px] text-text-muted">{{ organization.country }}</p>
        </div>
      </li>
    </ul>
  </section>
</template>

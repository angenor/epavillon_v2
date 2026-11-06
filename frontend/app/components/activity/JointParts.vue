<script setup lang="ts">
import type { JointProposalDetail } from '~/types/views'
import type { AttachedImage } from '~/types/media'

/**
 * Séance commune : une carte par dossier réuni dans le créneau, dans l'ordre.
 * Numéro et sigle sur filet, couverture, porteur, titre d'origine, description,
 * intervenants et organisations associées du dossier. Pas d'horaire de passage
 * (arbitré le 01/11) : la séance a un début et une fin, pas ses dossiers.
 */

interface Props {
  proposals: JointProposalDetail[]
  /** Terminée ou annulée : les couvertures passent en gris, comme l'image de la séance. */
  faded?: boolean
  /** La photo de l'édition : l'image d'un dossier sans couverture. */
  editionImage?: AttachedImage | null
}

const props = withDefaults(defineProps<Props>(), { editionImage: null })

const { t } = useI18n()
const { tr } = useI18nText()
const { jointParts } = useJointSession()

const ROLE_ORDER = ['co_organizer', 'partner', 'sponsor']

const cards = computed(() => {
  const details = new Map(props.proposals.map((proposal) => [proposal.proposal_id, proposal]))
  return jointParts(props.proposals).flatMap((part) => {
    const detail = details.get(part.id)
    if (!detail) return []
    const associates = [...detail.organizations]
      .filter((organization) => Boolean(organization.name) && organization.organization_id !== part.organizationId)
      .sort((a, b) => ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role) || a.sort_order - b.sort_order)
      .map((organization) => ({
        id: organization.organization_id,
        name: organization.name ?? '',
        acronym: organization.acronym ?? null,
        logo: organization.logo ?? null,
        role: t(`activity.organizations.role.${organization.role}`),
      }))
    return [{ ...part, description: detail.description ? tr(detail.description) : '', speakers: detail.speakers, associates }]
  })
})

const ruleStyle = { '--rule-solid': 'var(--color-day-solid)', '--rule-on': 'var(--color-day-on-solid)' }
</script>

<template>
  <section v-if="cards.length" aria-labelledby="dossiers-titre">
    <!-- Titre pour les lecteurs d'écran seulement : la maquette enchaîne les cartes sous le diaporama. -->
    <h2 id="dossiers-titre" class="sr-only">
      {{ t('activity.joint.title') }}
    </h2>
    <div class="grid grid-cols-1 gap-5 md:grid-cols-2">
      <article
        v-for="card in cards"
        :key="card.id"
        class="flex min-w-0 flex-col overflow-hidden rounded-lg border border-border-subtle bg-surface-raised"
        :aria-labelledby="`dossier-${card.number}`"
      >
        <div class="rule-line" :style="ruleStyle">
          <span class="rule-label">
            <b class="text-[1.4rem]" aria-hidden="true">{{ card.number }}</b>
            <span class="min-w-0 truncate">{{ card.label }}</span>
          </span>
        </div>

        <div class="relative aspect-video overflow-hidden bg-(--color-day-surface)">
          <UiImage
            v-if="card.cover"
            :image="card.cover"
            ratio="auto"
            frame-class="size-full"
            class="absolute inset-0"
            :class="{ grayscale: props.faded }"
            sizes="(min-width: 1024px) 26rem, (min-width: 768px) 45vw, 100vw"
          />
          <ActivityCoverFallback
            v-else
            :image="props.editionImage"
            :label="(card.acronym ?? initialsOf(card.name)).slice(0, 8)"
            text-class="text-[2.4rem]"
            :faded="props.faded"
            sizes="(min-width: 1024px) 26rem, (min-width: 768px) 45vw, 100vw"
          >
            <span
              class="absolute inset-0 flex items-center justify-center px-4 font-condensed text-[2.4rem] font-bold tracking-[0.04em] text-(--color-day)"
              aria-hidden="true"
            >
              {{ (card.acronym ?? initialsOf(card.name)).slice(0, 8) }}
            </span>
          </ActivityCoverFallback>
        </div>

        <div class="flex flex-1 flex-col gap-3 px-5 pt-4.5 pb-5">
          <p class="flex min-w-0 items-center gap-3">
            <OrganizationLogoMark :name="card.name" :acronym="card.acronym" :logo="card.logo" />
            <span class="min-w-0">
              <b class="block text-[0.95rem] leading-snug font-bold text-text">{{ card.name }}</b>
              <span class="text-[0.78rem] text-text-muted">{{ t('activity.organizations.role.lead') }}</span>
            </span>
          </p>
          <h3
            :id="`dossier-${card.number}`"
            class="font-condensed text-[1.45rem] leading-[1.12] font-semibold tracking-[0.005em] text-pretty text-heading"
          >
            {{ card.title }}
          </h3>
          <ActivityFold v-if="card.description" class="[--fold-fade:var(--color-surface-raised)]">
            <UiRichContent class="text-[0.95rem] leading-[1.7]" :html="card.description" />
          </ActivityFold>
          <ActivityPeople :speakers="card.speakers" compact />
          <ul v-if="card.associates.length" class="grid gap-2 border-t border-border-subtle pt-3" :aria-label="t('activity.organizations.title')">
            <li v-for="associate in card.associates" :key="associate.id" class="flex min-w-0 items-center gap-2.5">
              <OrganizationLogoMark :name="associate.name" :acronym="associate.acronym" :logo="associate.logo" size="sm" />
              <span class="min-w-0 text-[0.85rem] leading-snug">
                <b class="block font-bold text-text">{{ associate.name }}</b>
                <span class="text-[0.78rem] text-text-muted">{{ associate.role }}</span>
              </span>
            </li>
          </ul>
        </div>
      </article>
    </div>
  </section>
</template>

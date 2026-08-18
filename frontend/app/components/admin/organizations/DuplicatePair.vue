<script setup lang="ts">
import type { DuplicatePair, DuplicateReason, DuplicateSide } from '~/types/admin-organizations'
import type { TimeZoneName } from '~/types/shared'
import type { Intent } from '~/types/ui'

/**
 * UNE PAIRE DE LA FILE DES DOUBLONS PRÉSUMÉS.
 *
 * LES DEUX FICHES SONT PRÉSENTÉES À ÉGALITÉ, côte à côte. L'ordre `left`/`right`
 * vient de `ck_duplicate_candidates_ordered` — une contrainte d'unicité, qui ne
 * dit rien de qui devrait absorber qui. Donner à l'une un traitement visuel
 * privilégié suggérerait une décision que le modèle ne porte pas.
 *
 * LES MOTIFS NE SE VALENT PAS, ET LA CARTE LE MONTRE. Un domaine de messagerie
 * partagé est une preuve matérielle : il est nommé, avec le domaine en question.
 * Un même pays, seul, ne prouve rien — il reste discret. C'est la demande du
 * prompt : « avec le MOTIF de la suspicion ».
 *
 * AUCUN BOUTON NE FUSIONNE D'ICI. « Examiner la fusion » ouvre l'écran de
 * comparaison ; la fusion se décide là-bas, avec son motif et sa confirmation.
 * Un raccourci depuis la file ferait sauter les deux.
 */

interface Props {
  pair: DuplicatePair
  timezone: TimeZoneName
  /** La fusion est-elle ouverte à cette personne ? `org.organization.merge`, portée globale. */
  canMerge: boolean
  /** Paire déjà tranchée : la carte se lit, elle ne se décide plus. */
  settled?: boolean
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  merge: [pair: DuplicatePair]
  distinct: [pair: DuplicatePair]
  defer: [pair: DuplicatePair]
  reopen: [pair: DuplicatePair]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()
const localePath = useLocalePath()

/**
 * Trois paliers, et ils viennent du SQL. 85 est le seuil de « correspondance
 * forte » commenté au § 5 de `040_organizations.sql` — celui qui justifie un
 * blocage doux au rattachement. En deçà de 60, le rapprochement tient souvent à
 * un seul signal faible.
 */
const strength = computed(() => {
  if (props.pair.score >= 85) return { key: 'strong', intent: 'danger' as Intent }
  if (props.pair.score >= 60) return { key: 'moderate', intent: 'warning' as Intent }
  return { key: 'weak', intent: 'neutral' as Intent }
})

/** Le domaine que les deux fiches partagent — ce qui rend le motif tangible. */
const sharedDomain = computed(() => {
  const right = new Set(props.pair.right.domains)
  return props.pair.left.domains.find((domain) => right.has(domain)) ?? null
})

function reasonHint(reason: DuplicateReason): string | null {
  if (reason === 'shared_domain') {
    return sharedDomain.value
      ? t('admin.organization.duplicates.reasons.sharedDomainHint', { domain: sharedDomain.value })
      : null
  }
  if (reason === 'name_similarity') return t('admin.organization.duplicates.reasons.nameSimilarityHint')
  if (reason === 'same_country') return t('admin.organization.duplicates.reasons.sameCountryHint')
  return t('admin.organization.duplicates.reasons.acronymMatchHint')
}

/** Les motifs, du plus probant au moins : un domaine partagé prime sur un pays commun. */
const REASON_ORDER: DuplicateReason[] = [
  'shared_domain',
  'acronym_match',
  'name_similarity',
  'same_country',
]

const orderedReasons = computed(() =>
  [...props.pair.reasons].sort(
    (a, b) => REASON_ORDER.indexOf(a) - REASON_ORDER.indexOf(b),
  ),
)

function createdLabel(side: DuplicateSide): string {
  return side.created_by_name
    ? t('admin.organization.duplicates.side.createdBy', {
        name: side.created_by_name,
        date: date(side.created_at, props.timezone),
      })
    : t('admin.organization.duplicates.side.createdOn', {
        date: date(side.created_at, props.timezone),
      })
}
</script>

<template>
  <article class="rounded-lg border border-border bg-surface-raised">
    <header class="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 border-b border-border px-4 py-3">
      <div class="flex flex-wrap items-center gap-3">
        <UiBadge
          :intent="strength.intent"
          solid
          :label="t('admin.organization.duplicates.similarity.value', { score: Math.round(props.pair.score) })"
        />
        <span class="text-sm font-medium text-text">
          {{ t('admin.organization.duplicates.similarity.' + strength.key) }}
        </span>
        <span class="text-xs text-text-subtle">
          {{ date(props.pair.detected_at, props.timezone) }}
        </span>
      </div>

      <UiBadge
        v-if="props.settled && props.pair.decision"
        :intent="props.pair.decision === 'merged' ? 'success' : 'neutral'"
        size="sm"
        :label="t('admin.organization.duplicates.decision.' + props.pair.decision)"
      />
    </header>

    <!-- LES MOTIFS, du plus probant au moins. Le prompt les demande nommés :
         « similarité de nom, domaine de courriel partagé, même pays,
         correspondance de sigle ». -->
    <div class="border-b border-border px-4 py-3">
      <h3 class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
        {{ t('admin.organization.duplicates.reasons.label') }}
      </h3>
      <ul class="mt-2 flex flex-col gap-1.5">
        <li
          v-for="reason in orderedReasons"
          :key="reason"
          class="flex flex-wrap items-baseline gap-x-2 text-sm"
        >
          <UiBadge
            size="sm"
            :intent="reason === 'shared_domain' ? 'danger' : 'neutral'"
            :label="t('admin.organization.duplicates.reasons.' + reason)"
          />
          <span class="text-xs text-text-muted">{{ reasonHint(reason) }}</span>
        </li>
      </ul>
    </div>

    <!-- LES DEUX FICHES À ÉGALITÉ. Aucun ordre suggéré : c'est l'écran de fusion
         qui propose un sens, et l'équipe qui tranche. -->
    <div class="grid gap-px bg-border sm:grid-cols-2">
      <section
        v-for="side in [props.pair.left, props.pair.right]"
        :key="side.organization_id"
        class="bg-surface-raised p-4"
      >
        <h3 class="flex items-start gap-2 text-base font-semibold text-balance text-text">
          <NuxtLink
            :to="localePath(`/admin/organisations/${side.organization_id}`)"
            class="no-underline hover:underline"
          >
            {{ side.legal_name }}
          </NuxtLink>
          <UiIcon
            v-if="side.verified_at"
            name="shield-check"
            size="1rem"
            class="mt-1 shrink-0 text-success"
            :aria-label="t('admin.organization.duplicates.side.verified')"
          />
        </h3>

        <p class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-text-muted">
          <span v-if="side.acronym" class="font-mono">{{ side.acronym }}</span>
          <span v-if="side.organization_type_label">{{ tr(side.organization_type_label) }}</span>
          <span v-if="side.country_name">{{ tr(side.country_name) }}</span>
          <span v-if="side.city">{{ side.city }}</span>
        </p>

        <dl class="mt-3 grid grid-cols-3 gap-2 text-sm">
          <div>
            <dt class="text-xs text-text-subtle">{{ t('admin.organization.detail.figures.members') }}</dt>
            <dd class="font-mono tabular-nums text-text">{{ side.member_count }}</dd>
          </div>
          <div>
            <dt class="text-xs text-text-subtle">{{ t('admin.organization.detail.figures.proposals') }}</dt>
            <dd class="font-mono tabular-nums text-text">{{ side.proposal_count }}</dd>
          </div>
          <div>
            <dt class="text-xs text-text-subtle">{{ t('admin.organization.list.trust.label') }}</dt>
            <dd class="font-mono tabular-nums text-text">{{ side.trust_score }}</dd>
          </div>
        </dl>

        <div class="mt-3">
          <p class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
            {{ t('admin.organization.duplicates.side.domains') }}
          </p>
          <p v-if="side.domains.length === 0" class="mt-1 text-xs text-text-subtle">
            {{ t('admin.organization.duplicates.side.noDomain') }}
          </p>
          <ul v-else class="mt-1 flex flex-wrap gap-1.5">
            <li v-for="domain in side.domains" :key="domain">
              <!-- Le domaine partagé est mis en évidence : c'est la preuve
                   matérielle du doublon, pas une coordonnée parmi d'autres. -->
              <UiBadge
                size="sm"
                :intent="domain === sharedDomain ? 'danger' : 'neutral'"
                :label="domain"
              />
            </li>
          </ul>
        </div>

        <p class="mt-3 text-xs text-text-subtle">{{ createdLabel(side) }}</p>
      </section>
    </div>

    <footer
      class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-t border-border px-4 py-3"
    >
      <p v-if="props.settled" class="text-xs text-text-muted">
        {{
          props.pair.reviewed_by_name
            ? t('admin.organization.duplicates.settled.reviewedBy', {
                name: props.pair.reviewed_by_name,
                date: date(props.pair.reviewed_at, props.timezone),
              })
            : t('admin.organization.duplicates.settled.reviewedAnonymous', {
                date: date(props.pair.reviewed_at, props.timezone),
              })
        }}
      </p>
      <div v-else />

      <div class="flex flex-wrap items-center gap-2">
        <template v-if="props.settled">
          <UiButton
            v-if="props.pair.decision !== 'merged' && props.canMerge"
            variant="ghost"
            size="sm"
            :disabled="props.busy"
            @click="emit('reopen', props.pair)"
          >
            {{ t('admin.organization.duplicates.actions.reopen') }}
          </UiButton>
        </template>

        <template v-else-if="props.canMerge">
          <UiButton
            variant="ghost"
            size="sm"
            :disabled="props.busy"
            @click="emit('defer', props.pair)"
          >
            {{ t('admin.organization.duplicates.actions.defer') }}
          </UiButton>
          <UiButton
            variant="secondary"
            size="sm"
            :disabled="props.busy"
            @click="emit('distinct', props.pair)"
          >
            {{ t('admin.organization.duplicates.actions.distinct') }}
          </UiButton>
          <UiButton size="sm" :disabled="props.busy" @click="emit('merge', props.pair)">
            {{ t('admin.organization.duplicates.actions.merge') }}
          </UiButton>
        </template>
      </div>
    </footer>
  </article>
</template>

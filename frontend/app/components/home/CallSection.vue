<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'

/**
 * L'APPEL À PROPOSITIONS DE L'ÉDITION EN COURS — `id="appel-a-propositions"`.
 *
 * ── L'ANCRE EXISTE TOUJOURS, MÊME QUAND L'APPEL N'EXISTE PAS ────────────────
 *
 * `layouts/public.vue` pointe sur `/#appel-a-propositions` depuis le pied de
 * page de CHAQUE écran du site. Une ancre absente enverrait ce lien en haut de
 * l'accueil, sans un mot d'explication — l'utilisateur croirait au lien mort.
 * La section est donc toujours rendue ; c'est son CONTENU qui change, et le cas
 * « pas d'appel » se dit en une phrase plutôt que de laisser une page muette.
 *
 * ── L'ÉDITION VIENT DES DONNÉES, JAMAIS D'UNE CONSTANTE ─────────────────────
 *
 * `HomeScreen.currentEdition` arrive déjà choisie par l'API : la première
 * édition à pavillon non terminée, à défaut la plus récente. Le jour où la COP32
 * est annoncée, l'accueil la suit sans qu'on touche au code. Une COP écrite en
 * dur, c'est un accueil qui pointe encore la précédente six mois après.
 *
 * ── L'ÉCHÉANCE AFFICHÉE EST L'ÉCHÉANCE EFFECTIVE ────────────────────────────
 *
 * `call_deadline` sort de `event.effective_deadline()` : PROLONGATION COMPRISE.
 * C'est la date que les organisations tiennent, et la seule qu'on ait le droit
 * de leur annoncer.
 *
 * ── LES COULEURS D'ÉTAT NE SONT PAS CELLES QU'ON CROIT ──────────────────────
 *
 * Cyan pour l'information et l'action — un appel ouvert. Jaune pour ce qui
 * demande attention, donc pour les dernières 48 heures : une échéance qui
 * approche en demande. Gris pour ce qui est clos, qui n'est ni un succès ni un
 * échec. Aucun vert : rien n'est confirmé ici.
 */

interface Props {
  /** `HomeScreen.currentEdition` — `null` quand aucune édition ne tient de pavillon. */
  edition: PublicEditionRow | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime, zoneLabel } = useDateTime()
const localePath = useLocalePath()

/** L'édition porte-t-elle réellement un appel ? La ligne peut exister sans. */
const hasCall = computed(() => Boolean(props.edition?.call_id))

const deadline = computed(() => props.edition?.call_deadline ?? null)
const countdown = useCountdown(deadline)

const isOpen = computed(() => Boolean(props.edition?.call_is_open))
const isUrgent = computed(() => isOpen.value && Boolean(countdown.value?.imminent))

const deadlineLabel = computed(() => {
  const edition = props.edition
  if (!edition || !edition.call_deadline) return ''
  return dateTime(edition.call_deadline, edition.timezone)
})

const zone = computed(() =>
  props.edition ? zoneLabel(props.edition.timezone, props.edition.city ?? undefined) : '',
)

const tone = computed(() => {
  if (!isOpen.value) return 'border-border bg-surface-sunken'
  return isUrgent.value ? 'border-warning-border bg-warning-surface' : 'border-accent bg-info-surface'
})

const editionPath = computed(() =>
  props.edition ? localePath(`/evenements/${props.edition.slug}`) : localePath('/'),
)
</script>

<template>
  <section id="appel-a-propositions" class="scroll-mt-24" aria-labelledby="appel-titre">
    <!-- AUCUN APPEL : la section reste, réduite à ce qu'elle peut honnêtement
         dire. Le lien du pied de page atterrit donc quelque part de sensé. -->
    <div
      v-if="!props.edition || !hasCall"
      class="rounded-lg border border-border bg-surface-sunken px-5 py-6 sm:px-7"
    >
      <h2 id="appel-titre" class="font-display text-2xl">{{ t('home.call.none.title') }}</h2>
      <p class="mt-2 text-text-secondary" :style="{ maxWidth: 'var(--measure)' }">
        {{ t('home.call.none.description') }}
      </p>
      <UiButton
        v-if="props.edition"
        class="mt-4"
        variant="secondary"
        :to="editionPath"
        icon-trailing="arrow-right"
        :label="t('home.call.none.action')"
      />
    </div>

    <div
      v-else
      class="rounded-lg border-(length:--border-medium) px-5 py-6 sm:px-7"
      :class="tone"
    >
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="min-w-0">
          <UiBadge
            :intent="isOpen ? (isUrgent ? 'warning' : 'info') : 'neutral'"
            size="sm"
            :solid="isOpen"
          >
            {{ t(isOpen ? 'home.call.state.open' : 'home.call.state.closed') }}
          </UiBadge>
          <h2 id="appel-titre" class="mt-2 font-display text-2xl leading-snug">
            {{ t('home.call.title', { edition: props.edition.edition_label ?? tr(props.edition.title) }) }}
          </h2>
          <p class="mt-2 text-text-secondary" :style="{ maxWidth: 'var(--measure)' }">
            {{ richTextToPlain(tr(props.edition.description)) }}
          </p>
        </div>

        <!-- LE REBOURS n'a de sens qu'ouvert. Absent du rendu serveur, il se
             remplit à l'hydratation — `useCountdown()` explique pourquoi. -->
        <div
          v-if="isOpen && countdown && !countdown.expired"
          class="rounded-md border border-border bg-surface-raised px-4 py-3 text-center"
        >
          <p
            class="text-xs uppercase text-text-subtle"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ t('home.call.remaining') }}
          </p>
          <p
            class="mt-1 font-display text-2xl tabular-nums"
            :class="isUrgent ? 'text-warning' : 'text-accent'"
          >
            <template v-if="countdown.days > 0">
              {{ t('home.call.countdown.days', { count: countdown.days }, countdown.days) }}
            </template>
            <template v-else-if="countdown.hours > 0">
              {{ t('home.call.countdown.hours', { count: countdown.hours }, countdown.hours) }}
            </template>
            <template v-else>
              {{ t('home.call.countdown.minutes', { count: countdown.minutes }, countdown.minutes) }}
            </template>
          </p>
        </div>
      </div>

      <dl v-if="deadlineLabel" class="mt-5 grid gap-4 sm:grid-cols-2">
        <div>
          <dt
            class="text-xs uppercase text-text-subtle"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ t('home.call.deadline') }}
          </dt>
          <dd class="mt-1 flex items-start gap-2 text-text">
            <UiIcon name="clock" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
            <span>
              {{ deadlineLabel }}
              <span class="block text-sm text-text-muted">{{ zone }}</span>
            </span>
          </dd>
        </div>

        <div v-if="props.edition.city || props.edition.country_name">
          <dt
            class="text-xs uppercase text-text-subtle"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ t('home.call.place') }}
          </dt>
          <dd class="mt-1 flex items-start gap-2 text-text">
            <UiIcon name="map-pin" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
            <span>{{ [props.edition.city, tr(props.edition.country_name)].filter(Boolean).join(', ') }}</span>
          </dd>
        </div>
      </dl>

      <div class="mt-6 flex flex-wrap gap-3">
        <UiButton
          v-if="isOpen"
          variant="primary"
          :to="localePath('/deposer-une-proposition')"
          icon-trailing="arrow-right"
          :label="t('home.call.action.submit')"
        />
        <UiButton
          variant="secondary"
          :to="editionPath"
          :label="t('home.call.action.edition')"
        />
      </div>
    </div>
  </section>
</template>

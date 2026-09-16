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
 * ── UN APLAT INSTITUTIONNEL, UNE PHOTOGRAPHIE À DROITE (16/09) ─────────────
 *
 * Le texte vit sur l'aplat foncé, qui ne s'inverse pas au thème sombre ; la
 * couverture de l'édition occupe la droite et s'y fond par `.fade-inverse-start`.
 * Sous 1024 px l'image passe derrière tout le bloc, sous un voile. Sans image,
 * le libellé de l'édition en filigrane — aucun visuel inventé.
 *
 * ── SUR PHOTOGRAPHIE, L'URGENCE PASSE PAR LA PASTILLE ───────────────────────
 *
 * Comme dans `EventHeroCall` : `--color-warning` est illisible sur un fond foncé.
 * Le rebours reste blanc ; la pastille, aplat opaque, porte le jaune des
 * dernières 48 heures. Cyan pour un appel ouvert, gris pour un appel clos.
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

const place = computed(() => {
  const edition = props.edition
  if (!edition) return ''
  return [edition.city, tr(edition.country_name)].filter(Boolean).join(', ')
})

const picture = computed(() => props.edition?.cover ?? props.edition?.banner ?? null)

const stamp = computed(() =>
  props.edition ? (props.edition.edition_label ?? String(props.edition.edition_year)) : '',
)

const showCountdown = computed(() => isOpen.value && Boolean(countdown.value) && !countdown.value?.expired)

/** Le panneau du rebours est de verre sur la photographie, en relief sur l'aplat. */
const countdownPanel = computed(() =>
  picture.value
    ? 'border-glass-border bg-glass shadow-glass backdrop-blur-glass'
    : 'border-border-on-inverse bg-surface-inverse-raised',
)

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

    <!-- `rounded-xl` : bloc d'affiche, comme les cartes d'édition voisines. -->
    <div
      v-else
      class="relative isolate overflow-hidden rounded-xl bg-surface-inverse text-text-on-inverse shadow-md"
    >
      <div class="absolute inset-0 -z-10 lg:left-2/5" aria-hidden="true">
        <template v-if="picture">
          <UiImage
            :image="picture"
            ratio="auto"
            frame-class="size-full"
            class="size-full"
            :class="{ grayscale: !isOpen }"
            sizes="(min-width: 1024px) 45rem, 100vw"
          />
          <div class="absolute inset-0 bg-scrim/60 lg:bg-scrim/15" />
          <div class="fade-inverse-start absolute inset-y-0 left-0 hidden w-3/5 lg:block" />
        </template>
        <div v-else class="flex size-full items-center justify-end overflow-hidden pr-8">
          <span
            class="font-display text-8xl leading-none whitespace-nowrap text-text-on-inverse/10 tabular-nums sm:text-9xl"
          >
            {{ stamp }}
          </span>
        </div>
      </div>

      <div
        class="grid gap-8 px-5 py-8 sm:px-10 sm:py-12 lg:min-h-96 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end lg:px-12"
      >
        <div class="max-w-xl">
          <div class="flex flex-wrap items-center gap-3">
            <UiBadge
              :intent="isOpen ? (isUrgent ? 'warning' : 'info') : 'neutral'"
              size="sm"
              solid
            >
              {{ t(isOpen ? 'home.call.state.open' : 'home.call.state.closed') }}
            </UiBadge>
            <span
              v-if="props.edition.series_name"
              class="text-xs uppercase text-text-on-inverse-muted"
              :style="{ letterSpacing: 'var(--tracking-caps)' }"
            >
              {{ tr(props.edition.series_name) }}
            </span>
          </div>

          <h2
            id="appel-titre"
            class="mt-4 font-display text-2xl leading-tight text-text-on-inverse sm:text-4xl"
          >
            {{ t('home.call.title', { edition: props.edition.edition_label ?? tr(props.edition.title) }) }}
          </h2>
          <p class="mt-3 line-clamp-4 text-text-on-inverse-muted">
            {{ richTextToPlain(tr(props.edition.description)) }}
          </p>

          <dl v-if="deadlineLabel || place" class="mt-6 grid gap-4 sm:grid-cols-2">
            <div v-if="deadlineLabel" class="flex items-start gap-3">
              <UiIcon name="clock" size="1.15rem" class="mt-0.5 shrink-0 text-text-on-inverse-muted" />
              <div>
                <dt
                  class="text-xs uppercase text-text-on-inverse-muted"
                  :style="{ letterSpacing: 'var(--tracking-caps)' }"
                >
                  {{ t('home.call.deadline') }}
                </dt>
                <dd class="mt-1 font-bold">
                  {{ deadlineLabel }}
                  <span class="block text-sm font-normal text-text-on-inverse-muted">{{ zone }}</span>
                </dd>
              </div>
            </div>

            <div v-if="place" class="flex items-start gap-3">
              <UiIcon name="map-pin" size="1.15rem" class="mt-0.5 shrink-0 text-text-on-inverse-muted" />
              <div>
                <dt
                  class="text-xs uppercase text-text-on-inverse-muted"
                  :style="{ letterSpacing: 'var(--tracking-caps)' }"
                >
                  {{ t('home.call.place') }}
                </dt>
                <dd class="mt-1 font-bold">{{ place }}</dd>
              </div>
            </div>
          </dl>

          <div class="mt-8 flex flex-wrap items-center gap-x-6 gap-y-3">
            <UiButton
              v-if="isOpen"
              size="lg"
              :to="localePath('/deposer-une-proposition')"
              icon-trailing="arrow-right"
              :label="t('home.call.action.submit')"
            />
            <NuxtLink
              :to="editionPath"
              class="inline-flex min-h-(--target-min) items-center gap-2 font-bold text-text-on-inverse underline-offset-4 hover:underline"
            >
              {{ t('home.call.action.edition') }}
              <UiIcon name="arrow-right" size="1rem" />
            </NuxtLink>
          </div>
        </div>

        <!-- LE REBOURS n'a de sens qu'ouvert. Absent du rendu serveur, il se
             remplit à l'hydratation — `useCountdown()` explique pourquoi. -->
        <div
          v-if="showCountdown && countdown"
          class="justify-self-start rounded-lg border px-6 py-5 lg:justify-self-end"
          :class="countdownPanel"
        >
          <p
            class="text-xs uppercase text-text-on-inverse-muted"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ t('home.call.remaining') }}
          </p>
          <p class="mt-1 font-display text-4xl leading-none text-text-on-inverse tabular-nums sm:text-5xl">
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
    </div>
  </section>
</template>

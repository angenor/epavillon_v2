<script setup lang="ts">
import type { IsoDateTime } from '~/types/shared'
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'

/**
 * LA FRISE DES ACTIVITÉS RETENUES — le second bloc du panneau « À venir ».
 *
 * Ce que le visiteur vient chercher : ce qui se passe, et quand. Les séances
 * s'y suivent dans l'ordre du temps, groupées par journée, la plus proche en
 * tête. Un rail vertical relie les journées ; chacune porte sa pastille.
 *
 * ── CE NE SONT QUE DES ACTIVITÉS RETENUES ───────────────────────────────────
 *
 * `programme.v_public_schedule` ne rend que les séances PUBLIÉES — donc issues
 * d'un dossier accepté, ou programmées directement par l'IFDD. Le panneau n'a
 * aucun filtre à appliquer et ne doit surtout pas en réinventer un : le jour où
 * la vue changerait de règle, deux endroits seraient à corriger.
 *
 * ── UNE ÉDITION NOMMÉE, OU AUCUNE ───────────────────────────────────────────
 *
 * Quand toutes les séances viennent de la même édition, le titre du bloc la
 * nomme (« Au programme — COP31 ») et les cartes n'ont pas à la répéter. Quand
 * la frise en mêle plusieurs, le titre reste nu et c'est chaque carte qui dit
 * d'où elle vient.
 *
 * ── LE JOUR RELATIF SE LIT MIEUX QUE LA DATE ────────────────────────────────
 *
 * « Aujourd'hui » et « Demain » précèdent la date, ils ne la remplacent pas :
 * un visiteur qui prépare son déplacement a besoin du quantième, celui qui
 * regarde en passant a besoin de savoir que c'est maintenant.
 */

interface Props {
  /** Les séances à venir ou en cours, déjà choisies et bornées par l'API. */
  sessions: PublicScheduleRow[]
  /** L'historique complet : sert à nommer l'édition d'une séance et à la relier. */
  editions: PublicEditionRow[]
  /** Instant de composition de la réponse — l'horloge qui fait autorité. */
  now: IsoDateTime
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dayLong } = useDateTime()
const localePath = useLocalePath()

const days = computed(() => groupProgrammeDays(props.sessions, props.now))

/** L'édition commune, s'il y en a une : elle décide de l'intitulé du bloc. */
const singleEdition = computed(() => {
  const eventId = commonEditionId(props.sessions)
  if (!eventId) return null
  return props.editions.find((edition) => edition.id === eventId) ?? null
})

const heading = computed(() => {
  const edition = singleEdition.value
  if (!edition) return t('home.aside.programme.title')
  return t('home.aside.programme.titleOf', {
    edition: edition.edition_label ?? edition.acronym ?? tr(edition.title),
  })
})

/**
 * Une séance ne porte pas son édition en slug : la vue publique n'expose que
 * `event_id`. La correspondance vient donc de la liste des éditions, déjà en
 * main — aucune requête de plus pour fabriquer un lien ou un sigle.
 */
const editionByEvent = computed(
  () => new Map(props.editions.map((edition) => [edition.id, edition])),
)

function sessionTo(session: PublicScheduleRow): string | undefined {
  const edition = editionByEvent.value.get(session.event_id)
  return edition ? localePath(`/programmations?edition=${edition.slug}`) : undefined
}

/** La ville de l'édition, sans quoi « heure de Belem » s'affiche sans accent. */
function sessionZoneLabel(session: PublicScheduleRow): string | undefined {
  return editionByEvent.value.get(session.event_id)?.city ?? undefined
}

/** Nul dès que la frise ne montre qu'une édition : le titre la nomme déjà. */
function sessionEditionLabel(session: PublicScheduleRow): string | null {
  if (singleEdition.value) return null
  const edition = editionByEvent.value.get(session.event_id)
  if (!edition) return null
  return edition.edition_label ?? edition.acronym ?? null
}

/** « Aujourd'hui », « Demain », ou rien — la date suit dans tous les cas. */
function relativeLabel(daysAhead: number): string {
  if (daysAhead <= 0) return t('home.aside.programme.today')
  if (daysAhead === 1) return t('home.aside.programme.tomorrow')
  return ''
}
</script>

<template>
  <section v-if="days.length">
    <div class="flex items-baseline justify-between gap-2">
      <h3
        class="text-xs font-bold uppercase text-text-on-inverse-muted"
        :style="{ letterSpacing: 'var(--tracking-caps)' }"
      >
        {{ heading }}
      </h3>
      <NuxtLink
        :to="localePath('/programmations')"
        class="text-xs text-text-on-inverse no-underline hover:underline"
      >
        {{ t('home.aside.programme.all') }}
      </NuxtLink>
    </div>

    <!-- LE RAIL. Un trait vertical porté par la bordure du conteneur, et une
         pastille par journée posée dessus : deux éléments, pas un pseudo-élément
         par carte. La pastille du jour le plus proche est pleine, les suivantes
         sourdes — c'est ce qui donne au regard son point d'entrée. -->
    <ol class="mt-3 ms-1 flex list-none flex-col gap-4 border-s border-glass-border ps-[18px]">
      <li v-for="day in days" :key="day.key" class="relative flex flex-col gap-2">
        <span
          class="absolute -start-[23px] top-1 size-[9px] rounded-full ring-[3px] ring-glass"
          :class="day.daysAhead <= 1 ? 'bg-text-on-inverse' : 'bg-glass-border-strong'"
          aria-hidden="true"
        />

        <h4 class="text-[0.8125rem] font-bold text-text-on-inverse">
          <template v-if="relativeLabel(day.daysAhead)">
            {{ relativeLabel(day.daysAhead) }}
            <span class="font-normal text-text-on-inverse-muted">
              · {{ dayLong(day.startsAt, day.timezone) }}
            </span>
          </template>
          <template v-else>{{ dayLong(day.startsAt, day.timezone) }}</template>
        </h4>

        <HomeAsideSession
          v-for="session in day.sessions"
          :key="session.id"
          :session="session"
          :to="sessionTo(session)"
          :zone-label="sessionZoneLabel(session)"
          :edition-label="sessionEditionLabel(session)"
        />
      </li>
    </ol>
  </section>
</template>

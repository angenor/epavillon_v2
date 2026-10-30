<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'
import type { ProgrammeData } from '~/types/event-programme'

/**
 * PROGRAMMATIONS — l'écran qui porte les programmes publiés, toutes éditions
 * confondues.
 *
 * ── POURQUOI UNE PAGE, ET PLUS UNE SECTION ──────────────────────────────────
 *
 * La programmation vivait dans la page de l'édition. Le sélecteur d'année y
 * proposait pourtant la COP30, la COP29 et le cycle de webinaires PACO : on
 * lisait le programme de PACO sous le titre « COP31 — Belém », ce qui n'a aucun
 * sens. Une seule chose peut être le sujet d'un écran, et sur celui-ci c'est
 * l'ÉDITION CHOISIE — nommée en titre, avec ses dates et son lieu.
 *
 * La page d'une édition ne porte donc plus qu'un lien, qui arrive ici avec son
 * édition déjà sélectionnée (`?edition=<slug>`).
 *
 * ── L'ÉDITION PAR DÉFAUT ────────────────────────────────────────────────────
 *
 * Sans paramètre — quand on entre par la barre de navigation —, l'édition en
 * cours, sinon la plus proche à venir, sinon la dernière passée
 * (`defaultProgrammeEdition()`, arbitré par le commanditaire le 16/09).
 *
 * ── LE BANDEAU SUIT L'ÉDITION CONSULTÉE ─────────────────────────────────────
 *
 * `EventProgrammeHero` porte le sélecteur ; le changement passe par
 * `EventProgramme.select()`, qui charge le programme avant de le signaler. Un
 * bandeau qui resterait sur la COP31 pendant qu'on lit PACO referait le défaut
 * qui a créé cette page.
 *
 * ── CE QUE LA PAGE CHARGE ───────────────────────────────────────────────────
 *
 * Les éditions publiques, puis le programme de la seule édition ouverte à
 * l'arrivée. Les autres se chargent à la demande, une fois chacune, dans
 * `EventProgramme`.
 *
 * ── LE BANDEAU D'INCIDENT ───────────────────────────────────────────────────
 *
 * Publier un message d'incident ne servait à rien tant qu'aucune page publique
 * ne le montrait. Ici, à l'échelle de l'ÉDITION, par la fonction descendante du
 * modèle ; la page d'une activité, sa salle et celle d'une journée spéciale
 * montrent le leur (31/10). Trois au plus : `EventIncidentBanners`.
 *
 * Il se charge À PART du programme, et une panne de sa lecture ne casse pas la
 * page : un bandeau absent se lit comme une édition sans incident, ce qui est le
 * cas normal.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/programmations', en: '/programmes' } })

const route = useRoute()
const api = useApi()
const { t } = useI18n()

const { data, status, error, refresh } = await useAsyncData('programme-page', async () => {
  const editions = await api.events.publicList()
  if (!editions.length) return null

  const wanted = typeof route.query.edition === 'string' ? route.query.edition : null
  const edition: PublicEditionRow | undefined =
    (wanted ? editions.find((entry) => entry.slug === wanted) : undefined) ??
    defaultProgrammeEdition(editions)

  if (!edition) return null

  const [schedule, days, rooms, tracks] = await Promise.all([
    api.sessions.schedule(edition.id),
    api.events.days(edition.id),
    api.events.rooms(edition.id),
    api.sessions.scheduleTracks(edition.id).catch(() => []),
  ])

  return { editions, edition, programme: { schedule, days, rooms, tracks } satisfies ProgrammeData }
}, { lazy: true })

const current = ref<PublicEditionRow | null>(null)
const shown = computed(() => current.value ?? data.value?.edition ?? null)
const programme = useTemplateRef<{ select: (eventId: string) => Promise<void> }>('programme')

/**
 * Les messages actifs de l'édition ouverte. Chargés à part, et **sans casser la
 * page s'ils manquent** : le programme reste le sujet de l'écran.
 */
const { data: incidents } = await useAsyncData(
  () => `programme-incidents-${shown.value?.id ?? 'aucune'}`,
  async () => {
    const eventId = shown.value?.id
    if (!eventId) return []
    return api.incidents.forEvent(eventId).catch(() => [])
  },
  // Signal à écouter ici : `edition:<id>:public:incidents`, puis relire.
  { default: () => [], lazy: true },
)

useHead(() => ({ title: t('programme.title') }))
</script>

<template>
  <div class="flex flex-col gap-8">
    <!-- Le bandeau déborde la gouttière et remonte sous le menu : il doit
         rester le premier enfant de la page. -->
    <EventProgrammeHero
      v-if="shown && data"
      :edition="shown"
      :editions="data.editions"
      @select="programme?.select($event)"
    />

    <EventProgrammeHeroSkeleton v-else-if="status === 'pending'" />

    <header v-else>
      <h1 class="font-display text-3xl">{{ t('programme.title') }}</h1>
      <p class="mt-2 max-w-(--measure) text-text-muted">{{ t('programme.description') }}</p>
    </header>

    <!-- Juste sous le bandeau : ce qui ne se passe pas comme prévu se lit avant le programme. -->
    <EventIncidentBanners :incidents="incidents ?? []" :timezone="shown?.timezone" :zone-label="shown?.city ?? undefined" />

    <UiLoadingState v-if="status === 'pending'" :label="t('programme.loading')">
      <EventProgrammeSkeleton />
    </UiLoadingState>

    <UiErrorState
      v-else-if="error"
      :title="t('programme.error.title')"
      :description="t('programme.error.description')"
      @retry="refresh()"
    />

    <!-- Aucune édition publique : une plateforme fraîchement installée, ou tout
         en brouillon. Ce n'est pas une panne, et le dire comme telle enverrait
         le visiteur chercher un problème qui n'existe pas. -->
    <UiEmptyState
      v-else-if="!data"
      icon="calendar"
      :title="t('programme.noEdition.title')"
      :description="t('programme.noEdition.description')"
    />

    <EventProgramme
      v-else
      ref="programme"
      :edition="data.edition"
      :initial="data.programme"
      :editions="data.editions"
      @update:edition="current = $event"
    />
  </div>
</template>

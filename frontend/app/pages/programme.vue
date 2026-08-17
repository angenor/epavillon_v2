<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
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
 * Sans paramètre — quand on entre par la barre de navigation — l'écran ouvre la
 * dernière édition dont le programme est PUBLIÉ. Ouvrir sur l'édition en cours
 * serait plus cohérent avec le reste du site, mais son programme ne l'est
 * justement pas encore : on afficherait « programme à venir » à quelqu'un qui
 * vient lire un programme. À défaut de toute publication, l'édition en cours
 * fait l'affaire et l'écran l'annonce.
 *
 * ── CE QUE LA PAGE CHARGE ───────────────────────────────────────────────────
 *
 * Les éditions publiques, les séries (leur `kind` sépare les conférences du
 * reste), puis le programme de la seule édition ouverte à l'arrivée. Les autres
 * se chargent à la demande, une fois chacune, dans `EventProgramme`.
 */

definePageMeta({ layout: 'public' })
defineI18nRoute({ paths: { fr: '/programmations', en: '/programmes' } })

const route = useRoute()
const api = useApi()
const { t } = useI18n()

const { data, status, error, refresh } = await useAsyncData('programme-page', async () => {
  const [editions, series] = await Promise.all([api.events.publicList(), api.events.series()])
  if (!editions.length) return null

  const wanted = typeof route.query.edition === 'string' ? route.query.edition : null
  const published = editions.filter((edition) => edition.programme_published_at !== null)

  // `publicList()` rend les éditions de la plus récente à la plus ancienne.
  const edition: EventEdition | undefined =
    (wanted ? editions.find((entry) => entry.slug === wanted) : undefined) ??
    published[0] ??
    editions[0]

  if (!edition) return null

  const [schedule, days, rooms] = await Promise.all([
    api.sessions.schedule(edition.id),
    api.events.days(edition.id),
    api.events.rooms(edition.id),
  ])

  return { editions, series, edition, programme: { schedule, days, rooms } satisfies ProgrammeData }
})

useHead(() => ({ title: t('programme.title') }))
</script>

<template>
  <div class="flex flex-col gap-8">
    <header>
      <h1 class="font-display text-3xl">{{ t('programme.title') }}</h1>
      <p class="mt-2 max-w-(--measure) text-text-muted">{{ t('programme.description') }}</p>
    </header>

    <UiLoadingState
      v-if="status === 'pending'"
      variant="card"
      :lines="3"
      :label="t('programme.loading')"
    />

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
      :edition="data.edition"
      :initial="data.programme"
      :editions="data.editions"
      :series="data.series"
    />
  </div>
</template>

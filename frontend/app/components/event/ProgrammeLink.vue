<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'

/**
 * RENVOI VERS LA PROGRAMMATION — le seul reste, sur la page d'une édition, de ce
 * qui fut une section entière.
 *
 * La programmation a sa page (`/programmations`) parce qu'elle porte un
 * sélecteur d'édition : la garder ici revenait à afficher le programme du cycle
 * PACO sous le titre « COP31 — Belém ». Le lien part donc avec l'édition de
 * cette page déjà sélectionnée, et l'écran d'arrivée la nomme en titre.
 *
 * IL ANNONCE CE QU'IL Y A DERRIÈRE. Un lien nu — « Voir la programmation » — ne
 * dit pas s'il mène à quarante activités ou à une page vide ; le nombre publié
 * et le nombre de jours suffisent à décider de cliquer. Quand rien n'est encore
 * publié, il le dit et propose les éditions passées plutôt qu'une page qui
 * s'excusera.
 */

interface Props {
  edition: EventEdition
  /** Activités publiées de cette édition — `programme.v_public_schedule`. */
  sessionCount: number
  /** Jours qui en portent au moins une. */
  dayCount: number
}

const props = defineProps<Props>()

const { t } = useI18n()
const localePath = useLocalePath()

const isPublished = computed(() => props.edition.programme_published_at !== null)

const to = computed(() =>
  isPublished.value
    ? `${localePath('/programmations')}?edition=${props.edition.slug}`
    : localePath('/programmations'),
)
</script>

<template>
  <section
    id="programmation"
    class="scroll-mt-24 rounded-lg border border-border bg-surface-raised p-5 sm:p-6"
    aria-labelledby="programmation-lien-titre"
  >
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="min-w-0">
        <h2 id="programmation-lien-titre" class="font-display text-xl">
          {{ t('event.public.programmeLink.title') }}
        </h2>

        <p v-if="isPublished" class="mt-1 text-sm text-text-muted">
          {{ t('event.public.programmeLink.count', { count: props.sessionCount }, props.sessionCount) }}
          <span v-if="props.dayCount">
            · {{ t('event.public.programmeLink.dayCount', { count: props.dayCount }, props.dayCount) }}
          </span>
        </p>
        <p v-else class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('event.public.programmeLink.unpublished') }}
        </p>
      </div>

      <UiButton
        :variant="isPublished ? 'primary' : 'secondary'"
        :to="to"
        icon-trailing="arrow-right"
        :label="
          isPublished
            ? t('event.public.programmeLink.see')
            : t('event.public.programmeLink.seePast')
        "
      />
    </div>
  </section>
</template>

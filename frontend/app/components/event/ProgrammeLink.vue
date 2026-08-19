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
  <!-- UN APLAT INSTITUTIONNEL, ET NON UNE CARTE DE PLUS. C'est le renvoi le plus
       fréquenté de la page une fois le programme publié : posé sur le bleu de la
       charte, il se repère en balayant l'écran, là où quatre cartes grises se
       ressemblent toutes. L'aplat ne s'inverse pas au thème sombre — c'est un
       bloc de mise en page, pas une surface de thème. -->
  <section
    id="programmation"
    class="scroll-mt-24 overflow-hidden rounded-xl bg-surface-inverse p-6 text-text-on-inverse sm:p-8"
    aria-labelledby="programmation-lien-titre"
  >
    <div class="flex flex-wrap items-end justify-between gap-6">
      <div class="min-w-0">
        <h2
          id="programmation-lien-titre"
          class="font-display text-2xl text-text-on-inverse sm:text-3xl"
        >
          {{ t('event.public.programmeLink.title') }}
        </h2>

        <!-- LE CHIFFRE EN GRAND, parce qu'il décide du clic : un lien nu ne dit
             pas s'il mène à quarante activités ou à une page vide. -->
        <p v-if="isPublished" class="mt-4 flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <span class="font-display text-display leading-none tabular-nums text-text-on-inverse">
            {{ props.sessionCount }}
          </span>
          <span class="text-text-on-inverse-muted">
            {{ t('event.public.programmeLink.countUnit', props.sessionCount) }}
            <template v-if="props.dayCount">
              · {{ t('event.public.programmeLink.dayCount', { count: props.dayCount }, props.dayCount) }}
            </template>
          </span>
        </p>
        <p v-else class="mt-4 max-w-(--measure) text-sm text-text-on-inverse-muted">
          {{ t('event.public.programmeLink.unpublished') }}
        </p>
      </div>

      <UiButton
        variant="primary"
        size="lg"
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

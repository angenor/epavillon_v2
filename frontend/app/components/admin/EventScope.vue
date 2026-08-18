<script setup lang="ts">
/**
 * SÉLECTEUR D'ÉVÉNEMENT — en tête de tous les écrans du back-office.
 *
 * RÈGLE MÉTIER N° 8, ET ELLE SE JOUE ICI. Un administrateur peut n'avoir accès
 * qu'à UNE SEULE édition. Dans ce cas, il n'y a pas de liste déroulante à une
 * entrée, pas de mention « votre compte n'administre qu'un événement », pas de
 * compteur : le back-office se lit comme s'il n'existait qu'une COP, parce que
 * pour cette personne c'est le cas. Une liste à une entrée est un aveu — elle
 * dit qu'il existe une liste.
 *
 * POURQUOI EN TÊTE DE PAGE ET NON DANS LA NAVIGATION LATÉRALE, où il était posé
 * depuis A0.1 : sur écran étroit, la navigation latérale est un tiroir qu'il
 * faut ouvrir. Le périmètre d'un écran ne peut pas être caché derrière un
 * bouton — c'est le sujet de tout ce qu'on lit en dessous. Un seul exemplaire
 * dans l'application, posé par le layout, lu par tous les écrans.
 *
 * LE FUSEAU DE L'ÉDITION EST ANNONCÉ ICI, une fois : toutes les heures des
 * écrans du back-office s'entendent dans ce fuseau, et le répéter sur chaque
 * ligne d'un tableau dense le rendrait illisible.
 */

import type { SelectOption } from '~/types/ui'

const { t } = useI18n()
const { tr } = useI18nText()
const { zoneLabel } = useDateTime()
const adminScope = useAdminScopeStore()
const route = useRoute()
const router = useRouter()

/**
 * L'ÉDITION COURANTE VIT DANS L'URL (`?evenement=…`), comme l'organisation
 * courante de l'espace organisation (A5).
 *
 * Un tableau de bord se transmet : « regarde ce qui traîne sur la COP30 » est un
 * lien, pas une consigne de manipulation. Sans ce paramètre, deux membres de
 * l'équipe s'envoient une adresse qui n'ouvre pas la même chose — et le premier
 * écran du back-office est justement celui qu'on s'envoie.
 *
 * Le paramètre reste FILTRÉ PAR LE PÉRIMÈTRE : `selectEvent()` refuse une
 * édition hors liste, une URL forgée à la main ne fait donc rien changer. Le
 * filtrage qui compte reste celui de l'API.
 */
const requested = computed(() => {
  const value = route.query.evenement
  return (Array.isArray(value) ? value[0] : value) ?? null
})

watch(
  requested,
  (id) => {
    if (!id) return
    adminScope.selectEvent(id)
    // L'ÉDITION DEMANDÉE A ÉTÉ REFUSÉE — elle n'est pas dans le périmètre.
    // L'écran affiche déjà la bonne, mais l'URL continuerait d'annoncer l'autre :
    // on la ramène à ce qui est réellement montré, sinon le lien recopié promet
    // une édition qu'il n'ouvre pas.
    if (import.meta.client && adminScope.currentEventId !== id) {
      void router.replace({
        query: { ...route.query, evenement: adminScope.currentEventId ?? undefined },
      })
    }
  },
  { immediate: true },
)

/**
 * Le TITRE seul, jamais « sigle — titre » : les titres d'édition du modèle
 * commencent déjà par leur sigle (« COP31 — Conférence des Nations unies… »), et
 * le préfixer une seconde fois donne « COP31 — COP31 — Conférence… ». Le sigle
 * vit dans `event.events.acronym` pour les écrans qui n'ont la place que de lui.
 */
const options = computed<SelectOption[]>(() =>
  adminScope.events.map((event) => ({ value: event.id, label: tr(event.title) })),
)

const selectedEventId = computed({
  get: () => adminScope.currentEventId ?? '',
  set: (value: string) => {
    adminScope.selectEvent(value === '' ? null : value)
    // `replace` et non `push` : changer de périmètre n'est pas une navigation,
    // et le bouton « précédent » ne doit pas rejouer une suite de sélections.
    void router.replace({ query: { ...route.query, evenement: value || undefined } })
  },
})

const current = computed(() => adminScope.currentEvent)
</script>

<template>
  <div
    v-if="!adminScope.isEmpty"
    class="flex flex-wrap items-center justify-between gap-x-6 gap-y-3 border-b border-border bg-surface-raised px-4 py-3 sm:px-6"
  >
    <div class="min-w-0">
      <p class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
        {{ t('nav.admin.eventScope.label') }}
      </p>
      <p class="mt-0.5 truncate font-display text-lg leading-tight text-heading">
        {{ tr(current?.title) }}
      </p>
    </div>

    <!-- Le sélecteur n'existe qu'à partir de deux éditions. En dessous, il n'y
         a rien à choisir et rien à laisser entendre. -->
    <div v-if="!adminScope.isRestricted" class="flex w-full flex-col gap-1 sm:w-80">
      <UiSelect
        id="admin-event-scope"
        v-model="selectedEventId"
        :options="options"
        :label="t('nav.admin.eventScope.label')"
        :hint="t('nav.admin.eventScope.hint')"
        hide-label
        hide-optional
      />
    </div>

    <p v-else-if="current" class="text-sm text-text-subtle">
      {{ zoneLabel(current.timezone, current.city ?? undefined) }}
    </p>
  </div>
</template>

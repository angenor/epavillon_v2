<script setup lang="ts">
import type { PublicEditionRow, PublicScheduleRow, ShowcaseRow } from '~/types/views'
import type { EventId, TimeZoneName } from '~/types/shared'

/**
 * LE PANNEAU « À VENIR » — la colonne gauche du bandeau.
 *
 * ── CE N'EST PAS UN TIROIR, ET CE N'EST PAS NÉGOCIABLE ──────────────────────
 *
 * Sur grand écran, une colonne de 340 px posée dans le bandeau, avec son propre
 * défilement. En dessous de `lg`, il sort du bandeau et devient une section à
 * part entière, juste en dessous — jamais un bouton « Voir plus ». C'est
 * l'information la PLUS utile de la page : ce qui commence bientôt. La cacher
 * derrière une interaction serait un contresens.
 *
 * ── TROIS BLOCS, DANS CET ORDRE ─────────────────────────────────────────────
 *
 *   1. Les ÉPINGLES ÉDITORIALES (`home_aside`) — ce que l'IFDD veut dire
 *      aujourd'hui. Elles remplacent cinq widgets d'annonce écrits en dur.
 *   2. Les PROCHAINES SÉANCES, toutes éditions confondues. C'est le bloc qu'on
 *      vient chercher.
 *   3. Les PROCHAINS RENDEZ-VOUS — les éditions qui s'ouvrent.
 *
 * ── RIEN N'EST RE-FILTRÉ ────────────────────────────────────────────────────
 *
 * `api.home.screen()` sert des séances déjà réduites aux six premières à venir
 * ou en cours, annulations exclues, et des épingles dont la vue a déjà appliqué
 * le statut et la fenêtre de diffusion. Le seul choix fait ici est celui des
 * trois prochaines éditions — un tri, pas un filtre de publication.
 *
 * ── LE DIRECT SE DÉCLARE, IL NE SE DEVINE PAS ───────────────────────────────
 *
 * Règle métier n° 4 : un seul direct à la fois, tous événements confondus.
 * `useLiveSession()` porte cette unicité pour toute l'application ; sans
 * déclaration, aucune carte ne montre le repère. L'accueil déclare donc la
 * séance qu'il connaît, et une seule — la première de sa liste dont le statut
 * est `live`.
 */

interface Props {
  /** `content.v_showcase`, `placement = 'home_aside'`, trié par `sort_order`. */
  pins: ShowcaseRow[]
  /** Les six prochaines séances, déjà choisies par l'API. */
  sessions: PublicScheduleRow[]
  /** L'historique COMPLET : le panneau n'en montre que les prochaines. */
  editions: PublicEditionRow[]
}

const props = defineProps<Props>()

const { t } = useI18n()
const localePath = useLocalePath()
const { setLive } = useLiveSession()

const nextThree = computed(() => nextEditions(props.editions, 3))

/**
 * Une séance ne porte pas son édition en slug : la vue publique n'expose que
 * `event_id`. La correspondance vient donc de la liste des éditions, déjà en
 * main — aucune requête de plus pour fabriquer un lien.
 */
const slugByEvent = computed<Record<EventId, string>>(() =>
  Object.fromEntries(props.editions.map((edition) => [edition.id, edition.slug])),
)

function sessionTo(session: PublicScheduleRow): string | undefined {
  const slug = slugByEvent.value[session.event_id]
  return slug ? localePath(`/programmations?edition=${slug}`) : undefined
}

/**
 * Le fuseau de l'édition de rattachement d'une épingle. `v_showcase` ne le
 * porte pas — la vue n'expose que `event_id` — et il décide pourtant du JOUR
 * affiché : une fenêtre close le 30 septembre à 23 h 59 heure de Belém tombe le
 * 1er octobre en temps universel. La liste des éditions est déjà en main.
 */
const timezoneByEvent = computed<Record<EventId, TimeZoneName>>(() =>
  Object.fromEntries(props.editions.map((edition) => [edition.id, edition.timezone])),
)

function pinTimezone(pin: ShowcaseRow): TimeZoneName | null {
  return pin.event_id ? (timezoneByEvent.value[pin.event_id] ?? null) : null
}

/** La séance en direct, déclarée UNE fois pour toute l'application. */
watch(
  () => props.sessions,
  (sessions) => {
    const live = sessions.find((session) => session.status === 'live')
    if (live) setLive(live.id)
  },
  { immediate: true },
)

const isEmpty = computed(
  () => props.pins.length === 0 && props.sessions.length === 0 && nextThree.value.length === 0,
)
</script>

<template>
  <!-- LE PANNEAU EST UNE SURFACE DE VERRE, comme sur la plateforme de référence
       (arbitrage du 19/08).

       DEUX FONDS POUR UNE SEULE MATIÈRE. À partir de `lg`, il flotte sur la
       photographie du bandeau : le verre suffit, il laisse voir l'image
       derrière. En dessous, il sort du bandeau et n'a plus rien sous lui — d'où
       l'aplat institutionnel, qui donne au verre de quoi se poser. Le TEXTE, lui,
       ne change pas : `--color-text-on-inverse` dans les deux cas, parce que le
       fond est sombre dans les deux cas. Faire basculer les cartes d'une
       matière à l'autre selon la largeur aurait donné deux dessins à tenir. -->
  <aside
    class="flex h-full flex-col bg-surface-inverse text-text-on-inverse lg:bg-glass lg:backdrop-blur-glass"
    :aria-label="t('home.aside.title')"
  >
    <div class="border-b border-glass-border px-4 py-4 sm:px-6 lg:px-5">
      <div class="mx-auto w-full max-w-[1280px] lg:mx-0">
        <!-- LA COULEUR EST EXPLICITE, et il le faut : `main.css` peint tous les
             `h1..h6` en `--color-heading` par une règle d'ÉLÉMENT, laquelle ne
             connaît pas le fond sur lequel le titre se pose. Sur ce panneau
             sombre, le bleu nuit institutionnel devenait invisible. -->
        <h2 class="font-display text-xl text-text-on-inverse">{{ t('home.aside.title') }}</h2>
      </div>
    </div>

    <!-- LE DÉFILEMENT EST INTERNE, ET SEULEMENT SUR GRAND ÉCRAN : `min-h-0` est
         indispensable dans une colonne flex, sans quoi le contenu pousse le
         panneau au lieu de défiler dedans. En dessous de `lg`, la section suit
         le flux de la page — un panneau qui défilerait dans un téléphone
         emprisonnerait le geste. -->
    <div class="min-h-0 flex-1 px-4 py-4 sm:px-6 lg:overflow-y-auto lg:px-5">
      <div class="mx-auto flex w-full max-w-[1280px] flex-col gap-6 lg:mx-0">
        <UiEmptyState
          v-if="isEmpty"
          compact
          icon="calendar"
          :title="t('home.aside.empty.title')"
          :description="t('home.aside.empty.description')"
        />

        <!-- L'ORDRE DES TROIS BLOCS EST CELUI DE LA DEMANDE, PAS CELUI DU MODÈLE.
             Le panneau s'appelle « À venir » : ce qu'on vient y chercher, ce
             sont les prochaines séances et les prochaines éditions. Les
             épingles éditoriales — ce que l'IFDD veut faire remarquer — passent
             après, sans quoi le premier écran d'un panneau qui promet un
             calendrier n'en montre aucun. Elles restent DANS le panneau et non
             dans le bandeau : ce sont des rappels datés, pas des diapositives. -->
        <section v-if="props.sessions.length">
          <div class="flex items-baseline justify-between gap-2">
            <h3
              class="text-xs font-bold uppercase text-text-on-inverse-muted"
              :style="{ letterSpacing: 'var(--tracking-caps)' }"
            >
              {{ t('home.aside.sessions.title') }}
            </h3>
            <NuxtLink :to="localePath('/programmations')" class="text-xs text-text-on-inverse no-underline hover:underline">
              {{ t('home.aside.sessions.all') }}
            </NuxtLink>
          </div>
          <div class="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-1">
            <HomeAsideSession
              v-for="session in props.sessions"
              :key="session.id"
              :session="session"
              :to="sessionTo(session)"
            />
          </div>
        </section>

        <section v-if="nextThree.length">
          <h3
            class="text-xs font-bold uppercase text-text-on-inverse-muted"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ t('home.aside.editions.title') }}
          </h3>
          <div class="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-1">
            <HomeAsideEdition
              v-for="edition in nextThree"
              :key="edition.id"
              :edition="edition"
            />
          </div>
        </section>

        <section v-if="props.pins.length">
          <h3
            class="text-xs font-bold uppercase text-text-on-inverse-muted"
            :style="{ letterSpacing: 'var(--tracking-caps)' }"
          >
            {{ t('home.aside.pins.title') }}
          </h3>
          <div class="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-1">
            <HomeAsidePin
              v-for="pin in props.pins"
              :key="pin.id"
              :pin="pin"
              :timezone="pinTimezone(pin)"
            />
          </div>
        </section>
      </div>
    </div>
  </aside>
</template>

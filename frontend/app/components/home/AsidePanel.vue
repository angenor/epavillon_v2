<script setup lang="ts">
import type { EditionStatsRow, PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { EventId, IsoDateTime } from '~/types/shared'

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
 * ── DEUX BLOCS, ET RIEN QUI SE COMPOSE À LA MAIN ────────────────────────────
 *
 *   1. LES ÉVÉNEMENTS À VENIR — le prochain rendez-vous en carte pleine, les
 *      suivants en lignes. Dès qu'une édition à venir existe, elle est là.
 *   2. LA FRISE DES ACTIVITÉS RETENUES — jour par jour, la programmation qui
 *      vient. Dès qu'une activité est retenue et publiée, elle est là.
 *
 * CE PANNEAU EST AUTOMATIQUE, arbitrage du commanditaire du 24/08. Il portait
 * jusque-là les épingles éditoriales de la vitrine (`home_aside`), et sur une
 * plateforme dont la base ne portait encore ni édition ni séance, elles étaient
 * seules à s'afficher : un panneau qui promet un calendrier ne montrait que des
 * annonces. Rien ne s'y compose plus depuis le back-office — la vitrine reste
 * ce qu'elle est, mais pour le BANDEAU, pas pour cette colonne.
 *
 * ── RIEN N'EST RE-FILTRÉ ────────────────────────────────────────────────────
 *
 * `api.home.screen()` sert des séances déjà réduites aux premières à venir ou
 * en cours, annulations exclues. Le seul choix fait ici est celui des trois
 * prochaines éditions — un tri, pas un filtre de publication.
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
  /** Les prochaines séances, déjà choisies par l'API. */
  sessions: PublicScheduleRow[]
  /** L'historique COMPLET : le panneau n'en montre que les prochaines. */
  editions: PublicEditionRow[]
  /** `programme.v_edition_stats`, indexée par édition. Absent vaut zéro. */
  stats: Record<EventId, EditionStatsRow>
  /** Instant de composition de la réponse — l'horloge qui fait autorité. */
  now: IsoDateTime
}

const props = defineProps<Props>()

const { t } = useI18n()
const localePath = useLocalePath()
const { setLive } = useLiveSession()

const nextThree = computed(() => nextEditions(props.editions, 3))

/** La séance en direct, déclarée UNE fois pour toute l'application. */
watch(
  () => props.sessions,
  (sessions) => {
    const live = sessions.find((session) => session.status === 'live')
    if (live) setLive(live.id)
  },
  { immediate: true },
)

const isEmpty = computed(() => props.sessions.length === 0 && nextThree.value.length === 0)
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

        <!-- 1. LES ÉVÉNEMENTS À VENIR -->
        <section v-if="nextThree.length">
          <div class="flex items-baseline justify-between gap-2">
            <h3
              class="text-xs font-bold uppercase text-text-on-inverse-muted"
              :style="{ letterSpacing: 'var(--tracking-caps)' }"
            >
              {{ t('home.aside.editions.title') }}
            </h3>
            <NuxtLink
              :to="{ path: localePath('/'), query: { periode: 'a-venir' }, hash: '#editions' }"
              class="text-xs text-text-on-inverse no-underline hover:underline"
            >
              {{ t('home.aside.editions.all') }}
            </NuxtLink>
          </div>
          <div class="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-1">
            <HomeAsideEdition
              v-for="(edition, index) in nextThree"
              :key="edition.id"
              :edition="edition"
              :featured="index === 0"
              :session-count="publishedSessionCount(props.stats, edition.id)"
              :next-session="nextSessionOfEdition(props.sessions, edition.id)"
              :now="props.now"
              :class="index === 0 ? 'sm:col-span-2 lg:col-span-1' : ''"
            />
          </div>
        </section>

        <!-- 2. LA FRISE DES ACTIVITÉS RETENUES -->
        <HomeAsideTimeline :sessions="props.sessions" :editions="props.editions" :now="props.now" />
      </div>
    </div>
  </aside>
</template>

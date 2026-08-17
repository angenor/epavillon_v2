<script setup lang="ts">
import type { Country, TaxonomyTerm } from '~/types/reference'
import type { Membership, Organization, SimilarOrganization } from '~/types/org'
import type {
  CreateOrganizationPayload,
  EmailDomainMatch,
} from '~/types/organization-join'
import { STRONG_MATCH_SCORE } from '~/types/organization-join'
import type { SelectOption } from '~/types/ui'
import type { Uuid } from '~/types/shared'

/**
 * Rattachement à une organisation — `/rattachement-organisation`, `/en/join-organization`.
 *
 * L'ÉCRAN LE PLUS IMPORTANT DU JALON POUR LA QUALITÉ DES DONNÉES. Tout ce qui
 * suit découle d'une seule phrase du cadrage, citée en tête de
 * `040_organizations.sql` : « 2 personnes ont créé deux fois la même
 * organisation […] certains cherchaient par nom complet tandis que d'autres par
 * sigle ». La base pose quatre verrous ; cet écran est celui qui rend le premier
 * — chercher avant de créer — utilisable par quelqu'un de pressé.
 *
 * CINQ DÉCISIONS D'INTERFACE, ET CHACUNE SE PAIE SI ON L'INVERSE :
 *
 *  1. UN SEUL CHAMP. Ni « nom », ni « sigle », ni liste déroulante de pays à
 *     filtrer d'abord. Deux champs, c'est deux façons de chercher, donc deux
 *     populations qui ne trouvent pas la même chose — le défaut de la v1, à la
 *     lettre. Le champ interroge `find_similar_organizations()`, qui regarde
 *     TOUTES les dénominations : nom légal, sigle, traduction, ancien nom, faute
 *     de frappe connue.
 *  2. LA CRÉATION N'APPARAÎT QU'APRÈS UNE RECHERCHE. Un bouton « Créer » visible
 *     d'emblée est plus rapide que n'importe quelle recherche, et c'est
 *     exactement pour cela qu'il fabrique des doublons. Il apparaît quand la
 *     recherche n'a rien donné — ou, en second rang sous la liste, quand aucun
 *     résultat n'est le bon, ce qui est aussi une recherche infructueuse.
 *  3. LE DOMAINE DE L'ADRESSE PARLE AVANT L'UTILISATEUR. Une adresse
 *     @ujfc.org sur un domaine vérifié désigne l'organisation sans ambiguïté :
 *     on la propose en tête, avant même la première frappe.
 *  4. ON N'EMPÊCHE JAMAIS DE CRÉER. Devant une correspondance forte, un écran
 *     intermédiaire montre côte à côte ce qui va être créé et ce qui existe. Il
 *     avertit, il ne refuse pas : un refus fait recommencer avec une autre
 *     orthographe, et le doublon devient alors introuvable.
 *  5. REJOINDRE N'EST PAS ENTRER. Sauf domaine vérifié, l'adhésion naît
 *     `pending` et un référent doit l'accepter. L'écran le dit avant le clic et
 *     après : laisser croire l'affaire réglée, c'est produire la demande en
 *     double la semaine suivante.
 *
 * QUATRE ÉTATS, comme partout : chargement (référentiels), erreur (avec
 * reprise), vide (recherche infructueuse), accès refusé (session perdue en cours
 * de route — le middleware `auth` couvre l'arrivée, pas l'expiration).
 */

definePageMeta({ middleware: 'auth' })
defineI18nRoute({ paths: { fr: '/rattachement-organisation', en: '/join-organization' } })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const api = useApi()
const auth = useAuthStore()
const memberships = useMembershipStore()

useHead(() => ({ title: t('organization.join.title') }))

// ---------------------------------------------------------------------------
// Le contexte de l'écran : référentiels, adhésions, domaine de l'adresse
// ---------------------------------------------------------------------------

interface JoinContext {
  countries: Country[]
  types: TaxonomyTerm[]
  memberships: { membership: Membership; organization: Organization }[]
  domainMatch: EmailDomainMatch | null
}

/**
 * Un seul chargement pour les quatre sources : elles arrivent ensemble ou pas du
 * tout, et l'écran n'a rien de sensé à montrer avec deux sur quatre. `lazy`,
 * pour que le squelette se voie au lieu d'un temps mort avant la page.
 */
const {
  data: context,
  status: contextStatus,
  error: contextError,
  refresh: reloadContext,
} = useAsyncData<JoinContext>(
  'organization-join-context',
  async () => {
    const person = auth.person
    if (!person) return { countries: [], types: [], memberships: [], domainMatch: null }

    const [countries, types, memberships, domainMatch] = await Promise.all([
      api.reference.countries(),
      api.reference.terms('organization_type'),
      api.organizations.membershipsOf(person.id),
      api.organizations.byEmailDomain(person.primary_email),
    ])

    // Une adhésion pointe toujours vers une fiche existante (clé étrangère) ;
    // le filtrage ne couvre que la fenêtre pendant laquelle une fusion vient de
    // déplacer la fiche visée — auquel cas mieux vaut ne rien afficher qu'un
    // encart vide.
    const entries = await Promise.all(
      memberships.map(async (membership) => {
        const organization = await api.organizations.byId(membership.organization_id)
        return organization ? { membership, organization } : null
      }),
    )

    return {
      countries,
      types,
      memberships: entries.filter((entry) => entry !== null),
      domainMatch,
    }
  },
  { lazy: true, watch: [() => auth.person?.id] },
)

const isLoadingContext = computed(
  () => contextStatus.value === 'pending' || contextStatus.value === 'idle',
)

// ---------------------------------------------------------------------------
// Résolution des libellés venus de la BASE — jamais des fichiers i18n
// ---------------------------------------------------------------------------

function countryNameOf(countryId: string | null): string | null {
  if (!countryId) return null
  const country = context.value?.countries.find((c) => c.id === countryId)
  return country ? tr(country.name) : null
}

function typeLabelOf(code: string): string | null {
  const term = context.value?.types.find((x) => x.code === code)
  return term ? tr(term.label) : null
}

const countryOptions = computed<SelectOption[]>(() =>
  (context.value?.countries ?? [])
    .filter((country) => country.is_active)
    .map((country) => ({ value: country.id, label: tr(country.name) }))
    // Trié dans la LANGUE AFFICHÉE : « Bénin » se range où on le cherche.
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
)

const typeOptions = computed<SelectOption[]>(() =>
  (context.value?.types ?? []).map((term) => ({ value: term.code, label: tr(term.label) })),
)

/** Adhésion vivante de la personne pour cette fiche, s'il y en a une. */
function membershipStatusOf(organizationId: string): 'active' | 'pending' | null {
  const entry = context.value?.memberships.find((x) => x.organization.id === organizationId)
  if (!entry) return null
  return entry.membership.status === 'active' ? 'active' : 'pending'
}

/**
 * La suggestion par domaine s'efface dès que la personne est membre de la fiche
 * qu'elle désigne : proposer de rejoindre ce qu'on a rejoint est du bruit.
 */
const domainSuggestion = computed<EmailDomainMatch | null>(() => {
  const match = context.value?.domainMatch ?? null
  if (!match) return null
  return membershipStatusOf(match.organization.id) === null ? match : null
})

// ---------------------------------------------------------------------------
// La recherche
// ---------------------------------------------------------------------------

const query = ref('')
const results = ref<SimilarOrganization[]>([])
const isSearching = ref(false)
const searchError = ref<Error | null>(null)
/** Une recherche a-t-elle abouti au moins une fois ? Commande le bouton de création. */
const hasSearched = ref(false)

/**
 * Chaque recherche porte un numéro et seule la dernière écrit le résultat.
 * Sans cela, une réponse lente à « obs » écraserait la réponse rapide à
 * « observatoire » — et l'écran afficherait des résultats sans rapport avec ce
 * qui est écrit dans le champ.
 */
let searchSequence = 0

async function runSearch(value: string): Promise<void> {
  const term = value.trim()
  query.value = term

  if (term.length < 2) {
    results.value = []
    hasSearched.value = false
    searchError.value = null
    return
  }

  const sequence = ++searchSequence
  isSearching.value = true
  searchError.value = null
  try {
    const found = await api.organizations.similar({
      name: term,
      country_id: auth.person?.country_id ?? null,
      email: auth.person?.primary_email ?? null,
    })
    if (sequence !== searchSequence) return
    /**
     * ON ÉCARTE CE QUI NE RESSEMBLE PAS À CE QUI A ÉTÉ TAPÉ. La fonction du
     * modèle fait aussi remonter les fiches qui partagent le DOMAINE de
     * l'adresse, sans regarder le nom — bonus de 40 points. C'est juste pour le
     * back-office, qui cherche « tout ce qui pourrait être la même entité » ;
     * ça ne l'est pas ici : chercher « Agence spatiale du Sahel » renvoyait
     * l'organisation du domaine de la personne, sans aucun rapport, alors qu'un
     * bandeau la propose déjà nommément au-dessus du champ. Le bonus reste
     * utile — il hisse la bonne fiche quand le nom correspond AUSSI, et la carte
     * le dit. Constaté dans le navigateur, pas déduit du code.
     */
    results.value = found.filter((row) => row.match_reasons.includes('name_similarity'))
    hasSearched.value = true
  } catch (error) {
    if (sequence !== searchSequence) return
    searchError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    if (sequence === searchSequence) isSearching.value = false
  }
}

const strongResults = computed(() => results.value.filter((r) => r.score >= STRONG_MATCH_SCORE))

// ---------------------------------------------------------------------------
// L'écran est-il une ÉTAPE IMPOSÉE, ou une visite volontaire ?
//
// LE RATTACHEMENT N'EST PAS OBLIGATOIRE POUR AVOIR UN COMPTE — décision du
// commanditaire, 17/08. On s'inscrit pour suivre des séances et lire le
// programme sans appartenir à quoi que ce soit. Certaines ACTIONS, elles,
// s'exercent nécessairement au nom d'une organisation : le middleware
// `requires-organization` les garde et renvoie ici, en disant d'où l'on vient
// (`redirect`) et pourquoi (`reason`).
//
// Les deux situations ne se ressemblent pas, et l'écran ne doit pas les
// confondre : la visite volontaire offre de PARTIR sans rien faire, l'étape
// imposée explique ce qu'elle bloque et ramène à l'action une fois réglée.
// ---------------------------------------------------------------------------

const route = useRoute()

/** Destination à reprendre une fois le rattachement obtenu. */
const requiredFor = computed(() => {
  const raw = route.query.redirect
  const value = internalRedirect(raw, '')
  return value.length > 0 ? value : null
})

/** Clé du message expliquant POURQUOI cette étape s'est imposée. */
const requiredReason = computed(() => {
  const raw = route.query.reason
  const value = Array.isArray(raw) ? raw[0] : raw
  return typeof value === 'string' && /^[a-z][a-z0-9-]*$/.test(value) ? value : null
})

/**
 * Le message d'exigence, avec repli sur une formulation générique quand la page
 * appelante n'a pas déclaré sa raison. `te()` — et non un `try` — parce qu'une
 * clé absente doit produire le repli, jamais la clé brute à l'écran.
 */
const { te } = useI18n()
const requiredMessage = computed(() => {
  const key = `organization.join.required.reasons.${requiredReason.value}`
  return requiredReason.value !== null && te(key) ? t(key) : t('organization.join.required.generic')
})

// ---------------------------------------------------------------------------
// Les trois moments de l'écran
// ---------------------------------------------------------------------------

type Step = 'search' | 'create' | 'duplicate'
const step = ref<Step>('search')

/** Ce qui s'est passé, une fois l'écriture faite. L'écran ne montre plus que cela. */
const outcome = ref<
  | { kind: 'joined' | 'pending'; organization: Organization }
  | { kind: 'created'; organization: Organization }
  | null
>(null)

const actionError = ref<Error | null>(null)
/** Message d'information non bloquant — « vous êtes déjà membre », par exemple. */
const notice = ref<string | null>(null)

// ---------------------------------------------------------------------------
// Rejoindre
// ---------------------------------------------------------------------------

const joinTarget = ref<{ id: Uuid; name: string; immediate: boolean } | null>(null)
const isJoinDialogOpen = ref(false)
const joiningId = ref<Uuid | null>(null)

function askToJoin(target: { id: Uuid; name: string; immediate: boolean }): void {
  actionError.value = null
  notice.value = null
  joinTarget.value = target
  isJoinDialogOpen.value = true
}

function askToJoinMatch(match: SimilarOrganization): void {
  // Une fiche trouvée par la recherche n'ouvre un rattachement immédiat que si
  // c'est aussi celle que désigne le domaine de l'adresse.
  const domain = context.value?.domainMatch ?? null
  askToJoin({
    id: match.organization_id,
    name: match.legal_name,
    immediate: domain !== null && domain.organization.id === match.organization_id && domain.can_auto_join,
  })
}

function askToJoinDomain(match: EmailDomainMatch): void {
  askToJoin({
    id: match.organization.id,
    name: match.organization.legal_name,
    immediate: match.can_auto_join,
  })
}

async function confirmJoin(jobTitle: string | null): Promise<void> {
  const target = joinTarget.value
  const person = auth.person
  if (!target || !person) return

  joiningId.value = target.id
  actionError.value = null
  try {
    const result = await api.organizations.join(person.id, {
      organization_id: target.id,
      job_title: jobTitle,
    })
    isJoinDialogOpen.value = false

    if (result.status === 'already_member') {
      notice.value = t(
        result.membership_status === 'active'
          ? 'organization.join.notices.alreadyMember'
          : 'organization.join.notices.alreadyRequested',
        { organization: result.organization.legal_name },
      )
      await reloadContext()
      return
    }

    if (result.status === 'joined') auth.attachOrganization(result.organization.id)
    outcome.value = { kind: result.status, organization: result.organization }
    await reloadContext()
    // La garde lit le store, pas cette page : sans ce rafraîchissement, elle
    // renverrait ici une seconde fois, juste après un rattachement réussi.
    await memberships.refresh()
  } catch (error) {
    actionError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    joiningId.value = null
  }
}

// ---------------------------------------------------------------------------
// Créer
// ---------------------------------------------------------------------------

/** Correspondances trouvées PENDANT la saisie du formulaire de création. */
const probeResults = ref<SimilarOrganization[]>([])
const isProbing = ref(false)
let probeSequence = 0

async function probe(input: { name: string; website: string; countryId: string }): Promise<void> {
  if (input.name.trim().length < 2) {
    probeResults.value = []
    return
  }
  const sequence = ++probeSequence
  isProbing.value = true
  try {
    const found = await api.organizations.similar({
      name: input.name,
      country_id: input.countryId || auth.person?.country_id || null,
      website: input.website || null,
      email: auth.person?.primary_email ?? null,
    })
    if (sequence === probeSequence) probeResults.value = found
  } finally {
    if (sequence === probeSequence) isProbing.value = false
  }
}

const probeStrongMatches = computed(() =>
  probeResults.value.filter((r) => r.score >= STRONG_MATCH_SCORE),
)

/** Le dossier en cours de création, retenu pendant l'écran d'avertissement. */
const draft = ref<CreateOrganizationPayload | null>(null)
const isCreating = ref(false)

function startCreation(): void {
  actionError.value = null
  notice.value = null
  // La recherche déjà faite amorce le formulaire : ce que la personne a tapé est
  // très probablement le nom qu'elle s'apprête à saisir.
  probeResults.value = results.value
  step.value = 'create'
}

async function submitCreation(payload: Omit<CreateOrganizationPayload, 'acknowledged_match_ids'>): Promise<void> {
  const acknowledged = probeStrongMatches.value.map((m) => m.organization_id)
  draft.value = { ...payload, acknowledged_match_ids: acknowledged }

  // Une correspondance FORTE ouvre l'écran intermédiaire. Rien n'est refusé :
  // il s'agit de montrer, une dernière fois, ce qui existe déjà.
  if (acknowledged.length > 0) {
    step.value = 'duplicate'
    return
  }
  await sendCreation()
}

async function sendCreation(): Promise<void> {
  const payload = draft.value
  const person = auth.person
  if (!payload || !person) return

  isCreating.value = true
  actionError.value = null
  try {
    const result = await api.organizations.create(person.id, payload)

    if (result.status === 'name_taken') {
      // Le doublon EXACT que la base refuse (`ux_organizations_name_country`).
      // On ne s'arrête pas sur une erreur : on montre la fiche en cause, avec de
      // quoi la rejoindre — c'est la seule suite utile.
      notice.value = t('organization.join.notices.nameTaken')
      probeResults.value = [result.existing]
      step.value = 'duplicate'
      return
    }

    auth.attachOrganization(result.organization.id)
    outcome.value = { kind: 'created', organization: result.organization }
    await reloadContext()
    await memberships.refresh()
  } catch (error) {
    actionError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isCreating.value = false
  }
}

function backToSearch(): void {
  step.value = 'search'
  draft.value = null
  actionError.value = null
}

/**
 * Où mène « Continuer » une fois l'affaire réglée.
 *
 * On reprend l'action interrompue — et SEULEMENT si elle est de nouveau
 * possible. Une demande restée `pending` ne donne aucun droit : y renvoyer
 * ferait rejouer la garde, qui ramènerait ici, et la personne tournerait en
 * rond sans comprendre. Dans ce cas on rentre à l'accueil, l'écran ayant déjà
 * expliqué ce qu'on attend.
 */
const continueTo = computed(() => {
  const target = requiredFor.value
  if (target !== null && outcome.value?.kind !== 'pending') return target
  return localePath('/')
})
</script>

<template>
  <div class="mx-auto w-full max-w-205">
    <!-- ACCÈS REFUSÉ — la session a disparu après l'arrivée sur la page. Le
         middleware `auth` couvre l'entrée, pas l'expiration. -->
    <UiForbiddenState
      v-if="auth.isResolved && !auth.isAuthenticated"
      :title="t('common.states.forbidden.title')"
      :description="t('organization.join.states.signedOut')"
      :action-label="t('organization.join.states.signIn')"
      :action-to="localePath('/auth/login')"
    />

    <UiErrorState
      v-else-if="contextError"
      :title="t('common.states.error.title')"
      :description="t('organization.join.states.contextError')"
      :detail="contextError.message"
      :retry-label="t('common.actions.retry')"
      @retry="reloadContext()"
    />

    <UiLoadingState
      v-else-if="isLoadingContext"
      variant="form"
      :lines="4"
      :label="t('common.states.loading.label')"
    />

    <!-- CE QUI S'EST PASSÉ. Une fois l'adhésion demandée ou la fiche créée,
         l'écran ne montre plus la recherche : il dit où l'on en est et ce qui
         reste à attendre. -->
    <section v-else-if="outcome" class="grid gap-5">
      <UiAlert
        :intent="outcome.kind === 'pending' ? 'warning' : 'success'"
        live
        :title="t(`organization.join.outcome.${outcome.kind}.title`, { organization: outcome.organization.legal_name })"
        :message="t(`organization.join.outcome.${outcome.kind}.description`)"
      />

      <div class="flex flex-col gap-3 sm:flex-row">
        <UiButton
          variant="primary"
          size="lg"
          :to="continueTo"
          :label="requiredFor !== null && outcome.kind !== 'pending'
            ? t('organization.join.outcome.resume')
            : t('organization.join.outcome.continue')"
        />
        <UiButton
          variant="ghost"
          :label="t('organization.join.outcome.joinAnother')"
          @click="outcome = null; backToSearch()"
        />
      </div>
    </section>

    <div v-else class="grid gap-6">
      <header>
        <h1 class="font-display text-2xl leading-tight font-bold text-text sm:text-3xl">
          {{ t('organization.join.title') }}
        </h1>
        <p class="mt-2 max-w-(--measure) text-text-muted">
          {{ requiredFor !== null ? t('organization.join.required.description') : t('organization.join.description') }}
        </p>
      </header>

      <!-- ÉTAPE IMPOSÉE — on arrive ici parce qu'une action l'exigeait. Le
           bandeau dit LAQUELLE : « une organisation est nécessaire » sans dire
           pour quoi faire se lit comme une tracasserie administrative. -->
      <UiAlert
        v-if="requiredFor !== null && step === 'search'"
        intent="info"
        :title="t('organization.join.required.title')"
        :message="requiredMessage"
      />

      <UiAlert
        v-if="actionError"
        intent="danger"
        live
        :title="t('validation.server.generic')"
        :message="actionError.message"
      />
      <UiAlert v-if="notice" intent="info" live :message="notice" dismissible @dismiss="notice = null" />

      <OrganizationMembershipSummary
        v-if="context && step === 'search'"
        :entries="context.memberships"
        :country-name-of="countryNameOf"
      />

      <!-- MOMENT 1 — la recherche. -->
      <template v-if="step === 'search'">
        <OrganizationDomainSuggestion
          v-if="domainSuggestion"
          :suggestion="domainSuggestion"
          :country-name="countryNameOf(domainSuggestion.organization.country_id)"
          :busy="joiningId === domainSuggestion.organization.id"
          @join="askToJoinDomain"
        />

        <section class="grid gap-4">
          <UiSearchInput
            :model-value="query"
            :label="t('organization.join.search.label')"
            :hint="t('organization.join.search.hint')"
            :placeholder="t('organization.join.search.placeholder')"
            :loading="isSearching"
            :result-count="hasSearched ? results.length : null"
            size="lg"
            @search="runSearch"
          />

          <UiErrorState
            v-if="searchError"
            :title="t('common.states.error.title')"
            :description="t('organization.join.states.searchError')"
            :detail="searchError.message"
            :retry-label="t('common.actions.retry')"
            @retry="runSearch(query)"
          />

          <!-- VIDE — la recherche n'a rien ramené. C'est le SEUL endroit où la
               création s'offre en premier plan, et elle reste secondaire. -->
          <UiEmptyState
            v-else-if="hasSearched && results.length === 0"
            filtered
            icon="building"
            :title="t('organization.join.empty.title', { query })"
            :description="t('organization.join.empty.description')"
          >
            <template #actions>
              <UiButton
                variant="secondary"
                icon="plus"
                :label="t('organization.join.empty.create')"
                @click="startCreation()"
              />
            </template>
          </UiEmptyState>

          <template v-else-if="results.length > 0">
            <p class="text-sm text-text-muted">
              {{ t('organization.join.results.count', results.length) }}
            </p>

            <OrganizationMatchCard
              v-for="match in results"
              :key="match.organization_id"
              :match="match"
              :country-name="countryNameOf(match.country_id)"
              :type-label="typeLabelOf(match.organization_type_code)"
              :membership="membershipStatusOf(match.organization_id)"
              :busy="joiningId === match.organization_id"
              @join="askToJoinMatch"
            />

            <!-- Aucune de ces fiches n'est la bonne : c'est aussi une recherche
                 infructueuse, et la création s'offre alors, en second rang. -->
            <p class="border-t border-border pt-4 text-sm text-text-muted">
              {{ t('organization.join.results.noneMine') }}
              <UiButton
                class="mt-2 sm:mt-0 sm:ml-2"
                variant="ghost"
                size="sm"
                icon="plus"
                :label="t('organization.join.empty.create')"
                @click="startCreation()"
              />
            </p>
          </template>

          <!-- Avant toute frappe : ce que l'écran attend, et pourquoi. Aucune
               liste d'organisations n'est proposée — le référentiel compte des
               centaines de fiches, et parcourir n'est pas chercher. -->
          <p v-else class="rounded-md bg-surface-sunken px-4 py-3 text-sm text-text-muted">
            {{ t('organization.join.search.idle') }}
          </p>
        </section>

        <!-- LA PORTE DE SORTIE, et elle est explicite.
             Se rattacher à une organisation n'est PAS obligatoire pour avoir un
             compte : on s'inscrit pour suivre des séances sans appartenir à
             quoi que ce soit. Un écran d'accueil qu'on ne peut pas quitter
             enseignerait le contraire — et pousserait à créer une fiche bidon
             pour en sortir, ce qui est précisément ce que tout cet écran essaie
             d'éviter. Le libellé change selon qu'on est passé ici de son plein
             gré ou qu'une action l'exigeait : dans le second cas, partir signifie
             renoncer à cette action, et il faut le dire. -->
        <section class="border-t border-border pt-5">
          <p class="text-sm text-text-muted">
            {{ requiredFor !== null ? t('organization.join.skip.required') : t('organization.join.skip.description') }}
          </p>
          <UiButton
            class="mt-3"
            variant="ghost"
            :to="localePath('/')"
            :label="requiredFor !== null ? t('organization.join.skip.giveUp') : t('organization.join.skip.action')"
          />
        </section>
      </template>

      <!-- MOMENT 2 — la création, qui continue de chercher pendant la saisie. -->
      <OrganizationCreateForm
        v-else-if="step === 'create'"
        :matches="probeResults"
        :strong-matches="probeStrongMatches"
        :country-options="countryOptions"
        :type-options="typeOptions"
        :searching="isProbing"
        :submitting="isCreating"
        :busy-join-id="joiningId"
        :country-name-of="countryNameOf"
        :type-label-of="typeLabelOf"
        :default-country-id="auth.person?.country_id ?? null"
        @probe="probe"
        @submit="submitCreation"
        @join="askToJoinMatch"
        @cancel="backToSearch()"
      />

      <!-- MOMENT 3 — l'avertissement avant création. -->
      <OrganizationDuplicateWarning
        v-else-if="step === 'duplicate' && draft"
        :draft="draft"
        :matches="probeStrongMatches.length > 0 ? probeStrongMatches : probeResults"
        :draft-country-name="countryNameOf(draft.country_id)"
        :draft-type-label="typeLabelOf(draft.organization_type_code)"
        :country-name-of="countryNameOf"
        :type-label-of="typeLabelOf"
        :busy-join-id="joiningId"
        :creating="isCreating"
        @join="askToJoinMatch"
        @create-anyway="sendCreation()"
        @back="step = 'create'"
      />
    </div>

    <OrganizationJoinDialog
      v-if="joinTarget"
      v-model:open="isJoinDialogOpen"
      :organization-name="joinTarget.name"
      :immediate="joinTarget.immediate"
      :submitting="joiningId !== null"
      :default-job-title="auth.person?.job_title ?? null"
      @confirm="confirmJoin"
    />
  </div>
</template>

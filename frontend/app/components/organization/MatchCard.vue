<script setup lang="ts">
import type { SimilarOrganization } from '~/types/org'
import { STRONG_MATCH_SCORE } from '~/types/organization-join'

/**
 * Un résultat de recherche d'organisation.
 *
 * CETTE CARTE DOIT PERMETTRE DE DIRE « C'EST BIEN LA MIENNE », et rien d'autre.
 * Chaque élément qu'elle porte est là pour cela, et la v1 ne montrait aucun des
 * quatre : la VILLE et le PAYS distinguent deux homonymes, le NOMBRE DE MEMBRES
 * dit qu'on n'est pas le premier à s'être posé la question, le SCEAU dit que
 * l'IFDD a regardé la fiche, et la DÉNOMINATION TROUVÉE explique pourquoi ce
 * résultat apparaît quand on a tapé autre chose que le nom affiché — « OSED »
 * ramène « Observatoire du Sahel pour l'énergie durable », et il faut le dire.
 *
 * Le score n'est JAMAIS affiché tel quel. « 125,0 » ne veut rien dire pour qui
 * dépose un dossier ; ce qu'il faut savoir tient dans une pastille — cette fiche
 * est très probablement la vôtre — et le seuil vient du modèle, pas d'ici.
 */

interface Props {
  match: SimilarOrganization
  /** Pays résolu par l'appelant depuis `reference.countries`. */
  countryName?: string | null
  /** Type d'organisation résolu depuis `reference.taxonomy_terms`. */
  typeLabel?: string | null
  /** La personne est-elle déjà membre, ou sa demande est-elle déposée ? */
  membership?: 'active' | 'pending' | null
  /** Une demande est en cours d'envoi pour CETTE fiche. */
  busy?: boolean
}

const props = withDefaults(defineProps<Props>(), { membership: null })
const emit = defineEmits<{ join: [match: SimilarOrganization] }>()

const { t } = useI18n()

const isStrong = computed(() => props.match.score >= STRONG_MATCH_SCORE)

/**
 * La dénomination qui a produit la correspondance ne se montre que si elle
 * APPREND quelque chose : ni le nom légal déjà affiché, ni le sigle déjà à côté.
 * Sinon la carte répète le titre sous le titre.
 */
const foundUnder = computed(() => {
  const name = props.match.matched_name
  if (!name) return null
  if (name === props.match.legal_name || name === props.match.acronym) return null
  return name
})

const location = computed(() =>
  [props.match.city, props.countryName].filter(Boolean).join(' · '),
)
</script>

<template>
  <UiCard :selected="isStrong">
    <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
      <div class="min-w-0">
        <div class="flex flex-wrap items-center gap-2">
          <h3 class="font-display text-base leading-snug font-bold text-text">
            {{ props.match.legal_name }}
          </h3>
          <UiBadge v-if="props.match.acronym" size="sm" :label="props.match.acronym" />
          <UiBadge
            v-if="props.match.verified_at"
            size="sm"
            intent="success"
            icon="check"
            :label="t('organization.join.results.verified')"
          />
        </div>

        <p v-if="location" class="mt-1 text-sm text-text-muted">
          {{ location }}<span v-if="props.typeLabel"> — {{ props.typeLabel }}</span>
        </p>
        <p v-else-if="props.typeLabel" class="mt-1 text-sm text-text-muted">{{ props.typeLabel }}</p>

        <p class="mt-2 flex items-center gap-1.5 text-sm text-text-secondary">
          <UiIcon name="users" size="1rem" class="shrink-0 text-text-subtle" />
          {{ t('organization.join.results.members', props.match.member_count) }}
        </p>

        <p v-if="foundUnder" class="mt-2 flex items-start gap-1.5 text-xs text-text-muted">
          <UiIcon name="search" size="0.9rem" class="mt-0.5 shrink-0" />
          <span>{{ t('organization.join.results.foundUnder', { name: foundUnder }) }}</span>
        </p>

        <!-- Le domaine partagé est le signal le plus fiable du modèle : il mérite
             d'être nommé, là où « pays identique » n'apprendrait rien. -->
        <p
          v-if="props.match.match_reasons.includes('shared_domain')"
          class="mt-2 flex items-start gap-1.5 text-xs text-info"
        >
          <UiIcon name="mail" size="0.9rem" class="mt-0.5 shrink-0" />
          <span>{{ t('organization.join.results.sharedDomain') }}</span>
        </p>
      </div>

      <div class="flex shrink-0 flex-col items-start gap-2 sm:items-end">
        <!-- Pastille en CONTOUR, pas en aplat : l'aplat cyan est celui du bouton
             « Rejoindre », juste dessous. Deux aplats de la même couleur à trois
             centimètres l'un de l'autre se disputent le regard, et c'est
             l'ACTION qui doit le gagner. Vu à l'écran, pas prévu. -->
        <UiBadge
          v-if="isStrong"
          size="sm"
          intent="info"
          :label="t('organization.join.results.strongMatch')"
        />

        <UiBadge
          v-if="props.membership === 'active'"
          size="sm"
          intent="success"
          :label="t('organization.join.results.alreadyMember')"
        />
        <UiBadge
          v-else-if="props.membership === 'pending'"
          size="sm"
          intent="warning"
          :label="t('organization.join.results.requestPending')"
        />
        <UiButton
          v-else
          variant="primary"
          :loading="props.busy"
          :label="t('organization.join.results.join')"
          @click="emit('join', props.match)"
        />
      </div>
    </div>
  </UiCard>
</template>

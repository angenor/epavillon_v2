<script setup lang="ts">
import type { EmailDomainMatch } from '~/types/organization-join'

/**
 * « Votre adresse appartient à cette organisation. »
 *
 * LE SIGNAL LE PLUS FIABLE DU MODÈLE, et le seul rattachement qui puisse être
 * IMMÉDIAT. `040_organizations.sql` § 3 : « un agent qui s'inscrit avec une
 * adresse @ifdd.francophonie.org rejoint l'organisation sans intervention
 * humaine ». Deux conditions, toutes deux portées par la base : le domaine est
 * vérifié (`verified_at`) et l'organisation a demandé ce rattachement
 * (`auto_join`). Une seule des deux ne suffit pas — d'où `can_auto_join`.
 *
 * QUAND LE DOMAINE EST CONNU MAIS SANS RATTACHEMENT AUTOMATIQUE, on le dit quand
 * même : la fiche est presque certainement la bonne, et la demande passera par un
 * référent comme n'importe quelle autre. Ce qui change est le VERBE du bouton,
 * pas la proposition — promettre un rattachement immédiat qui n'arrive pas est
 * pire que de ne rien promettre.
 *
 * Ce bloc n'apparaît QUE tant que la personne n'est pas déjà membre : proposer
 * de rejoindre ce qu'on a déjà rejoint est un bruit, pas un service.
 */

interface Props {
  suggestion: EmailDomainMatch
  countryName?: string | null
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ join: [suggestion: EmailDomainMatch] }>()

const { t } = useI18n()
</script>

<template>
  <section
    class="rounded-lg border border-info-border bg-info-surface p-4 sm:p-5"
    :aria-label="t('organization.join.domain.title')"
  >
    <div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
      <div class="min-w-0">
        <h2 class="flex items-center gap-2 font-display text-base font-bold text-text">
          <UiIcon name="mail" size="1.1rem" class="shrink-0 text-info" />
          {{ t('organization.join.domain.title') }}
        </h2>

        <p class="mt-2 text-sm text-text">
          <i18n-t keypath="organization.join.domain.description" tag="span" scope="global">
            <template #domain>
              <span class="font-mono text-sm">@{{ props.suggestion.domain }}</span>
            </template>
            <template #organization>
              <strong>{{ props.suggestion.organization.legal_name }}</strong>
            </template>
          </i18n-t>
        </p>

        <p class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-text-muted">
          <span v-if="props.countryName" class="flex items-center gap-1.5">
            <UiIcon name="globe" size="1rem" class="shrink-0" />
            {{ props.countryName }}
          </span>
          <span class="flex items-center gap-1.5">
            <UiIcon name="users" size="1rem" class="shrink-0" />
            {{ t('organization.join.results.members', props.suggestion.member_count) }}
          </span>
        </p>

        <p class="mt-3 text-xs text-text-muted">
          {{
            props.suggestion.can_auto_join
              ? t('organization.join.domain.immediate')
              : t('organization.join.domain.approval')
          }}
        </p>
      </div>

      <UiButton
        class="shrink-0"
        variant="primary"
        icon="check"
        :loading="props.busy"
        :label="
          props.suggestion.can_auto_join
            ? t('organization.join.domain.actionImmediate')
            : t('organization.join.domain.actionRequest')
        "
        @click="emit('join', props.suggestion)"
      />
    </div>
  </section>
</template>

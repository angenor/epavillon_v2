<script setup lang="ts">
import type { Membership, Organization } from '~/types/org'

/**
 * Les rattachements que la personne a déjà.
 *
 * TROIS RAISONS DE L'AFFICHER EN TÊTE D'ÉCRAN, ET AUCUNE N'EST DÉCORATIVE.
 * D'abord, l'écran est accessible depuis le profil : y arriver sans savoir où
 * l'on en est n'aurait aucun sens. Ensuite, une demande EN ATTENTE explique à
 * elle seule pourquoi rien ne se passe — sans elle, la personne redemande, et
 * c'est le référent qui reçoit deux fois la même chose. Enfin, `org.memberships`
 * autorise PLUSIEURS adhésions : quelqu'un peut légitimement appartenir à un
 * ministère et à une association, et l'écran ne doit pas laisser croire qu'un
 * second rattachement effacerait le premier.
 *
 * La primauté (`is_primary`) est signalée mais pas modifiable ici : elle est
 * tenue par la base — première adhésion active, `tg_memberships_default_primary`
 * — et se change depuis le profil, avec les autres préférences.
 */

interface Props {
  entries: { membership: Membership; organization: Organization }[]
  countryNameOf: (countryId: string | null) => string | null
}

const props = defineProps<Props>()

const { t } = useI18n()
</script>

<template>
  <section v-if="props.entries.length > 0" class="grid gap-3">
    <h2 class="font-display text-sm tracking-wide text-text-subtle uppercase">
      {{ t('organization.join.current.title') }}
    </h2>

    <UiCard v-for="entry in props.entries" :key="entry.membership.id" sunken>
      <div class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <h3 class="font-display text-base font-bold text-text">
              {{ entry.organization.legal_name }}
            </h3>
            <UiBadge v-if="entry.organization.acronym" size="sm" :label="entry.organization.acronym" />
          </div>
          <p class="mt-1 text-sm text-text-muted">
            <span v-if="entry.membership.job_title">{{ entry.membership.job_title }} — </span>
            {{ t(`organization.join.current.roles.${entry.membership.role}`) }}
            <span v-if="props.countryNameOf(entry.organization.country_id)">
              · {{ props.countryNameOf(entry.organization.country_id) }}
            </span>
          </p>
        </div>

        <div class="flex shrink-0 flex-wrap items-center gap-2">
          <UiBadge
            v-if="entry.membership.is_primary && entry.membership.status === 'active'"
            size="sm"
            :label="t('organization.join.current.primary')"
          />
          <UiBadge
            v-if="entry.membership.status === 'active'"
            size="sm"
            intent="success"
            icon="check"
            :label="t('organization.join.current.active')"
          />
          <UiBadge
            v-else
            size="sm"
            intent="warning"
            icon="clock"
            :label="t('organization.join.current.pending')"
          />
        </div>
      </div>

      <p v-if="entry.membership.status === 'pending'" class="mt-3 text-xs text-text-muted">
        {{ t('organization.join.current.pendingHint') }}
      </p>
    </UiCard>
  </section>
</template>

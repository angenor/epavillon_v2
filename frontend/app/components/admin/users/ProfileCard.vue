<script setup lang="ts">
import type { UserDetail } from '~/types/admin-users'
import type { TimeZoneName } from '~/types/shared'

/**
 * LA FICHE D'IDENTITÉ ET LES MOYENS DE CONNEXION.
 *
 * DEUX BLOCS, PARCE QUE CE SONT DEUX TABLES ET DEUX RÉALITÉS. `identity.people`
 * décrit une PERSONNE — elle existe sans compte, invitée ou saisie comme
 * intervenante ; `identity.accounts` décrit les MOYENS DE SE CONNECTER, qui
 * peuvent être plusieurs. La v1 confondait les deux dans `public.users` et l'a
 * payé trois fois (voir l'en-tête de `030_identity.sql`) : un écran qui les
 * fondrait à nouveau ferait renaître la confusion.
 *
 * AUCUN SECRET N'EST AFFICHÉ, ET IL N'Y EN A PAS À AFFICHER : le type `Account`
 * ne déclare ni empreinte de mot de passe ni secret de second facteur — ils ne
 * franchissent jamais la frontière de l'API. Ce qui se lit ici sont des DATES,
 * qui disent l'état du compte sans rien en révéler.
 *
 * LE STATUT PORTE SON MOTIF ET SON AUTEUR. Une porte fermée sans explication est
 * la première chose qu'un administrateur cherche, et la dernière qu'on pense à
 * afficher.
 */

interface Props {
  user: UserDetail
  timezone: TimeZoneName
}

defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()

const STATUS_INTENT = {
  active: 'success',
  suspended: 'warning',
  blocked: 'danger',
  anonymized: 'neutral',
} as const
</script>

<template>
  <div class="grid gap-6 lg:grid-cols-2">
    <!-- LA PERSONNE -->
    <UiCard :title="t('admin.user.detail.identity.title')">
      <dl class="space-y-3 text-sm">
        <div>
          <dt class="text-text-muted">{{ t('admin.user.detail.identity.email') }}</dt>
          <dd class="flex flex-wrap items-center gap-2">
            <a :href="`mailto:${user.primary_email}`" class="break-all">{{ user.primary_email }}</a>
            <UiBadge
              :intent="user.email_verified_at ? 'success' : 'warning'"
              size="sm"
              :label="
                user.email_verified_at
                  ? t('admin.user.detail.identity.verified')
                  : t('admin.user.detail.identity.unverified')
              "
            />
          </dd>
        </div>

        <div v-if="user.other_emails.length">
          <dt class="text-text-muted">{{ t('admin.user.detail.identity.otherEmails') }}</dt>
          <dd>
            <ul>
              <li v-for="entry in user.other_emails" :key="entry.email" class="break-all">
                {{ entry.email }}
              </li>
            </ul>
          </dd>
        </div>

        <div v-if="user.job_title">
          <dt class="text-text-muted">{{ t('admin.user.detail.identity.jobTitle') }}</dt>
          <dd>{{ user.job_title }}</dd>
        </div>

        <div>
          <dt class="text-text-muted">{{ t('admin.user.detail.identity.organization') }}</dt>
          <dd>
            <NuxtLink
              v-if="user.organization_id"
              :to="useLocalePath()(`/admin/organisations/${user.organization_id}`)"
            >
              {{ user.organization_name }}
            </NuxtLink>
            <span v-else class="text-text-subtle">{{ t('admin.user.detail.identity.noOrganization') }}</span>
          </dd>
        </div>

        <div>
          <dt class="text-text-muted">{{ t('admin.user.detail.identity.location') }}</dt>
          <dd>
            {{ [user.city, user.country_name ? tr(user.country_name) : null].filter(Boolean).join(', ') || '—' }}
          </dd>
        </div>

        <div>
          <dt class="text-text-muted">{{ t('admin.user.detail.identity.preferences') }}</dt>
          <dd>
            {{ t('admin.user.detail.identity.locale', { locale: user.preferred_locale.toUpperCase() }) }}
            · {{ user.timezone }}
          </dd>
        </div>

        <div>
          <dt class="text-text-muted">{{ t('admin.user.detail.identity.directory') }}</dt>
          <dd>
            {{
              user.is_directory_visible
                ? t('admin.user.detail.identity.directoryVisible')
                : t('admin.user.detail.identity.directoryHidden')
            }}
          </dd>
        </div>

        <div>
          <dt class="text-text-muted">{{ t('admin.user.detail.identity.created') }}</dt>
          <dd>{{ date(user.created_at, timezone) }}</dd>
        </div>
      </dl>
    </UiCard>

    <div class="space-y-6">
      <!-- LE STATUT, AVEC SON MOTIF -->
      <UiCard :title="t('admin.user.detail.status.title')">
        <div class="space-y-3 text-sm">
          <UiBadge :intent="STATUS_INTENT[user.status]" :label="t(`admin.user.status.${user.status}`)" />

          <p v-if="user.status_reason" class="max-w-(--measure)">« {{ user.status_reason }} »</p>

          <p v-if="user.suspended_until" class="text-warning">
            {{ t('admin.user.detail.status.until', { date: date(user.suspended_until, timezone) }) }}
          </p>

          <p v-if="user.status_changed_at" class="text-text-muted">
            {{ dateTime(user.status_changed_at, timezone) }}
            <template v-if="user.status_changed_by_name">
              · {{ t('admin.user.detail.status.by', { name: user.status_changed_by_name }) }}
            </template>
          </p>
        </div>
      </UiCard>

      <!-- LES COMPTES — des dates, jamais un secret -->
      <UiCard :title="t('admin.user.detail.accounts.title')">
        <UiEmptyState
          v-if="user.accounts.length === 0"
          compact
          icon="lock"
          :title="t('admin.user.detail.accounts.empty.title')"
          :description="t('admin.user.detail.accounts.empty.description')"
        />

        <ul v-else class="space-y-3 text-sm">
          <li
            v-for="account in user.accounts"
            :key="account.id"
            class="rounded-md border border-border p-3"
          >
            <div class="flex flex-wrap items-center gap-2">
              <span class="font-medium">
                {{ t(`admin.user.detail.accounts.provider.${account.provider}`) }}
              </span>
              <UiBadge
                v-if="account.mfa_enabled_at"
                intent="success"
                size="sm"
                icon="shield-check"
                :label="t('admin.user.detail.accounts.mfa')"
              />
              <UiBadge
                v-if="account.locked_until"
                intent="warning"
                size="sm"
                icon="lock"
                :label="t('admin.user.detail.accounts.locked', { date: dateTime(account.locked_until, timezone) })"
              />
            </div>

            <dl class="mt-2 space-y-1 text-text-muted">
              <div class="flex flex-wrap gap-x-2">
                <dt>{{ t('admin.user.detail.accounts.lastLogin') }}</dt>
                <dd>
                  {{
                    account.last_login_at
                      ? dateTime(account.last_login_at, timezone)
                      : t('admin.user.detail.accounts.never')
                  }}
                </dd>
              </div>
              <div v-if="account.password_changed_at" class="flex flex-wrap gap-x-2">
                <dt>{{ t('admin.user.detail.accounts.passwordChanged') }}</dt>
                <dd>{{ date(account.password_changed_at, timezone) }}</dd>
              </div>
              <div v-if="account.failed_attempts > 0" class="flex flex-wrap gap-x-2 text-warning">
                <dt>{{ t('admin.user.detail.accounts.failedAttempts') }}</dt>
                <dd>{{ account.failed_attempts }}</dd>
              </div>
            </dl>
          </li>
        </ul>
      </UiCard>

      <!-- CONSENTEMENTS — la preuve que le RGPD exige -->
      <UiCard v-if="user.consents.length" :title="t('admin.user.detail.consents.title')">
        <ul class="space-y-2 text-sm">
          <li
            v-for="consent in user.consents"
            :key="consent.purpose"
            class="flex flex-wrap items-center justify-between gap-2"
          >
            <span>{{ t(`admin.user.privacy.purpose.${consent.purpose}`) }}</span>
            <span class="flex items-center gap-2">
              <UiBadge
                :intent="consent.is_granted ? 'success' : 'neutral'"
                size="sm"
                :label="
                  consent.is_granted
                    ? t('admin.user.detail.consents.granted')
                    : t('admin.user.detail.consents.refused')
                "
              />
              <span class="text-text-muted">{{ date(consent.recorded_at, timezone) }}</span>
            </span>
          </li>
        </ul>
      </UiCard>
    </div>
  </div>
</template>

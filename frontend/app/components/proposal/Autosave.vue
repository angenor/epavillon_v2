<script setup lang="ts">
import type { DraftSaveState } from '~/composables/useProposalDraft'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'ÉTAT DE L'ENREGISTREMENT AUTOMATIQUE, écrit en toutes lettres.
 *
 * UN POINT VERT NE SUFFIT PAS. La question que se pose quelqu'un qui remplit un
 * dossier depuis vingt minutes est « puis-je fermer cet onglet ? » ; elle appelle
 * une phrase, pas une pastille. On écrit donc l'état ET l'heure.
 *
 * L'HEURE PORTE SON FUSEAU, comme toute heure de la plateforme. Ici ce n'est pas
 * celui de l'édition mais celui de la PERSONNE (`identity.people.timezone`) :
 * « enregistré à 14:32 » répond à « qu'ai-je fait il y a cinq minutes », et cette
 * question se pose depuis Dakar, pas depuis Belém. C'est le seul horodatage de
 * la plateforme dont le fuseau de référence soit celui du lecteur — d'où la
 * mention explicite, qui interdit de le confondre avec un horaire d'activité.
 *
 * L'ÉCHEC EST ACTIONNABLE. « Échec de l'enregistrement » sans bouton laisse la
 * personne devant un choix qu'elle ne peut pas faire : recopier son texte
 * ailleurs, ou espérer. Le bouton « Réessayer » relance la même écriture.
 */

interface Props {
  state: DraftSaveState
  /** Instant d'enregistrement tel que le SERVEUR l'a daté. */
  savedAt: string | null
  /** Numéro de dossier, connu dès le premier enregistrement. */
  referenceCode: string | null
  timezone: TimeZoneName
  /** Libellé du fuseau, quand on en a un plus lisible que l'identifiant IANA. */
  zoneLabel?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ retry: [] }>()

const { t } = useI18n()
const { timeWithZone } = useDateTime()

const savedLabel = computed(() =>
  props.savedAt ? timeWithZone(props.savedAt, props.timezone, props.zoneLabel) : '',
)
</script>

<template>
  <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
    <p
      class="flex items-center gap-2"
      :class="props.state === 'error' ? 'text-danger' : 'text-text-muted'"
      role="status"
    >
      <UiSpinner v-if="props.state === 'saving'" size="0.9rem" />
      <UiIcon v-else-if="props.state === 'saved'" name="check-circle" size="1rem" class="text-success" />
      <UiIcon v-else-if="props.state === 'error'" name="error" size="1rem" />
      <UiIcon v-else-if="props.state === 'dirty'" name="clock" size="1rem" />

      <span v-if="props.state === 'saving'">{{ t('proposal.form.autosave.saving') }}</span>
      <span v-else-if="props.state === 'saved' && savedLabel">
        {{ t('proposal.form.autosave.savedAt', { time: savedLabel }) }}
      </span>
      <span v-else-if="props.state === 'dirty'">{{ t('proposal.form.autosave.dirty') }}</span>
      <span v-else-if="props.state === 'error'">{{ t('proposal.form.autosave.error') }}</span>
      <span v-else>{{ t('proposal.form.autosave.untouched') }}</span>
    </p>

    <UiButton
      v-if="props.state === 'error'"
      variant="ghost"
      size="sm"
      icon="refresh"
      :label="t('common.actions.retry')"
      @click="emit('retry')"
    />

    <!-- Le numéro de dossier existe dès la première écriture : le trigger
         l'attribue à l'insertion. L'annoncer tout de suite donne à la personne
         de quoi parler de son dossier avant même de l'avoir envoyé. -->
    <p v-if="props.referenceCode" class="text-text-subtle">
      {{ t('proposal.form.autosave.reference') }}
      <span class="font-mono font-bold text-text-secondary">{{ props.referenceCode }}</span>
    </p>
  </div>
</template>

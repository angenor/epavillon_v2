<script setup lang="ts">
import type { ThemeChoice } from '~/stores/preferences'

/**
 * Bascule de thème — système, clair, sombre.
 *
 * FACTORISATION ANNONCÉE : les deux layouts portaient chacun leur copie de ce
 * groupe de boutons, avec leurs trois pictogrammes en double. Le layout du
 * back-office en portait la note (« le prompt A0.4 les factorisera en composant
 * d'interface ») ; c'est fait ici.
 *
 * TROIS CHOIX, PAS DEUX. « Système » n'est pas un état intermédiaire : c'est le
 * défaut, et le seul qui suive la préférence du navigateur quand elle change en
 * cours de journée. Une bascule binaire l'aurait supprimé.
 *
 * `aria-pressed` plutôt qu'un groupe de radios : ce sont trois boutons dont un
 * seul est enfoncé, et l'effet est immédiat — aucun formulaire à envoyer.
 */

interface Props {
  /** Version compacte : un seul bouton qui fait défiler les trois choix. */
  compact?: boolean
}

const props = defineProps<Props>()
const { t } = useI18n()
const preferences = usePreferencesStore()

const CHOICES: { value: ThemeChoice; labelKey: string; icon: string }[] = [
  { value: 'system', labelKey: 'nav.theme.system', icon: 'monitor' },
  { value: 'light', labelKey: 'nav.theme.light', icon: 'sun' },
  { value: 'dark', labelKey: 'nav.theme.dark', icon: 'moon' },
]

const current = computed(() => CHOICES.find((choice) => choice.value === preferences.theme) ?? CHOICES[0]!)
</script>

<template>
  <button
    v-if="props.compact"
    type="button"
    class="inline-flex size-9 items-center justify-center rounded-md border border-border text-text-muted transition-colors hover:bg-surface-hover hover:text-text"
    :title="t('nav.theme.label')"
    @click="preferences.cycleTheme()"
  >
    <span class="sr-only">{{ t('nav.theme.label') }} — {{ t(current.labelKey) }}</span>
    <UiIcon :name="current.icon" size="1.05rem" />
  </button>

  <div v-else class="flex items-center rounded-md border border-border" role="group" :aria-label="t('nav.theme.label')">
    <button
      v-for="choice in CHOICES"
      :key="choice.value"
      type="button"
      class="px-2.5 py-1.5 text-xs transition-colors first:rounded-l-md last:rounded-r-md"
      :class="
        preferences.theme === choice.value
          ? 'bg-accent-surface text-accent'
          : 'text-text-subtle hover:bg-surface-hover hover:text-text'
      "
      :aria-pressed="preferences.theme === choice.value"
      :title="t(choice.labelKey)"
      @click="preferences.setTheme(choice.value)"
    >
      <span class="sr-only">{{ t(choice.labelKey) }}</span>
      <UiIcon :name="choice.icon" size="1rem" />
    </button>
  </div>
</template>

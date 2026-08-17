<script setup lang="ts">
/**
 * EMPLACEMENT du second facteur — prévu, pas implémenté.
 *
 * QUAND CET ÉCRAN APPARAÎT. La connexion a réussi — mot de passe vérifié — et le
 * compte porte `identity.accounts.mfa_enabled_at` : l'API répond alors
 * `mfa_required` et non `authenticated`. Le jeu de données simulées contient
 * exactement un compte dans ce cas (Claire Perret), pour que la branche soit
 * VISIBLE et non seulement écrite.
 *
 * CE QUI EST RÉEL ICI : la place du champ, sa forme (six chiffres, saisie
 * numérique, `autocomplete="one-time-code"` qui déclenche la proposition du
 * code sur téléphone), et le fait que la session ne s'ouvre PAS.
 *
 * CE QUI NE L'EST PAS : la vérification du code. Elle demande le secret TOTP
 * (`mfa_secret_encrypted`, chiffré côté application), qui ne franchit jamais la
 * frontière de l'API — il n'y a rien à simuler ici qui aurait un sens.
 *
 * POURQUOI LE CHAMP EST DÉSACTIVÉ PLUTÔT QU'ABSENT. Un formulaire qui accepte
 * une saisie pour la refuser ensuite est pire qu'un champ inerte accompagné de
 * sa raison. Le message dit quoi faire en attendant : se connecter avec un autre
 * compte, ou écrire à l'assistance.
 */

const { t } = useI18n()

defineEmits<{ cancel: [] }>()
</script>

<template>
  <div>
    <UiAlert
      intent="warning"
      :title="t('auth-form.mfa.title')"
      :message="t('auth-form.mfa.notImplemented')"
      live
    />

    <div class="mt-5" aria-hidden="true">
      <UiInput
        :model-value="''"
        :label="t('auth-form.mfa.codeLabel')"
        :hint="t('auth-form.mfa.codeHint')"
        :placeholder="'••••••'"
        inputmode="numeric"
        autocomplete="one-time-code"
        :maxlength="6"
        disabled
      />
    </div>

    <UiButton
      class="mt-5"
      variant="secondary"
      block
      icon="arrow-left"
      :label="t('auth-form.mfa.back')"
      @click="$emit('cancel')"
    />
  </div>
</template>

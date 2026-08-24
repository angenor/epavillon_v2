<script setup lang="ts">
/**
 * BANDEAU DE DONNÉES D'EXEMPLE — cet écran n'est pas encore servi par l'API.
 *
 * Il ne s'affiche QUE lorsque l'API est configurée : tant qu'elle ne l'est pas,
 * toute la plateforme tourne sur les données simulées et le dire à chaque page
 * n'apprendrait rien à personne. Une fois branchée, en revanche, la différence
 * compte — et elle ne se devine pas à l'écran.
 *
 * Il nomme les routes attendues. C'est la dette, écrite là où on la subit.
 *
 * IL APPARAÎT À L'HYDRATATION, PAS AU RENDU SERVEUR. Vue rend l'arbre en une
 * passe : ce bandeau vit dans la mise en page, donc au-dessus de la page, et il
 * est évalué avant que celle-ci n'ait chargé quoi que ce soit. La liste est bien
 * dans le payload — l'écran l'a remplie ensuite —, et le bandeau se montre donc
 * dès la reprise côté navigateur. On l'accepte : le déplacer dans chaque page
 * concernée pour gagner ces quelques dizaines de millisecondes obligerait à
 * poser la même balise sur sept écrans, et à ne pas l'oublier sur le huitième.
 */
const { active, paths } = useMockData()
const { isConfigured } = useApi()
const { t } = useI18n()
</script>

<template>
  <div
    v-if="isConfigured && active"
    role="note"
    class="w-full border-b border-info-border bg-info-surface text-text"
  >
    <div class="mx-auto flex w-full max-w-[1280px] flex-col gap-1 px-4 py-3 sm:px-6">
      <p class="font-medium">{{ t('api.mockData.title') }}</p>
      <p class="text-sm text-text-secondary">
        {{ t('api.mockData.description') }}
        <span class="font-mono text-xs">{{ paths.join(' · ') }}</span>
      </p>
    </div>
  </div>
</template>

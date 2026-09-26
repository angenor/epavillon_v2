<script setup lang="ts">
/**
 * 11 · 1h — la liste « Validation », ouverte depuis Ressources. Seule la file des
 * signalements existe à cette étape : les autres files ne paraissent pas, même
 * désactivées (hors périmètre de 3b).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const k = (cle: string, params: Record<string, unknown> = {}, n?: number) =>
  n === undefined ? t(`guide-nego.validation.${cle}`, params) : t(`guide-nego.validation.${cle}`, params, n)

const compte = useGnSession()
const acces = useGnAcces()
const tentee = ref(false)

onMounted(async () => {
  await compte.assurer()
  if (compte.connectee.value) await acces.rafraichir().catch(() => undefined)
  tentee.value = true
})

const attente = computed(() => !tentee.value && !acces.pret.value)
const peutValider = computed(() => compte.connectee.value && (acces.acces.value.can_validate_reports ?? false))
const aTraiter = computed(() => acces.acces.value.reports_to_review ?? 0)
const sousTitre = computed(() => (peutValider.value ? k('liste.sous-titre', { count: aTraiter.value }, aTraiter.value) : undefined))

useHead({ title: k('liste.titre') })
</script>

<template>
  <GnEcran :titre="k('liste.titre')" :sous-titre="sousTitre" retour="/guide-nego/ressources" :onglets="false">
    <template v-if="peutValider" #action>
      <span class="gn-validation__role">
        <GnPicto nom="shield-check" :taille="20" />
        {{ k('role') }}
      </span>
    </template>

    <GnChargement v-if="attente" forme="squelette" :lignes="2" :libelle="k('chargement')" />

    <GnEtatVide
      v-else-if="!peutValider"
      picto="lock"
      :titre="k('refuse.titre')"
      :texte="k('refuse.texte')"
      :sortie="k('refuse.sortie')"
      sortie-vers="/guide-nego/ressources"
    />

    <GnLigneReglage
      v-else
      :libelle="k('liste.signalements')"
      :valeur="k('liste.signalements-detail')"
      picto="flag"
      vers="/guide-nego/validation/signalements"
      derniere
    >
      <span class="gn-validation__compteur">{{ aTraiter }}</span>
    </GnLigneReglage>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-validation__role {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-succes);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-validation__compteur {
  flex: none;
  color: var(--gn-titre);
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  font-variant-numeric: tabular-nums;
}
</style>

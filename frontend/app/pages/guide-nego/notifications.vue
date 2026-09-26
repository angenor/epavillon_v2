<script setup lang="ts">
import type { Notification } from '~/types/engagement'
import { dayKeyInZone } from '~/utils/datetime'

/**
 * Maquette 02 · 10 — le centre de notifications, par jour dans le fuseau de la COP.
 * Un toucher marque lu (par la file) puis ouvre la fiche ; « Tout marquer comme lu »
 * ne touche pas aux avis du site.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { dayLong, zoneOf } = useDateTime()
const router = useRouter()
const connexion = useGnConnexion()
const session = useGnSession()
const edition = useGnEdition()
const notifications = useGnNotifications()

const k = (cle: string, params: Record<string, unknown> = {}, n?: number) =>
  n === undefined ? t(`guide-nego.notifications.${cle}`, params) : t(`guide-nego.notifications.${cle}`, params, n)

const retour = ref('/guide-nego')
const tentee = ref(false)

async function relire(): Promise<void> {
  await notifications.rafraichir()
  tentee.value = true
}

onMounted(async () => {
  const precedent = router.options.history.state.back
  if (typeof precedent === 'string' && precedent.startsWith('/guide-nego') && !precedent.startsWith('/guide-nego/notifications')) {
    retour.value = precedent
  }
  void edition.rafraichir()
  await session.assurer()
  if (!session.connectee.value) return
  notifications.assurer()
  await relire()
})

const enLigne = computed(() => connexion.etat.value.enLigne)
const sansCompte = computed(() => session.pret.value && !session.connectee.value)
const items = computed(() => notifications.notifications.value)
const attente = computed(() => !session.pret.value || (!notifications.connu.value && !tentee.value && !items.value.length))
const nonLues = computed(() => notifications.nonLues.value)
const fuseau = computed(() => notifications.fuseau.value)

const sousTitre = computed(() => {
  if (!session.connectee.value || !notifications.connu.value || !items.value.length) return undefined
  return nonLues.value ? k('non-lues', { count: nonLues.value }, nonLues.value) : k('toutes-lues')
})

const zone = computed(() => zoneOf(edition.edition.value?.city ?? fuseau.value.split('/').pop()?.replace(/_/g, ' ') ?? ''))

function titreDuJour(jour: string): string {
  const maintenant = new Date()
  const aujourdhui = dayKeyInZone(maintenant, fuseau.value)
  const hier = dayKeyInZone(new Date(maintenant.getTime() - 86_400_000), fuseau.value)
  if (jour === aujourdhui) return k('aujourdhui')
  const long = dayLong(`${jour}T12:00:00Z`, 'UTC')
  return jour === hier ? k('hier', { jour: long }) : long
}

async function ouvrir(n: Notification): Promise<void> {
  await notifications.marquerLue(n.id)
  if (n.link_path) await navigateTo(n.link_path)
}

useHead({ title: k('titre') })
</script>

<template>
  <GnEcran :titre="k('titre')" :sous-titre="sousTitre" :retour="retour" :onglets="false">
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="notifications.luA.value" />
    </template>
    <template v-if="nonLues > 0" #sous-titre-action>
      <button type="button" class="gn-notifications__tout" @click="notifications.toutMarquer()">
        {{ k('tout-marquer') }}
      </button>
    </template>

    <GnEtatVide
      v-if="sansCompte"
      picto="user"
      :titre="k('sans-compte.titre')"
      :texte="k('sans-compte.texte')"
      :sortie="k('sans-compte.connexion')"
      sortie-vers="/guide-nego/connexion"
    />

    <GnChargement v-else-if="attente" forme="squelette" :lignes="5" :libelle="k('chargement')" />

    <GnEtatVide
      v-else-if="!notifications.connu.value && !items.length && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lu.titre')"
      :texte="k('jamais-lu.texte')"
    />

    <GnEtatErreur
      v-else-if="!notifications.connu.value && !items.length"
      :titre="k('erreur.titre')"
      :texte="k('erreur.texte')"
      :sortie="k('erreur.reessayer')"
      @sortie="relire()"
    />

    <GnEtatVide v-else-if="!items.length" picto="bell" :titre="k('vide.titre')" :texte="k('vide.texte')" />

    <template v-else>
      <section v-for="(j, rang) in notifications.parJour.value" :key="j.jour">
        <GnEnteteGroupe :titre="titreDuJour(j.jour)" :note="rang === 0 ? zone : undefined" />
        <ul class="gn-notifications__liste">
          <li v-for="(n, i) in j.notifications" :key="n.id">
            <GnLigneNotification
              :notification="n"
              :fuseau="fuseau"
              :derniere="i === j.notifications.length - 1"
              @ouvrir="ouvrir"
            />
          </li>
        </ul>
      </section>
    </template>

    <NuxtLink v-if="session.connectee.value" to="/guide-nego/ressources/reglages" class="gn-notifications__pied">
      <span>{{ k('regler') }}</span>
      <span class="gn-notifications__profil">{{ k('profil') }}</span>
    </NuxtLink>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-notifications__tout {
  min-height: var(--gn-cible);
  padding: 0;
  border: none;
  background: none;
  color: var(--gn-accent);
  font: inherit;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-notifications__pied {
  min-height: var(--gn-ligne-reglage);
  margin-top: var(--gn-espace-8);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gn-espace-12);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-notifications__profil {
  flex: none;
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>

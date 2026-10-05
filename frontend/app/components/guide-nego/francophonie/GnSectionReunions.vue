<script setup lang="ts">
import { dayKeyInZone } from '~/utils/datetime'
import { placeDuMaintenant } from '~/utils/guide-nego/sessions'

/**
 * La section Réunions de l'onglet Francophonie, en frise par jour (Nuit 02). Lecture publique, gardée pour
 * le hors connexion ; le fuseau se dit une fois au-dessus des heures (écart 47).
 */
const { t } = useI18n()
const { zoneLabel, zoneOffsetShort, dayLong, time } = useDateTime()
const connexion = useGnConnexion()
const session = useGnSession()
const lecture = useGnReunions()
const inscriptions = useGnInscriptionsReunions()
const pavillon = useGnPavillon()

const k = (cle: string, params: Record<string, unknown> = {}) => t(`guide-nego.francophonie.reunions.${cle}`, params)

const maintenant = shallowRef(new Date())
let horloge: ReturnType<typeof setInterval> | undefined
const tentee = ref(false)
const relecture = ref(false)

async function relire(): Promise<void> {
  relecture.value = true
  try {
    await lecture.rafraichir()
    // L'étiquette « Se tient aussi au Pavillon » ouvre l'activité liée : son slug vient de l'édition.
    if (lecture.reunions.value.some((r) => r.pavilion_session_id)) void pavillon.rafraichir()
  } finally {
    relecture.value = false
    tentee.value = true
  }
}

onMounted(async () => {
  horloge = setInterval(() => (maintenant.value = new Date()), 60_000)
  void relire()
  await session.assurer()
  inscriptions.assurer()
})
onBeforeUnmount(() => clearInterval(horloge))

const enLigne = computed(() => connexion.etat.value.enLigne)
const attente = computed(() => !lecture.pret.value && !tentee.value)
const lue = computed(() => lecture.etat.value.valeur !== null)
const fuseau = computed(() => lecture.fuseau.value ?? 'UTC')

const lignes = computed(() =>
  lecture.reunions.value.map((reunion) => ({
    reunion,
    etat: inscriptions.etat(reunion.id, maintenant.value) ?? 'prevue',
  })),
)

const majuscule = (texte: string) => texte.charAt(0).toLocaleUpperCase() + texte.slice(1)
const aujourdhui = computed(() => dayKeyInZone(maintenant.value, fuseau.value))
const jours = computed(() => {
  const parJour = new Map<string, typeof lignes.value>()
  for (const l of lignes.value) {
    const jour = dayKeyInZone(l.reunion.start_at, fuseau.value)
    parJour.set(jour, [...(parJour.get(jour) ?? []), l])
  }
  return [...parJour].map(([jour, du]) => ({
    jour,
    legende: majuscule(dayLong(`${jour}T12:00:00Z`, 'UTC')),
    lignes: du,
    maintenantA: jour === aujourdhui.value ? placeDuMaintenant(du.map((l) => l.reunion.start_at), maintenant.value) : null,
  }))
})
const heureDeMaintenant = computed(() => time(maintenant.value, fuseau.value))

const legende = computed(() => {
  const premiere = lecture.reunions.value[0]
  return k('fuseau', {
    zone: zoneLabel(fuseau.value, lecture.ville.value ?? undefined),
    decalage: zoneOffsetShort(fuseau.value, premiere?.start_at),
  })
})
</script>

<template>
  <div class="gn-reunions">
    <GnChargement v-if="attente" forme="squelette" :lignes="4" :libelle="k('chargement')" class="gn-reunions__attente" />

    <GnEtatVide
      v-else-if="lecture.sansEdition.value"
      picto="franco"
      :titre="k('sans-edition.titre')"
      :texte="k('sans-edition.texte')"
    />

    <GnEtatVide
      v-else-if="!lue && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lue.titre')"
      :texte="k('jamais-lue.texte')"
    />

    <GnEtatErreur
      v-else-if="!lue"
      :titre="k('erreur.titre')"
      :texte="k('erreur.texte')"
      :sortie="relecture ? undefined : k('erreur.reessayer')"
      @sortie="relire()"
    />

    <GnEtatVide
      v-else-if="!lignes.length"
      picto="franco"
      :titre="k('vide.titre')"
      :texte="k('vide.texte')"
    />

    <template v-else>
      <p class="gn-reunions__fuseau">{{ legende }}</p>
      <section v-for="j in jours" :key="j.jour" class="gn-reunions__jour">
        <h2 class="gn-reunions__legende">{{ j.legende }}</h2>
        <div class="gn-reunions__liste">
          <template v-for="(l, i) in j.lignes" :key="l.reunion.id">
            <GnFriseMaintenant v-if="j.maintenantA === i" :heure="heureDeMaintenant" />
            <GnLigneReunion
              :reunion="l.reunion"
              :etat="l.etat"
              :fuseau="fuseau"
              :ville="lecture.ville.value"
              :jour="false"
              :vers="`/guide-nego/francophonie/reunions/${l.reunion.id}`"
            />
          </template>
          <GnFriseMaintenant v-if="j.maintenantA === j.lignes.length" :heure="heureDeMaintenant" />
        </div>
      </section>
    </template>

    <i18n-t v-if="!attente" keypath="guide-nego.francophonie.reunions.pied" tag="p" class="gn-reunions__pied" scope="global">
      <template #negociations>
        <NuxtLink to="/guide-nego/negociations" class="gn-reunions__renvoi">{{ k('negociations') }}</NuxtLink>
      </template>
      <template #pavillon>
        <NuxtLink :to="{ query: { section: 'pavillon' } }" class="gn-reunions__renvoi" aria-current="false">{{ k('pavillon') }}</NuxtLink>
      </template>
    </i18n-t>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-reunions__attente {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-reunions__fuseau {
  padding-top: 18px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-reunions__jour {
  padding-top: 18px;
}

[data-app="guide-nego"] .gn-reunions__legende {
  padding-bottom: var(--gn-espace-4);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-18);
  line-height: var(--gn-interligne-18);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-reunions__fuseau::first-letter {
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-reunions__pied {
  padding-block: var(--gn-espace-16);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

/* Dans la phrase, la cible déborde en hauteur jusqu'à 48 px (ligne de 22 px + 2 × 13). */
[data-app="guide-nego"] .gn-reunions__renvoi {
  padding-block: 14px;
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>

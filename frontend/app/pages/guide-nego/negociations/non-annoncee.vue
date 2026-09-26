<script setup lang="ts">
import { dayKeyInZone } from '~/utils/datetime'
import { corpsDeLaReunion } from '~/utils/guide-nego/signalements'

/**
 * Maquette 09, 1d — signaler une réunion non annoncée, depuis le bas de la liste d'un jour.
 * « Quoi » seul est requis (FR-004) ; une seule thématique, prise dans la base, ou « Autre ».
 * L'envoi passe par la file comme un changement : sans réseau, il part au retour.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { dayLong, zoneLabel } = useDateTime()
const route = useRoute()
const compte = useGnSession()
const acces = useGnAcces()
const thematiques = useGnThematiques()
const lecture = useGnSessions()
const signalements = useGnSignalements()

const k = (cle: string, params: Record<string, unknown> = {}) => t(`guide-nego.non-annoncee.${cle}`, params)

const fuseau = computed(() => lecture.fuseau.value ?? 'UTC')
const jour = computed(() => {
  const demande = typeof route.query.jour === 'string' ? route.query.jour : ''
  return /^\d{4}-\d{2}-\d{2}$/.test(demande) ? demande : dayKeyInZone(new Date(), fuseau.value)
})
const fil = computed(() => k('fil', { jour: dayLong(`${jour.value}T12:00:00Z`, 'UTC') }))
const zone = computed(() => zoneLabel(fuseau.value, lecture.ville.value ?? undefined))

const pret = ref(false)
onMounted(async () => {
  void lecture.rafraichir()
  await compte.assurer()
  if (compte.connectee.value) {
    await acces.assurer()
    void thematiques.assurer()
  } else {
    void thematiques.assurerLeVocabulaire()
  }
  pret.value = true
})

const admise = computed(() => compte.connectee.value && acces.ouvert.value)

/** Les suivies d'abord : ce sont celles qu'on signale. */
const choixDeThematique = computed(() => {
  const miennes = new Set(thematiques.mesCodes.value)
  return [...thematiques.thematiques.value].sort((a, b) => Number(miennes.has(b.code)) - Number(miennes.has(a.code)))
})

const quoi = ref('')
const ou = ref('')
const quand = ref('')
/** `undefined` : rien de choisi ; `null` : « Autre ». */
const thematique = ref<string | null | undefined>(undefined)
const erreurQuoi = ref<string | undefined>(undefined)
const envoi = ref(false)
const refus = ref<string | null>(null)
const envoye = ref(false)

function choisir(code: string | null): void {
  thematique.value = thematique.value === code ? undefined : code
}

async function envoyer(): Promise<void> {
  if (envoi.value) return
  refus.value = null
  const corps = corpsDeLaReunion(
    { quoi: quoi.value, ou: ou.value, quand: quand.value, thematique: thematique.value ?? null },
    jour.value,
    fuseau.value,
  )
  erreurQuoi.value = corps ? undefined : k('quoi-requis')
  if (!corps) return
  envoi.value = true
  try {
    const issue = await signalements.signaler(corps)
    if (issue.issue === 'refuse') refus.value = issue.message ?? k('refuse')
    else envoye.value = true
  } finally {
    envoi.value = false
  }
}

watch(signalements.refus, (avis) => {
  if (avis?.message) refus.value = avis.message
})

const versLaListe = '/guide-nego/negociations'

useHead({ title: k('titre') })
</script>

<template>
  <GnEcran :titre="k('titre')" :sous-titre="fil" :retour="versLaListe" :onglets="false">
    <GnChargement v-if="!pret" forme="squelette" :lignes="5" :libelle="k('chargement')" class="gn-non-annoncee__attente" />

    <GnVerrou
      v-else-if="!admise"
      :propos="k('verrou.propos')"
      :reste-ouvert="k('verrou.reste')"
      :retour="k('verrou.retour')"
      :retour-vers="versLaListe"
    />

    <div v-else-if="envoye" class="gn-non-annoncee__envoye">
      <GnEtatVide
        picto="send"
        :titre="k('envoye.titre')"
        :texte="k('envoye.texte')"
        :sortie="k('envoye.voir')"
        sortie-vers="/guide-nego/negociations/signalements?depuis=sessions"
      />
      <GnBouton variante="discret" :vers="versLaListe">{{ k('envoye.retour') }}</GnBouton>
    </div>

    <form v-else class="gn-non-annoncee" novalidate @submit.prevent="envoyer">
      <p class="gn-non-annoncee__propos">{{ k('propos') }}</p>

      <GnChamp v-model="quoi" :libelle="k('quoi')" :erreur="erreurQuoi" required autocomplete="off" />

      <div class="gn-non-annoncee__rangee">
        <GnChamp v-model="ou" :libelle="k('ou')" autocomplete="off" class="gn-non-annoncee__ou" />
        <GnChamp v-model="quand" type="time" :libelle="k('quand')" />
      </div>
      <p class="gn-non-annoncee__zone">{{ k('quand-aide', { zone }) }}</p>

      <fieldset class="gn-non-annoncee__thematiques">
        <legend class="gn-non-annoncee__legende">{{ k('thematique') }}</legend>
        <div class="gn-non-annoncee__pilules">
          <GnPilule
            v-for="th in choixDeThematique"
            :key="th.code"
            :choisie="thematique === th.code"
            @clic="choisir(th.code)"
          >
            {{ thematiques.nomDe(th.code) }}
          </GnPilule>
          <GnPilule :choisie="thematique === null" @clic="choisir(null)">{{ k('autre') }}</GnPilule>
        </div>
      </fieldset>

      <p class="gn-non-annoncee__bouclier">
        <GnPicto nom="shield-check" :taille="20" />
        <span>{{ k('bouclier') }}</span>
      </p>

      <p v-if="refus" class="gn-non-annoncee__refus" role="alert">
        <GnPicto nom="warn" :taille="20" />
        {{ refus }}
      </p>

      <div class="gn-non-annoncee__actions">
        <GnBouton type="submit" picto="send" :chargement="envoi">{{ k('envoyer') }}</GnBouton>
        <GnBouton variante="discret" :vers="versLaListe">{{ k('annuler') }}</GnBouton>
      </div>
    </form>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-non-annoncee__attente {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-ecran__contenu:has(> .gn-non-annoncee) {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-non-annoncee {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-non-annoncee__propos,
[data-app="guide-nego"] .gn-non-annoncee__zone,
[data-app="guide-nego"] .gn-non-annoncee__bouclier {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-non-annoncee__zone {
  margin-top: calc(-1 * var(--gn-espace-8));
  text-align: end;
}

[data-app="guide-nego"] .gn-non-annoncee__rangee {
  display: flex;
  gap: var(--gn-espace-8);
  align-items: flex-start;
}

[data-app="guide-nego"] .gn-non-annoncee__ou {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-non-annoncee__thematiques {
  margin: 0;
  padding: 0;
  border: none;
  min-width: 0;
}

[data-app="guide-nego"] .gn-non-annoncee__legende {
  padding: 0;
  margin-bottom: var(--gn-espace-8);
  color: var(--gn-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-non-annoncee__pilules {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-non-annoncee__bouclier,
[data-app="guide-nego"] .gn-non-annoncee__refus {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-non-annoncee__bouclier .gn-picto,
[data-app="guide-nego"] .gn-non-annoncee__refus .gn-picto {
  flex: none;
}

[data-app="guide-nego"] .gn-non-annoncee__refus {
  color: var(--gn-danger);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-non-annoncee__actions {
  margin-top: auto;
  padding-block: var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-non-annoncee__envoye {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}
</style>

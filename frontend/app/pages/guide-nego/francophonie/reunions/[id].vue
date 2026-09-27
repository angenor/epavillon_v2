<script setup lang="ts">
import type { FrancophoneMeeting } from '~/types/negotiation-meetings'
import type { IssueDeLInscription } from '~/composables/guide-nego/useGnInscriptionsReunions'
import { dayKeyInZone } from '~/utils/datetime'
import { PREFIXE_FILE_INSCRIPTION_REUNION } from '~/utils/guide-nego/reunions'
import { cheminDeLActivite } from '~/utils/guide-nego/pavillon'
import { adresseDeLaSortie, sortieDuVerrou } from '~/utils/guide-nego/verrou'

/**
 * Écran 10 · 1b — la fiche d'une réunion de la Francophonie, lue dans la liste gardée :
 * elle s'ouvre sans réseau avec ce qui a été lu. Le lien de visio ne vient que des
 * inscriptions de la personne (R6) ; la lecture publique n'en porte jamais.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { dayLong, time, zoneLabel, zoneOffsetShort } = useDateTime()
const { tr } = useI18nText()
const route = useRoute()
const connexion = useGnConnexion()
const compte = useGnSession()
const lecture = useGnReunions()
const inscriptions = useGnInscriptionsReunions()
const acces = useGnAcces()
const pavillon = useGnPavillon()

const k = (cle: string, params: Record<string, unknown> = {}) => t(`guide-nego.francophonie-reunion.${cle}`, params)

const id = computed(() => String(route.params.id ?? ''))
const retour = '/guide-nego/francophonie'

const maintenant = shallowRef(new Date())
let horloge: ReturnType<typeof setInterval> | undefined
const tentee = ref(false)
const relecture = ref(false)

async function relire(): Promise<void> {
  relecture.value = true
  try {
    await lecture.rafraichir()
    if (reunion.value?.pavilion_session_id) void pavillon.rafraichir()
  } finally {
    relecture.value = false
    tentee.value = true
  }
}

onMounted(async () => {
  horloge = setInterval(() => (maintenant.value = new Date()), 60_000)
  void relire()
  await compte.assurer()
  inscriptions.assurer()
  if (compte.connectee.value) void acces.assurer()
})
onBeforeUnmount(() => clearInterval(horloge))

const enLigne = computed(() => connexion.etat.value.enLigne)
const attente = computed(() => !lecture.pret.value && !tentee.value)
const lue = computed(() => lecture.etat.value.valeur !== null)
const reunion = computed<FrancophoneMeeting | null>(() => lecture.reunion(id.value))
const fuseau = computed(() => lecture.fuseau.value ?? 'UTC')
const ville = computed(() => lecture.ville.value ?? undefined)

const etat = computed(() => inscriptions.etat(id.value, maintenant.value))
const annulee = computed(() => etat.value === 'annulee')
const titre = computed(() => (reunion.value ? tr(reunion.value.title) : k('titre')))

const majuscule = (texte: string) => texte.charAt(0).toLocaleUpperCase() + texte.slice(1)

const heure = computed(() => {
  const r = reunion.value
  if (!r) return null
  const f = fuseau.value
  const debut = time(r.start_at, f)
  const fin = time(r.end_at, f)
  const jour = dayLong(r.start_at, f)
  const valeur =
    dayKeyInZone(r.start_at, f) === dayKeyInZone(r.end_at, f)
      ? k('heure', { jour, debut, fin })
      : k('heure-deux-jours', { jour, debut, jourFin: dayLong(r.end_at, f), fin })
  return {
    valeur: majuscule(valeur),
    precision: k('fuseau', { zone: zoneLabel(f, ville.value), decalage: zoneOffsetShort(f, r.start_at) }),
  }
})

const lieu = computed(() => {
  const r = reunion.value
  if (!r) return null
  if (r.format === 'online') return { valeur: k('en-ligne'), precision: null }
  return {
    valeur: r.venue ?? k('lieu-inconnu'),
    precision: r.format === 'hybrid' ? k('aussi-en-ligne') : null,
  }
})

const versPavillon = computed(() => {
  const slug = pavillon.slugDeLActiviteLiee(reunion.value?.pavilion_session_id ?? null)
  return slug ? cheminDeLActivite(slug) : undefined
})

const lien = computed(() => inscriptions.lienVisio(id.value))
const accesLimite = computed(() => {
  const r = reunion.value
  if (!r || r.open_access) return null
  const public_ = r.access_audience ? tr(r.access_audience) : ''
  return public_ ? k('acces-limite', { public: public_ }) : t('gn-marque-etat.acces-limite')
})
const description = computed(() => (reunion.value?.description ? tr(reunion.value.description) : null))

// --- S'inscrire (FR-008 à FR-010) -------------------------------------------

const bouton = computed(() => inscriptions.bouton(id.value, maintenant.value))
const aEnvoyer = computed(() => inscriptions.aEnvoyer(id.value))
const lecturePendante = computed(() => compte.connectee.value && !inscriptions.pret.value)
const envoi = ref(false)
const refus = ref<string | null>(null)
const feuilleReservee = ref(false)
const sortieReservee = computed(() => sortieDuVerrou(acces.acces.value, compte.connectee.value))

const message = ref<{ texte: string; action?: string; agir?: () => void; rang: number } | null>(null)
const annoncer = (texte: string, action?: string, agir?: () => void) =>
  (message.value = { texte, action, agir, rang: (message.value?.rang ?? 0) + 1 })

const libelleAttente = (position: number | null) =>
  position ? k('inscription.attente-position', { position }) : t('gn-marque-etat.liste-attente')

function dire(issue: IssueDeLInscription): void {
  if (issue.issue === 'refusee') {
    message.value = null
    refus.value = issue.message ?? k('inscription.refusee')
  } else if (issue.issue === 'inscrite') annoncer(k('inscription.faite'))
  else if (issue.issue === 'liste-attente') annoncer(libelleAttente(issue.position))
  else if (issue.issue === 'en-attente') annoncer(k('inscription.en-file'))
}

async function envoyer(geste: () => Promise<IssueDeLInscription>): Promise<IssueDeLInscription> {
  envoi.value = true
  refus.value = null
  try {
    return await geste()
  } finally {
    envoi.value = false
  }
}

async function inscrire(): Promise<void> {
  if (compte.connectee.value) await acces.assurer()
  if (!compte.connectee.value || !acces.ouvert.value) {
    feuilleReservee.value = true
    return
  }
  dire(await envoyer(() => inscriptions.inscrire(id.value)))
}

async function desinscrire(): Promise<void> {
  const quitteLAttente = bouton.value?.libelle === 'liste-attente'
  const issue = await envoyer(() => inscriptions.desinscrire(id.value))
  if (issue.issue === 'refusee') return dire(issue)
  const cle = issue.issue === 'en-attente' ? 'en-file' : quitteLAttente ? 'attente-quittee' : 'faite'
  annoncer(k(`desinscription.${cle}`), k('desinscription.annuler'), () => {
    message.value = null
    void envoyer(() => inscriptions.inscrire(id.value)).then(dire)
  })
}

// Un geste reparti de la file et refusé se dit ici ; le layout se tait sur cette fiche.
watch(inscriptions.refus, (avis) => {
  if (avis?.message && avis.cle === `${PREFIXE_FILE_INSCRIPTION_REUNION}${id.value}`) refus.value = avis.message
})

useHead({ title: titre })
</script>

<template>
  <GnEcran
    :titre="titre"
    :surtitre="reunion ? k('origine') : undefined"
    :retour="retour"
    :onglets="false"
    :ce-qui-se-lit="k('hors-connexion')"
  >
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="lecture.luA.value" />
    </template>

    <GnChargement v-if="attente" forme="squelette" :lignes="6" :libelle="k('chargement')" class="gn-reunion__attente" />

    <GnEtatVide
      v-else-if="lecture.sansEdition.value"
      picto="franco"
      :titre="k('sans-edition.titre')"
      :texte="k('sans-edition.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnEtatVide
      v-else-if="!lue && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lue.titre')"
      :texte="k('jamais-lue.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnEtatErreur
      v-else-if="!lue"
      :titre="k('erreur.titre')"
      :texte="k('erreur.texte')"
      :sortie="relecture ? undefined : k('erreur.reessayer')"
      @sortie="relire()"
    />

    <GnEtatVide
      v-else-if="!reunion"
      picto="franco"
      :titre="k('inconnue.titre')"
      :texte="k('inconnue.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <div v-else class="gn-reunion" :class="{ 'gn-reunion--annulee': annulee }">
      <div class="gn-reunion__etat">
        <GnMarqueEtat v-if="etat" :etat="etat" />
        <p v-if="annulee && reunion.cancellation_reason" class="gn-reunion__motif">
          {{ k('annulee-motif', { motif: reunion.cancellation_reason }) }}
        </p>
      </div>

      <dl class="gn-reunion__lignes">
        <div v-if="heure" class="gn-reunion__ligne">
          <dt>{{ k('cle.heure') }}</dt>
          <dd class="gn-reunion__double">
            <span class="gn-reunion__valeur">{{ heure.valeur }}</span>
            <span class="gn-reunion__precision">{{ heure.precision }}</span>
          </dd>
        </div>
        <div v-if="lieu" class="gn-reunion__ligne">
          <dt>{{ k('cle.lieu') }}</dt>
          <dd class="gn-reunion__double">
            <span class="gn-reunion__lieu">{{ lieu.valeur }}</span>
            <span v-if="lieu.precision" class="gn-reunion__precision">{{ lieu.precision }}</span>
          </dd>
        </div>
        <div v-if="lien || reunion.has_video" class="gn-reunion__ligne">
          <dt>{{ k('cle.visio') }}</dt>
          <dd>
            <a v-if="lien" :href="lien" class="gn-reunion__visio" target="_blank" rel="noopener">
              <GnPicto nom="external" :taille="20" />
              {{ k('rejoindre') }}
            </a>
            <span v-else class="gn-reunion__reserve">
              <GnPicto nom="lock" :taille="18" />
              {{ k('reserve') }}
            </span>
          </dd>
        </div>
        <div class="gn-reunion__ligne">
          <dt>{{ k('cle.organisateur') }}</dt>
          <dd class="gn-reunion__simple">{{ reunion.organizer }}</dd>
        </div>
        <div class="gn-reunion__ligne">
          <dt>{{ k('cle.nature') }}</dt>
          <dd class="gn-reunion__simple">{{ tr(reunion.type.label) }}</dd>
        </div>
        <div v-if="accesLimite" class="gn-reunion__ligne">
          <dt>{{ k('cle.acces') }}</dt>
          <dd><GnMarqueEtat etat="acces-limite" :libelle="accesLimite" /></dd>
        </div>
      </dl>

      <GnEtiquettePavillon v-if="reunion.pavilion_session_id" :vers="versPavillon" class="gn-reunion__pavillon" />

      <p v-if="description" class="gn-reunion__description">{{ description }}</p>

      <div v-if="bouton" class="gn-reunion__actions">
        <GnBouton
          v-if="bouton.libelle === 'inscrire'"
          :chargement="envoi || lecturePendante"
          @clic="inscrire"
        >
          {{ k('inscription.inscrire') }}
        </GnBouton>
        <GnBouton
          v-else-if="bouton.libelle === 'inscrite'"
          variante="secondaire"
          picto="check"
          actif
          :desactive="!bouton.geste"
          :chargement="envoi"
          @clic="desinscrire"
        >
          {{ t('gn-marque-etat.inscrite') }}
        </GnBouton>
        <template v-else-if="bouton.libelle === 'rejoindre-attente'">
          <GnBouton variante="secondaire" picto="clock" :chargement="envoi || lecturePendante" @clic="inscrire">
            {{ k('inscription.rejoindre-attente') }}
          </GnBouton>
        </template>
        <template v-else-if="bouton.libelle === 'liste-attente'">
          <GnMarqueEtat etat="liste-attente" :libelle="libelleAttente(bouton.position)" />
          <GnBouton v-if="bouton.geste" variante="secondaire" :chargement="envoi" @clic="desinscrire">
            {{ k('inscription.quitter-attente') }}
          </GnBouton>
        </template>
        <GnMarqueEtat v-else-if="bouton.libelle === 'complet'" etat="complet" :libelle="k('inscription.complet')" />
        <p v-else class="gn-reunion__closes">
          <GnPicto nom="lock" :taille="18" />
          {{ k('inscription.closes') }}
        </p>

        <GnMarqueEtat v-if="aEnvoyer" etat="a-envoyer" :libelle="k('inscription.a-envoyer')" />
        <p v-if="refus" class="gn-reunion__refus" role="alert">
          <GnPicto nom="warn" :taille="20" />
          {{ refus }}
        </p>
      </div>
    </div>

    <GnFeuilleBasse
      v-model="feuilleReservee"
      :titre="t('gn-verrou.titre')"
      :sous-titre="k('inscription.reserve')"
      :fermeture="k('inscription.fermer')"
    >
      <div class="gn-reunion__sorties">
        <GnBouton :vers="adresseDeLaSortie(sortieReservee)" :picto="sortieReservee === 'compte' ? 'user' : 'lock'">
          {{ t(`gn-verrou.sortie.${sortieReservee}`) }}
        </GnBouton>
        <GnBouton v-if="sortieReservee === 'compte'" variante="secondaire" vers="/guide-nego/connexion">
          {{ t('gn-verrou.sortie.connexion') }}
        </GnBouton>
      </div>
    </GnFeuilleBasse>

    <GnMessageEphemere
      v-if="message"
      :key="message.rang"
      :texte="message.texte"
      :action="message.action"
      @agir="message.agir?.()"
      @fini="message = null"
    />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-ecran:has(.gn-reunion--annulee) .gn-entete__titre {
  text-decoration: line-through;
  text-decoration-thickness: 2px;
}

[data-app="guide-nego"] .gn-reunion__attente {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-ecran__contenu:has(> .gn-reunion) {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-reunion {
  flex: 1;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-reunion:not(:has(.gn-reunion__actions)) {
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-reunion__etat {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  padding: var(--gn-espace-12) 0 10px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-reunion__etat .gn-marque-etat {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-reunion__motif {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-reunion__ligne {
  display: flex;
  justify-content: space-between;
  gap: var(--gn-espace-12);
  padding-block: 10px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-reunion__ligne dt {
  flex: none;
  padding-top: 2px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-reunion__ligne dd {
  min-width: 0;
  display: flex;
  justify-content: flex-end;
  text-align: end;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-reunion__ligne dd.gn-reunion__double {
  flex-direction: column;
  align-items: flex-end;
}

[data-app="guide-nego"] .gn-reunion__simple,
[data-app="guide-nego"] .gn-reunion__valeur {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-reunion__precision {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-reunion__lieu {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-accent);
}

/* La ligne reste basse, la cible déborde en hauteur jusqu'à 48 px. */
[data-app="guide-nego"] .gn-reunion__visio {
  min-height: var(--gn-cible);
  margin-block: -13px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-reunion__reserve {
  display: inline-flex;
  align-items: flex-start;
  gap: 6px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-reunion__reserve .gn-picto {
  flex: none;
  margin-block-start: 2px;
}

[data-app="guide-nego"] .gn-reunion__pavillon {
  margin-top: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-reunion__description {
  padding-top: var(--gn-espace-8);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  white-space: pre-line;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-reunion__actions {
  position: sticky;
  bottom: 0;
  margin-top: auto;
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding: var(--gn-espace-16) var(--gn-marge-ecran) calc(var(--gn-espace-16) + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
  background: var(--gn-fond);
}

[data-app="guide-nego"] .gn-reunion__closes,
[data-app="guide-nego"] .gn-reunion__refus {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-reunion__closes {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-reunion__refus {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-reunion__closes .gn-picto,
[data-app="guide-nego"] .gn-reunion__refus .gn-picto {
  flex: none;
  margin-block-start: 1px;
}

[data-app="guide-nego"] .gn-reunion__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-reunion--annulee .gn-reunion__lieu {
  color: var(--gn-texte-2);
}
</style>

<script setup lang="ts">
import { libelleDuTerme, marquesDeLigne } from '~/utils/guide-nego/documents'
import { favorisConnus, placeDesCopies, surLeTelephone } from '~/utils/guide-nego/mes-documents'
import { tailleLisible } from '~/utils/guide-nego/place'

/**
 * « Mes documents » (06a, 06b) — ce qui est gardé sur ce téléphone, sa place, et les
 * favoris du compte. S'ouvre sans compte : les téléchargements n'en demandent pas.
 *
 * « Tout retirer » n'efface que les copies, jamais les lectures qui font marcher
 * l'application sans réseau (FR-032). L'accès refusé est sans objet : rien ici n'est
 * réservé à un compte, sauf les favoris, qui invitent à en créer un.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const session = useGnSession()
const connexion = useGnConnexion()
const copies = useGnCopies()
const { etat, rafraichir, bibliotheque, documents, nouveaux } = useGnDocuments()
const { favoris, pret: favorisPrets, connus: favorisConnusIci, assurer: assurerLesFavoris } = useGnFavoris()
const { place, mesuree, mesurer } = useGnPlace()
const { momentLisible } = useGnMomentLecture()

const copiesLues = ref(false)

// Venue du profil, on y revient ; sinon aux Ressources.
const retour = ref('/guide-nego/ressources')

onMounted(async () => {
  if (window.history.state?.back === '/guide-nego/ressources/reglages') retour.value = '/guide-nego/ressources/reglages'
  void rafraichir()
  void mesurer()
  // Les copies sont sur le téléphone : elles n'attendent pas la réponse du compte.
  void copies
    .recharger()
    .catch(() => undefined)
    .finally(() => (copiesLues.value = true))
  await session.assurer()
  assurerLesFavoris()
})

const connectee = computed(() => session.connectee.value)
const enLigne = computed(() => connexion.etat.value.enLigne)
const types = computed(() => bibliotheque.value?.vocabulary.types ?? [])
const telecharges = computed(() => new Set(copies.copies.value.map((c) => c.id)))

const chargement = computed(() => !copiesLues.value || !etat.value.pret)
// Sans la bibliothèque, une copie n'a pas de titre ; sans copie, rien ne manque.
const illisible = computed(() => !chargement.value && !bibliotheque.value && copies.copies.value.length > 0)
// Des favoris sans la bibliothèque n'ont pas de titre : « aucun » serait faux.
const favorisLisibles = computed(() => connectee.value && favorisPrets.value && favorisConnusIci.value && !!bibliotheque.value)

function ligneDe(document: (typeof documents.value)[number]) {
  return {
    type: libelleDuTerme(types.value, document.type) ?? undefined,
    marques: marquesDeLigne(document, {
      telecharge: telecharges.value.has(document.id),
      nouveau: nouveaux.value.has(document.id),
      enLigne: enLigne.value,
    }),
    vers: `/guide-nego/ressources/documents/${document.id}`,
  }
}

const gardes = computed(() => surLeTelephone(copies.copies.value, documents.value))
const nombreDeCopies = computed(() => copies.copies.value.length)
const placeGardee = computed(() => placeDesCopies(copies.copies.value))
/** Dépubliées depuis la dernière lecture : sans titre, elles gardent leur place jusqu'au rapprochement. */
const orphelines = computed(() => {
  const nommees = new Set(gardes.value.map((g) => g.copie.id))
  return copies.copies.value.filter((c) => !nommees.has(c.id))
})
const titresEnAttente = computed(() => {
  const ids = new Set([...copies.enAttente.value, ...Object.keys(copies.progressions.value)])
  return documents.value.filter((d) => ids.has(d.id)).map((d) => d.title)
})
const mesFavoris = computed(() => (connectee.value ? favorisConnus(favoris.value, documents.value) : []))

const sousTitre = computed(() => {
  if (chargement.value) return undefined
  const n = nombreDeCopies.value
  const telecharges = t('guide-nego.mes-documents.resume.telecharges', { count: n }, n)
  if (!favorisLisibles.value) return telecharges
  const f = mesFavoris.value.length
  return `${telecharges} · ${t('guide-nego.mes-documents.resume.favoris', { count: f }, f)}`
})

const confirmation = ref(false)
const retraitEnCours = ref(false)
const message = ref<{ texte: string; rang: number } | null>(null)

const liste = (morceaux: string[]) => new Intl.ListFormat(locale.value, { type: 'conjunction' }).format(morceaux)

const phraseDeConfirmation = computed(() => {
  const suffixe = connectee.value ? '' : '-sans-compte'
  const noms = gardes.value.map(({ document, copie }) =>
    t('guide-nego.mes-documents.tout-retirer.document', {
      titre: document.title,
      taille: tailleLisible(copie.octets, locale.value),
    }),
  )
  const o = orphelines.value.length
  if (o) {
    const taille = tailleLisible(placeDesCopies(orphelines.value), locale.value)
    noms.push(t('guide-nego.mes-documents.tout-retirer.orphelines', { count: o, taille }, o))
  }
  const phrase =
    nombreDeCopies.value === 1 && !o
      ? t(`guide-nego.mes-documents.tout-retirer.un${suffixe}`, { document: noms[0] })
      : t(`guide-nego.mes-documents.tout-retirer.plusieurs${suffixe}`, {
          documents: liste(noms),
          taille: tailleLisible(placeGardee.value, locale.value),
        })
  const a = titresEnAttente.value.length
  if (!a) return phrase
  return `${phrase} ${t('guide-nego.mes-documents.tout-retirer.attente', { documents: liste(titresEnAttente.value) }, a)}`
})

async function toutRetirer(): Promise<void> {
  const n = nombreDeCopies.value
  retraitEnCours.value = true
  try {
    const reussi = await copies.toutRetirer()
    await mesurer()
    const texte = reussi
      ? t('guide-nego.mes-documents.tout-retirer.fait', { count: n }, n)
      : t('guide-nego.mes-documents.tout-retirer.echec')
    message.value = { texte, rang: (message.value?.rang ?? 0) + 1 }
  } finally {
    retraitEnCours.value = false
  }
}

useHead({ title: t('guide-nego.mes-documents.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.mes-documents.titre')" :sous-titre="sousTitre" :retour="retour">
    <GnChargement v-if="chargement" forme="squelette" :lignes="4" :libelle="t('guide-nego.mes-documents.chargement')" />

    <GnEtatErreur
      v-else-if="illisible"
      :titre="t('guide-nego.mes-documents.illisible.titre')"
      :texte="t(enLigne ? 'guide-nego.mes-documents.illisible.texte-en-ligne' : 'guide-nego.mes-documents.illisible.texte')"
      :sortie="t('guide-nego.mes-documents.illisible.sortie')"
      sortie-vers="/guide-nego/ressources/documents"
    />

    <template v-else>
      <section>
        <GnEnteteGroupe
          :titre="t('guide-nego.mes-documents.telephone.titre')"
          :note="nombreDeCopies ? tailleLisible(placeGardee, locale) : undefined"
        />
        <ul v-if="gardes.length" role="list">
          <li v-for="{ document, copie } in gardes" :key="document.id">
            <GnLigneDocument
              :document="document"
              v-bind="ligneDe(document)"
              :morceaux="['pages', 'taille']"
              :octets="copie.octets"
              :precision="t('guide-nego.mes-documents.telephone.garde', { moment: momentLisible(copie.gardee_a) })"
            />
          </li>
        </ul>
        <p v-if="orphelines.length" class="gn-mes-documents__texte">
          {{ t('guide-nego.mes-documents.telephone.orphelines', { count: orphelines.length }, orphelines.length) }}
        </p>
        <template v-if="!nombreDeCopies">
          <p class="gn-mes-documents__texte">{{ t('guide-nego.mes-documents.telephone.vide') }}</p>
          <GnBouton variante="secondaire" picto="doc" vers="/guide-nego/ressources/documents">
            {{ t('guide-nego.mes-documents.telephone.vers-la-bibliotheque') }}
          </GnBouton>
        </template>

        <div class="gn-mes-documents__place">
          <GnChargement v-if="!mesuree" forme="squelette" :lignes="1" :libelle="t('guide-nego.mes-documents.telephone.mesure')" />
          <!-- Le navigateur ne mesure pas : on le dit, jamais un zéro faux (R8). -->
          <GnLigneInformation v-else-if="!place" :texte="t('guide-nego.mes-documents.telephone.impossible')" />
          <GnJauge v-else :place="place" />
          <GnLigneInformation
            v-if="nombreDeCopies && copies.persistance.value === 'refusee'"
            :texte="t('guide-nego.mes-documents.telephone.persistance-refusee')"
          />
        </div>
      </section>

      <section>
        <GnEnteteGroupe
          :titre="t('guide-nego.mes-documents.favoris.titre')"
          :compteur="favorisLisibles ? mesFavoris.length : undefined"
        />
        <template v-if="!connectee">
          <p class="gn-mes-documents__texte">{{ t('guide-nego.mes-documents.favoris.sans-compte') }}</p>
          <div class="gn-mes-documents__sorties">
            <GnBouton variante="principal" vers="/guide-nego/compte">
              {{ t('guide-nego.mes-documents.favoris.creer') }}
            </GnBouton>
            <GnBouton variante="discret" vers="/guide-nego/connexion">
              {{ t('guide-nego.mes-documents.favoris.connexion') }}
            </GnBouton>
          </div>
        </template>
        <GnChargement v-else-if="!favorisPrets" forme="squelette" :lignes="2" :libelle="t('guide-nego.mes-documents.chargement')" />
        <p v-else-if="!favorisLisibles" class="gn-mes-documents__texte">{{ t('guide-nego.mes-documents.favoris.pas-encore-lus') }}</p>
        <template v-else-if="mesFavoris.length">
          <ul role="list">
            <li v-for="document in mesFavoris" :key="document.id">
              <GnLigneDocument :document="document" v-bind="ligneDe(document)" />
            </li>
          </ul>
          <p class="gn-mes-documents__texte">{{ t('guide-nego.mes-documents.favoris.rappel') }}</p>
        </template>
        <p v-else class="gn-mes-documents__texte">{{ t('guide-nego.mes-documents.favoris.vide') }}</p>
      </section>

      <!-- Sans copie, le geste n'aurait aucun effet : il ne s'offre pas. -->
      <div v-if="nombreDeCopies" class="gn-mes-documents__retirer">
        <GnBouton variante="dangereux" picto="warn" :chargement="retraitEnCours" @clic="confirmation = true">
          {{ t('guide-nego.mes-documents.tout-retirer.bouton') }}
        </GnBouton>
        <p class="gn-mes-documents__reste">
          {{ t(connectee ? 'guide-nego.mes-documents.tout-retirer.reste' : 'guide-nego.mes-documents.tout-retirer.reste-sans-compte') }}
        </p>
      </div>
    </template>

    <GnConfirmation
      v-model="confirmation"
      :question="t('guide-nego.mes-documents.tout-retirer.question', { count: nombreDeCopies }, nombreDeCopies)"
      :phrase="phraseDeConfirmation"
      :action="t('guide-nego.mes-documents.tout-retirer.action')"
      dangereuse
      @confirmer="toutRetirer"
    />
    <GnMessageEphemere v-if="message" :key="message.rang" :texte="message.texte" @fini="message = null" />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-mes-documents__place {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-mes-documents__texte {
  padding-block: var(--gn-espace-12);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
  max-width: var(--gn-mesure-lecture);
}

[data-app="guide-nego"] .gn-mes-documents__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-mes-documents__retirer {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  margin-top: var(--gn-espace-16);
  padding-block: var(--gn-espace-16);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-mes-documents__reste {
  text-align: center;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}
</style>
